use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::error::AuthError;
use domain::service::AuthService;
use domain::user::{
    ApiToken, ApiTokenId, Device, DeviceId, DeviceRegistration, IssuedToken, PendingLink,
    Principal, Role, TokenPair, UserId,
};
use jiff::Timestamp;

#[derive(Debug, Clone)]
struct Account {
    username: String,
    password: String,
    user: UserId,
    role: Role,
}

#[derive(Debug, Default)]
struct State {
    accounts: Vec<Account>,
    link_codes: HashMap<String, IssuedToken>,
    pending: HashMap<String, PendingLink>,
}

#[derive(Clone, Default)]
pub struct MockAuthService {
    state: Arc<Mutex<State>>,
}

impl MockAuthService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_account(&self, username: &str, password: &str, user: UserId, role: Role) {
        self.state.lock().unwrap().accounts.push(Account {
            username: username.to_string(),
            password: password.to_string(),
            user,
            role,
        });
    }

    pub fn add_link_code(&self, code: &str, token: IssuedToken) {
        self.state.lock().unwrap().link_codes.insert(code.to_string(), token);
    }
}

impl AuthService for MockAuthService {
    async fn login(&self, username: &str, password: &str) -> Result<TokenPair, AuthError> {
        if password.trim().is_empty() {
            return Err(AuthError::InvalidCredentials);
        }
        let state = self.state.lock().unwrap();
        let account = state
            .accounts
            .iter()
            .find(|a| a.username == username)
            .ok_or(AuthError::InvalidCredentials)?;
        if account.password != password {
            return Err(AuthError::InvalidCredentials);
        }
        Ok(TokenPair {
            access_token: format!("access:{}", account.user.0),
            refresh_token: format!("refresh:{}", account.user.0),
        })
    }

    async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, AuthError> {
        let id = refresh_token.strip_prefix("refresh:").ok_or(AuthError::InvalidToken)?;
        let state = self.state.lock().unwrap();
        if !state.accounts.iter().any(|a| a.user.0 == id) {
            return Err(AuthError::InvalidToken);
        }
        Ok(TokenPair {
            access_token: format!("access:{id}"),
            refresh_token: format!("refresh:{id}"),
        })
    }

    async fn redeem_link_code(
        &self,
        code: &str,
        _device: DeviceRegistration,
    ) -> Result<IssuedToken, AuthError> {
        self.state.lock().unwrap().link_codes.get(code).cloned().ok_or(AuthError::UnknownLinkCode)
    }

    async fn authenticate(&self, access_token: &str) -> Result<Principal, AuthError> {
        let id = access_token.strip_prefix("access:").ok_or(AuthError::InvalidToken)?;
        let state = self.state.lock().unwrap();
        let account =
            state.accounts.iter().find(|a| a.user.0 == id).ok_or(AuthError::InvalidToken)?;
        Ok(Principal { user: account.user.clone(), role: account.role })
    }

    async fn create_link_code(
        &self,
        _caller: &Principal,
        user: Option<UserId>,
        _ttl_secs: Option<i64>,
    ) -> Result<PendingLink, AuthError> {
        let link = PendingLink {
            code: "link-code".into(),
            user: user.unwrap_or(UserId("u1".into())),
            role: Role::Player,
            expires_at: Timestamp::from_second(4_102_444_800).unwrap(),
        };
        self.state.lock().unwrap().pending.insert(link.code.clone(), link.clone());
        Ok(link)
    }

    async fn list_link_codes(&self, user: &UserId) -> Result<Vec<PendingLink>, AuthError> {
        let mut links: Vec<PendingLink> = self
            .state
            .lock()
            .unwrap()
            .pending
            .values()
            .filter(|link| &link.user == user)
            .cloned()
            .collect();
        links.sort_by(|a, b| a.code.cmp(&b.code));
        Ok(links)
    }

    async fn revoke_link_code(&self, user: &UserId, code: &str) -> Result<(), AuthError> {
        self.state
            .lock()
            .unwrap()
            .pending
            .retain(|_, link| !(link.code == code && &link.user == user));
        Ok(())
    }

    async fn logout(&self, refresh_token: &str) -> Result<(), AuthError> {
        let id = refresh_token.strip_prefix("refresh:").ok_or(AuthError::InvalidToken)?;
        let state = self.state.lock().unwrap();
        if !state.accounts.iter().any(|a| a.user.0 == id) {
            return Err(AuthError::InvalidToken);
        }
        Ok(())
    }

    async fn logout_all(&self, _user: &UserId) -> Result<(), AuthError> {
        Ok(())
    }

    async fn list_devices(&self, _user: &UserId) -> Result<Vec<Device>, AuthError> {
        Ok(Vec::new())
    }

    async fn list_api_tokens(&self, _user: &UserId) -> Result<Vec<ApiToken>, AuthError> {
        Ok(Vec::new())
    }

    async fn revoke_device(&self, _user: &UserId, _device: &DeviceId) -> Result<(), AuthError> {
        Ok(())
    }

    async fn revoke_api_token(&self, _user: &UserId, _token: &ApiTokenId) -> Result<(), AuthError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> MockAuthService {
        let svc = MockAuthService::new();
        svc.add_account("alice", "pw", UserId("u1".into()), Role::Admin);
        svc
    }

    #[tokio::test]
    async fn login_success_and_failures() {
        let svc = service();
        let tokens = svc.login("alice", "pw").await.unwrap();
        assert_eq!(tokens.access_token, "access:u1");
        assert_eq!(tokens.refresh_token, "refresh:u1");

        assert!(matches!(
            svc.login("alice", "wrong").await.unwrap_err(),
            AuthError::InvalidCredentials
        ));
        assert!(matches!(
            svc.login("ghost", "pw").await.unwrap_err(),
            AuthError::InvalidCredentials
        ));
        for blank in ["", "   "] {
            assert!(matches!(
                svc.login("alice", blank).await.unwrap_err(),
                AuthError::InvalidCredentials
            ));
        }
    }

    #[tokio::test]
    async fn refresh_success_and_failures() {
        let svc = service();
        let tokens = svc.refresh("refresh:u1").await.unwrap();
        assert_eq!(tokens.access_token, "access:u1");
        assert_eq!(tokens.refresh_token, "refresh:u1");

        assert!(matches!(svc.refresh("garbage").await.unwrap_err(), AuthError::InvalidToken));
        assert!(matches!(svc.refresh("refresh:ghost").await.unwrap_err(), AuthError::InvalidToken));
    }

    #[tokio::test]
    async fn authenticate_success_and_failures() {
        let svc = service();
        let principal = svc.authenticate("access:u1").await.unwrap();
        assert_eq!(principal.user, UserId("u1".into()));
        assert_eq!(principal.role, Role::Admin);

        assert!(matches!(svc.authenticate("garbage").await.unwrap_err(), AuthError::InvalidToken));
        assert!(matches!(
            svc.authenticate("access:ghost").await.unwrap_err(),
            AuthError::InvalidToken
        ));
    }

    #[tokio::test]
    async fn redeem_link_code_success_and_failure() {
        let svc = service();
        svc.add_link_code("ABCD", IssuedToken { token: "player-token".into(), expires_at: None });
        let device = DeviceRegistration { name: "Roku".into(), platform: "roku".into() };
        let issued = svc.redeem_link_code("ABCD", device.clone()).await.unwrap();
        assert_eq!(issued.token, "player-token");

        assert!(matches!(
            svc.redeem_link_code("ZZZZ", device).await.unwrap_err(),
            AuthError::UnknownLinkCode
        ));
    }

    #[tokio::test]
    async fn logout_and_logout_all() {
        let svc = service();
        svc.logout("refresh:u1").await.unwrap();
        svc.logout_all(&UserId("u1".into())).await.unwrap();
        assert!(matches!(svc.logout("garbage").await.unwrap_err(), AuthError::InvalidToken));
        assert!(matches!(svc.logout("refresh:ghost").await.unwrap_err(), AuthError::InvalidToken));
    }

    #[tokio::test]
    async fn device_and_token_endpoints_are_empty_and_ok() {
        let svc = service();
        let user = UserId("u1".into());
        assert!(svc.list_devices(&user).await.unwrap().is_empty());
        assert!(svc.list_api_tokens(&user).await.unwrap().is_empty());
        svc.revoke_device(&user, &DeviceId("d1".into())).await.unwrap();
        svc.revoke_api_token(&user, &ApiTokenId("t1".into())).await.unwrap();
    }

    #[tokio::test]
    async fn create_link_code_returns_code() {
        let svc = service();
        let caller = Principal { user: UserId("admin".into()), role: Role::Admin };
        let link = svc.create_link_code(&caller, None, None).await.unwrap();
        assert_eq!(link.code, "link-code");
        assert_eq!(link.role, Role::Player);
    }

    #[tokio::test]
    async fn create_lists_then_revokes_link_code() {
        let svc = service();
        let caller = Principal { user: UserId("u1".into()), role: Role::Admin };
        svc.create_link_code(&caller, None, None).await.unwrap();
        let user = UserId("u1".into());
        let listed = svc.list_link_codes(&user).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].code, "link-code");

        svc.revoke_link_code(&user, "link-code").await.unwrap();
        assert!(svc.list_link_codes(&user).await.unwrap().is_empty());
    }
}
