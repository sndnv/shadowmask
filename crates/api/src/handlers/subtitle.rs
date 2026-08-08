use std::collections::HashSet;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::debug;

use domain::catalog::VersionId;
use domain::common::LanguageCode;
use domain::error::SubtitleError;
use domain::media::{
    SubtitleFile, SubtitleFileId, SubtitleProvider, SubtitleQuery, SubtitleReader, SubtitleSource,
    SubtitleStore, prune_orphaned_translations,
};
use domain::repository::CatalogRepository;

use crate::dto::catalog::{
    DownloadSubtitleRequest, RenameSubtitleRequest, SubtitleCandidateDto, SubtitleTextResponse,
};
use crate::error::{ApiError, ApiResult};
use crate::extract::RequireAdmin;
use crate::state::{SubtitleSearchState, SubtitleState};

fn is_managed(source: SubtitleSource) -> bool {
    matches!(
        source,
        SubtitleSource::OpenSubtitles
            | SubtitleSource::Generated
            | SubtitleSource::MachineTranslated
            | SubtitleSource::Combined
    )
}

fn cleaned(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[derive(Debug, Deserialize)]
pub struct SubtitleSearchParams {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub season: Option<u16>,
    #[serde(default)]
    pub episode: Option<u16>,
}

pub async fn search<C, S, P>(
    State(state): State<SubtitleSearchState<C, S, P>>,
    RequireAdmin(principal): RequireAdmin,
    Path(version): Path<String>,
    Query(params): Query<SubtitleSearchParams>,
) -> ApiResult<Json<Vec<SubtitleCandidateDto>>>
where
    C: CatalogRepository + Send + Sync,
    S: Send + Sync,
    P: SubtitleProvider + Send + Sync,
{
    let actor = &principal.user.0;
    let version = VersionId(version);
    state
        .catalog
        .version_detail(&version)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(|| ApiError::not_found("version not found"))?;
    let languages = cleaned(params.language)
        .map(|value| vec![LanguageCode(value)])
        .unwrap_or_default();
    let query = SubtitleQuery {
        imdb_id: None,
        query: cleaned(params.q),
        languages,
        season: params.season,
        episode: params.episode,
    };
    let mut candidates = match state.provider.search(&query).await {
        Ok(candidates) => candidates,
        Err(SubtitleError::NotFound) => Vec::new(),
        Err(err) => {
            debug!("subtitle search failed for version [{}]: {err}", version.0);
            return Err(ApiError::bad_gateway("subtitle provider error"));
        }
    };
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.download_count));
    debug!(
        "user [{actor}] searched subtitles for version [{}] ({} candidate(s))",
        version.0,
        candidates.len()
    );
    Ok(Json(
        candidates
            .into_iter()
            .map(SubtitleCandidateDto::from)
            .collect(),
    ))
}

pub async fn download<C, S, P>(
    State(state): State<SubtitleSearchState<C, S, P>>,
    RequireAdmin(principal): RequireAdmin,
    Path(version): Path<String>,
    Json(request): Json<DownloadSubtitleRequest>,
) -> ApiResult<StatusCode>
where
    C: CatalogRepository + Send + Sync,
    S: SubtitleStore + Send + Sync,
    P: SubtitleProvider + Send + Sync,
{
    let actor = &principal.user.0;
    let version = VersionId(version);
    let detail = state
        .catalog
        .version_detail(&version)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(|| ApiError::not_found("version not found"))?;
    let file_id = request.file_id.trim();
    if file_id.is_empty() {
        return Err(ApiError::bad_request("file_id is required"));
    }
    let fetched = match state.provider.download(file_id).await {
        Ok(fetched) => fetched,
        Err(SubtitleError::NotFound) => return Err(ApiError::not_found("subtitle not found")),
        Err(err) => {
            debug!(
                "subtitle download failed for version [{}]: {err}",
                version.0
            );
            return Err(ApiError::bad_gateway("subtitle provider error"));
        }
    };
    let path = state
        .subtitles
        .store(&version, file_id, fetched.format, &fetched.content)
        .await
        .map_err(|_| ApiError::internal())?;
    let subtitle = SubtitleFile {
        id: SubtitleFileId(format!("opensubtitles:{}:{}", version.0, file_id)),
        version: version.clone(),
        language: cleaned(request.language).map(LanguageCode),
        format: fetched.format,
        source: SubtitleSource::OpenSubtitles,
        path,
        translated_from: None,
    };
    let mut merged: Vec<SubtitleFile> = detail
        .subtitle_files
        .into_iter()
        .filter(|file| file.id != subtitle.id)
        .collect();
    merged.push(subtitle);
    state
        .catalog
        .set_subtitle_files(&version, &merged)
        .await
        .map_err(|_| ApiError::internal())?;
    debug!(
        "user [{actor}] downloaded subtitle [{file_id}] for version [{}]",
        version.0
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn view<C, S>(
    State(state): State<SubtitleState<C, S>>,
    RequireAdmin(principal): RequireAdmin,
    Path((version, subtitle)): Path<(String, String)>,
) -> ApiResult<Json<SubtitleTextResponse>>
where
    C: CatalogRepository + Send + Sync,
    S: SubtitleReader + Send + Sync,
{
    let actor = &principal.user.0;
    let version = VersionId(version);
    let detail = state
        .catalog
        .version_detail(&version)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(|| ApiError::not_found("version not found"))?;
    let file = detail
        .subtitle_files
        .iter()
        .find(|file| file.id.0 == subtitle)
        .ok_or_else(|| ApiError::not_found("subtitle not found"))?;
    let content = state
        .subtitles
        .load(&file.path)
        .await
        .map_err(|_| ApiError::internal())?;
    debug!(
        "user [{actor}] read subtitle [{subtitle}] of version [{}]",
        version.0
    );
    Ok(Json(SubtitleTextResponse { content }))
}

pub async fn rename<C, S>(
    State(state): State<SubtitleState<C, S>>,
    RequireAdmin(principal): RequireAdmin,
    Path((version, subtitle)): Path<(String, String)>,
    Json(request): Json<RenameSubtitleRequest>,
) -> ApiResult<StatusCode>
where
    C: CatalogRepository + Send + Sync,
    S: Send + Sync,
{
    let actor = &principal.user.0;
    let version = VersionId(version);
    let detail = state
        .catalog
        .version_detail(&version)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(|| ApiError::not_found("version not found"))?;
    let target = detail
        .subtitle_files
        .iter()
        .find(|file| file.id.0 == subtitle)
        .ok_or_else(|| ApiError::not_found("subtitle not found"))?;
    if !is_managed(target.source) {
        return Err(ApiError::forbidden(
            "only downloaded or generated subtitles can be renamed",
        ));
    }
    let language = request
        .language
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| LanguageCode(value.to_owned()));
    let updated: Vec<SubtitleFile> = detail
        .subtitle_files
        .iter()
        .cloned()
        .map(|mut file| {
            if file.id.0 == subtitle {
                file.language = language.clone();
            }
            file
        })
        .collect();
    state
        .catalog
        .set_subtitle_files(&version, &updated)
        .await
        .map_err(|_| ApiError::internal())?;
    debug!(
        "user [{actor}] renamed subtitle [{subtitle}] of version [{}]",
        version.0
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete<C, S>(
    State(state): State<SubtitleState<C, S>>,
    RequireAdmin(principal): RequireAdmin,
    Path((version, subtitle)): Path<(String, String)>,
) -> ApiResult<StatusCode>
where
    C: CatalogRepository + Send + Sync,
    S: SubtitleStore + Send + Sync,
{
    let actor = &principal.user.0;
    let version = VersionId(version);
    let detail = state
        .catalog
        .version_detail(&version)
        .await
        .map_err(|_| ApiError::internal())?
        .ok_or_else(|| ApiError::not_found("version not found"))?;
    let target = detail
        .subtitle_files
        .iter()
        .find(|file| file.id.0 == subtitle)
        .ok_or_else(|| ApiError::not_found("subtitle not found"))?;
    if !is_managed(target.source) {
        return Err(ApiError::forbidden(
            "only downloaded or generated subtitles can be deleted",
        ));
    }

    let remaining: Vec<_> = detail
        .subtitle_files
        .iter()
        .filter(|file| file.id.0 != subtitle)
        .cloned()
        .collect();
    let kept = prune_orphaned_translations(remaining);
    let kept_ids: HashSet<&str> = kept.iter().map(|file| file.id.0.as_str()).collect();
    let removed_paths: Vec<String> = detail
        .subtitle_files
        .iter()
        .filter(|file| !kept_ids.contains(file.id.0.as_str()))
        .map(|file| file.path.clone())
        .collect();

    state
        .catalog
        .set_subtitle_files(&version, &kept)
        .await
        .map_err(|_| ApiError::internal())?;
    for path in &removed_paths {
        if let Err(err) = state.subtitles.remove(path).await {
            debug!("could not remove subtitle file [{path}]: {err}");
        }
    }
    debug!(
        "user [{actor}] deleted subtitle [{subtitle}] of version [{}]",
        version.0
    );
    Ok(StatusCode::NO_CONTENT)
}
