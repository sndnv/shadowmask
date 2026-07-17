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
    pub added_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl VersionResponse {
    pub fn with_path(version: Version, include_path: bool) -> Self {
        let path = include_path.then(|| version.path.clone());
        let mut response = VersionResponse::from(version);
        response.path = path;
        response
    }
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
            added_at: v.added_at.to_string(),
            updated_at: v.updated_at.to_string(),
            path: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId, VersionId};
    use domain::common::Quality;
    use domain::library::LibraryId;
    use jiff::Timestamp;

    fn version() -> Version {
        Version {
            id: VersionId("v1".into()),
            title: TitleId::Movie(MovieId("m1".into())),
            library: LibraryId("lib1".into()),
            quality: Quality::Fhd,
            container: "mkv".into(),
            path: "/media/v1.mkv".into(),
            size_bytes: 1,
            duration_ms: 1000,
            edition: None,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    #[test]
    fn from_omits_path() {
        let response = VersionResponse::from(version());
        assert_eq!(response.path, None);
    }

    #[test]
    fn with_path_omits_path_for_non_admin() {
        let response = VersionResponse::with_path(version(), false);
        assert_eq!(response.path, None);
        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("path"));
        assert!(!json.contains("/media/v1.mkv"));
    }

    #[test]
    fn with_path_includes_path_for_admin() {
        let response = VersionResponse::with_path(version(), true);
        assert_eq!(response.path.as_deref(), Some("/media/v1.mkv"));
    }
}
