use std::path::PathBuf;

use domain::catalog::ArtworkId;
use domain::error::{ArtworkError, CacheError};
use domain::media::{DerivedAssetDir, DerivedAssetFile, DerivedAssetStore};
use domain::metadata::{ArtworkFormat, ArtworkStore, ProcessedArtwork};

#[derive(Debug, Clone)]
pub struct FsArtworkStore {
    root: PathBuf,
}

impl FsArtworkStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl ArtworkStore for FsArtworkStore {
    async fn store(
        &self,
        id: &ArtworkId,
        width: u32,
        art: &ProcessedArtwork,
    ) -> Result<(), ArtworkError> {
        let dir = self.root.join(&id.0);
        tokio::fs::create_dir_all(&dir).await.map_err(|e| ArtworkError::Store(e.to_string()))?;
        tokio::fs::write(dir.join(format!("{width}.{}", art.format.extension())), &art.bytes)
            .await
            .map_err(|e| ArtworkError::Store(e.to_string()))
    }

    fn path_for(&self, id: &ArtworkId, width: u32, format: ArtworkFormat) -> PathBuf {
        self.root.join(&id.0).join(format!("{width}.{}", format.extension()))
    }
}

impl DerivedAssetStore for FsArtworkStore {
    fn label(&self) -> &'static str {
        "artwork"
    }

    async fn list_dirs(&self) -> Result<Vec<DerivedAssetDir>, CacheError> {
        crate::derived_assets::list_dirs(&self.root).await
    }

    async fn remove_dir(&self, owner: &str) -> Result<(), CacheError> {
        crate::derived_assets::remove_dir(&self.root, owner).await
    }

    async fn list_files(&self, owner: &str) -> Result<Vec<DerivedAssetFile>, CacheError> {
        crate::derived_assets::list_files(&self.root, owner).await
    }

    async fn remove_file(&self, path: &str) -> Result<(), CacheError> {
        crate::derived_assets::remove_file(&self.root, path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn art(format: ArtworkFormat, bytes: &[u8]) -> ProcessedArtwork {
        ProcessedArtwork { bytes: bytes.to_vec(), width: 1, height: 1, format }
    }

    #[test]
    fn path_for_joins_id_width_and_format() {
        let store = FsArtworkStore::new("/data/artwork");
        assert_eq!(
            store.path_for(&ArtworkId("abc".into()), 480, ArtworkFormat::Jpeg),
            PathBuf::from("/data/artwork/abc/480.jpg")
        );
        assert_eq!(
            store.path_for(&ArtworkId("abc".into()), 480, ArtworkFormat::Png),
            PathBuf::from("/data/artwork/abc/480.png")
        );
    }

    #[tokio::test]
    async fn store_creates_dir_and_writes_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsArtworkStore::new(dir.path());
        let id = ArtworkId("poster-1".into());

        store.store(&id, 180, &art(ArtworkFormat::Jpeg, b"jpeg-bytes")).await.expect("store");

        let path = store.path_for(&id, 180, ArtworkFormat::Jpeg);
        assert_eq!(path, dir.path().join("poster-1").join("180.jpg"));
        assert_eq!(std::fs::read(&path).expect("read"), b"jpeg-bytes");
    }

    #[tokio::test]
    async fn a_transparent_image_keeps_the_png_extension() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsArtworkStore::new(dir.path());
        let id = ArtworkId("logo-1".into());

        store.store(&id, 180, &art(ArtworkFormat::Png, b"png-bytes")).await.expect("store");

        assert!(dir.path().join("logo-1").join("180.png").exists());
        assert!(!dir.path().join("logo-1").join("180.jpg").exists());
    }

    #[tokio::test]
    async fn store_reports_io_error() {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        let store = FsArtworkStore::new(file.path());

        let err = store
            .store(&ArtworkId("x".into()), 960, &art(ArtworkFormat::Jpeg, b"bytes"))
            .await
            .expect_err("io error");
        assert!(matches!(err, ArtworkError::Store(_)));
    }

    #[tokio::test]
    async fn the_sweep_sees_one_dir_per_artwork_id_and_can_remove_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsArtworkStore::new(dir.path());
        assert_eq!(DerivedAssetStore::label(&store), "artwork");
        store
            .store(&ArtworkId("poster-1".into()), 180, &art(ArtworkFormat::Jpeg, b"jpeg-bytes"))
            .await
            .expect("store");

        let dirs = store.list_dirs().await.expect("list");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].owner, "poster-1");

        let files = store.list_files("poster-1").await.expect("list files");
        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("180.jpg"));
        store.remove_file(&files[0].path).await.expect("remove file");
        assert!(store.list_files("poster-1").await.expect("list files").is_empty());

        store.remove_dir("poster-1").await.expect("remove");
        assert!(store.list_dirs().await.expect("list").is_empty());
    }
}
