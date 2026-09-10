use std::path::Path;

use domain::catalog::VersionId;
use domain::error::RepositoryError;
use domain::playback::{Favorite, SubtitleTrackRef, UserSubtitleOffset, WatchlistItem};
use domain::repository::PreferencesRepository;
use domain::user::UserId;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteRow;

use crate::codec::{subtitle_ref_parts, title_from_parts, title_kind};
use crate::metrics::DbOpGuard;
use crate::pool::{DEFAULT_USER_POOL_CAPACITY, UserPools, backend, column, from_millis, to_millis};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/prefs");

#[derive(Clone)]
pub struct SqlitePreferencesRepo {
    pools: UserPools,
}

impl SqlitePreferencesRepo {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            pools: UserPools::new(base_dir, "prefs.db", &MIGRATOR, DEFAULT_USER_POOL_CAPACITY),
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
}

fn row_to_watchlist(user: &UserId, row: &SqliteRow) -> Result<WatchlistItem, RepositoryError> {
    Ok(WatchlistItem {
        user: user.clone(),
        title: title_from_parts(
            &column::<String>(row, "title_kind")?,
            column(row, "title_id")?,
        )?,
        added_at: from_millis(column(row, "added_at")?)?,
    })
}

fn row_to_favorite(user: &UserId, row: &SqliteRow) -> Result<Favorite, RepositoryError> {
    Ok(Favorite {
        user: user.clone(),
        title: title_from_parts(
            &column::<String>(row, "title_kind")?,
            column(row, "title_id")?,
        )?,
        added_at: from_millis(column(row, "added_at")?)?,
    })
}

fn row_to_offset(
    user: &UserId,
    version: &VersionId,
    subtitle: &SubtitleTrackRef,
    row: &SqliteRow,
) -> Result<UserSubtitleOffset, RepositoryError> {
    Ok(UserSubtitleOffset {
        user: user.clone(),
        version: version.clone(),
        subtitle: subtitle.clone(),
        offset_ms: column::<i64>(row, "offset_ms")?,
    })
}

impl PreferencesRepository for SqlitePreferencesRepo {
    async fn list_watchlist(&self, user: &UserId) -> Result<Vec<WatchlistItem>, RepositoryError> {
        let _op = DbOpGuard::new("preferences", "list_watchlist");
        let pool = self.pools.get(user).await?;
        let rows = sqlx::query(
            "SELECT title_kind, title_id, added_at FROM watchlist \
             ORDER BY added_at, title_kind, title_id",
        )
        .fetch_all(&pool)
        .await
        .map_err(backend)?;
        rows.iter().map(|row| row_to_watchlist(user, row)).collect()
    }

    async fn add_watchlist(&self, item: WatchlistItem) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("preferences", "add_watchlist");
        let pool = self.pools.get(&item.user).await?;
        sqlx::query(
            "INSERT OR REPLACE INTO watchlist (title_kind, title_id, added_at) VALUES (?, ?, ?)",
        )
        .bind(title_kind(&item.title))
        .bind(item.title.id())
        .bind(to_millis(item.added_at))
        .execute(&pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn remove_watchlist(&self, user: &UserId, title_id: &str) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("preferences", "remove_watchlist");
        let pool = self.pools.get(user).await?;
        sqlx::query("DELETE FROM watchlist WHERE title_id = ?")
            .bind(title_id)
            .execute(&pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn list_favorites(&self, user: &UserId) -> Result<Vec<Favorite>, RepositoryError> {
        let _op = DbOpGuard::new("preferences", "list_favorites");
        let pool = self.pools.get(user).await?;
        let rows = sqlx::query(
            "SELECT title_kind, title_id, added_at FROM favorites \
             ORDER BY added_at, title_kind, title_id",
        )
        .fetch_all(&pool)
        .await
        .map_err(backend)?;
        rows.iter().map(|row| row_to_favorite(user, row)).collect()
    }

    async fn add_favorite(&self, item: Favorite) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("preferences", "add_favorite");
        let pool = self.pools.get(&item.user).await?;
        sqlx::query(
            "INSERT OR REPLACE INTO favorites (title_kind, title_id, added_at) VALUES (?, ?, ?)",
        )
        .bind(title_kind(&item.title))
        .bind(item.title.id())
        .bind(to_millis(item.added_at))
        .execute(&pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn remove_favorite(&self, user: &UserId, title_id: &str) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("preferences", "remove_favorite");
        let pool = self.pools.get(user).await?;
        sqlx::query("DELETE FROM favorites WHERE title_id = ?")
            .bind(title_id)
            .execute(&pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn get_subtitle_offset(
        &self,
        user: &UserId,
        version: &VersionId,
        subtitle: &SubtitleTrackRef,
    ) -> Result<Option<UserSubtitleOffset>, RepositoryError> {
        let _op = DbOpGuard::new("preferences", "get_subtitle_offset");
        let pool = self.pools.get(user).await?;
        let (kind, value) = subtitle_ref_parts(subtitle);
        let row = sqlx::query(
            "SELECT offset_ms FROM subtitle_offsets \
             WHERE version_id = ? AND subtitle_kind = ? AND subtitle_ref = ?",
        )
        .bind(version.0.as_str())
        .bind(kind)
        .bind(value.as_str())
        .fetch_optional(&pool)
        .await
        .map_err(backend)?;
        row.as_ref()
            .map(|row| row_to_offset(user, version, subtitle, row))
            .transpose()
    }

    async fn set_subtitle_offset(&self, offset: UserSubtitleOffset) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("preferences", "set_subtitle_offset");
        let pool = self.pools.get(&offset.user).await?;
        let (kind, value) = subtitle_ref_parts(&offset.subtitle);
        sqlx::query(
            "INSERT OR REPLACE INTO subtitle_offsets \
             (version_id, subtitle_kind, subtitle_ref, offset_ms) VALUES (?, ?, ?, ?)",
        )
        .bind(offset.version.0.as_str())
        .bind(kind)
        .bind(value.as_str())
        .bind(offset.offset_ms)
        .execute(&pool)
        .await
        .map_err(backend)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId};
    use jiff::Timestamp;

    #[tokio::test]
    async fn surfaces_backend_error_after_close() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqlitePreferencesRepo::new(dir.path());
        let user = UserId("u1".into());
        repo.list_watchlist(&user).await.unwrap();
        repo.pools.close_cached().await;

        let title = TitleId::Movie(MovieId("m1".into()));
        let version = VersionId("v1".into());
        let track = SubtitleTrackRef::Embedded(0);

        assert!(repo.list_watchlist(&user).await.is_err());
        assert!(
            repo.add_watchlist(WatchlistItem {
                user: user.clone(),
                title: title.clone(),
                added_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .is_err()
        );
        assert!(repo.remove_watchlist(&user, "m1").await.is_err());
        assert!(repo.list_favorites(&user).await.is_err());
        assert!(
            repo.add_favorite(Favorite {
                user: user.clone(),
                title,
                added_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .is_err()
        );
        assert!(repo.remove_favorite(&user, "m1").await.is_err());
        assert!(
            repo.get_subtitle_offset(&user, &version, &track)
                .await
                .is_err()
        );
        assert!(
            repo.set_subtitle_offset(UserSubtitleOffset {
                user,
                version,
                subtitle: track,
                offset_ms: 250,
            })
            .await
            .is_err()
        );
    }
}
