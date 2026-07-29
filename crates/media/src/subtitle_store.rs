use std::path::PathBuf;

use domain::catalog::VersionId;
use domain::error::SubtitleError;
use domain::media::{SubtitleFormat, SubtitleReader, SubtitleStore};

#[derive(Debug, Clone)]
pub struct FsSubtitleStore {
    root: PathBuf,
}

impl FsSubtitleStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl SubtitleStore for FsSubtitleStore {
    async fn store(
        &self,
        version: &VersionId,
        file_id: &str,
        format: SubtitleFormat,
        content: &str,
    ) -> Result<String, SubtitleError> {
        let dir = self.root.join(&version.0);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| SubtitleError::Store(e.to_string()))?;
        let path = dir.join(format!("{file_id}.{}", format.extension()));
        tokio::fs::write(&path, content)
            .await
            .map_err(|e| SubtitleError::Store(e.to_string()))?;
        Ok(path.to_string_lossy().into_owned())
    }

    async fn remove(&self, path: &str) -> Result<(), SubtitleError> {
        match tokio::fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(SubtitleError::Store(e.to_string())),
        }
    }
}

impl SubtitleReader for FsSubtitleStore {
    async fn load(&self, path: &str) -> Result<String, SubtitleError> {
        tokio::fs::read_to_string(path)
            .await
            .map_err(|e| SubtitleError::Backend(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn store_creates_dir_and_writes_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsSubtitleStore::new(dir.path());
        let version = VersionId("v1".to_owned());

        let path = store
            .store(&version, "file-42", SubtitleFormat::Srt, "1\nhello\n")
            .await
            .expect("store");

        let expected = dir.path().join("v1").join("file-42.srt");
        assert_eq!(PathBuf::from(&path), expected);
        assert_eq!(
            std::fs::read_to_string(&expected).expect("read"),
            "1\nhello\n"
        );
    }

    #[tokio::test]
    async fn store_on_unwritable_root_is_store_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("not-a-dir");
        std::fs::write(&file, b"x").expect("write blocker file");
        let store = FsSubtitleStore::new(&file);

        let err = store
            .store(
                &VersionId("v1".to_owned()),
                "file-42",
                SubtitleFormat::Vtt,
                "WEBVTT\n",
            )
            .await
            .expect_err("store under a file path must fail");
        assert!(matches!(err, SubtitleError::Store(_)));
    }

    #[tokio::test]
    async fn load_returns_stored_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsSubtitleStore::new(dir.path());
        let path = store
            .store(
                &VersionId("v1".to_owned()),
                "file-42",
                SubtitleFormat::Vtt,
                "WEBVTT\n\nhello\n",
            )
            .await
            .expect("store");

        let content = store.load(&path).await.expect("load");
        assert_eq!(content, "WEBVTT\n\nhello\n");
    }

    #[tokio::test]
    async fn load_missing_file_is_backend_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsSubtitleStore::new(dir.path());
        let missing = dir.path().join("nope.vtt");

        let err = store
            .load(&missing.to_string_lossy())
            .await
            .expect_err("missing file must fail");
        assert!(matches!(err, SubtitleError::Backend(_)));
    }

    #[tokio::test]
    async fn remove_deletes_the_file_and_missing_is_ok() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsSubtitleStore::new(dir.path());
        let path = store
            .store(
                &VersionId("v1".to_owned()),
                "file-42",
                SubtitleFormat::Vtt,
                "WEBVTT\n",
            )
            .await
            .expect("store");

        store.remove(&path).await.expect("remove");
        assert!(!PathBuf::from(&path).exists());
        store
            .remove(&path)
            .await
            .expect("removing a missing file is ok");
    }

    #[tokio::test]
    async fn remove_on_a_directory_is_store_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsSubtitleStore::new(dir.path());
        let err = store
            .remove(&dir.path().to_string_lossy())
            .await
            .expect_err("removing a directory must fail");
        assert!(matches!(err, SubtitleError::Store(_)));
    }
}
