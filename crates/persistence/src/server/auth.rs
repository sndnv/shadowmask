use std::path::Path;

use domain::error::RepositoryError;
use domain::repository::AuthTokenRepository;
use domain::user::{AuthSession, AuthSessionId, PendingLink, UserId};
use jiff::Timestamp;
use sqlx::SqlitePool;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteRow;

use crate::codec::{role_from_str, role_to_str};
use crate::pool::{backend, column, from_millis, open, to_millis};

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

    pub async fn close(&self) {
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

impl AuthTokenRepository for SqliteAuthTokenRepo {
    async fn store_refresh(&self, session: AuthSession) -> Result<(), RepositoryError> {
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
        let row = sqlx::query("SELECT * FROM refresh_tokens WHERE jti = ?")
            .bind(jti.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref().map(row_to_session).transpose()
    }

    async fn revoke_refresh(&self, jti: &AuthSessionId) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM refresh_tokens WHERE jti = ?")
            .bind(jti.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn revoke_all_for_user(&self, user: &UserId) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
            .bind(user.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn store_link_code(&self, link: PendingLink) -> Result<(), RepositoryError> {
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
        let row =
            sqlx::query("DELETE FROM link_codes WHERE code = ? AND expires_at > ? RETURNING *")
                .bind(code)
                .bind(to_millis(now))
                .fetch_optional(&self.pool)
                .await
                .map_err(backend)?;
        row.as_ref().map(row_to_link).transpose()
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
    }
}
