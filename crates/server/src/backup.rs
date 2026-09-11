use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{ConnectOptions, Connection};
use walkdir::WalkDir;

use crate::lockfile::ServerLock;

pub const MANIFEST_NAME: &str = "manifest.json";
pub const MANIFEST_VERSION: u32 = 1;
const DB_SUBTREES: [&str; 2] = ["server", "users"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub created_at_ms: i64,
    pub databases: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
}

pub async fn snapshot(
    db_root: &Path,
    out: &Path,
    created_at_ms: i64,
) -> Result<Manifest, BackupError> {
    let databases = db_files(db_root)?;
    if databases.is_empty() {
        return Err(BackupError::Invalid(format!(
            "no databases found under {}",
            db_root.display()
        )));
    }
    let work = tempfile::tempdir()?;
    for rel in &databases {
        let dest = work.path().join(rel);
        std::fs::create_dir_all(dest.parent().unwrap_or(work.path()))?;
        vacuum_into(&db_root.join(rel), &dest).await?;
    }
    let manifest = Manifest { version: MANIFEST_VERSION, created_at_ms, databases };
    std::fs::write(work.path().join(MANIFEST_NAME), serde_json::to_vec_pretty(&manifest)?)?;
    write_archive(work.path(), &manifest, out)?;
    Ok(manifest)
}

fn db_files(db_root: &Path) -> Result<Vec<String>, BackupError> {
    let mut rels = Vec::new();
    for sub in DB_SUBTREES {
        let root = db_root.join(sub);
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&root).sort_by_file_name() {
            let entry = entry.map_err(std::io::Error::from)?;
            let path = entry.path();
            if entry.file_type().is_file()
                && path.extension().and_then(|ext| ext.to_str()) == Some("db")
            {
                let rel = path
                    .strip_prefix(db_root)
                    .map_err(|error| BackupError::Invalid(error.to_string()))?;
                rels.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    rels.sort();
    rels.dedup();
    Ok(rels)
}

async fn vacuum_into(src: &Path, dest: &Path) -> Result<(), BackupError> {
    let mut conn = SqliteConnectOptions::new()
        .filename(src)
        .create_if_missing(false)
        .journal_mode(SqliteJournalMode::Wal)
        .connect()
        .await?;
    sqlx::query("VACUUM INTO ?")
        .bind(dest.to_string_lossy().into_owned())
        .execute(&mut conn)
        .await?;
    conn.close().await?;
    Ok(())
}

fn write_archive(work: &Path, manifest: &Manifest, out: &Path) -> Result<(), BackupError> {
    let parent = out
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&parent)?;
    let tmp = tempfile::NamedTempFile::new_in(&parent)?;
    {
        let mut builder = tar::Builder::new(tmp.reopen()?);
        builder.append_path_with_name(work.join(MANIFEST_NAME), MANIFEST_NAME)?;
        for rel in &manifest.databases {
            builder.append_path_with_name(work.join(rel), rel)?;
        }
        builder.finish()?;
    }
    tmp.persist(out).map_err(|error| error.error)?;
    Ok(())
}

pub fn restore(from: &Path, db_root: &Path) -> Result<Manifest, BackupError> {
    let Some(_lock) = ServerLock::try_acquire(db_root)? else {
        return Err(BackupError::Invalid(format!(
            "{} is locked; a server appears to be running, refusing to recover",
            db_root.display()
        )));
    };
    let unpacked = tempfile::tempdir_in(db_root)?;
    tar::Archive::new(std::fs::File::open(from)?).unpack(unpacked.path())?;
    let manifest = read_manifest(unpacked.path())?;
    for rel in &manifest.databases {
        let target = checked_target(db_root, rel)?;
        let source = unpacked.path().join(rel);
        if !source.is_file() {
            return Err(BackupError::Invalid(format!(
                "manifest lists {rel} but it is missing from the archive"
            )));
        }
        std::fs::create_dir_all(target.parent().unwrap_or(db_root))?;
        remove_if_present(&target)?;
        remove_if_present(&sidecar(&target, "-wal"))?;
        remove_if_present(&sidecar(&target, "-shm"))?;
        std::fs::rename(&source, &target)?;
    }
    Ok(manifest)
}

fn read_manifest(dir: &Path) -> Result<Manifest, BackupError> {
    let path = dir.join(MANIFEST_NAME);
    if !path.is_file() {
        return Err(BackupError::Invalid(format!("archive is missing {MANIFEST_NAME}")));
    }
    let manifest: Manifest = serde_json::from_slice(&std::fs::read(&path)?)?;
    if manifest.version != MANIFEST_VERSION {
        return Err(BackupError::Invalid(format!(
            "unsupported manifest version {} (expected {MANIFEST_VERSION})",
            manifest.version
        )));
    }
    Ok(manifest)
}

fn checked_target(db_root: &Path, rel: &str) -> Result<PathBuf, BackupError> {
    let rel_path = Path::new(rel);
    if !rel_path.components().all(|component| matches!(component, Component::Normal(_))) {
        return Err(BackupError::Invalid(format!("unsafe path in manifest: {rel}")));
    }
    Ok(db_root.join(rel_path))
}

fn sidecar(db: &Path, suffix: &str) -> PathBuf {
    let mut name = db.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn remove_if_present(path: &Path) -> Result<(), BackupError> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn seed(path: &Path, rows: &[&str]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut conn = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .connect()
            .await
            .unwrap();
        sqlx::query("CREATE TABLE t (v TEXT NOT NULL)").execute(&mut conn).await.unwrap();
        for row in rows {
            sqlx::query("INSERT INTO t (v) VALUES (?)")
                .bind(*row)
                .execute(&mut conn)
                .await
                .unwrap();
        }
        conn.close().await.unwrap();
    }

    async fn rows(path: &Path) -> Vec<String> {
        let mut conn = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(false)
            .connect()
            .await
            .unwrap();
        let values = sqlx::query_scalar::<_, String>("SELECT v FROM t ORDER BY v")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        conn.close().await.unwrap();
        values
    }

    fn unpack(tar_path: &Path, into: &Path) -> Manifest {
        let mut archive = tar::Archive::new(std::fs::File::open(tar_path).unwrap());
        archive.unpack(into).unwrap();
        let bytes = std::fs::read(into.join(MANIFEST_NAME)).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn bundles_every_db_and_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        seed(&root.join("server/catalog.db"), &["a", "b"]).await;
        seed(&root.join("server/users.db"), &["u"]).await;
        seed(&root.join("users/alice/progress.db"), &["watched"]).await;

        let out = root.join("snap.tar");
        let manifest = snapshot(root, &out, 1_700_000_000_000).await.unwrap();

        assert_eq!(manifest.version, MANIFEST_VERSION);
        assert_eq!(manifest.created_at_ms, 1_700_000_000_000);
        assert_eq!(
            manifest.databases,
            vec!["server/catalog.db", "server/users.db", "users/alice/progress.db",]
        );

        let extracted = tempfile::tempdir().unwrap();
        let read = unpack(&out, extracted.path());
        assert_eq!(read, manifest);
        assert_eq!(rows(&extracted.path().join("server/catalog.db")).await, vec!["a", "b"]);
        assert_eq!(rows(&extracted.path().join("users/alice/progress.db")).await, vec!["watched"]);
    }

    #[tokio::test]
    async fn snapshot_has_no_wal_sidecars() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        seed(&root.join("server/catalog.db"), &["a"]).await;

        let out = root.join("snap.tar");
        snapshot(root, &out, 0).await.unwrap();

        let extracted = tempfile::tempdir().unwrap();
        unpack(&out, extracted.path());
        assert!(extracted.path().join("server/catalog.db").exists());
        assert!(!extracted.path().join("server/catalog.db-wal").exists());
        assert!(!extracted.path().join("server/catalog.db-shm").exists());
    }

    #[tokio::test]
    async fn snapshot_succeeds_while_source_is_open() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        seed(&root.join("server/catalog.db"), &["live"]).await;
        seed(&root.join("users/bob/prefs.db"), &["theme"]).await;
        let live = SqliteConnectOptions::new()
            .filename(root.join("server/catalog.db"))
            .create_if_missing(false)
            .journal_mode(SqliteJournalMode::Wal)
            .connect()
            .await
            .unwrap();

        let out = root.join("snap.tar");
        snapshot(root, &out, 0).await.unwrap();
        drop(live);

        let extracted = tempfile::tempdir().unwrap();
        unpack(&out, extracted.path());
        assert_eq!(rows(&extracted.path().join("server/catalog.db")).await, vec!["live"]);
        assert_eq!(rows(&extracted.path().join("users/bob/prefs.db")).await, vec!["theme"]);
    }

    #[tokio::test]
    async fn errors_when_no_databases_present() {
        let dir = tempfile::tempdir().unwrap();
        let result = snapshot(dir.path(), &dir.path().join("snap.tar"), 0).await;
        assert!(matches!(result, Err(BackupError::Invalid(_))));
    }

    #[tokio::test]
    async fn ignores_non_db_and_cache_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        seed(&root.join("server/catalog.db"), &["a"]).await;
        std::fs::create_dir_all(root.join("transcode")).unwrap();
        std::fs::write(root.join("transcode/segment.ts"), b"x").unwrap();
        std::fs::write(root.join("server/notes.txt"), b"x").unwrap();

        let out = root.join("snap.tar");
        let manifest = snapshot(root, &out, 0).await.unwrap();
        assert_eq!(manifest.databases, vec!["server/catalog.db"]);
    }

    fn build_tar(path: &Path, entries: &[(&str, Vec<u8>)]) {
        let mut builder = tar::Builder::new(std::fs::File::create(path).unwrap());
        for (name, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            builder.append_data(&mut header, name, data.as_slice()).unwrap();
        }
        builder.finish().unwrap();
    }

    fn manifest_bytes(version: u32, databases: &[&str]) -> Vec<u8> {
        serde_json::to_vec(&Manifest {
            version,
            created_at_ms: 0,
            databases: databases.iter().map(|db| (*db).to_owned()).collect(),
        })
        .unwrap()
    }

    #[tokio::test]
    async fn round_trip_restores_original_rows() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        seed(&root.join("server/catalog.db"), &["a", "b"]).await;
        seed(&root.join("users/alice/progress.db"), &["watched"]).await;
        let out = root.join("snap.tar");
        snapshot(root, &out, 0).await.unwrap();

        std::fs::remove_dir_all(root.join("server")).unwrap();
        std::fs::remove_dir_all(root.join("users")).unwrap();

        let manifest = restore(&out, root).unwrap();
        assert_eq!(manifest.databases, vec!["server/catalog.db", "users/alice/progress.db"]);
        assert_eq!(rows(&root.join("server/catalog.db")).await, vec!["a", "b"]);
        assert_eq!(rows(&root.join("users/alice/progress.db")).await, vec!["watched"]);
    }

    #[tokio::test]
    async fn clears_stale_sidecars_before_placing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        seed(&root.join("server/catalog.db"), &["fresh"]).await;
        let out = root.join("snap.tar");
        snapshot(root, &out, 0).await.unwrap();

        std::fs::write(root.join("server/catalog.db-wal"), b"stale").unwrap();
        std::fs::write(root.join("server/catalog.db-shm"), b"stale").unwrap();

        restore(&out, root).unwrap();
        assert!(!root.join("server/catalog.db-wal").exists());
        assert!(!root.join("server/catalog.db-shm").exists());
        assert_eq!(rows(&root.join("server/catalog.db")).await, vec!["fresh"]);
    }

    #[tokio::test]
    async fn refuses_when_db_root_is_locked() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        seed(&root.join("server/catalog.db"), &["a"]).await;
        let out = root.join("snap.tar");
        snapshot(root, &out, 0).await.unwrap();

        let lock = ServerLock::try_acquire(root).unwrap();
        assert!(lock.is_some());
        assert!(matches!(restore(&out, root), Err(BackupError::Invalid(_))));
        drop(lock);
        restore(&out, root).unwrap();
    }

    #[test]
    fn errors_on_missing_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("bad.tar");
        build_tar(&archive, &[("server/catalog.db", b"x".to_vec())]);
        let target = tempfile::tempdir().unwrap();
        assert!(matches!(restore(&archive, target.path()), Err(BackupError::Invalid(_))));
    }

    #[test]
    fn errors_on_malformed_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("bad.tar");
        build_tar(&archive, &[(MANIFEST_NAME, b"{ not json".to_vec())]);
        let target = tempfile::tempdir().unwrap();
        assert!(matches!(restore(&archive, target.path()), Err(BackupError::Json(_))));
    }

    #[test]
    fn errors_on_unsupported_version() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("bad.tar");
        build_tar(
            &archive,
            &[(MANIFEST_NAME, manifest_bytes(MANIFEST_VERSION + 1, &["server/catalog.db"]))],
        );
        let target = tempfile::tempdir().unwrap();
        assert!(matches!(restore(&archive, target.path()), Err(BackupError::Invalid(_))));
    }

    #[test]
    fn errors_on_unsafe_manifest_path() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("bad.tar");
        build_tar(
            &archive,
            &[(MANIFEST_NAME, manifest_bytes(MANIFEST_VERSION, &["../escape.db"]))],
        );
        let target = tempfile::tempdir().unwrap();
        assert!(matches!(restore(&archive, target.path()), Err(BackupError::Invalid(_))));
    }

    #[test]
    fn errors_when_listed_db_absent_from_archive() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("bad.tar");
        build_tar(
            &archive,
            &[(MANIFEST_NAME, manifest_bytes(MANIFEST_VERSION, &["server/catalog.db"]))],
        );
        let target = tempfile::tempdir().unwrap();
        assert!(matches!(restore(&archive, target.path()), Err(BackupError::Invalid(_))));
    }
}
