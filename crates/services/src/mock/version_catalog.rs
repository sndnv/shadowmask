use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::catalog::{VersionDetail, VersionId};
use domain::error::RepositoryError;
use domain::repository::VersionCatalog;

#[derive(Clone, Default)]
pub struct MockVersionCatalog {
    versions: Arc<Mutex<HashMap<VersionId, VersionDetail>>>,
    fail: Arc<AtomicBool>,
}

impl MockVersionCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, detail: VersionDetail) {
        self.versions
            .lock()
            .unwrap()
            .insert(detail.version.id.clone(), detail);
    }

    pub fn remove(&self, id: &VersionId) {
        self.versions.lock().unwrap().remove(id);
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }

    fn guard(&self) -> Result<(), RepositoryError> {
        if self.fail.load(Ordering::Relaxed) {
            Err(RepositoryError::Backend(
                "mock version catalog failure".to_owned(),
            ))
        } else {
            Ok(())
        }
    }
}

impl VersionCatalog for MockVersionCatalog {
    async fn version_detail(
        &self,
        id: &VersionId,
    ) -> Result<Option<VersionDetail>, RepositoryError> {
        self.guard()?;
        Ok(self.versions.lock().unwrap().get(id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId, Version};
    use domain::common::Quality;
    use domain::library::LibraryId;
    use domain::media::DetectedMarkers;
    use jiff::Timestamp;

    fn detail(id: &str) -> VersionDetail {
        VersionDetail {
            version: Version {
                id: VersionId(id.to_owned()),
                title: TitleId::Movie(MovieId("m1".to_owned())),
                library: LibraryId("lib1".to_owned()),
                quality: Quality::Sd,
                container: "mp4".to_owned(),
                path: "/media/m1.mp4".to_owned(),
                size_bytes: 1,
                duration_ms: 1000,
                edition: None,
                available: true,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
            },
            video: Vec::new(),
            audio: Vec::new(),
            subtitles: Vec::new(),
            subtitle_files: Vec::new(),
            chapters: Vec::new(),
            markers: DetectedMarkers::default(),
            trickplay: Vec::new(),
        }
    }

    #[tokio::test]
    async fn insert_get_remove() {
        let catalog = MockVersionCatalog::new();
        assert!(
            catalog
                .version_detail(&VersionId("v1".into()))
                .await
                .unwrap()
                .is_none()
        );
        catalog.insert(detail("v1"));
        assert!(
            catalog
                .version_detail(&VersionId("v1".into()))
                .await
                .unwrap()
                .is_some()
        );
        catalog.remove(&VersionId("v1".into()));
        assert!(
            catalog
                .version_detail(&VersionId("v1".into()))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn surfaces_backend_failure() {
        let catalog = MockVersionCatalog::new();
        catalog.set_fail();
        assert!(
            catalog
                .version_detail(&VersionId("v1".into()))
                .await
                .is_err()
        );
    }
}
