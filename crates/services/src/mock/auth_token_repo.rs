use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::error::RepositoryError;
use domain::repository::AuthTokenRepository;
use domain::user::{AuthSession, AuthSessionId, PendingLink, UserId};
use jiff::Timestamp;

#[derive(Default)]
struct State {
    refresh: HashMap<AuthSessionId, AuthSession>,
    links: HashMap<String, PendingLink>,
}

#[derive(Clone, Default)]
pub struct MockAuthTokenRepo {
    state: Arc<Mutex<State>>,
    fail: Arc<AtomicBool>,
}

impl MockAuthTokenRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }

    fn guard(&self) -> Result<(), RepositoryError> {
        if self.fail.load(Ordering::Relaxed) {
            Err(RepositoryError::Backend(
                "mock auth token failure".to_owned(),
            ))
        } else {
            Ok(())
        }
    }
}

impl AuthTokenRepository for MockAuthTokenRepo {
    async fn store_refresh(&self, session: AuthSession) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .refresh
            .insert(session.id.clone(), session);
        Ok(())
    }

    async fn find_refresh(
        &self,
        jti: &AuthSessionId,
    ) -> Result<Option<AuthSession>, RepositoryError> {
        self.guard()?;
        Ok(self.state.lock().unwrap().refresh.get(jti).cloned())
    }

    async fn revoke_refresh(&self, jti: &AuthSessionId) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state.lock().unwrap().refresh.remove(jti);
        Ok(())
    }

    async fn revoke_all_for_user(&self, user: &UserId) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .refresh
            .retain(|_, session| &session.user != user);
        Ok(())
    }

    async fn store_link_code(&self, link: PendingLink) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .links
            .insert(link.code.clone(), link);
        Ok(())
    }

    async fn redeem_link_code(
        &self,
        code: &str,
        now: Timestamp,
    ) -> Result<Option<PendingLink>, RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        match state.links.get(code) {
            Some(link) if link.expires_at > now => Ok(state.links.remove(code)),
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::user::Role;

    fn session(jti: &str, user: &str) -> AuthSession {
        AuthSession {
            id: AuthSessionId(jti.to_owned()),
            user: UserId(user.to_owned()),
            refresh_token_hash: "hash".to_owned(),
            issued_at: Timestamp::UNIX_EPOCH,
            expires_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn link(code: &str, expires_at: Timestamp) -> PendingLink {
        PendingLink {
            code: code.to_owned(),
            user: UserId("u1".to_owned()),
            role: Role::Player,
            expires_at,
        }
    }

    #[tokio::test]
    async fn refresh_store_find_revoke() {
        let repo = MockAuthTokenRepo::new();
        repo.store_refresh(session("j1", "u1")).await.unwrap();
        repo.store_refresh(session("j2", "u1")).await.unwrap();
        repo.store_refresh(session("j3", "u2")).await.unwrap();

        assert!(
            repo.find_refresh(&AuthSessionId("j1".into()))
                .await
                .unwrap()
                .is_some()
        );
        repo.revoke_refresh(&AuthSessionId("j1".into()))
            .await
            .unwrap();
        assert!(
            repo.find_refresh(&AuthSessionId("j1".into()))
                .await
                .unwrap()
                .is_none()
        );

        repo.revoke_all_for_user(&UserId("u1".into()))
            .await
            .unwrap();
        assert!(
            repo.find_refresh(&AuthSessionId("j2".into()))
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            repo.find_refresh(&AuthSessionId("j3".into()))
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn link_code_redeemed_once_and_respects_expiry() {
        let repo = MockAuthTokenRepo::new();
        let future = Timestamp::from_second(4_000_000_000).unwrap();
        let past = Timestamp::UNIX_EPOCH;
        let now = Timestamp::from_second(1_000_000_000).unwrap();

        repo.store_link_code(link("LIVE", future)).await.unwrap();
        repo.store_link_code(link("DEAD", past)).await.unwrap();

        assert!(repo.redeem_link_code("LIVE", now).await.unwrap().is_some());
        assert!(repo.redeem_link_code("LIVE", now).await.unwrap().is_none());
        assert!(repo.redeem_link_code("DEAD", now).await.unwrap().is_none());
        assert!(
            repo.redeem_link_code("MISSING", now)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn surfaces_backend_failure() {
        let repo = MockAuthTokenRepo::new();
        repo.set_fail();
        assert!(repo.store_refresh(session("j1", "u1")).await.is_err());
        assert!(
            repo.find_refresh(&AuthSessionId("j1".into()))
                .await
                .is_err()
        );
        assert!(
            repo.revoke_refresh(&AuthSessionId("j1".into()))
                .await
                .is_err()
        );
        assert!(
            repo.revoke_all_for_user(&UserId("u1".into()))
                .await
                .is_err()
        );
        assert!(
            repo.store_link_code(link("X", Timestamp::UNIX_EPOCH))
                .await
                .is_err()
        );
        assert!(
            repo.redeem_link_code("X", Timestamp::UNIX_EPOCH)
                .await
                .is_err()
        );
    }
}
