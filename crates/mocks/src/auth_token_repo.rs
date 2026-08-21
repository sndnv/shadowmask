use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::error::RepositoryError;
use domain::repository::AuthTokenRepository;
use domain::user::{
    ApiToken, ApiTokenId, AuthSession, AuthSessionId, Device, DeviceId, PendingLink, UserId,
};
use jiff::Timestamp;

#[derive(Default)]
struct State {
    refresh: HashMap<AuthSessionId, AuthSession>,
    links: HashMap<String, PendingLink>,
    devices: HashMap<DeviceId, Device>,
    api_tokens: HashMap<String, ApiToken>,
    purged: Vec<UserId>,
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

    pub fn purged(&self) -> Vec<UserId> {
        self.state.lock().unwrap().purged.clone()
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
    async fn purge_user(&self, user: &UserId) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        state.refresh.retain(|_, s| &s.user != user);
        state.links.retain(|_, l| &l.user != user);
        state.api_tokens.retain(|_, t| &t.user != user);
        state.devices.retain(|_, d| &d.user != user);
        state.purged.push(user.clone());
        Ok(())
    }

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

    async fn list_link_codes(
        &self,
        user: &UserId,
        now: Timestamp,
    ) -> Result<Vec<PendingLink>, RepositoryError> {
        self.guard()?;
        let mut links: Vec<PendingLink> = self
            .state
            .lock()
            .unwrap()
            .links
            .values()
            .filter(|link| &link.user == user && link.expires_at > now)
            .cloned()
            .collect();
        links.sort_by(|a, b| a.code.cmp(&b.code));
        Ok(links)
    }

    async fn delete_link_code(&self, code: &str, user: &UserId) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .links
            .retain(|_, link| !(link.code == code && &link.user == user));
        Ok(())
    }

    async fn upsert_device(&self, device: Device) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        let mut device = device;
        if let Some(existing) = state.devices.get(&device.id) {
            device.created_at = existing.created_at;
        }
        state.devices.insert(device.id.clone(), device);
        Ok(())
    }

    async fn get_device(&self, id: &DeviceId) -> Result<Option<Device>, RepositoryError> {
        self.guard()?;
        Ok(self.state.lock().unwrap().devices.get(id).cloned())
    }

    async fn store_api_token(&self, token: ApiToken) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .api_tokens
            .insert(token.token_hash.clone(), token);
        Ok(())
    }

    async fn find_api_token_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<ApiToken>, RepositoryError> {
        self.guard()?;
        Ok(self.state.lock().unwrap().api_tokens.get(hash).cloned())
    }

    async fn touch_api_token(
        &self,
        id: &ApiTokenId,
        now: Timestamp,
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        if let Some(token) = self
            .state
            .lock()
            .unwrap()
            .api_tokens
            .values_mut()
            .find(|token| &token.id == id)
        {
            token.last_used_at = Some(now);
        }
        Ok(())
    }

    async fn list_devices(&self, user: &UserId) -> Result<Vec<Device>, RepositoryError> {
        self.guard()?;
        let mut devices: Vec<Device> = self
            .state
            .lock()
            .unwrap()
            .devices
            .values()
            .filter(|device| &device.user == user)
            .cloned()
            .collect();
        devices.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        Ok(devices)
    }

    async fn list_api_tokens(&self, user: &UserId) -> Result<Vec<ApiToken>, RepositoryError> {
        self.guard()?;
        let mut tokens: Vec<ApiToken> = self
            .state
            .lock()
            .unwrap()
            .api_tokens
            .values()
            .filter(|token| &token.user == user)
            .cloned()
            .collect();
        tokens.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        Ok(tokens)
    }

    async fn delete_device(&self, id: &DeviceId) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state.lock().unwrap().devices.remove(id);
        Ok(())
    }

    async fn revoke_api_token(&self, id: &ApiTokenId) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .api_tokens
            .retain(|_, token| &token.id != id);
        Ok(())
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

    fn device(id: &str) -> Device {
        Device {
            id: DeviceId(id.to_owned()),
            user: UserId("u1".to_owned()),
            name: "Roku".to_owned(),
            platform: "roku".to_owned(),
            created_at: Timestamp::UNIX_EPOCH,
            last_seen: Some(Timestamp::UNIX_EPOCH),
        }
    }

    fn api_token(id: &str, hash: &str) -> ApiToken {
        ApiToken {
            id: ApiTokenId(id.to_owned()),
            user: UserId("u1".to_owned()),
            device: DeviceId("d1".to_owned()),
            token_hash: hash.to_owned(),
            created_at: Timestamp::UNIX_EPOCH,
            last_used_at: None,
        }
    }

    #[tokio::test]
    async fn list_and_revoke_scoped_by_user() {
        let repo = MockAuthTokenRepo::new();
        repo.upsert_device(device("d1")).await.unwrap();
        repo.upsert_device(Device {
            user: UserId("u2".into()),
            ..device("d2")
        })
        .await
        .unwrap();
        repo.store_api_token(api_token("t1", "h1")).await.unwrap();
        repo.store_api_token(ApiToken {
            user: UserId("u2".into()),
            ..api_token("t2", "h2")
        })
        .await
        .unwrap();

        let u1 = UserId("u1".into());
        assert_eq!(repo.list_devices(&u1).await.unwrap().len(), 1);
        let tokens = repo.list_api_tokens(&u1).await.unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].id, ApiTokenId("t1".into()));

        repo.revoke_api_token(&ApiTokenId("t1".into()))
            .await
            .unwrap();
        assert!(repo.list_api_tokens(&u1).await.unwrap().is_empty());
        repo.revoke_api_token(&ApiTokenId("t1".into()))
            .await
            .unwrap();

        repo.delete_device(&DeviceId("d1".into())).await.unwrap();
        assert!(repo.list_devices(&u1).await.unwrap().is_empty());
        repo.delete_device(&DeviceId("d1".into())).await.unwrap();

        let u2 = UserId("u2".into());
        assert_eq!(repo.list_devices(&u2).await.unwrap().len(), 1);
        assert_eq!(repo.list_api_tokens(&u2).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn device_and_api_token_round_trip() {
        let repo = MockAuthTokenRepo::new();
        repo.upsert_device(device("d1")).await.unwrap();
        assert_eq!(
            repo.get_device(&DeviceId("d1".into()))
                .await
                .unwrap()
                .unwrap()
                .name,
            "Roku"
        );
        assert!(
            repo.get_device(&DeviceId("missing".into()))
                .await
                .unwrap()
                .is_none()
        );

        repo.store_api_token(api_token("t1", "hash-1"))
            .await
            .unwrap();
        assert_eq!(
            repo.find_api_token_by_hash("hash-1")
                .await
                .unwrap()
                .unwrap()
                .id
                .0,
            "t1"
        );
        assert!(repo.find_api_token_by_hash("nope").await.unwrap().is_none());
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
    async fn lists_live_codes_per_user_and_deletes_scoped_to_owner() {
        let repo = MockAuthTokenRepo::new();
        let future = Timestamp::from_second(4_000_000_000).unwrap();
        let past = Timestamp::UNIX_EPOCH;
        let now = Timestamp::from_second(1_000_000_000).unwrap();
        repo.store_link_code(link("LIVE", future)).await.unwrap();
        repo.store_link_code(link("DEAD", past)).await.unwrap();
        let mut other = link("OTHER", future);
        other.user = UserId("u2".into());
        repo.store_link_code(other).await.unwrap();

        let live = repo
            .list_link_codes(&UserId("u1".into()), now)
            .await
            .unwrap();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].code, "LIVE");

        repo.delete_link_code("LIVE", &UserId("u2".into()))
            .await
            .unwrap();
        assert_eq!(
            repo.list_link_codes(&UserId("u1".into()), now)
                .await
                .unwrap()
                .len(),
            1
        );
        repo.delete_link_code("LIVE", &UserId("u1".into()))
            .await
            .unwrap();
        assert!(
            repo.list_link_codes(&UserId("u1".into()), now)
                .await
                .unwrap()
                .is_empty()
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
        assert!(
            repo.list_link_codes(&UserId("u1".into()), Timestamp::UNIX_EPOCH)
                .await
                .is_err()
        );
        assert!(
            repo.delete_link_code("X", &UserId("u1".into()))
                .await
                .is_err()
        );
        assert!(repo.upsert_device(device("d1")).await.is_err());
        assert!(repo.get_device(&DeviceId("d1".into())).await.is_err());
        assert!(repo.store_api_token(api_token("t1", "h")).await.is_err());
        assert!(repo.find_api_token_by_hash("h").await.is_err());
        assert!(
            repo.touch_api_token(&ApiTokenId("t1".into()), Timestamp::UNIX_EPOCH)
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
