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
            ResolveTarget::Existing(title) => ResolveTargetDto::Existing {
                title: title.into(),
            },
            ResolveTarget::Provider(id) => ResolveTargetDto::Provider {
                source: id.source,
                value: id.value,
            },
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
