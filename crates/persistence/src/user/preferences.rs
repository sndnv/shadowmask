use std::collections::HashSet;
use std::path::Path;

use domain::catalog::{TitleId, TitleSet, VersionId};
use domain::error::RepositoryError;
use domain::playback::{
    Favorite, Playlist, PlaylistId, PlaylistItem, QueueEntry, QueueEntryId, SubtitleTrackRef,
    UserSubtitleOffset, WatchlistItem,
};
use domain::repository::PreferencesRepository;
use domain::user::UserId;
use jiff::Timestamp;
use sqlx::SqliteConnection;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteRow;

use crate::codec::{subtitle_ref_parts, title_from_parts, title_kind};
use crate::metrics::DbOpGuard;
use crate::pool::{DEFAULT_USER_POOL_CAPACITY, UserPools, backend, column, from_millis, to_millis};
use crate::user::forget::Forget;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/prefs");

const QUEUE_IN_ORDER_SQL: &str = "INSERT INTO queue_items (position, title_kind, title_id) \
     SELECT (SELECT COALESCE(MAX(position), 0) FROM queue_items) \
     + ROW_NUMBER() OVER (ORDER BY position, title_kind, title_id), title_kind, title_id \
     FROM playlist_items WHERE playlist_id = ?";
const QUEUE_SHUFFLED_SQL: &str = "INSERT INTO queue_items (position, title_kind, title_id) \
     SELECT (SELECT COALESCE(MAX(position), 0) FROM queue_items) \
     + ROW_NUMBER() OVER (ORDER BY random()), title_kind, title_id \
     FROM playlist_items WHERE playlist_id = ?";

#[derive(Clone)]
pub struct SqlitePreferencesRepo {
    pools: UserPools,
}

impl SqlitePreferencesRepo {
    pub fn new(base_dir: &Path) -> Self {
        Self { pools: UserPools::new(base_dir, "prefs.db", &MIGRATOR, DEFAULT_USER_POOL_CAPACITY) }
    }

    pub async fn ensure_migrated(&self, user: &UserId) -> Result<(), RepositoryError> {
        self.pools.get(user).await?;
        Ok(())
    }

    pub async fn purge(&self, user: &UserId) -> Result<(), RepositoryError> {
        self.pools.purge(user).await
    }

    pub async fn forget(
        &self,
        user: &UserId,
        titles: &TitleSet,
        versions: &HashSet<VersionId>,
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("preferences", "forget");
        let pool = self.pools.get(user).await?;
        Forget {
            by_title: &[
                "DELETE FROM watchlist WHERE title_kind = ? AND title_id = ?",
                "DELETE FROM favorites WHERE title_kind = ? AND title_id = ?",
                "DELETE FROM playlist_items WHERE title_kind = ? AND title_id = ?",
                "DELETE FROM queue_items WHERE title_kind = ? AND title_id = ?",
            ],
            by_version: &["DELETE FROM subtitle_offsets WHERE version_id = ?"],
        }
        .run(&pool, titles, versions)
        .await
    }

    pub async fn close(&self) {
        self.pools.close_all().await;
    }
}

fn row_to_watchlist(user: &UserId, row: &SqliteRow) -> Result<WatchlistItem, RepositoryError> {
    Ok(WatchlistItem {
        user: user.clone(),
        title: title_from_parts(&column::<String>(row, "title_kind")?, column(row, "title_id")?)?,
        added_at: from_millis(column(row, "added_at")?)?,
    })
}

fn row_to_favorite(user: &UserId, row: &SqliteRow) -> Result<Favorite, RepositoryError> {
    Ok(Favorite {
        user: user.clone(),
        title: title_from_parts(&column::<String>(row, "title_kind")?, column(row, "title_id")?)?,
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

fn row_to_playlist(row: &SqliteRow) -> Result<Playlist, RepositoryError> {
    Ok(Playlist {
        id: PlaylistId(column(row, "id")?),
        name: column(row, "name")?,
        created_at: from_millis(column(row, "created_at")?)?,
        updated_at: from_millis(column(row, "updated_at")?)?,
    })
}

fn row_to_playlist_item(row: &SqliteRow) -> Result<PlaylistItem, RepositoryError> {
    Ok(PlaylistItem {
        position: u32::try_from(column::<i64>(row, "position")?).map_err(backend)?,
        title: title_from_parts(&column::<String>(row, "title_kind")?, column(row, "title_id")?)?,
        added_at: from_millis(column(row, "added_at")?)?,
    })
}

fn row_to_queue_entry(row: &SqliteRow) -> Result<QueueEntry, RepositoryError> {
    Ok(QueueEntry {
        id: QueueEntryId(column::<i64>(row, "id")?.to_string()),
        title: title_from_parts(&column::<String>(row, "title_kind")?, column(row, "title_id")?)?,
    })
}

async fn touch_playlist(
    conn: &mut SqliteConnection,
    id: &PlaylistId,
    at: Timestamp,
) -> Result<(), RepositoryError> {
    sqlx::query("UPDATE playlists SET updated_at = ? WHERE id = ?")
        .bind(to_millis(at))
        .bind(id.0.as_str())
        .execute(&mut *conn)
        .await
        .map_err(backend)?;
    Ok(())
}

struct Slot {
    position: i64,
    kind: String,
    title: String,
}

fn row_to_slot(row: &SqliteRow) -> Result<Slot, RepositoryError> {
    Ok(Slot {
        position: column(row, "position")?,
        kind: column(row, "title_kind")?,
        title: column(row, "title_id")?,
    })
}

fn moved(mut order: Vec<Slot>, title_id: &str, position: u32) -> Option<Vec<Slot>> {
    let from = order.iter().position(|slot| slot.title == title_id)?;
    let to = order
        .iter()
        .position(|slot| slot.position >= i64::from(position))
        .unwrap_or(order.len() - 1);
    let slot = order.remove(from);
    order.insert(to, slot);
    Some(order)
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
        row.as_ref().map(|row| row_to_offset(user, version, subtitle, row)).transpose()
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

    async fn list_playlists(&self, user: &UserId) -> Result<Vec<Playlist>, RepositoryError> {
        let _op = DbOpGuard::new("preferences", "list_playlists");
        let pool = self.pools.get(user).await?;
        let rows = sqlx::query(
            "SELECT id, name, created_at, updated_at FROM playlists \
             ORDER BY name COLLATE NOCASE, id",
        )
        .fetch_all(&pool)
        .await
        .map_err(backend)?;
        rows.iter().map(row_to_playlist).collect()
    }

    async fn get_playlist(
        &self,
        user: &UserId,
        id: &PlaylistId,
    ) -> Result<Option<Playlist>, RepositoryError> {
        let _op = DbOpGuard::new("preferences", "get_playlist");
        let pool = self.pools.get(user).await?;
        let row =
            sqlx::query("SELECT id, name, created_at, updated_at FROM playlists WHERE id = ?")
                .bind(id.0.as_str())
                .fetch_optional(&pool)
                .await
                .map_err(backend)?;
        row.as_ref().map(row_to_playlist).transpose()
    }

    async fn save_playlist(
        &self,
        user: &UserId,
        playlist: Playlist,
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("preferences", "save_playlist");
        let pool = self.pools.get(user).await?;
        sqlx::query(
            "INSERT INTO playlists (id, name, created_at, updated_at) VALUES (?, ?, ?, ?) \
             ON CONFLICT (id) DO UPDATE SET name = excluded.name, updated_at = excluded.updated_at",
        )
        .bind(playlist.id.0.as_str())
        .bind(playlist.name.as_str())
        .bind(to_millis(playlist.created_at))
        .bind(to_millis(playlist.updated_at))
        .execute(&pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn delete_playlist(
        &self,
        user: &UserId,
        id: &PlaylistId,
    ) -> Result<bool, RepositoryError> {
        let _op = DbOpGuard::new("preferences", "delete_playlist");
        let pool = self.pools.get(user).await?;
        let mut tx = pool.begin().await.map_err(backend)?;
        sqlx::query("DELETE FROM playlist_items WHERE playlist_id = ?")
            .bind(id.0.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        let deleted = sqlx::query("DELETE FROM playlists WHERE id = ?")
            .bind(id.0.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?
            .rows_affected();
        tx.commit().await.map_err(backend)?;
        Ok(deleted > 0)
    }

    async fn list_playlist_items(
        &self,
        user: &UserId,
        id: &PlaylistId,
    ) -> Result<Vec<PlaylistItem>, RepositoryError> {
        let _op = DbOpGuard::new("preferences", "list_playlist_items");
        let pool = self.pools.get(user).await?;
        let rows = sqlx::query(
            "SELECT position, title_kind, title_id, added_at FROM playlist_items \
             WHERE playlist_id = ? ORDER BY position, title_kind, title_id",
        )
        .bind(id.0.as_str())
        .fetch_all(&pool)
        .await
        .map_err(backend)?;
        rows.iter().map(row_to_playlist_item).collect()
    }

    async fn add_playlist_items(
        &self,
        user: &UserId,
        id: &PlaylistId,
        titles: &[TitleId],
        at: Timestamp,
    ) -> Result<Option<usize>, RepositoryError> {
        let _op = DbOpGuard::new("preferences", "add_playlist_items");
        let pool = self.pools.get(user).await?;
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await.map_err(backend)?;
        let row = sqlx::query(
            "SELECT COALESCE(MAX(position), -1) + 1 AS next, \
             EXISTS (SELECT 1 FROM playlists WHERE id = ?) AS present \
             FROM playlist_items WHERE playlist_id = ?",
        )
        .bind(id.0.as_str())
        .bind(id.0.as_str())
        .fetch_one(&mut *tx)
        .await
        .map_err(backend)?;
        if !column::<bool>(&row, "present")? {
            return Ok(None);
        }
        let mut position = column::<i64>(&row, "next")?;
        let mut added = 0;
        for title in titles {
            let inserted = sqlx::query(
                "INSERT OR IGNORE INTO playlist_items \
                 (playlist_id, position, title_kind, title_id, added_at) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(id.0.as_str())
            .bind(position)
            .bind(title_kind(title))
            .bind(title.id())
            .bind(to_millis(at))
            .execute(&mut *tx)
            .await
            .map_err(backend)?
            .rows_affected();
            if inserted > 0 {
                position += 1;
                added += 1;
            }
        }
        if added > 0 {
            touch_playlist(&mut tx, id, at).await?;
        }
        tx.commit().await.map_err(backend)?;
        Ok(Some(added))
    }

    async fn remove_playlist_item(
        &self,
        user: &UserId,
        id: &PlaylistId,
        title_id: &str,
        at: Timestamp,
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("preferences", "remove_playlist_item");
        let pool = self.pools.get(user).await?;
        let mut tx = pool.begin().await.map_err(backend)?;
        let removed = sqlx::query(
            "DELETE FROM playlist_items WHERE playlist_id = ? AND title_id = ? RETURNING position",
        )
        .bind(id.0.as_str())
        .bind(title_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(backend)?;
        if let Some(row) = removed {
            sqlx::query(
                "UPDATE playlist_items SET position = position - 1 \
                 WHERE playlist_id = ? AND position > ?",
            )
            .bind(id.0.as_str())
            .bind(column::<i64>(&row, "position")?)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
            touch_playlist(&mut tx, id, at).await?;
        }
        tx.commit().await.map_err(backend)
    }

    async fn move_playlist_item(
        &self,
        user: &UserId,
        id: &PlaylistId,
        title_id: &str,
        position: u32,
        at: Timestamp,
    ) -> Result<bool, RepositoryError> {
        let _op = DbOpGuard::new("preferences", "move_playlist_item");
        let pool = self.pools.get(user).await?;
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await.map_err(backend)?;
        let rows = sqlx::query(
            "SELECT position, title_kind, title_id FROM playlist_items \
             WHERE playlist_id = ? ORDER BY position, title_kind, title_id",
        )
        .bind(id.0.as_str())
        .fetch_all(&mut *tx)
        .await
        .map_err(backend)?;
        let slots = rows.iter().map(row_to_slot).collect::<Result<Vec<_>, _>>()?;
        let Some(order) = moved(slots, title_id, position) else {
            return Ok(false);
        };
        for (index, slot) in (0_i64..).zip(&order) {
            if slot.position == index {
                continue;
            }
            sqlx::query(
                "UPDATE playlist_items SET position = ? \
                 WHERE playlist_id = ? AND title_kind = ? AND title_id = ?",
            )
            .bind(index)
            .bind(id.0.as_str())
            .bind(slot.kind.as_str())
            .bind(slot.title.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        touch_playlist(&mut tx, id, at).await?;
        tx.commit().await.map_err(backend)?;
        Ok(true)
    }

    async fn list_queue(&self, user: &UserId) -> Result<Vec<QueueEntry>, RepositoryError> {
        let _op = DbOpGuard::new("preferences", "list_queue");
        let pool = self.pools.get(user).await?;
        let rows =
            sqlx::query("SELECT id, title_kind, title_id FROM queue_items ORDER BY position, id")
                .fetch_all(&pool)
                .await
                .map_err(backend)?;
        rows.iter().map(row_to_queue_entry).collect()
    }

    async fn queue_playlist(
        &self,
        user: &UserId,
        id: &PlaylistId,
        shuffle: bool,
    ) -> Result<usize, RepositoryError> {
        let _op = DbOpGuard::new("preferences", "queue_playlist");
        let pool = self.pools.get(user).await?;
        let sql = if shuffle { QUEUE_SHUFFLED_SQL } else { QUEUE_IN_ORDER_SQL };
        let added = sqlx::query(sql)
            .bind(id.0.as_str())
            .execute(&pool)
            .await
            .map_err(backend)?
            .rows_affected() as usize;
        Ok(added)
    }

    async fn clear_queue(&self, user: &UserId) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("preferences", "clear_queue");
        let pool = self.pools.get(user).await?;
        sqlx::query("DELETE FROM queue_items").execute(&pool).await.map_err(backend)?;
        Ok(())
    }

    async fn remove_queue_entry(
        &self,
        user: &UserId,
        id: &QueueEntryId,
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("preferences", "remove_queue_entry");
        let Ok(id) = id.0.parse::<i64>() else {
            return Ok(());
        };
        let pool = self.pools.get(user).await?;
        sqlx::query("DELETE FROM queue_items WHERE id = ?")
            .bind(id)
            .execute(&pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn remove_first_queued(
        &self,
        user: &UserId,
        title: &TitleId,
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("preferences", "remove_first_queued");
        let pool = self.pools.get(user).await?;
        sqlx::query(
            "DELETE FROM queue_items WHERE id = (SELECT id FROM queue_items \
             WHERE title_kind = ? AND title_id = ? ORDER BY position, id LIMIT 1)",
        )
        .bind(title_kind(title))
        .bind(title.id())
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
        assert!(repo.get_subtitle_offset(&user, &version, &track).await.is_err());
        assert!(
            repo.set_subtitle_offset(UserSubtitleOffset {
                user: user.clone(),
                version,
                subtitle: track,
                offset_ms: 250,
            })
            .await
            .is_err()
        );
        assert!(repo.forget(&user, &TitleSet::default(), &HashSet::new()).await.is_err());

        let playlist = PlaylistId("p1".into());
        let queued = TitleId::Movie(MovieId("m1".into()));
        let at = Timestamp::UNIX_EPOCH;
        assert!(repo.list_playlists(&user).await.is_err());
        assert!(repo.get_playlist(&user, &playlist).await.is_err());
        assert!(
            repo.save_playlist(
                &user,
                Playlist { id: playlist.clone(), name: "p".into(), created_at: at, updated_at: at },
            )
            .await
            .is_err()
        );
        assert!(repo.delete_playlist(&user, &playlist).await.is_err());
        assert!(repo.list_playlist_items(&user, &playlist).await.is_err());
        let titles = std::slice::from_ref(&queued);
        assert!(repo.add_playlist_items(&user, &playlist, titles, at).await.is_err());
        assert!(repo.remove_playlist_item(&user, &playlist, "m1", at).await.is_err());
        assert!(repo.move_playlist_item(&user, &playlist, "m1", 0, at).await.is_err());
        assert!(repo.list_queue(&user).await.is_err());
        assert!(repo.queue_playlist(&user, &playlist, true).await.is_err());
        assert!(repo.clear_queue(&user).await.is_err());
        assert!(repo.remove_queue_entry(&user, &QueueEntryId("1".into())).await.is_err());
        assert!(repo.remove_first_queued(&user, &queued).await.is_err());
    }

    fn slots(stored: &[(i64, &str)]) -> Vec<Slot> {
        stored
            .iter()
            .map(|(position, title)| Slot {
                position: *position,
                kind: "movie".into(),
                title: (*title).into(),
            })
            .collect()
    }

    fn titles(order: &[Slot]) -> Vec<&str> {
        order.iter().map(|slot| slot.title.as_str()).collect()
    }

    #[test]
    fn a_move_takes_the_slot_of_the_first_item_at_or_after_its_position() {
        let gapped = [(0, "a"), (2, "b"), (5, "c")];
        assert_eq!(titles(&moved(slots(&gapped), "c", 2).unwrap()), ["a", "c", "b"]);
        assert_eq!(titles(&moved(slots(&gapped), "a", 3).unwrap()), ["b", "c", "a"]);
        assert_eq!(titles(&moved(slots(&gapped), "b", 40).unwrap()), ["a", "c", "b"]);
        assert!(moved(slots(&gapped), "z", 0).is_none());
    }

    #[tokio::test]
    async fn a_move_after_a_deleted_title_closes_the_gap() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqlitePreferencesRepo::new(dir.path());
        let user = UserId("u1".into());
        let playlist = PlaylistId("p1".into());
        let at = Timestamp::UNIX_EPOCH;
        let movie = |id: &str| TitleId::Movie(MovieId(id.into()));
        repo.save_playlist(
            &user,
            Playlist { id: playlist.clone(), name: "p".into(), created_at: at, updated_at: at },
        )
        .await
        .unwrap();
        repo.add_playlist_items(&user, &playlist, &[movie("a"), movie("b"), movie("c")], at)
            .await
            .unwrap();
        let gone = TitleSet { movies: HashSet::from([MovieId("b".into())]), ..TitleSet::default() };
        repo.forget(&user, &gone, &HashSet::new()).await.unwrap();

        assert!(repo.move_playlist_item(&user, &playlist, "c", 0, at).await.unwrap());

        let order: Vec<(u32, String)> = repo
            .list_playlist_items(&user, &playlist)
            .await
            .unwrap()
            .into_iter()
            .map(|item| (item.position, item.title.id().to_owned()))
            .collect();
        assert_eq!(order, [(0, "c".to_owned()), (1, "a".to_owned())]);
    }
}
