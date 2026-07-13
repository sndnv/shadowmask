use std::path::PathBuf;

use domain::catalog::ArtworkId;
use domain::error::ArtworkError;
use domain::metadata::ArtworkStore;

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
    async fn store(&self, id: &ArtworkId, width: u32, bytes: &[u8]) -> Result<(), ArtworkError> {
        let dir = self.root.join(&id.0);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| ArtworkError::Store(e.to_string()))?;
        tokio::fs::write(dir.join(format!("{width}.png")), bytes)
            .await
            .map_err(|e| ArtworkError::Store(e.to_string()))
    }

    fn path_for(&self, id: &ArtworkId, width: u32) -> PathBuf {
        self.root.join(&id.0).join(format!("{width}.png"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_for_joins_id_and_width() {
        let store = FsArtworkStore::new("/data/artwork");
        assert_eq!(
            store.path_for(&ArtworkId("abc".into()), 480),
            PathBuf::from("/data/artwork/abc/480.png")
        );
    }

    #[tokio::test]
    async fn store_creates_dir_and_writes_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsArtworkStore::new(dir.path());
        let id = ArtworkId("poster-1".into());

        store.store(&id, 180, b"png-bytes").await.expect("store");

        let path = store.path_for(&id, 180);
        assert_eq!(path, dir.path().join("poster-1").join("180.png"));
        assert_eq!(std::fs::read(&path).expect("read"), b"png-bytes");
    }

    #[tokio::test]
    async fn store_reports_io_error() {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        let store = FsArtworkStore::new(file.path());

        let err = store
            .store(&ArtworkId("x".into()), 960, b"bytes")
            .await
            .expect_err("io error");
        assert!(matches!(err, ArtworkError::Store(_)));
    }
}
