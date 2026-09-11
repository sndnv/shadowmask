use std::path::Path;

use domain::catalog::{TitleId, VersionId};
use domain::common::{Page, PageRequest};
use domain::error::RepositoryError;
use domain::playback::{PlaybackProgress, WatchHistory};
use domain::repository::ProgressRepository;
use domain::user::UserId;
use jiff::Timestamp;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteRow;

use crate::codec::{
    subtitle_override_from_parts, subtitle_override_parts, title_from_parts, title_kind,
};
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
            pools: UserPools::new(base_dir, "progress.db", &MIGRATOR, DEFAULT_USER_POOL_CAPACITY),
        }
    }

    pub async fn ensure_migrated(&self, user: &UserId) -> Result<(), RepositoryError> {
        self.pools.get(user).await?;
        Ok(())
    }

    pub async fn purge(&self, user: &UserId) -> Result<(), RepositoryError> {
        self.pools.purge(user).await
    }

    pub async fn close(&self) {
        self.pools.close_all().await;
    }

    async fn prune(&self, pool: &sqlx::SqlitePool) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM watch_history WHERE play_count = 0 AND watched = 0")
            .execute(pool)
            .await
            .map_err(backend)?;
        Ok(())
    }
}

fn row_to_progress(user: &UserId, row: &SqliteRow) -> Result<PlaybackProgress, RepositoryError> {
    let subtitle = match column::<Option<String>>(row, "subtitle_kind")? {
        Some(kind) => Some(subtitle_override_from_parts(
            &kind,
            column::<Option<String>>(row, "subtitle_ref")?,
        )?),
        None => None,
    };
    Ok(PlaybackProgress {
        user: user.clone(),
        version: VersionId(column::<String>(row, "version_id")?),
        position_ms: column::<i64>(row, "position_ms")? as u64,
        audio_track: column::<Option<i64>>(row, "audio_track")?.map(|index| index as u32),
        subtitle,
        updated_at: from_millis(column(row, "updated_at")?)?,
    })
}

fn row_to_history(user: &UserId, row: &SqliteRow) -> Result<WatchHistory, RepositoryError> {
    let title = title_from_parts(&column::<String>(row, "title_kind")?, column(row, "title_id")?)?;
    let last_watched_at =
        column::<Option<i64>>(row, "last_watched_at")?.map(from_millis).transpose()?;
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
            "SELECT version_id, position_ms, audio_track, subtitle_kind, subtitle_ref, updated_at \
             FROM playback_progress WHERE version_id = ?",
        )
        .bind(version.0.as_str())
        .fetch_optional(&pool)
        .await
        .map_err(backend)?;
        row.as_ref().map(|row| row_to_progress(user, row)).transpose()
    }

    async fn upsert(&self, progress: PlaybackProgress) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("progress", "upsert");
        let pool = self.pools.get(&progress.user).await?;
        let subtitle = progress.subtitle.as_ref().map(subtitle_override_parts);
        sqlx::query(
            "INSERT OR REPLACE INTO playback_progress \
             (version_id, position_ms, audio_track, subtitle_kind, subtitle_ref, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(progress.version.0.as_str())
        .bind(progress.position_ms as i64)
        .bind(progress.audio_track.map(i64::from))
        .bind(subtitle.as_ref().map(|(kind, _)| *kind))
        .bind(subtitle.as_ref().and_then(|(_, value)| value.as_deref()))
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
            "SELECT version_id, position_ms, audio_track, subtitle_kind, subtitle_ref, updated_at \
             FROM playback_progress ORDER BY updated_at DESC, version_id",
        )
        .fetch_all(&pool)
        .await
        .map_err(backend)?;
        rows.iter().map(|row| row_to_progress(user, row)).collect()
    }

    async fn record_view(
        &self,
        user: &UserId,
        title: &TitleId,
        at: Timestamp,
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("progress", "record_view");
        let pool = self.pools.get(user).await?;
        sqlx::query(
            "INSERT INTO watch_history \
             (title_kind, title_id, watched, play_count, last_watched_at, completed) \
             VALUES (?, ?, 1, 1, ?, 1) \
             ON CONFLICT (title_kind, title_id) DO UPDATE SET \
             watched = 1, completed = 1, play_count = play_count + 1, \
             last_watched_at = excluded.last_watched_at",
        )
        .bind(title_kind(title))
        .bind(title.id())
        .bind(to_millis(at))
        .execute(&pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn set_watched_flags(
        &self,
        user: &UserId,
        title: &TitleId,
        watched: bool,
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("progress", "set_watched_flags");
        let pool = self.pools.get(user).await?;
        sqlx::query(
            "INSERT INTO watch_history \
             (title_kind, title_id, watched, play_count, last_watched_at, completed) \
             VALUES (?, ?, ?, 0, NULL, ?) \
             ON CONFLICT (title_kind, title_id) DO UPDATE SET \
             watched = excluded.watched, completed = excluded.completed",
        )
        .bind(title_kind(title))
        .bind(title.id())
        .bind(watched)
        .bind(watched)
        .execute(&pool)
        .await
        .map_err(backend)?;
        self.prune(&pool).await
    }

    async fn delete_history(&self, user: &UserId, title_id: &str) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("progress", "delete_history");
        let pool = self.pools.get(user).await?;
        sqlx::query(
            "UPDATE watch_history SET play_count = 0, last_watched_at = NULL WHERE title_id = ?",
        )
        .bind(title_id)
        .execute(&pool)
        .await
        .map_err(backend)?;
        self.prune(&pool).await
    }

    async fn clear_history(&self, user: &UserId) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("progress", "clear_history");
        let pool = self.pools.get(user).await?;
        sqlx::query("UPDATE watch_history SET play_count = 0, last_watched_at = NULL")
            .execute(&pool)
            .await
            .map_err(backend)?;
        self.prune(&pool).await
    }

    async fn watched_state(&self, user: &UserId) -> Result<Vec<WatchHistory>, RepositoryError> {
        let _op = DbOpGuard::new("progress", "watched_state");
        let pool = self.pools.get(user).await?;
        let rows = sqlx::query(
            "SELECT title_kind, title_id, watched, play_count, last_watched_at, completed \
             FROM watch_history",
        )
        .fetch_all(&pool)
        .await
        .map_err(backend)?;
        rows.iter().map(|row| row_to_history(user, row)).collect()
    }

    async fn history(
        &self,
        user: &UserId,
        page: PageRequest,
    ) -> Result<Page<WatchHistory>, RepositoryError> {
        let _op = DbOpGuard::new("progress", "history");
        let pool = self.pools.get(user).await?;
        let count_row = sqlx::query("SELECT COUNT(*) AS n FROM watch_history WHERE play_count > 0")
            .fetch_one(&pool)
            .await
            .map_err(backend)?;
        let total = column::<i64>(&count_row, "n")? as u64;
        let rows = sqlx::query(
            "SELECT title_kind, title_id, watched, play_count, last_watched_at, completed \
             FROM watch_history WHERE play_count > 0 \
             ORDER BY last_watched_at DESC, title_kind, title_id \
             LIMIT ? OFFSET ?",
        )
        .bind(i64::from(page.limit))
        .bind(i64::from(page.offset))
        .fetch_all(&pool)
        .await
        .map_err(backend)?;
        let items = rows.iter().map(|row| row_to_history(user, row)).collect::<Result<_, _>>()?;
        Ok(Page { items, total, offset: page.offset, limit: page.limit })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::MovieId;
    use domain::playback::SubtitleOverride;

    fn user() -> UserId {
        UserId("u1".into())
    }

    fn version() -> VersionId {
        VersionId("v1".into())
    }

    fn title() -> TitleId {
        TitleId::Movie(MovieId("m1".into()))
    }

    fn page() -> PageRequest {
        PageRequest { offset: 0, limit: 10 }
    }

    fn row() -> PlaybackProgress {
        PlaybackProgress {
            user: user(),
            version: version(),
            position_ms: 10,
            audio_track: None,
            subtitle: None,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    async fn seeded(dir: &tempfile::TempDir, sql: &'static str) -> SqliteProgressRepo {
        let repo = SqliteProgressRepo::new(dir.path());
        let pool = repo.pools.get(&user()).await.unwrap();
        sqlx::query(sql).execute(&pool).await.unwrap();
        repo
    }

    // A row the codec cannot read must fail loudly. Reading it as "no override" would
    // silently discard the viewer's choice and look like the feature never worked.
    #[tokio::test]
    async fn an_unreadable_subtitle_override_is_an_error_not_a_silent_none() {
        let dir = tempfile::tempdir().unwrap();
        let repo = seeded(
            &dir,
            "INSERT INTO playback_progress \
             (version_id, position_ms, audio_track, subtitle_kind, subtitle_ref, updated_at) \
             VALUES ('v1', 10, NULL, 'elsewhere', '3', 0)",
        )
        .await;

        assert!(repo.get(&user(), &version()).await.is_err());
        assert!(repo.list_in_progress(&user()).await.is_err());
    }

    #[tokio::test]
    async fn a_subtitle_kind_without_its_reference_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let repo = seeded(
            &dir,
            "INSERT INTO playback_progress \
             (version_id, position_ms, audio_track, subtitle_kind, subtitle_ref, updated_at) \
             VALUES ('v1', 10, NULL, 'embedded', NULL, 0)",
        )
        .await;

        assert!(repo.get(&user(), &version()).await.is_err());
    }

    #[tokio::test]
    async fn an_unreadable_title_kind_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let repo = seeded(
            &dir,
            "INSERT INTO watch_history \
             (title_kind, title_id, watched, play_count, last_watched_at, completed) \
             VALUES ('sideways', 't1', 1, 1, 0, 1)",
        )
        .await;

        assert!(repo.history(&user(), page()).await.is_err());
        assert!(repo.watched_state(&user()).await.is_err());
    }

    #[tokio::test]
    async fn an_override_survives_the_round_trip_through_sqlite() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteProgressRepo::new(dir.path());
        repo.upsert(PlaybackProgress {
            audio_track: Some(4),
            subtitle: Some(SubtitleOverride::Off),
            ..row()
        })
        .await
        .unwrap();

        let listed = repo.list_in_progress(&user()).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].audio_track, Some(4));
        assert_eq!(listed[0].subtitle, Some(SubtitleOverride::Off));
    }

    #[tokio::test]
    async fn every_operation_surfaces_a_backend_error_after_close() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteProgressRepo::new(dir.path());
        repo.list_in_progress(&user()).await.unwrap();
        repo.pools.close_cached().await;

        assert!(repo.get(&user(), &version()).await.is_err());
        assert!(repo.upsert(row()).await.is_err());
        assert!(repo.delete(&user(), &version()).await.is_err());
        assert!(repo.list_in_progress(&user()).await.is_err());
        assert!(repo.record_view(&user(), &title(), Timestamp::UNIX_EPOCH).await.is_err());
        assert!(repo.set_watched_flags(&user(), &title(), true).await.is_err());
        assert!(repo.delete_history(&user(), "m1").await.is_err());
        assert!(repo.clear_history(&user()).await.is_err());
        assert!(repo.watched_state(&user()).await.is_err());
        assert!(repo.history(&user(), page()).await.is_err());
    }
}
