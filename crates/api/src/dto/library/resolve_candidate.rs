use serde::Serialize;

use domain::library::{ResolveCandidate, ResolveTarget};
use domain::metadata::MediaKind;

use crate::dto::common::TitleRefDto;

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResolveTargetDto {
    Existing { title: TitleRefDto },
    Provider { source: String, value: String },
}

impl From<ResolveTarget> for ResolveTargetDto {
    fn from(target: ResolveTarget) -> Self {
        match target {
            ResolveTarget::Existing(title) => ResolveTargetDto::Existing { title: title.into() },
            ResolveTarget::Provider(id) => {
                ResolveTargetDto::Provider { source: id.source, value: id.value }
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ResolveCandidateResponse {
    pub source: &'static str,
    pub target: ResolveTargetDto,
    pub title: String,
    pub year: Option<u16>,
    pub kind: &'static str,
}

impl From<ResolveCandidate> for ResolveCandidateResponse {
    fn from(candidate: ResolveCandidate) -> Self {
        let source = match candidate.target {
            ResolveTarget::Existing(_) => "catalog",
            ResolveTarget::Provider(_) => "provider",
        };
        let kind = match candidate.kind {
            MediaKind::Movie => "movie",
            MediaKind::Series => "series",
        };
        ResolveCandidateResponse {
            source,
            target: candidate.target.into(),
            title: candidate.title,
            year: candidate.year,
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId};
    use domain::metadata::ExternalId;

    #[test]
    fn an_existing_target_is_sourced_from_the_catalog() {
        let response = ResolveCandidateResponse::from(ResolveCandidate {
            target: ResolveTarget::Existing(TitleId::Movie(MovieId("m1".into()))),
            title: "The Matrix".into(),
            year: Some(1999),
            kind: MediaKind::Movie,
        });

        assert_eq!(response.source, "catalog");
        assert_eq!(response.kind, "movie");
        assert_eq!(response.title, "The Matrix");
        assert_eq!(response.year, Some(1999));
        assert_eq!(
            serde_json::to_value(&response.target).unwrap(),
            serde_json::json!({"kind": "existing", "title": {"type": "movie", "id": "m1"}})
        );
    }

    #[test]
    fn a_provider_target_carries_the_external_id_through() {
        let response = ResolveCandidateResponse::from(ResolveCandidate {
            target: ResolveTarget::Provider(ExternalId {
                source: "tmdb".into(),
                value: "tv/63639".into(),
            }),
            title: "The Expanse".into(),
            year: None,
            kind: MediaKind::Series,
        });

        assert_eq!(response.source, "provider");
        assert_eq!(response.kind, "series");
        assert_eq!(response.year, None);
        assert_eq!(
            serde_json::to_value(&response.target).unwrap(),
            serde_json::json!({"kind": "provider", "source": "tmdb", "value": "tv/63639"})
        );
    }
}
