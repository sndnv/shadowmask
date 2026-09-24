use std::path::PathBuf;

use domain::catalog::VersionId;
use domain::error::{CacheError, TranscodeError};
use domain::media::{DerivedAssetDir, DerivedAssetFile, DerivedAssetStore};
use domain::session::SoftSubtitleSource;
use sha2::{Digest, Sha256};

const IDENTITY_LEN: usize = 16;

#[derive(Debug, Clone)]
pub struct FsSubtitleExtractionStore {
    root: PathBuf,
}

#[derive(Debug)]
pub(crate) struct ExtractionEntry {
    pub(crate) dir: PathBuf,
    pub(crate) path: PathBuf,
    pub(crate) prefix: String,
}

impl FsSubtitleExtractionStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub(crate) async fn entry(
        &self,
        version: &VersionId,
        input_path: &str,
        source: &SoftSubtitleSource,
    ) -> Result<ExtractionEntry, TranscodeError> {
        if !crate::derived_assets::is_plain_name(&version.0) {
            #[rustfmt::skip]
            return Err(TranscodeError::Spawn(format!("refusing to cache a subtitle outside the store for version [{}]", version.0)));
        }
        let read = match source {
            SoftSubtitleSource::Embedded(_) => input_path,
            SoftSubtitleSource::File(path) => path.as_str(),
        };
        let prefix = match source {
            SoftSubtitleSource::Embedded(index) => format!("embedded-{index}"),
            SoftSubtitleSource::File(path) => format!("file-{}", digest(path.as_bytes())),
        };
        let (size, modified) = match tokio::fs::metadata(read).await {
            Ok(meta) => (meta.len(), meta.modified().ok()),
            Err(_) => (0, None),
        };
        let modified = modified
            .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |since| since.as_secs());
        let identity = digest(format!("{read}|{prefix}|{size}|{modified}").as_bytes());
        let dir = self.root.join(&version.0);
        let path = dir.join(format!("{prefix}-{identity}.vtt"));
        Ok(ExtractionEntry { dir, path, prefix })
    }

    pub(crate) async fn prune_siblings(&self, entry: &ExtractionEntry) {
        let Ok(mut entries) = tokio::fs::read_dir(&entry.dir).await else {
            return;
        };
        let lead = format!("{}-", entry.prefix);
        while let Ok(Some(found)) = entries.next_entry().await {
            let path = found.path();
            if path == entry.path {
                continue;
            }
            let Some(name) = found.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if name.starts_with(&lead) && name.ends_with(".vtt") {
                let _ = tokio::fs::remove_file(&path).await;
            }
        }
    }
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().take(IDENTITY_LEN / 2).map(|byte| format!("{byte:02x}")).collect()
}

impl DerivedAssetStore for FsSubtitleExtractionStore {
    fn label(&self) -> &'static str {
        "subtitle extraction"
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

    fn store() -> (tempfile::TempDir, FsSubtitleExtractionStore) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsSubtitleExtractionStore::new(dir.path());
        (dir, store)
    }

    fn version() -> VersionId {
        VersionId("ver-1".to_owned())
    }

    #[tokio::test]
    async fn an_embedded_track_is_named_after_its_index() {
        let (dir, store) = store();
        let entry = store
            .entry(&version(), "/media/movie.mkv", &SoftSubtitleSource::Embedded(3))
            .await
            .expect("entry");
        assert_eq!(entry.dir, dir.path().join("ver-1"));
        assert_eq!(entry.prefix, "embedded-3");
        let name = entry.path.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("embedded-3-"), "{name}");
        assert!(name.ends_with(".vtt"), "{name}");
    }

    #[tokio::test]
    async fn two_tracks_of_one_version_never_collide() {
        let (_dir, store) = store();
        let third = store
            .entry(&version(), "/media/movie.mkv", &SoftSubtitleSource::Embedded(3))
            .await
            .expect("entry");
        let fourth = store
            .entry(&version(), "/media/movie.mkv", &SoftSubtitleSource::Embedded(4))
            .await
            .expect("entry");
        let sidecar = store
            .entry(
                &version(),
                "/media/movie.mkv",
                &SoftSubtitleSource::File("/media/movie.en.srt".to_owned()),
            )
            .await
            .expect("entry");
        assert_ne!(third.path, fourth.path);
        assert_ne!(third.path, sidecar.path);
        assert_eq!(third.dir, sidecar.dir);
    }

    #[tokio::test]
    async fn two_sidecars_with_the_same_name_in_different_directories_differ() {
        let (_dir, store) = store();
        let one = store
            .entry(
                &version(),
                "/media/movie.mkv",
                &SoftSubtitleSource::File("/media/a/movie.en.srt".to_owned()),
            )
            .await
            .expect("entry");
        let two = store
            .entry(
                &version(),
                "/media/movie.mkv",
                &SoftSubtitleSource::File("/media/b/movie.en.srt".to_owned()),
            )
            .await
            .expect("entry");
        assert_ne!(one.path, two.path);
    }

    #[tokio::test]
    async fn a_file_that_changed_in_place_gets_a_new_name() {
        let (_dir, store) = store();
        let media = tempfile::tempdir().expect("tempdir");
        let path = media.path().join("movie.en.srt");
        std::fs::write(&path, b"one").expect("write");
        let before = store
            .entry(
                &version(),
                "/media/movie.mkv",
                &SoftSubtitleSource::File(path.to_string_lossy().into_owned()),
            )
            .await
            .expect("entry");
        std::fs::write(&path, b"a longer subtitle file").expect("write");
        let after = store
            .entry(
                &version(),
                "/media/movie.mkv",
                &SoftSubtitleSource::File(path.to_string_lossy().into_owned()),
            )
            .await
            .expect("entry");
        assert_eq!(before.prefix, after.prefix);
        assert_ne!(before.path, after.path);
    }

    #[tokio::test]
    async fn a_missing_file_still_yields_an_entry() {
        let (_dir, store) = store();
        let entry = store
            .entry(&version(), "/media/gone.mkv", &SoftSubtitleSource::Embedded(0))
            .await
            .expect("entry");
        assert!(entry.path.to_string_lossy().contains("embedded-0-"));
    }

    #[tokio::test]
    async fn a_version_that_is_not_a_plain_name_is_refused() {
        let (_dir, store) = store();
        let err = store
            .entry(
                &VersionId("../escape".to_owned()),
                "/media/movie.mkv",
                &SoftSubtitleSource::Embedded(0),
            )
            .await
            .expect_err("refused");
        assert!(err.to_string().contains("outside the store"));
    }

    #[tokio::test]
    async fn pruning_removes_the_earlier_extract_of_the_same_track() {
        let (_dir, store) = store();
        let entry = store
            .entry(&version(), "/media/movie.mkv", &SoftSubtitleSource::Embedded(3))
            .await
            .expect("entry");
        std::fs::create_dir_all(&entry.dir).expect("dir");
        std::fs::write(&entry.path, b"WEBVTT").expect("write");
        let stale = entry.dir.join("embedded-3-0000000000000000.vtt");
        let other = entry.dir.join("embedded-4-0000000000000000.vtt");
        let in_flight = entry.dir.join("embedded-3-0000000000000000.vtt.s9-1.part");
        std::fs::write(&stale, b"WEBVTT").expect("write");
        std::fs::write(&other, b"WEBVTT").expect("write");
        std::fs::write(&in_flight, b"WEBVTT").expect("write");
        store.prune_siblings(&entry).await;
        assert!(entry.path.exists());
        assert!(!stale.exists());
        assert!(other.exists());
        #[rustfmt::skip]
        assert!(in_flight.exists(), "pruning must not pull another session's extraction out from under it");
    }

    #[tokio::test]
    async fn pruning_a_directory_that_is_not_there_is_a_no_op() {
        let (_dir, store) = store();
        let entry = store
            .entry(&version(), "/media/movie.mkv", &SoftSubtitleSource::Embedded(3))
            .await
            .expect("entry");
        store.prune_siblings(&entry).await;
        assert!(!entry.dir.exists());
    }

    #[tokio::test]
    async fn the_store_lists_and_removes_by_version() {
        let (dir, store) = store();
        let owned = dir.path().join("ver-1");
        std::fs::create_dir_all(&owned).expect("dir");
        std::fs::write(owned.join("embedded-3-abc.vtt"), b"WEBVTT").expect("write");
        assert_eq!(store.label(), "subtitle extraction");
        let dirs = store.list_dirs().await.expect("dirs");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].owner, "ver-1");
        let files = store.list_files("ver-1").await.expect("files");
        assert_eq!(files.len(), 1);
        store.remove_file(&files[0].path).await.expect("remove file");
        assert!(store.list_files("ver-1").await.expect("files").is_empty());
        store.remove_dir("ver-1").await.expect("remove dir");
        assert!(!owned.exists());
    }
}
