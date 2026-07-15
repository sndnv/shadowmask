use serde::Serialize;

use domain::catalog::Version;

use crate::dto::common::{QualityDto, TitleRefDto};

#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub id: String,
    pub title: TitleRefDto,
    pub library_id: String,
    pub quality: QualityDto,
    pub container: String,
    pub size_bytes: u64,
    pub duration_ms: u64,
    pub edition: Option<String>,
    pub available: bool,
}

impl From<Version> for VersionResponse {
    fn from(v: Version) -> Self {
        VersionResponse {
            id: v.id.0,
            title: v.title.into(),
            library_id: v.library.0,
            quality: v.quality.into(),
            container: v.container,
            size_bytes: v.size_bytes,
            duration_ms: v.duration_ms,
            edition: v.edition,
            available: v.available,
        }
    }
}
