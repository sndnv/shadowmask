use std::path::Path;

use domain::common::{LanguageCode, Page, PageRequest};
use domain::error::RepositoryError;
use domain::library::LibraryId;
use domain::metadata::ContentRating;
use domain::repository::UserRepository;
use domain::user::{LibraryAccess, User, UserId};
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteRow;
use sqlx::{SqliteConnection, SqlitePool};

use crate::codec::{role_from_str, role_to_str};
use crate::metrics::DbOpGuard;
use crate::pool::{backend, checkpoint, column, from_millis, open, ping, to_millis};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/users");

#[derive(Clone)]
pub struct SqliteUserRepo {
    pool: SqlitePool,
}

impl SqliteUserRepo {
    pub async fn connect(path: &Path) -> Result<Self, RepositoryError> {
        Ok(Self {
            pool: open(path, &MIGRATOR).await?,
        })
    }

    pub async fn ping(&self) -> Result<(), RepositoryError> {
        ping(&self.pool).await
    }

    pub async fn close(&self) {
        checkpoint(&self.pool).await;
        self.pool.close().await;
    }

    async fn langs(
        &self,
        query: &'static str,
        key: &str,
    ) -> Result<Vec<LanguageCode>, RepositoryError> {
        let rows = sqlx::query(query)
            .bind(key)
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        rows.iter().map(row_to_lang).collect()
    }

    async fn load_user(&self, row: &SqliteRow) -> Result<User, RepositoryError> {
        let id: String = column(row, "id")?;
        let preferred_audio = self
            .langs(
                "SELECT lang AS value FROM user_preferred_audio WHERE user_id = ? ORDER BY ordinal",
                &id,
            )
            .await?;
        let preferred_subtitle = self
            .langs(
                "SELECT lang AS value FROM user_preferred_subtitle \
                 WHERE user_id = ? ORDER BY ordinal",
                &id,
            )
            .await?;
        let max_content_rating = match (
            column::<Option<String>>(row, "max_rating_system")?,
            column::<Option<String>>(row, "max_rating_code")?,
        ) {
            (Some(system), Some(code)) => Some(ContentRating { system, code }),
            _ => None,
        };
        Ok(User {
            username: column(row, "username")?,
            password_hash: column(row, "password_hash")?,
            role: role_from_str(&column::<String>(row, "role")?)?,
            max_content_rating,
            preferred_audio,
            preferred_subtitle,
            concurrent_stream_limit: column::<Option<i64>>(row, "concurrent_stream_limit")?
                .map(|value| value as u32),
            bitrate_cap: column::<Option<i64>>(row, "bitrate_cap")?.map(|value| value as u64),
            active: column::<i64>(row, "active")? != 0,
            created_at: from_millis(column(row, "created_at")?)?,
            updated_at: from_millis(column(row, "updated_at")?)?,
            id: UserId(id),
        })
    }
}

fn row_to_lang(row: &SqliteRow) -> Result<LanguageCode, RepositoryError> {
    Ok(LanguageCode(column::<String>(row, "value")?))
}

fn insert_error(error: sqlx::Error) -> RepositoryError {
    match &error {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            RepositoryError::Conflict("username already exists".to_owned())
        }
        _ => backend(error),
    }
}

async fn replace_prefs(conn: &mut SqliteConnection, user: &User) -> Result<(), RepositoryError> {
    let id = user.id.0.as_str();
    sqlx::query("DELETE FROM user_preferred_audio WHERE user_id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await
        .map_err(backend)?;
    for (ordinal, lang) in user.preferred_audio.iter().enumerate() {
        sqlx::query("INSERT INTO user_preferred_audio (user_id, ordinal, lang) VALUES (?, ?, ?)")
            .bind(id)
            .bind(ordinal as i64)
            .bind(lang.0.as_str())
            .execute(&mut *conn)
            .await
            .map_err(backend)?;
    }
    sqlx::query("DELETE FROM user_preferred_subtitle WHERE user_id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await
        .map_err(backend)?;
    for (ordinal, lang) in user.preferred_subtitle.iter().enumerate() {
        sqlx::query(
            "INSERT INTO user_preferred_subtitle (user_id, ordinal, lang) VALUES (?, ?, ?)",
        )
        .bind(id)
        .bind(ordinal as i64)
        .bind(lang.0.as_str())
        .execute(&mut *conn)
        .await
        .map_err(backend)?;
    }
    Ok(())
}

impl UserRepository for SqliteUserRepo {
    async fn create(&self, user: User) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("users", "create");
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query(
            "INSERT INTO users \
             (id, username, password_hash, role, max_rating_system, max_rating_code, \
              concurrent_stream_limit, bitrate_cap, active, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(user.id.0.as_str())
        .bind(user.username.as_str())
        .bind(user.password_hash.as_str())
        .bind(role_to_str(user.role))
        .bind(user.max_content_rating.as_ref().map(|r| r.system.as_str()))
        .bind(user.max_content_rating.as_ref().map(|r| r.code.as_str()))
        .bind(user.concurrent_stream_limit.map(|value| value as i64))
        .bind(user.bitrate_cap.map(|value| value as i64))
        .bind(i64::from(user.active))
        .bind(to_millis(user.created_at))
        .bind(to_millis(user.updated_at))
        .execute(&mut *tx)
        .await
        .map_err(insert_error)?;
        replace_prefs(&mut tx, &user).await?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn get(&self, id: &UserId) -> Result<Option<User>, RepositoryError> {
        let _op = DbOpGuard::new("users", "get");
        let row = sqlx::query("SELECT * FROM users WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        match row {
            Some(row) => Ok(Some(self.load_user(&row).await?)),
            None => Ok(None),
        }
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, RepositoryError> {
        let _op = DbOpGuard::new("users", "find_by_username");
        let row = sqlx::query("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        match row {
            Some(row) => Ok(Some(self.load_user(&row).await?)),
            None => Ok(None),
        }
    }

    async fn list(&self, page: PageRequest) -> Result<Page<User>, RepositoryError> {
        let _op = DbOpGuard::new("users", "list");
        let count_row = sqlx::query("SELECT COUNT(*) AS n FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(backend)?;
        let total = column::<i64>(&count_row, "n")? as u64;
        let rows =
            sqlx::query("SELECT * FROM users ORDER BY created_at ASC, id ASC LIMIT ? OFFSET ?")
                .bind(i64::from(page.limit))
                .bind(i64::from(page.offset))
                .fetch_all(&self.pool)
                .await
                .map_err(backend)?;
        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            items.push(self.load_user(row).await?);
        }
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
    }

    async fn update(&self, user: User) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("users", "update");
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query(
            "UPDATE users SET \
             username = ?, password_hash = ?, role = ?, max_rating_system = ?, \
             max_rating_code = ?, concurrent_stream_limit = ?, bitrate_cap = ?, active = ?, \
             created_at = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(user.username.as_str())
        .bind(user.password_hash.as_str())
        .bind(role_to_str(user.role))
        .bind(user.max_content_rating.as_ref().map(|r| r.system.as_str()))
        .bind(user.max_content_rating.as_ref().map(|r| r.code.as_str()))
        .bind(user.concurrent_stream_limit.map(|value| value as i64))
        .bind(user.bitrate_cap.map(|value| value as i64))
        .bind(i64::from(user.active))
        .bind(to_millis(user.created_at))
        .bind(to_millis(user.updated_at))
        .bind(user.id.0.as_str())
        .execute(&mut *tx)
        .await
        .map_err(insert_error)?;
        replace_prefs(&mut tx, &user).await?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn delete(&self, id: &UserId) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("users", "delete");
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn revoke_library_access(&self, library: &LibraryId) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("users", "revoke_library_access");
        sqlx::query("DELETE FROM user_library_access WHERE library_id = ?")
            .bind(library.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn list_library_access(
        &self,
        id: &UserId,
    ) -> Result<Vec<LibraryAccess>, RepositoryError> {
        let _op = DbOpGuard::new("users", "list_library_access");
        let rows = sqlx::query(
            "SELECT library_id FROM user_library_access WHERE user_id = ? ORDER BY library_id",
        )
        .bind(id.0.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter()
            .map(|row| {
                Ok(LibraryAccess {
                    user: id.clone(),
                    library: LibraryId(column::<String>(row, "library_id")?),
                })
            })
            .collect()
    }

    async fn set_library_access(
        &self,
        id: &UserId,
        libraries: &[LibraryId],
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("users", "set_library_access");
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query("DELETE FROM user_library_access WHERE user_id = ?")
            .bind(id.0.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        for library in libraries {
            sqlx::query("INSERT INTO user_library_access (user_id, library_id) VALUES (?, ?)")
                .bind(id.0.as_str())
                .bind(library.0.as_str())
                .execute(&mut *tx)
                .await
                .map_err(backend)?;
        }
        tx.commit().await.map_err(backend)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::user::Role;
    use jiff::Timestamp;

    #[test]
    fn non_database_errors_map_to_backend() {
        assert!(matches!(
            insert_error(sqlx::Error::PoolClosed),
            RepositoryError::Backend(_)
        ));
    }

    #[tokio::test]
    async fn surfaces_backend_error_after_close() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteUserRepo::connect(&dir.path().join("users.db"))
            .await
            .unwrap();
        repo.pool.close().await;
        let id = UserId("u1".into());
        assert!(
            repo.list(PageRequest {
                offset: 0,
                limit: 10
            })
            .await
            .is_err()
        );
        assert!(
            repo.revoke_library_access(&LibraryId("lib1".into()))
                .await
                .is_err()
        );
        assert!(repo.create(account("u1")).await.is_err());
        assert!(repo.get(&id).await.is_err());
        assert!(repo.find_by_username("u1").await.is_err());
        assert!(repo.update(account("u1")).await.is_err());
        assert!(repo.delete(&id).await.is_err());
        assert!(repo.list_library_access(&id).await.is_err());
        assert!(
            repo.set_library_access(&id, &[LibraryId("lib1".into())])
                .await
                .is_err()
        );
    }

    fn account(id: &str) -> User {
        User {
            id: UserId(id.into()),
            username: id.into(),
            password_hash: "hash".into(),
            role: Role::User,
            max_content_rating: None,
            preferred_audio: Vec::new(),
            preferred_subtitle: Vec::new(),
            concurrent_stream_limit: None,
            bitrate_cap: None,
            active: true,
            created_at: from_millis(0).unwrap(),
            updated_at: from_millis(0).unwrap(),
        }
    }

    async fn seeded(path: &Path) -> SqliteUserRepo {
        let repo = SqliteUserRepo::connect(&path.join("users.db"))
            .await
            .unwrap();
        for id in ["u1", "u2"] {
            repo.create(User {
                id: UserId(id.into()),
                username: id.into(),
                password_hash: "hash".into(),
                role: Role::User,
                max_content_rating: None,
                preferred_audio: Vec::new(),
                preferred_subtitle: Vec::new(),
                concurrent_stream_limit: None,
                bitrate_cap: None,
                active: true,
                created_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();
        }
        repo
    }

    #[tokio::test]
    async fn active_defaults_to_true_and_survives_a_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let repo = seeded(dir.path()).await;
        let id = UserId("u1".into());
        assert!(repo.get(&id).await.unwrap().unwrap().active);

        let mut user = repo.get(&id).await.unwrap().unwrap();
        user.active = false;
        repo.update(user).await.unwrap();

        assert!(!repo.get(&id).await.unwrap().unwrap().active);
        assert!(
            repo.get(&UserId("u2".into()))
                .await
                .unwrap()
                .unwrap()
                .active
        );
    }

    #[tokio::test]
    async fn revoking_a_library_clears_it_for_everyone_who_had_it() {
        let dir = tempfile::tempdir().unwrap();
        let repo = seeded(dir.path()).await;
        let dead = LibraryId("lib1".into());
        let kept = LibraryId("lib2".into());
        for id in ["u1", "u2"] {
            repo.set_library_access(&UserId(id.into()), &[dead.clone(), kept.clone()])
                .await
                .unwrap();
        }

        repo.revoke_library_access(&dead).await.unwrap();

        for id in ["u1", "u2"] {
            let granted: Vec<String> = repo
                .list_library_access(&UserId(id.into()))
                .await
                .unwrap()
                .into_iter()
                .map(|a| a.library.0)
                .collect();
            assert_eq!(granted, vec!["lib2".to_owned()]);
        }
    }
}
