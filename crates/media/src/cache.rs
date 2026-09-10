use std::fs::{DirEntry, Metadata};
use std::io;
use std::path::{Path, PathBuf};

use domain::error::CacheError;
use domain::media::TranscodeCacheMaintenance;
use jiff::Timestamp;
use walkdir::WalkDir;

pub struct CacheEntry {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub last_access: Timestamp,
}

pub fn plan_eviction(entries: &[CacheEntry], max_bytes: u64) -> Vec<PathBuf> {
    let total: u64 = entries.iter().map(|e| e.size_bytes).sum();
    if total <= max_bytes {
        return Vec::new();
    }
    let mut ordered: Vec<&CacheEntry> = entries.iter().collect();
    ordered.sort_by_key(|e| e.last_access);
    let mut remaining = total;
    let mut victims = Vec::new();
    for entry in ordered {
        if remaining <= max_bytes {
            break;
        }
        remaining -= entry.size_bytes;
        victims.push(entry.path.clone());
    }
    victims
}

#[derive(Clone)]
pub struct CacheEvictor {
    cache_root: PathBuf,
}

impl CacheEvictor {
    pub fn new(cache_root: impl Into<PathBuf>) -> Self {
        Self {
            cache_root: cache_root.into(),
        }
    }

    pub fn evict_blocking(&self, max_bytes: u64) -> io::Result<usize> {
        let entries = self.scan()?;
        let victims = plan_eviction(&entries, max_bytes);
        let count = victims.len();
        for path in victims {
            remove(&path)?;
        }
        Ok(count)
    }

    pub fn purge_blocking(&self) -> io::Result<usize> {
        let entries = self.entries()?;
        for entry in &entries {
            remove(&entry.path())?;
        }
        Ok(entries.len())
    }

    pub async fn purge(&self) -> Result<u64, CacheError> {
        self.off_runtime(CacheEvictor::purge_blocking).await
    }

    async fn off_runtime(
        &self,
        work: impl FnOnce(&CacheEvictor) -> io::Result<usize> + Send + 'static,
    ) -> Result<u64, CacheError> {
        let evictor = self.clone();
        tokio::task::spawn_blocking(move || work(&evictor))
            .await
            .map_err(|e| CacheError::Io(e.to_string()))?
            .map(|count| count as u64)
            .map_err(|e| CacheError::Io(e.to_string()))
    }

    fn entries(&self) -> io::Result<Vec<DirEntry>> {
        match std::fs::read_dir(&self.cache_root) {
            Ok(read) => read.collect(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    fn scan(&self) -> io::Result<Vec<CacheEntry>> {
        let mut entries = Vec::new();
        for entry in self.entries()? {
            let path = entry.path();
            let size_bytes = dir_size(&path);
            let last_access = access_time(&entry.metadata()?);
            entries.push(CacheEntry {
                path,
                size_bytes,
                last_access,
            });
        }
        Ok(entries)
    }
}

impl TranscodeCacheMaintenance for CacheEvictor {
    async fn evict(&self, max_bytes: u64) -> Result<u64, CacheError> {
        self.off_runtime(move |evictor| evictor.evict_blocking(max_bytes))
            .await
    }
}

fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

fn access_time(meta: &Metadata) -> Timestamp {
    meta.accessed()
        .or_else(|_| meta.modified())
        .ok()
        .and_then(|t| Timestamp::try_from(t).ok())
        .unwrap_or(Timestamp::UNIX_EPOCH)
}

fn remove(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::SignedDuration;

    fn entry(path: &str, size_bytes: u64, seconds: i64) -> CacheEntry {
        CacheEntry {
            path: PathBuf::from(path),
            size_bytes,
            last_access: Timestamp::UNIX_EPOCH + SignedDuration::from_secs(seconds),
        }
    }

    #[test]
    fn plan_is_empty_when_under_cap() {
        let entries = [entry("/a", 100, 1), entry("/b", 100, 2)];
        assert!(plan_eviction(&entries, 500).is_empty());
    }

    #[test]
    fn plan_evicts_oldest_first_until_under_cap() {
        let entries = [
            entry("/newest", 100, 30),
            entry("/oldest", 100, 10),
            entry("/middle", 100, 20),
        ];
        let victims = plan_eviction(&entries, 250);
        assert_eq!(victims, vec![PathBuf::from("/oldest")]);
    }

    #[test]
    fn plan_with_zero_cap_evicts_everything() {
        let entries = [entry("/a", 100, 1), entry("/b", 100, 2)];
        let victims = plan_eviction(&entries, 0);
        assert_eq!(victims.len(), 2);
    }

    fn write_file(dir: &Path, name: &str, bytes: usize) {
        std::fs::write(dir.join(name), vec![0u8; bytes]).expect("write file");
    }

    #[test]
    fn evict_missing_dir_is_noop() {
        let evictor = CacheEvictor::new("/no/such/shadowmask/cache");
        assert_eq!(evictor.evict_blocking(0).expect("evict"), 0);
    }

    #[test]
    fn evict_propagates_non_notfound_scan_error() {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        let evictor = CacheEvictor::new(file.path());
        assert!(evictor.evict_blocking(0).is_err());
    }

    #[test]
    fn evict_noop_when_under_cap() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_file(dir.path(), "a.ts", 1000);
        write_file(dir.path(), "b.ts", 1000);
        let evictor = CacheEvictor::new(dir.path());
        assert_eq!(evictor.evict_blocking(10_000).expect("evict"), 0);
        assert!(dir.path().join("a.ts").exists());
    }

    #[test]
    fn evict_zero_cap_clears_files_and_subdirs() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_file(dir.path(), "a.ts", 1000);
        let session = dir.path().join("session");
        std::fs::create_dir(&session).expect("subdir");
        write_file(&session, "seg.ts", 2000);

        let evictor = CacheEvictor::new(dir.path());
        assert_eq!(evictor.evict_blocking(0).expect("evict"), 2);
        assert!(!dir.path().join("a.ts").exists());
        assert!(!session.exists());
    }

    #[tokio::test]
    async fn maintenance_port_evicts_over_cap_off_the_runtime() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_file(dir.path(), "a.ts", 1000);
        write_file(dir.path(), "b.ts", 1000);
        let evictor = CacheEvictor::new(dir.path());
        let evicted = TranscodeCacheMaintenance::evict(&evictor, 1500)
            .await
            .expect("evict");
        assert_eq!(evicted, 1);
    }

    #[test]
    fn purge_clears_files_and_session_trees() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_file(dir.path(), "a.ts", 1000);
        let session = dir.path().join("session");
        std::fs::create_dir(&session).expect("subdir");
        write_file(&session, "seg.ts", 2000);

        let evictor = CacheEvictor::new(dir.path());
        assert_eq!(evictor.purge_blocking().expect("purge"), 2);
        assert!(!dir.path().join("a.ts").exists());
        assert!(!session.exists());
        assert!(dir.path().exists(), "the cache root itself stays");
    }

    // Eviction plans by size, so a session directory holding no files weighs
    // nothing and never gets planned. A purge is not a cap of zero.
    #[test]
    fn purge_removes_an_empty_session_directory_that_eviction_leaves_behind() {
        let dir = tempfile::tempdir().expect("tempdir");
        let session = dir.path().join("session");
        std::fs::create_dir(&session).expect("subdir");

        let evictor = CacheEvictor::new(dir.path());
        assert_eq!(evictor.evict_blocking(0).expect("evict"), 0);
        assert!(session.exists());

        assert_eq!(evictor.purge_blocking().expect("purge"), 1);
        assert!(!session.exists());
    }

    #[test]
    fn purge_missing_dir_is_noop() {
        let evictor = CacheEvictor::new("/no/such/shadowmask/cache");
        assert_eq!(evictor.purge_blocking().expect("purge"), 0);
    }

    #[test]
    fn purge_propagates_non_notfound_read_error() {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        let evictor = CacheEvictor::new(file.path());
        assert!(evictor.purge_blocking().is_err());
    }

    #[tokio::test]
    async fn purge_off_the_runtime_reports_what_it_removed() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_file(dir.path(), "a.ts", 1000);
        write_file(dir.path(), "b.ts", 1000);
        let evictor = CacheEvictor::new(dir.path());
        assert_eq!(evictor.purge().await.expect("purge"), 2);
    }

    #[tokio::test]
    async fn purge_off_the_runtime_maps_read_error() {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        let evictor = CacheEvictor::new(file.path());
        assert!(matches!(evictor.purge().await, Err(CacheError::Io(_))));
    }

    #[tokio::test]
    async fn maintenance_port_maps_scan_error() {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        let evictor = CacheEvictor::new(file.path());
        assert!(matches!(
            TranscodeCacheMaintenance::evict(&evictor, 0).await,
            Err(CacheError::Io(_))
        ));
    }
}
