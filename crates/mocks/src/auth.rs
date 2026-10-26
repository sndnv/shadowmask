use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::error::AuthError;
use domain::service::AuthService;
use domain::user::{
    ApiToken, ApiTokenId, CommandTarget, Device, DeviceId, DeviceRegistration, DeviceTarget,
    IssuedToken, PendingLink, Principal, Role, SharedTarget, SharedWith, TokenPair, UserId,
};
use jiff::Timestamp;

use crate::hold::{Hold, NextHold};

#[derive(Debug, Clone)]
struct Account {
    username: String,
    password: String,
    user: UserId,
    role: Role,
    account_admin: bool,
}

#[derive(Debug, Default)]
struct State {
    accounts: Vec<Account>,
    link_codes: HashMap<String, IssuedToken>,
    pending: HashMap<String, PendingLink>,
    devices: Vec<Device>,
    shares: Vec<(DeviceId, UserId)>,
    logged_out: Vec<UserId>,
    password_logged_out: Vec<UserId>,
}

impl State {
    fn username_of(&self, user: &UserId) -> String {
        let account = self.accounts.iter().find(|account| &account.user == user);
        account.map_or_else(|| user.0.clone(), |account| account.username.clone())
    }
}

#[derive(Clone, Default)]
pub struct MockAuthService {
    state: Arc<Mutex<State>>,
    password_logout_hold: NextHold,
}

impl MockAuthService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn logged_out(&self) -> Vec<UserId> {
        self.state.lock().unwrap().logged_out.clone()
    }

    pub fn password_logged_out(&self) -> Vec<UserId> {
        self.state.lock().unwrap().password_logged_out.clone()
    }

    pub fn hold_next_password_logout(&self) -> Hold {
        self.password_logout_hold.arm()
    }

    pub fn add_account(&self, username: &str, password: &str, user: UserId, role: Role) {
        self.state.lock().unwrap().accounts.push(Account {
            username: username.to_string(),
            password: password.to_string(),
            user,
            role,
            account_admin: role == Role::Admin,
        });
    }

    pub fn add_linked_device(&self, user: UserId, account_admin: bool) {
        self.state.lock().unwrap().accounts.push(Account {
            username: format!("device {}", user.0),
            password: String::new(),
            user,
            role: Role::Player,
            account_admin,
        });
    }

    pub fn add_link_code(&self, code: &str, token: IssuedToken) {
        self.state.lock().unwrap().link_codes.insert(code.to_string(), token);
    }

    pub fn add_device(&self, device: Device) {
        self.state.lock().unwrap().devices.push(device);
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
        let (id, device) = match access_token.strip_prefix("device:") {
            Some(id) => (id, true),
            None => (access_token.strip_prefix("access:").ok_or(AuthError::InvalidToken)?, false),
        };
        let state = self.state.lock().unwrap();
        if let Some(linked) = state.devices.iter().find(|d| device && d.id.0 == id) {
            return Ok(Principal {
                user: linked.user.clone(),
                role: Role::Player,
                account_admin: state
                    .accounts
                    .iter()
                    .any(|a| a.user == linked.user && a.role == Role::Admin),
                device: Some(linked.id.clone()),
            });
        }
        let account = state
            .accounts
            .iter()
            .find(|a| a.user.0 == id && (!device || a.role == Role::Player))
            .ok_or(AuthError::InvalidToken)?;
        Ok(Principal {
            user: account.user.clone(),
            role: account.role,
            account_admin: account.account_admin,
            device: device.then(|| DeviceId(id.to_owned())),
        })
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

    async fn logout_all(&self, user: &UserId) -> Result<(), AuthError> {
        self.state.lock().unwrap().logged_out.push(user.clone());
        Ok(())
    }

    async fn logout_password_sessions(&self, user: &UserId) -> Result<(), AuthError> {
        self.password_logout_hold.pass().await;
        self.state.lock().unwrap().password_logged_out.push(user.clone());
        Ok(())
    }

    async fn list_devices(&self, user: &UserId) -> Result<Vec<Device>, AuthError> {
        Ok(self.state.lock().unwrap().devices.iter().filter(|d| &d.user == user).cloned().collect())
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

    async fn owned_device(&self, user: &UserId, device: &DeviceId) -> Result<Device, AuthError> {
        let state = self.state.lock().unwrap();
        state
            .devices
            .iter()
            .find(|d| &d.id == device && &d.user == user)
            .cloned()
            .ok_or(AuthError::NotFound)
    }

    async fn report_target(
        &self,
        device: &DeviceId,
        target: DeviceTarget,
    ) -> Result<(), AuthError> {
        let mut state = self.state.lock().unwrap();
        if let Some(stored) = state.devices.iter_mut().find(|d| &d.id == device) {
            stored.target = Some(target);
        }
        Ok(())
    }

    async fn clear_target(&self, device: &DeviceId) -> Result<(), AuthError> {
        let mut state = self.state.lock().unwrap();
        if let Some(stored) = state.devices.iter_mut().find(|d| &d.id == device) {
            stored.target = None;
        }
        Ok(())
    }

    async fn share_device(
        &self,
        owner: &UserId,
        device: &DeviceId,
        username: &str,
    ) -> Result<SharedWith, AuthError> {
        self.owned_device(owner, device).await?;
        let mut state = self.state.lock().unwrap();
        let user = state
            .accounts
            .iter()
            .find(|account| account.username == username)
            .map(|account| account.user.clone())
            .ok_or(AuthError::UnknownUser)?;
        if &user == owner {
            return Err(AuthError::ShareWithSelf);
        }
        let share = (device.clone(), user.clone());
        if !state.shares.contains(&share) {
            state.shares.push(share);
        }
        Ok(SharedWith { device: device.clone(), user, username: username.to_owned() })
    }

    async fn unshare_device(
        &self,
        owner: &UserId,
        device: &DeviceId,
        user: &UserId,
    ) -> Result<(), AuthError> {
        self.owned_device(owner, device).await?;
        self.state.lock().unwrap().shares.retain(|(d, u)| d != device || u != user);
        Ok(())
    }

    async fn device_shares(&self, owner: &UserId) -> Result<Vec<SharedWith>, AuthError> {
        let state = self.state.lock().unwrap();
        Ok(state
            .shares
            .iter()
            .filter(|(device, _)| state.devices.iter().any(|d| &d.id == device && &d.user == owner))
            .map(|(device, user)| SharedWith {
                device: device.clone(),
                user: user.clone(),
                username: state.username_of(user),
            })
            .collect())
    }

    async fn shared_targets(&self, user: &UserId) -> Result<Vec<SharedTarget>, AuthError> {
        let state = self.state.lock().unwrap();
        Ok(state
            .devices
            .iter()
            .filter(|device| device.target.is_some())
            .filter(|device| state.shares.contains(&(device.id.clone(), user.clone())))
            .map(|device| SharedTarget {
                device: device.clone(),
                owner: state.username_of(&device.user),
            })
            .collect())
    }

    async fn command_target(
        &self,
        sender: &UserId,
        device: &DeviceId,
    ) -> Result<CommandTarget, AuthError> {
        let state = self.state.lock().unwrap();
        let found = state
            .devices
            .iter()
            .find(|d| {
                &d.id == device
                    && (&d.user == sender || state.shares.contains(&(d.id.clone(), sender.clone())))
            })
            .cloned()
            .ok_or(AuthError::NotFound)?;
        let account_admin = state
            .accounts
            .iter()
            .any(|account| account.user == found.user && account.account_admin);
        let plays_as = Principal {
            user: found.user.clone(),
            role: Role::Player,
            account_admin,
            device: Some(found.id.clone()),
        };
        Ok(CommandTarget { device: found, plays_as })
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
    async fn a_linked_device_token_is_a_player_carrying_its_account_flag() {
        let svc = service();
        assert!(svc.authenticate("access:u1").await.unwrap().account_admin);
        assert!(matches!(
            svc.authenticate("device:u1").await.unwrap_err(),
            AuthError::InvalidToken
        ));

        svc.add_linked_device(UserId("u1".into()), true);
        let device = svc.authenticate("device:u1").await.unwrap();

        assert_eq!(device.role, Role::Player);
        assert!(device.account_admin);
    }

    #[tokio::test]
    async fn redeem_link_code_success_and_failure() {
        let svc = service();
        svc.add_link_code(
            "ABCD",
            IssuedToken {
                token: "player-token".into(),
                expires_at: None,
                device: DeviceId("dev-1".into()),
            },
        );
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
        svc.logout_password_sessions(&UserId("u2".into())).await.unwrap();
        assert_eq!(svc.logged_out(), [UserId("u1".into())]);
        assert_eq!(svc.password_logged_out(), [UserId("u2".into())]);
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
    async fn an_added_device_signs_in_and_holds_its_target() {
        let svc = service();
        let user = UserId("u1".into());
        let roku = DeviceId("roku-1".into());
        svc.add_device(Device {
            id: roku.clone(),
            user: user.clone(),
            name: "Roku".into(),
            platform: "roku".into(),
            created_at: Timestamp::UNIX_EPOCH,
            last_seen: None,
            target: None,
        });

        let principal = svc.authenticate("device:roku-1").await.unwrap();
        assert_eq!(principal.user, user);
        assert_eq!(principal.role, Role::Player);
        assert!(principal.account_admin);
        assert_eq!(principal.device, Some(roku.clone()));
        assert_eq!(svc.authenticate("access:u1").await.unwrap().device, None);

        let target = DeviceTarget::new(
            "roku",
            &["10.0.0.2".to_owned()],
            8060,
            Some("dev".into()),
            Timestamp::UNIX_EPOCH,
        )
        .unwrap();
        svc.report_target(&roku, target.clone()).await.unwrap();
        svc.report_target(&DeviceId("ghost".into()), target.clone()).await.unwrap();
        assert_eq!(svc.owned_device(&user, &roku).await.unwrap().target, Some(target));
        assert_eq!(svc.list_devices(&user).await.unwrap().len(), 1);

        svc.clear_target(&roku).await.unwrap();
        svc.clear_target(&DeviceId("ghost".into())).await.unwrap();
        assert_eq!(svc.owned_device(&user, &roku).await.unwrap().target, None);
        assert!(matches!(
            svc.owned_device(&UserId("u2".into()), &roku).await.unwrap_err(),
            AuthError::NotFound
        ));
    }

    #[tokio::test]
    async fn create_link_code_returns_code() {
        let svc = service();
        let caller = Principal {
            user: UserId("admin".into()),
            role: Role::Admin,
            account_admin: true,
            device: None,
        };
        let link = svc.create_link_code(&caller, None, None).await.unwrap();
        assert_eq!(link.code, "link-code");
        assert_eq!(link.role, Role::Player);
    }

    #[tokio::test]
    async fn create_lists_then_revokes_link_code() {
        let svc = service();
        let caller = Principal {
            user: UserId("u1".into()),
            role: Role::Admin,
            account_admin: true,
            device: None,
        };
        svc.create_link_code(&caller, None, None).await.unwrap();
        let user = UserId("u1".into());
        let listed = svc.list_link_codes(&user).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].code, "link-code");

        svc.revoke_link_code(&user, "link-code").await.unwrap();
        assert!(svc.list_link_codes(&user).await.unwrap().is_empty());
    }
}
