use std::path::PathBuf;

use domain::catalog::VersionId;
use domain::error::SubtitleError;
use domain::media::{SubtitleFormat, SubtitleStore};

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
}
