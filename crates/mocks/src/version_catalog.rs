use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use domain::catalog::{Version, VersionDetail, VersionId};
use domain::error::RepositoryError;
use domain::repository::VersionCatalog;

pub const PREPARED_KEYFRAMES: [u64; 3] = [0, 4_000_000, 8_000_000];

#[derive(Clone, Default)]
pub struct MockVersionCatalog {
    versions: Arc<Mutex<HashMap<VersionId, VersionDetail>>>,
    keyframes: Arc<Mutex<HashMap<VersionId, Vec<u64>>>>,
    fail: Arc<AtomicBool>,
    detail_lookups: Arc<AtomicUsize>,
}

impl MockVersionCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, detail: VersionDetail) {
        self.set_keyframes(&detail.version.id, Some(PREPARED_KEYFRAMES.to_vec()));
        self.versions.lock().unwrap().insert(detail.version.id.clone(), detail);
    }

    pub fn set_keyframes(&self, id: &VersionId, keyframes: Option<Vec<u64>>) {
        let mut stored = self.keyframes.lock().unwrap();
        match keyframes {
            Some(keyframes) => stored.insert(id.clone(), keyframes),
            None => stored.remove(id),
        };
    }

    pub fn remove(&self, id: &VersionId) {
        self.versions.lock().unwrap().remove(id);
        self.keyframes.lock().unwrap().remove(id);
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }

    pub fn detail_lookup_count(&self) -> usize {
        self.detail_lookups.load(Ordering::Relaxed)
    }

    fn guard(&self) -> Result<(), RepositoryError> {
        if self.fail.load(Ordering::Relaxed) {
            Err(RepositoryError::Backend("mock version catalog failure".to_owned()))
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
        self.detail_lookups.fetch_add(1, Ordering::Relaxed);
        self.guard()?;
        Ok(self.versions.lock().unwrap().get(id).cloned())
    }

    async fn get_version(&self, id: &VersionId) -> Result<Option<Version>, RepositoryError> {
        self.guard()?;
        Ok(self.versions.lock().unwrap().get(id).map(|detail| detail.version.clone()))
    }

    async fn keyframes_us(&self, id: &VersionId) -> Result<Option<Vec<u64>>, RepositoryError> {
        self.guard()?;
        Ok(self.keyframes.lock().unwrap().get(id).cloned())
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
            extracted_subtitles: Vec::new(),
        }
    }

    #[tokio::test]
    async fn insert_get_remove() {
        let catalog = MockVersionCatalog::new();
        assert!(catalog.version_detail(&VersionId("v1".into())).await.unwrap().is_none());
        catalog.insert(detail("v1"));
        assert!(catalog.version_detail(&VersionId("v1".into())).await.unwrap().is_some());
        catalog.remove(&VersionId("v1".into()));
        assert!(catalog.version_detail(&VersionId("v1".into())).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn surfaces_backend_failure() {
        let catalog = MockVersionCatalog::new();
        catalog.set_fail();
        assert!(catalog.version_detail(&VersionId("v1".into())).await.is_err());
        assert!(catalog.keyframes_us(&VersionId("v1".into())).await.is_err());
    }

    #[tokio::test]
    async fn an_inserted_version_is_prepared_until_its_keyframes_are_cleared() {
        let catalog = MockVersionCatalog::new();
        let id = VersionId("v1".into());
        catalog.insert(detail("v1"));
        assert_eq!(catalog.keyframes_us(&id).await.unwrap(), Some(PREPARED_KEYFRAMES.to_vec()));

        catalog.set_keyframes(&id, None);
        assert_eq!(catalog.keyframes_us(&id).await.unwrap(), None);

        catalog.set_keyframes(&id, Some(vec![0, 2_000_000]));
        catalog.remove(&id);
        assert_eq!(catalog.keyframes_us(&id).await.unwrap(), None);
    }
}
