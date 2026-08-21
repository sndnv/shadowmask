use std::fmt::Display;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use domain::error::RepositoryError;
use domain::user::UserId;
use jiff::Timestamp;
use lru::LruCache;
use sqlx::migrate::Migrator;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow, SqliteSynchronous,
};
use sqlx::{Decode, Row, Sqlite, SqlitePool, Type};
use tokio::sync::{Mutex, OnceCell};

pub(crate) const DEFAULT_USER_POOL_CAPACITY: NonZeroUsize = NonZeroUsize::new(128).unwrap();

const USER_POOL_MAX_CONNECTIONS: u32 = 2;

const CACHE_KIB: u32 = 16_000;

pub(crate) fn backend(error: impl Display) -> RepositoryError {
    RepositoryError::Backend(error.to_string())
}

pub(crate) fn column<'r, T>(row: &'r SqliteRow, name: &str) -> Result<T, RepositoryError>
where
    T: Decode<'r, Sqlite> + Type<Sqlite>,
{
    row.try_get::<T, _>(name).map_err(backend)
}

fn connect_options(path: &Path) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .pragma("cache_size", format!("-{CACHE_KIB}"))
        .pragma("temp_store", "MEMORY")
}

pub(crate) async fn open(path: &Path, migrator: &Migrator) -> Result<SqlitePool, RepositoryError> {
    open_with(SqlitePoolOptions::new(), path, migrator).await
}

async fn open_with(
    options: SqlitePoolOptions,
    path: &Path,
    migrator: &Migrator,
) -> Result<SqlitePool, RepositoryError> {
    let pool = options
        .connect_with(connect_options(path))
        .await
        .map_err(backend)?;
    let store = store_label(path);
    let applied = applied_migrations(&pool).await;
    check_applied_migrations(&store, path, migrator, &applied)?;
    let pending: Vec<(i64, String)> = migrator
        .iter()
        .filter(|migration| {
            !applied
                .iter()
                .any(|(version, _)| *version == migration.version)
        })
        .map(|migration| (migration.version, migration.description.to_string()))
        .collect();
    migrator
        .run(&pool)
        .await
        .map_err(|error| backend(format!("store [{store}] migrations failed: {error}")))?;
    if pending.is_empty() {
        tracing::debug!(
            "Store [{store}] is up to date at migration [{}] of [{}] applied",
            applied.last().map(|(version, _)| *version).unwrap_or(0),
            applied.len()
        );
    } else {
        for (version, description) in &pending {
            tracing::info!("Store [{store}] applied migration [{version}] [{description}]");
        }
    }
    Ok(pool)
}

async fn applied_migrations(pool: &SqlitePool) -> Vec<(i64, Vec<u8>)> {
    sqlx::query("SELECT version, checksum FROM _sqlx_migrations ORDER BY version")
        .fetch_all(pool)
        .await
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    Some((
                        row.try_get::<i64, _>("version").ok()?,
                        row.try_get::<Vec<u8>, _>("checksum").ok()?,
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn check_applied_migrations(
    store: &str,
    path: &Path,
    migrator: &Migrator,
    applied: &[(i64, Vec<u8>)],
) -> Result<(), RepositoryError> {
    for (version, checksum) in applied {
        let db = path.display();
        let Some(migration) = migrator
            .iter()
            .find(|migration| migration.version == *version)
        else {
            return Err(backend(format!(
                "store [{store}] at [{db}]: migration [{version}] is applied but is not in this \
                 build"
            )));
        };
        if migration.checksum.as_ref() != checksum.as_slice() {
            return Err(backend(format!(
                "store [{store}] at [{db}]: migration [{version}] [{}] changed since it was \
                 applied",
                migration.description
            )));
        }
    }
    Ok(())
}

fn store_label(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub(crate) async fn ping(pool: &SqlitePool) -> Result<(), RepositoryError> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map_err(backend)?;
    Ok(())
}

pub(crate) async fn checkpoint(pool: &SqlitePool) {
    let _ = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(pool)
        .await;
}

#[derive(Clone)]
pub(crate) struct UserPools {
    base: PathBuf,
    filename: &'static str,
    migrator: &'static Migrator,
    cache: Arc<Mutex<LruCache<UserId, Arc<OnceCell<SqlitePool>>>>>,
}

impl UserPools {
    pub(crate) fn new(
        base: &Path,
        filename: &'static str,
        migrator: &'static Migrator,
        capacity: NonZeroUsize,
    ) -> Self {
        Self {
            base: base.to_path_buf(),
            filename,
            migrator,
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
        }
    }

    pub(crate) async fn get(&self, user: &UserId) -> Result<SqlitePool, RepositoryError> {
        let cell = {
            let mut cache = self.cache.lock().await;
            match cache.get(user) {
                Some(cell) => Arc::clone(cell),
                None => {
                    let cell = Arc::new(OnceCell::new());
                    cache.push(user.clone(), Arc::clone(&cell));
                    cell
                }
            }
        };
        cell.get_or_try_init(|| async {
            let dir = self.base.join(&user.0);
            tokio::fs::create_dir_all(&dir).await.map_err(backend)?;
            open_with(
                SqlitePoolOptions::new().max_connections(USER_POOL_MAX_CONNECTIONS),
                &dir.join(self.filename),
                self.migrator,
            )
            .await
        })
        .await
        .cloned()
    }

    pub(crate) async fn purge(&self, user: &UserId) -> Result<(), RepositoryError> {
        {
            let mut cache = self.cache.lock().await;
            if let Some(pool) = cache.pop(user).as_deref().and_then(OnceCell::get) {
                checkpoint(pool).await;
                pool.close().await;
            }
        }
        match tokio::fs::remove_dir_all(self.base.join(&user.0)).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(backend(error)),
        }
    }

    pub(crate) async fn close_all(&self) {
        let mut cache = self.cache.lock().await;
        for (_, cell) in cache.iter() {
            if let Some(pool) = cell.get() {
                checkpoint(pool).await;
                pool.close().await;
            }
        }
        cache.clear();
    }

    #[cfg(test)]
    pub(crate) async fn close_cached(&self) {
        let cache = self.cache.lock().await;
        for (_, cell) in cache.iter() {
            if let Some(pool) = cell.get() {
                pool.close().await;
            }
        }
    }
}

pub(crate) fn to_millis(ts: Timestamp) -> i64 {
    ts.as_millisecond()
}

pub(crate) fn from_millis(millis: i64) -> Result<Timestamp, RepositoryError> {
    Timestamp::from_millisecond(millis).map_err(backend)
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_MIGRATOR: Migrator = sqlx::migrate!("./migrations/progress");

    fn pools(base: &Path, capacity: NonZeroUsize) -> UserPools {
        UserPools::new(base, "test.db", &TEST_MIGRATOR, capacity)
    }

    #[tokio::test]
    async fn a_pool_commits_without_fsyncing_every_transaction() {
        let dir = tempfile::tempdir().unwrap();
        let pool = open(&dir.path().join("pragmas.db"), &TEST_MIGRATOR)
            .await
            .unwrap();

        let synchronous: i64 = sqlx::query("PRAGMA synchronous")
            .fetch_one(&pool)
            .await
            .unwrap()
            .try_get(0)
            .unwrap();
        let temp_store: i64 = sqlx::query("PRAGMA temp_store")
            .fetch_one(&pool)
            .await
            .unwrap()
            .try_get(0)
            .unwrap();
        let cache_size: i64 = sqlx::query("PRAGMA cache_size")
            .fetch_one(&pool)
            .await
            .unwrap()
            .try_get(0)
            .unwrap();

        assert_eq!(synchronous, 1, "NORMAL, the documented setting under WAL");
        assert_eq!(
            temp_store, 2,
            "MEMORY, so ORDER BY temp b-trees stay in RAM"
        );
        assert_eq!(
            cache_size,
            -i64::from(CACHE_KIB),
            "negative means kibibytes, positive would mean pages"
        );
    }

    #[tokio::test]
    async fn two_users_open_their_pools_without_queueing_behind_each_other() {
        let dir = tempfile::tempdir().unwrap();
        let cache = pools(dir.path(), DEFAULT_USER_POOL_CAPACITY);

        let (u1, u2) = (UserId("u1".into()), UserId("u2".into()));
        let (first, second) = tokio::join!(cache.get(&u1), cache.get(&u2));

        first.unwrap();
        second.unwrap();
        assert_eq!(cache.cache.lock().await.len(), 2);
    }

    #[tokio::test]
    async fn racing_opens_of_one_user_settle_on_a_single_pool() {
        let dir = tempfile::tempdir().unwrap();
        let cache = pools(dir.path(), DEFAULT_USER_POOL_CAPACITY);
        let user = UserId("u1".into());

        let (first, second) = tokio::join!(cache.get(&user), cache.get(&user));

        let first = first.expect("two opens of one user must not race the migrator");
        let second = second.expect("two opens of one user must not race the migrator");
        assert_eq!(cache.cache.lock().await.len(), 1);
        assert!(!first.is_closed());
        assert!(!second.is_closed());
        sqlx::query("SELECT 1").execute(&second).await.unwrap();
    }

    #[tokio::test]
    async fn a_user_whose_pool_failed_to_open_does_not_break_shutdown() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not-a-dir");
        tokio::fs::write(&file, b"x").await.unwrap();
        let cache = pools(&file, DEFAULT_USER_POOL_CAPACITY);
        let user = UserId("u1".into());

        assert!(cache.get(&user).await.is_err());
        assert_eq!(
            cache.cache.lock().await.len(),
            1,
            "the reservation stays behind so a retry reuses it rather than racing again"
        );

        cache.close_cached().await;
        cache.close_all().await;

        assert!(cache.cache.lock().await.is_empty());
    }

    #[tokio::test]
    async fn evicts_least_recently_used_pool() {
        let dir = tempfile::tempdir().unwrap();
        let cache = pools(dir.path(), NonZeroUsize::new(2).unwrap());
        for id in ["u1", "u2", "u3"] {
            cache.get(&UserId(id.into())).await.unwrap();
        }
        let inner = cache.cache.lock().await;
        assert_eq!(inner.len(), 2);
        assert!(!inner.contains(&UserId("u1".into())));
        assert!(inner.contains(&UserId("u2".into())));
        assert!(inner.contains(&UserId("u3".into())));
    }

    #[tokio::test]
    async fn purge_closes_the_pool_and_removes_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let cache = pools(dir.path(), DEFAULT_USER_POOL_CAPACITY);
        let user = UserId("u1".into());
        cache.get(&user).await.unwrap();
        assert!(dir.path().join("u1").join("test.db").exists());

        cache.purge(&user).await.unwrap();

        assert!(!dir.path().join("u1").exists());
        assert!(!cache.cache.lock().await.contains(&user));
    }

    #[tokio::test]
    async fn purging_a_user_with_nothing_on_disk_is_fine() {
        let dir = tempfile::tempdir().unwrap();
        let cache = pools(dir.path(), DEFAULT_USER_POOL_CAPACITY);

        cache.purge(&UserId("never-seen".into())).await.unwrap();
    }

    #[tokio::test]
    async fn a_purged_user_can_be_opened_again_from_scratch() {
        let dir = tempfile::tempdir().unwrap();
        let cache = pools(dir.path(), DEFAULT_USER_POOL_CAPACITY);
        let user = UserId("u1".into());
        cache.get(&user).await.unwrap();
        cache.purge(&user).await.unwrap();

        cache.get(&user).await.unwrap();

        assert!(dir.path().join("u1").join("test.db").exists());
    }

    #[tokio::test]
    async fn checkpoint_completes_on_open_pool() {
        let dir = tempfile::tempdir().unwrap();
        let cache = pools(dir.path(), DEFAULT_USER_POOL_CAPACITY);
        let pool = cache.get(&UserId("u1".into())).await.unwrap();
        checkpoint(&pool).await;
    }

    #[tokio::test]
    async fn close_all_closes_and_clears_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache = pools(dir.path(), DEFAULT_USER_POOL_CAPACITY);
        for id in ["u1", "u2"] {
            cache.get(&UserId(id.into())).await.unwrap();
        }
        assert_eq!(cache.cache.lock().await.len(), 2);
        cache.close_all().await;
        assert_eq!(cache.cache.lock().await.len(), 0);
        cache.get(&UserId("u1".into())).await.unwrap();
        assert_eq!(cache.cache.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn cache_hit_reuses_pool() {
        let dir = tempfile::tempdir().unwrap();
        let cache = pools(dir.path(), DEFAULT_USER_POOL_CAPACITY);
        let user = UserId("u1".into());
        cache.get(&user).await.unwrap();
        cache.get(&user).await.unwrap();
        assert_eq!(cache.cache.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn a_changed_migration_names_the_store_the_database_and_the_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog.db");
        let pool = open(&path, &TEST_MIGRATOR).await.unwrap();
        sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = 1")
            .bind(vec![0u8; 4])
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;

        let error = open(&path, &TEST_MIGRATOR).await.unwrap_err();

        let RepositoryError::Backend(message) = error else {
            panic!("expected a backend error");
        };
        assert!(message.contains("store [catalog]"), "{message}");
        assert!(message.contains("migration [1]"), "{message}");
        assert!(
            message.contains("changed since it was applied"),
            "{message}"
        );
        assert!(message.contains(&path.display().to_string()), "{message}");
    }

    #[tokio::test]
    async fn a_migration_dropped_from_the_build_is_reported_as_stale() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("progress.db");
        let pool = open(&path, &TEST_MIGRATOR).await.unwrap();
        sqlx::query(
            "INSERT INTO _sqlx_migrations \
             (version, description, installed_on, success, checksum, execution_time) \
             VALUES (9999, 'ghost', CURRENT_TIMESTAMP, 1, ?, 0)",
        )
        .bind(vec![0u8; 4])
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        let error = open(&path, &TEST_MIGRATOR).await.unwrap_err();

        let RepositoryError::Backend(message) = error else {
            panic!("expected a backend error");
        };
        assert!(message.contains("store [progress]"), "{message}");
        assert!(
            message.contains("migration [9999] is applied but is not in this build"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn reopening_an_unchanged_store_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("progress.db");
        open(&path, &TEST_MIGRATOR).await.unwrap().close().await;
        open(&path, &TEST_MIGRATOR).await.unwrap().close().await;
    }

    #[tokio::test]
    async fn create_dir_all_error_surfaces_as_backend() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not-a-dir");
        tokio::fs::write(&file, b"x").await.unwrap();
        let cache = pools(&file, DEFAULT_USER_POOL_CAPACITY);
        assert!(matches!(
            cache.get(&UserId("u1".into())).await,
            Err(RepositoryError::Backend(_))
        ));
    }
}
