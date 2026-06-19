use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::error::AuthError;
use domain::service::AuthService;
use domain::user::{
    AccessToken, DeviceRegistration, IssuedToken, Principal, Role, TokenPair, UserId,
};

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
        self.state
            .lock()
            .unwrap()
            .link_codes
            .insert(code.to_string(), token);
    }
}

impl AuthService for MockAuthService {
    async fn login(&self, username: &str, password: &str) -> Result<TokenPair, AuthError> {
        let state = self.state.lock().unwrap();
        let account = state
            .accounts
            .iter()
            .find(|a| a.username == username && a.password == password)
            .ok_or(AuthError::InvalidCredentials)?;
        Ok(TokenPair {
            access_token: format!("access:{}", account.user.0),
            refresh_token: format!("refresh:{}", account.user.0),
        })
    }

    async fn refresh(&self, refresh_token: &str) -> Result<AccessToken, AuthError> {
        let id = refresh_token
            .strip_prefix("refresh:")
            .ok_or(AuthError::InvalidToken)?;
        let state = self.state.lock().unwrap();
        if !state.accounts.iter().any(|a| a.user.0 == id) {
            return Err(AuthError::InvalidToken);
        }
        Ok(AccessToken {
            access_token: format!("access:{id}"),
            expires_in_s: 3600,
        })
    }

    async fn redeem_link_code(
        &self,
        code: &str,
        _device: DeviceRegistration,
    ) -> Result<IssuedToken, AuthError> {
        self.state
            .lock()
            .unwrap()
            .link_codes
            .get(code)
            .cloned()
            .ok_or(AuthError::UnknownLinkCode)
    }

    async fn authenticate(&self, access_token: &str) -> Result<Principal, AuthError> {
        let id = access_token
            .strip_prefix("access:")
            .ok_or(AuthError::InvalidToken)?;
        let state = self.state.lock().unwrap();
        let account = state
            .accounts
            .iter()
            .find(|a| a.user.0 == id)
            .ok_or(AuthError::InvalidToken)?;
        Ok(Principal {
            user: account.user.clone(),
            role: account.role,
        })
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
    }

    #[tokio::test]
    async fn refresh_success_and_failures() {
        let svc = service();
        let access = svc.refresh("refresh:u1").await.unwrap();
        assert_eq!(access.access_token, "access:u1");
        assert!(access.expires_in_s > 0);

        assert!(matches!(
            svc.refresh("garbage").await.unwrap_err(),
            AuthError::InvalidToken
        ));
        assert!(matches!(
            svc.refresh("refresh:ghost").await.unwrap_err(),
            AuthError::InvalidToken
        ));
    }

    #[tokio::test]
    async fn authenticate_success_and_failures() {
        let svc = service();
        let principal = svc.authenticate("access:u1").await.unwrap();
        assert_eq!(principal.user, UserId("u1".into()));
        assert_eq!(principal.role, Role::Admin);

        assert!(matches!(
            svc.authenticate("garbage").await.unwrap_err(),
            AuthError::InvalidToken
        ));
        assert!(matches!(
            svc.authenticate("access:ghost").await.unwrap_err(),
            AuthError::InvalidToken
        ));
    }

    #[tokio::test]
    async fn redeem_link_code_success_and_failure() {
        let svc = service();
        svc.add_link_code(
            "ABCD",
            IssuedToken {
                token: "player-token".into(),
                expires_at: None,
            },
        );
        let device = DeviceRegistration {
            name: "Roku".into(),
            platform: "roku".into(),
        };
        let issued = svc.redeem_link_code("ABCD", device.clone()).await.unwrap();
        assert_eq!(issued.token, "player-token");

        assert!(matches!(
            svc.redeem_link_code("ZZZZ", device).await.unwrap_err(),
            AuthError::UnknownLinkCode
        ));
    }
}
