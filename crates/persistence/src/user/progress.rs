use std::path::Path;

use domain::catalog::VersionId;
use domain::common::{Page, PageRequest};
use domain::error::RepositoryError;
use domain::playback::{PlaybackProgress, WatchHistory};
use domain::repository::ProgressRepository;
use domain::user::UserId;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteRow;

use crate::codec::{title_from_parts, title_kind};
use crate::metrics::DbOpGuard;
use crate::pool::{DEFAULT_USER_POOL_CAPACITY, UserPools, backend, column, from_millis, to_millis};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/progress");

#[derive(Clone)]
pub struct SqliteProgressRepo {
    pools: UserPools,
}

impl SqliteProgressRepo {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            pools: UserPools::new(
                base_dir,
                "progress.db",
                &MIGRATOR,
                DEFAULT_USER_POOL_CAPACITY,
            ),
        }
    }

    pub async fn ensure_migrated(&self, user: &UserId) -> Result<(), RepositoryError> {
        self.pools.get(user).await?;
        Ok(())
    }

    pub async fn close(&self) {
        self.pools.close_all().await;
    }
}

fn row_to_progress(user: &UserId, row: &SqliteRow) -> Result<PlaybackProgress, RepositoryError> {
    Ok(PlaybackProgress {
        user: user.clone(),
        version: VersionId(column::<String>(row, "version_id")?),
        position_ms: column::<i64>(row, "position_ms")? as u64,
        updated_at: from_millis(column(row, "updated_at")?)?,
    })
}

fn row_to_history(user: &UserId, row: &SqliteRow) -> Result<WatchHistory, RepositoryError> {
    let title = title_from_parts(
        &column::<String>(row, "title_kind")?,
        column(row, "title_id")?,
    )?;
    let last_watched_at = column::<Option<i64>>(row, "last_watched_at")?
        .map(from_millis)
        .transpose()?;
    Ok(WatchHistory {
        user: user.clone(),
        title,
        watched: column(row, "watched")?,
        play_count: column::<i64>(row, "play_count")? as u32,
        last_watched_at,
        completed: column(row, "completed")?,
    })
}

impl ProgressRepository for SqliteProgressRepo {
    async fn get(
        &self,
        user: &UserId,
        version: &VersionId,
    ) -> Result<Option<PlaybackProgress>, RepositoryError> {
        let _op = DbOpGuard::new("progress", "get");
        let pool = self.pools.get(user).await?;
        let row = sqlx::query(
            "SELECT version_id, position_ms, updated_at FROM playback_progress WHERE version_id = ?",
        )
        .bind(version.0.as_str())
        .fetch_optional(&pool)
        .await
        .map_err(backend)?;
        row.as_ref()
            .map(|row| row_to_progress(user, row))
            .transpose()
    }

    async fn upsert(&self, progress: PlaybackProgress) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("progress", "upsert");
        let pool = self.pools.get(&progress.user).await?;
        sqlx::query(
            "INSERT OR REPLACE INTO playback_progress (version_id, position_ms, updated_at) \
             VALUES (?, ?, ?)",
        )
        .bind(progress.version.0.as_str())
        .bind(progress.position_ms as i64)
        .bind(to_millis(progress.updated_at))
        .execute(&pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn delete(&self, user: &UserId, version: &VersionId) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("progress", "delete");
        let pool = self.pools.get(user).await?;
        sqlx::query("DELETE FROM playback_progress WHERE version_id = ?")
            .bind(version.0.as_str())
            .execute(&pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn list_in_progress(
        &self,
        user: &UserId,
    ) -> Result<Vec<PlaybackProgress>, RepositoryError> {
        let _op = DbOpGuard::new("progress", "list_in_progress");
        let pool = self.pools.get(user).await?;
        let rows = sqlx::query(
            "SELECT version_id, position_ms, updated_at FROM playback_progress \
             ORDER BY updated_at DESC, version_id",
        )
        .fetch_all(&pool)
        .await
        .map_err(backend)?;
        rows.iter().map(|row| row_to_progress(user, row)).collect()
    }

    async fn record_history(&self, history: WatchHistory) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("progress", "record_history");
        let pool = self.pools.get(&history.user).await?;
        sqlx::query(
            "INSERT OR REPLACE INTO watch_history \
             (title_kind, title_id, watched, play_count, last_watched_at, completed) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(title_kind(&history.title))
        .bind(history.title.id())
        .bind(history.watched)
        .bind(history.play_count as i64)
        .bind(history.last_watched_at.map(to_millis))
        .bind(history.completed)
        .execute(&pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn history(
        &self,
        user: &UserId,
        page: PageRequest,
    ) -> Result<Page<WatchHistory>, RepositoryError> {
        let _op = DbOpGuard::new("progress", "history");
        let pool = self.pools.get(user).await?;
        let count_row = sqlx::query("SELECT COUNT(*) AS n FROM watch_history")
            .fetch_one(&pool)
            .await
            .map_err(backend)?;
        let total = column::<i64>(&count_row, "n")? as u64;
        let rows = sqlx::query(
            "SELECT title_kind, title_id, watched, play_count, last_watched_at, completed \
             FROM watch_history ORDER BY last_watched_at DESC, title_kind, title_id \
             LIMIT ? OFFSET ?",
        )
        .bind(i64::from(page.limit))
        .bind(i64::from(page.offset))
        .fetch_all(&pool)
        .await
        .map_err(backend)?;
        let items = rows
            .iter()
            .map(|row| row_to_history(user, row))
            .collect::<Result<_, _>>()?;
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn surfaces_backend_error_after_close() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteProgressRepo::new(dir.path());
        let user = UserId("u1".into());
        repo.list_in_progress(&user).await.unwrap();
        repo.pools.close_cached().await;
        assert!(repo.list_in_progress(&user).await.is_err());
        assert!(repo.delete(&user, &VersionId("v1".into())).await.is_err());
    }
}
