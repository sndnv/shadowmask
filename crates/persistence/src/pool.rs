use std::fmt::Display;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use domain::error::RepositoryError;
use domain::user::UserId;
use jiff::Timestamp;
use lru::LruCache;
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow};
use sqlx::{Decode, Row, Sqlite, SqlitePool, Type};
use tokio::sync::Mutex;

pub(crate) const DEFAULT_USER_POOL_CAPACITY: NonZeroUsize = NonZeroUsize::new(32).unwrap();

const USER_POOL_MAX_CONNECTIONS: u32 = 2;

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
        .foreign_keys(true)
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
    migrator.run(&pool).await.map_err(backend)?;
    Ok(pool)
}

#[derive(Clone)]
pub(crate) struct UserPools {
    base: PathBuf,
    filename: &'static str,
    migrator: &'static Migrator,
    cache: Arc<Mutex<LruCache<UserId, SqlitePool>>>,
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
        let mut cache = self.cache.lock().await;
        if let Some(pool) = cache.get(user) {
            return Ok(pool.clone());
        }
        let dir = self.base.join(&user.0);
        tokio::fs::create_dir_all(&dir).await.map_err(backend)?;
        let pool = open_with(
            SqlitePoolOptions::new().max_connections(USER_POOL_MAX_CONNECTIONS),
            &dir.join(self.filename),
            self.migrator,
        )
        .await?;
        cache.push(user.clone(), pool.clone());
        Ok(pool)
    }

    #[cfg(test)]
    pub(crate) async fn close_cached(&self) {
        let cache = self.cache.lock().await;
        for (_, pool) in cache.iter() {
            pool.close().await;
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
    async fn cache_hit_reuses_pool() {
        let dir = tempfile::tempdir().unwrap();
        let cache = pools(dir.path(), DEFAULT_USER_POOL_CAPACITY);
        let user = UserId("u1".into());
        cache.get(&user).await.unwrap();
        cache.get(&user).await.unwrap();
        assert_eq!(cache.cache.lock().await.len(), 1);
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
