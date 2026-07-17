use std::path::Path;

use domain::error::RepositoryError;
use domain::repository::AuthTokenRepository;
use domain::user::{
    ApiToken, ApiTokenId, AuthSession, AuthSessionId, Device, DeviceId, PendingLink, UserId,
};
use jiff::Timestamp;
use sqlx::SqlitePool;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteRow;

use crate::codec::{role_from_str, role_to_str};
use crate::metrics::DbOpGuard;
use crate::pool::{backend, checkpoint, column, from_millis, open, ping, to_millis};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/auth");

#[derive(Clone)]
pub struct SqliteAuthTokenRepo {
    pool: SqlitePool,
}

impl SqliteAuthTokenRepo {
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
}

fn row_to_session(row: &SqliteRow) -> Result<AuthSession, RepositoryError> {
    Ok(AuthSession {
        id: AuthSessionId(column(row, "jti")?),
        user: UserId(column(row, "user_id")?),
        refresh_token_hash: column(row, "token_hash")?,
        issued_at: from_millis(column(row, "issued_at")?)?,
        expires_at: from_millis(column(row, "expires_at")?)?,
    })
}

fn row_to_link(row: &SqliteRow) -> Result<PendingLink, RepositoryError> {
    Ok(PendingLink {
        code: column(row, "code")?,
        user: UserId(column(row, "user_id")?),
        role: role_from_str(&column::<String>(row, "role")?)?,
        expires_at: from_millis(column(row, "expires_at")?)?,
    })
}

fn row_to_device(row: &SqliteRow) -> Result<Device, RepositoryError> {
    let last_seen = column::<Option<i64>>(row, "last_seen")?
        .map(from_millis)
        .transpose()?;
    Ok(Device {
        id: DeviceId(column(row, "id")?),
        user: UserId(column(row, "user_id")?),
        name: column(row, "name")?,
        platform: column(row, "platform")?,
        created_at: from_millis(column(row, "created_at")?)?,
        last_seen,
    })
}

fn row_to_api_token(row: &SqliteRow) -> Result<ApiToken, RepositoryError> {
    Ok(ApiToken {
        id: ApiTokenId(column(row, "id")?),
        user: UserId(column(row, "user_id")?),
        device: DeviceId(column(row, "device_id")?),
        token_hash: column(row, "token_hash")?,
        created_at: from_millis(column(row, "created_at")?)?,
        last_used_at: column::<Option<i64>>(row, "last_used_at")?
            .map(from_millis)
            .transpose()?,
    })
}

impl AuthTokenRepository for SqliteAuthTokenRepo {
    async fn store_refresh(&self, session: AuthSession) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("auth", "store_refresh");
        sqlx::query(
            "INSERT OR REPLACE INTO refresh_tokens \
             (jti, user_id, token_hash, issued_at, expires_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(session.id.0.as_str())
        .bind(session.user.0.as_str())
        .bind(session.refresh_token_hash.as_str())
        .bind(to_millis(session.issued_at))
        .bind(to_millis(session.expires_at))
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn find_refresh(
        &self,
        jti: &AuthSessionId,
    ) -> Result<Option<AuthSession>, RepositoryError> {
        let _op = DbOpGuard::new("auth", "find_refresh");
        let row = sqlx::query("SELECT * FROM refresh_tokens WHERE jti = ?")
            .bind(jti.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref().map(row_to_session).transpose()
    }

    async fn revoke_refresh(&self, jti: &AuthSessionId) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("auth", "revoke_refresh");
        sqlx::query("DELETE FROM refresh_tokens WHERE jti = ?")
            .bind(jti.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn revoke_all_for_user(&self, user: &UserId) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("auth", "revoke_all_for_user");
        sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
            .bind(user.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn store_link_code(&self, link: PendingLink) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("auth", "store_link_code");
        sqlx::query(
            "INSERT OR REPLACE INTO link_codes (code, user_id, role, expires_at) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(link.code.as_str())
        .bind(link.user.0.as_str())
        .bind(role_to_str(link.role))
        .bind(to_millis(link.expires_at))
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn redeem_link_code(
        &self,
        code: &str,
        now: Timestamp,
    ) -> Result<Option<PendingLink>, RepositoryError> {
        let _op = DbOpGuard::new("auth", "redeem_link_code");
        let row =
            sqlx::query("DELETE FROM link_codes WHERE code = ? AND expires_at > ? RETURNING *")
                .bind(code)
                .bind(to_millis(now))
                .fetch_optional(&self.pool)
                .await
                .map_err(backend)?;
        row.as_ref().map(row_to_link).transpose()
    }

    async fn list_link_codes(
        &self,
        user: &UserId,
        now: Timestamp,
    ) -> Result<Vec<PendingLink>, RepositoryError> {
        let _op = DbOpGuard::new("auth", "list_link_codes");
        let rows = sqlx::query(
            "SELECT * FROM link_codes WHERE user_id = ? AND expires_at > ? ORDER BY expires_at",
        )
        .bind(user.0.as_str())
        .bind(to_millis(now))
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter().map(row_to_link).collect()
    }

    async fn delete_link_code(&self, code: &str, user: &UserId) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("auth", "delete_link_code");
        sqlx::query("DELETE FROM link_codes WHERE code = ? AND user_id = ?")
            .bind(code)
            .bind(user.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn upsert_device(&self, device: Device) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("auth", "upsert_device");
        sqlx::query(
            "INSERT OR REPLACE INTO devices (id, user_id, name, platform, created_at, last_seen) \
             VALUES (?, ?, ?, ?, COALESCE((SELECT created_at FROM devices WHERE id = ?), ?), ?)",
        )
        .bind(device.id.0.as_str())
        .bind(device.user.0.as_str())
        .bind(device.name.as_str())
        .bind(device.platform.as_str())
        .bind(device.id.0.as_str())
        .bind(to_millis(device.created_at))
        .bind(device.last_seen.map(to_millis))
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn get_device(&self, id: &DeviceId) -> Result<Option<Device>, RepositoryError> {
        let _op = DbOpGuard::new("auth", "get_device");
        let row = sqlx::query("SELECT * FROM devices WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref().map(row_to_device).transpose()
    }

    async fn store_api_token(&self, token: ApiToken) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("auth", "store_api_token");
        sqlx::query(
            "INSERT OR REPLACE INTO api_tokens \
             (id, user_id, device_id, token_hash, created_at, last_used_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(token.id.0.as_str())
        .bind(token.user.0.as_str())
        .bind(token.device.0.as_str())
        .bind(token.token_hash.as_str())
        .bind(to_millis(token.created_at))
        .bind(token.last_used_at.map(to_millis))
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn find_api_token_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<ApiToken>, RepositoryError> {
        let _op = DbOpGuard::new("auth", "find_api_token_by_hash");
        let row = sqlx::query("SELECT * FROM api_tokens WHERE token_hash = ?")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref().map(row_to_api_token).transpose()
    }

    async fn touch_api_token(
        &self,
        id: &ApiTokenId,
        now: Timestamp,
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("auth", "touch_api_token");
        sqlx::query("UPDATE api_tokens SET last_used_at = ? WHERE id = ?")
            .bind(to_millis(now))
            .bind(id.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn list_devices(&self, user: &UserId) -> Result<Vec<Device>, RepositoryError> {
        let _op = DbOpGuard::new("auth", "list_devices");
        let rows = sqlx::query("SELECT * FROM devices WHERE user_id = ? ORDER BY id")
            .bind(user.0.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        rows.iter().map(row_to_device).collect()
    }

    async fn list_api_tokens(&self, user: &UserId) -> Result<Vec<ApiToken>, RepositoryError> {
        let _op = DbOpGuard::new("auth", "list_api_tokens");
        let rows = sqlx::query("SELECT * FROM api_tokens WHERE user_id = ? ORDER BY id")
            .bind(user.0.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        rows.iter().map(row_to_api_token).collect()
    }

    async fn delete_device(&self, id: &DeviceId) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("auth", "delete_device");
        sqlx::query("DELETE FROM devices WHERE id = ?")
            .bind(id.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn revoke_api_token(&self, id: &ApiTokenId) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("auth", "revoke_api_token");
        sqlx::query("DELETE FROM api_tokens WHERE id = ?")
            .bind(id.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn surfaces_backend_error_after_close() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteAuthTokenRepo::connect(&dir.path().join("auth.db"))
            .await
            .unwrap();
        repo.pool.close().await;
        assert!(repo.find_refresh(&AuthSessionId("x".into())).await.is_err());
        assert!(
            repo.list_link_codes(&UserId("u1".into()), Timestamp::UNIX_EPOCH)
                .await
                .is_err()
        );
        assert!(
            repo.delete_link_code("x", &UserId("u1".into()))
                .await
                .is_err()
        );
        assert!(repo.list_devices(&UserId("u1".into())).await.is_err());
        assert!(repo.list_api_tokens(&UserId("u1".into())).await.is_err());
        assert!(repo.delete_device(&DeviceId("d1".into())).await.is_err());
        assert!(
            repo.revoke_api_token(&ApiTokenId("t1".into()))
                .await
                .is_err()
        );
    }
}
