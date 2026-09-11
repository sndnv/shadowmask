use std::path::PathBuf;

use domain::catalog::VersionId;
use domain::error::{CacheError, SubtitleError};
use domain::media::{
    DerivedAssetDir, DerivedAssetFile, DerivedAssetStore, SubtitleFormat, SubtitleReader,
    SubtitleStore,
};

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
        if !crate::derived_assets::is_plain_name(&version.0)
            || !crate::derived_assets::is_plain_name(file_id)
        {
            return Err(SubtitleError::Store(
                "refusing to write a subtitle outside the store".to_owned(),
            ));
        }
        let dir = self.root.join(&version.0);
        tokio::fs::create_dir_all(&dir).await.map_err(|e| SubtitleError::Store(e.to_string()))?;
        let path = dir.join(format!("{file_id}.{}", format.extension()));
        tokio::fs::write(&path, content).await.map_err(|e| SubtitleError::Store(e.to_string()))?;
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

impl DerivedAssetStore for FsSubtitleStore {
    fn label(&self) -> &'static str {
        "subtitles"
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

impl SubtitleReader for FsSubtitleStore {
    async fn load(&self, path: &str) -> Result<String, SubtitleError> {
        tokio::fs::read_to_string(path).await.map_err(|e| SubtitleError::Backend(e.to_string()))
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
        assert_eq!(std::fs::read_to_string(&expected).expect("read"), "1\nhello\n");
    }

    #[tokio::test]
    async fn store_on_unwritable_root_is_store_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("not-a-dir");
        std::fs::write(&file, b"x").expect("write blocker file");
        let store = FsSubtitleStore::new(&file);

        let err = store
            .store(&VersionId("v1".to_owned()), "file-42", SubtitleFormat::Vtt, "WEBVTT\n")
            .await
            .expect_err("store under a file path must fail");
        assert!(matches!(err, SubtitleError::Store(_)));
    }

    #[tokio::test]
    async fn a_traversing_file_id_or_version_never_escapes_the_store() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("subtitles");
        let store = FsSubtitleStore::new(&root);
        let outside = dir.path().join("escaped.srt");

        for (version, file_id) in
            [("v1", "../../escaped"), ("../..", "file-42"), ("v1", ".."), ("v1", "nested/file-42")]
        {
            let err = store
                .store(&VersionId(version.to_owned()), file_id, SubtitleFormat::Srt, "1\npwned\n")
                .await
                .expect_err("a path component that is not a plain name must be refused");
            assert!(matches!(err, SubtitleError::Store(_)));
        }

        assert!(!outside.exists(), "nothing is written outside the root");
        assert!(!root.exists(), "a refused write does not even create the root");
    }

    #[tokio::test]
    async fn load_returns_stored_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsSubtitleStore::new(dir.path());
        let path = store
            .store(&VersionId("v1".to_owned()), "file-42", SubtitleFormat::Vtt, "WEBVTT\n\nhello\n")
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

        let err = store.load(&missing.to_string_lossy()).await.expect_err("missing file must fail");
        assert!(matches!(err, SubtitleError::Backend(_)));
    }

    #[tokio::test]
    async fn remove_deletes_the_file_and_missing_is_ok() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsSubtitleStore::new(dir.path());
        let path = store
            .store(&VersionId("v1".to_owned()), "file-42", SubtitleFormat::Vtt, "WEBVTT\n")
            .await
            .expect("store");

        store.remove(&path).await.expect("remove");
        assert!(!PathBuf::from(&path).exists());
        store.remove(&path).await.expect("removing a missing file is ok");
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

    #[tokio::test]
    async fn the_sweep_sees_one_dir_per_version_and_can_remove_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsSubtitleStore::new(dir.path());
        assert_eq!(DerivedAssetStore::label(&store), "subtitles");
        store
            .store(&VersionId("v1".to_owned()), "file-42", SubtitleFormat::Vtt, "WEBVTT\n")
            .await
            .expect("store");

        let dirs = store.list_dirs().await.expect("list");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].owner, "v1");

        let files = store.list_files("v1").await.expect("list files");
        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("file-42.vtt"));
        store.remove_file(&files[0].path).await.expect("remove file");
        assert!(store.list_files("v1").await.expect("list files").is_empty());

        store.remove_dir("v1").await.expect("remove");
        assert!(store.list_dirs().await.expect("list").is_empty());
    }
}
