use std::fmt::Write;

use domain::error::{AuthError, RepositoryError};
use domain::repository::{AuthTokenRepository, UserRepository};
use domain::service::AuthService;
use domain::user::{
    ApiToken, ApiTokenId, AuthSession, AuthSessionId, Device, DeviceId, DeviceRegistration,
    IssuedToken, PendingLink, Principal, Role, TokenPair, UserId,
};
use jiff::Timestamp;
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::password;

const AUDIENCE: &str = "shadowmask-api";
const TYP_ACCESS: &str = "access";
const TYP_REFRESH: &str = "refresh";
const API_TOKEN_PREFIX: &str = "smk_";
const LINK_CODE_TTL_SECS: i64 = 900;
const LINK_CODE_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const LINK_CODE_LEN: usize = 8;

fn generate_link_code() -> String {
    let mut value = Uuid::new_v4().as_u128();
    let mut code = String::with_capacity(LINK_CODE_LEN);
    for _ in 0..LINK_CODE_LEN {
        code.push(LINK_CODE_ALPHABET[(value % 32) as usize] as char);
        value /= 32;
    }
    code
}

fn normalize_link_code(input: &str) -> String {
    input
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect()
}

pub struct DefaultAuthService<U, T> {
    users: U,
    tokens: T,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    header: Header,
    validation: Validation,
    access_ttl_secs: i64,
    refresh_ttl_secs: i64,
}

impl<U, T> Clone for DefaultAuthService<U, T>
where
    U: Clone,
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            users: self.users.clone(),
            tokens: self.tokens.clone(),
            encoding_key: self.encoding_key.clone(),
            decoding_key: self.decoding_key.clone(),
            header: self.header.clone(),
            validation: self.validation.clone(),
            access_ttl_secs: self.access_ttl_secs,
            refresh_ttl_secs: self.refresh_ttl_secs,
        }
    }
}

impl<U, T> DefaultAuthService<U, T> {
    pub fn new(
        users: U,
        tokens: T,
        secret: &[u8],
        access_ttl_secs: i64,
        refresh_ttl_secs: i64,
    ) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&[AUDIENCE]);
        Self {
            users,
            tokens,
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            header: Header::new(Algorithm::HS256),
            validation,
            access_ttl_secs,
            refresh_ttl_secs,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    rol: String,
    aud: String,
    typ: String,
    exp: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    jti: Option<String>,
}

fn role_to_claim(role: Role) -> &'static str {
    match role {
        Role::Admin => "admin",
        Role::User => "user",
        Role::Player => "player",
        Role::Automation => "automation",
    }
}

fn role_from_claim(value: &str) -> Result<Role, AuthError> {
    match value {
        "admin" => Ok(Role::Admin),
        "user" => Ok(Role::User),
        "player" => Ok(Role::Player),
        "automation" => Ok(Role::Automation),
        _ => Err(AuthError::InvalidToken),
    }
}

fn backend_error(error: impl ToString) -> AuthError {
    AuthError::Repository(RepositoryError::Backend(error.to_string()))
}

fn signature_of(token: &str) -> String {
    token.rsplit('.').next().unwrap_or_default().to_owned()
}

fn hash_api_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

impl<U, T> DefaultAuthService<U, T> {
    fn encode(
        &self,
        user: &UserId,
        role: Role,
        typ: &str,
        exp: i64,
        jti: Option<String>,
    ) -> Result<String, AuthError> {
        let claims = Claims {
            sub: user.0.clone(),
            rol: role_to_claim(role).to_owned(),
            aud: AUDIENCE.to_owned(),
            typ: typ.to_owned(),
            exp,
            jti,
        };
        encode(&self.header, &claims, &self.encoding_key).map_err(backend_error)
    }

    fn decode(&self, token: &str, expected_typ: &str) -> Result<Claims, AuthError> {
        let claims = decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map_err(|error| match error.kind() {
                ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                _ => AuthError::InvalidToken,
            })?
            .claims;
        if claims.typ != expected_typ {
            return Err(AuthError::InvalidToken);
        }
        Ok(claims)
    }
}

impl<U, T> DefaultAuthService<U, T>
where
    U: UserRepository + Sync,
    T: AuthTokenRepository + Sync,
{
    async fn require_active(&self, user: &UserId) -> Result<(), AuthError> {
        match self.users.get(user).await? {
            Some(found) if found.active => Ok(()),
            Some(_) => Err(AuthError::AccountDisabled),
            None => Err(AuthError::InvalidToken),
        }
    }

    async fn issue_pair(&self, user: &UserId, role: Role) -> Result<TokenPair, AuthError> {
        let now = Timestamp::now().as_second();
        let access_token = self.encode(user, role, TYP_ACCESS, now + self.access_ttl_secs, None)?;
        let jti = Uuid::new_v4().to_string();
        let refresh_exp = now + self.refresh_ttl_secs;
        let refresh_token = self.encode(user, role, TYP_REFRESH, refresh_exp, Some(jti.clone()))?;
        self.tokens
            .store_refresh(AuthSession {
                id: AuthSessionId(jti),
                user: user.clone(),
                refresh_token_hash: signature_of(&refresh_token),
                issued_at: Timestamp::from_second(now).map_err(backend_error)?,
                expires_at: Timestamp::from_second(refresh_exp).map_err(backend_error)?,
            })
            .await?;
        Ok(TokenPair { access_token, refresh_token })
    }
}

impl<U, T> AuthService for DefaultAuthService<U, T>
where
    U: UserRepository + Sync,
    T: AuthTokenRepository + Sync,
{
    async fn login(&self, username: &str, password: &str) -> Result<TokenPair, AuthError> {
        if password.trim().is_empty() {
            return Err(AuthError::InvalidCredentials);
        }
        let user =
            self.users.find_by_username(username).await?.ok_or(AuthError::InvalidCredentials)?;
        if !password::verify(password, &user.password_hash).unwrap_or(false) {
            return Err(AuthError::InvalidCredentials);
        }
        if !user.active {
            return Err(AuthError::AccountDisabled);
        }
        self.issue_pair(&user.id, user.role).await
    }

    async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, AuthError> {
        let claims = self.decode(refresh_token, TYP_REFRESH)?;
        let jti = AuthSessionId(claims.jti.ok_or(AuthError::InvalidToken)?);
        let stored = self.tokens.find_refresh(&jti).await?.ok_or(AuthError::InvalidToken)?;
        if stored.refresh_token_hash != signature_of(refresh_token) {
            return Err(AuthError::InvalidToken);
        }
        let role = role_from_claim(&claims.rol)?;
        let user = UserId(claims.sub);
        self.require_active(&user).await?;
        self.tokens.revoke_refresh(&jti).await?;
        self.issue_pair(&user, role).await
    }

    async fn redeem_link_code(
        &self,
        code: &str,
        device: DeviceRegistration,
    ) -> Result<IssuedToken, AuthError> {
        let now = Timestamp::now();
        let link = self
            .tokens
            .redeem_link_code(&normalize_link_code(code), now)
            .await?
            .ok_or(AuthError::UnknownLinkCode)?;
        self.require_active(&link.user).await?;
        let device = Device {
            id: DeviceId(Uuid::new_v4().to_string()),
            user: link.user.clone(),
            name: device.name,
            platform: device.platform,
            created_at: now,
            last_seen: Some(now),
        };
        self.tokens.upsert_device(device.clone()).await?;
        let token = format!("{API_TOKEN_PREFIX}{}", Uuid::new_v4().simple());
        self.tokens
            .store_api_token(ApiToken {
                id: ApiTokenId(Uuid::new_v4().to_string()),
                user: link.user,
                device: device.id.clone(),
                token_hash: hash_api_token(&token),
                created_at: now,
                last_used_at: None,
            })
            .await?;
        Ok(IssuedToken { token, expires_at: None, device: device.id })
    }

    async fn authenticate(&self, access_token: &str) -> Result<Principal, AuthError> {
        if access_token.starts_with(API_TOKEN_PREFIX) {
            let token = self
                .tokens
                .find_api_token_by_hash(&hash_api_token(access_token))
                .await?
                .ok_or(AuthError::InvalidToken)?;
            self.require_active(&token.user).await?;
            self.tokens.touch_api_token(&token.id, Timestamp::now()).await?;
            return Ok(Principal { user: token.user, role: Role::Player });
        }
        let claims = self.decode(access_token, TYP_ACCESS)?;
        let user = UserId(claims.sub);
        self.require_active(&user).await?;
        Ok(Principal { user, role: role_from_claim(&claims.rol)? })
    }

    async fn create_link_code(
        &self,
        caller: &Principal,
        user: Option<UserId>,
        ttl_secs: Option<i64>,
    ) -> Result<PendingLink, AuthError> {
        let now = Timestamp::now();
        let ttl = ttl_secs.unwrap_or(LINK_CODE_TTL_SECS);
        let link = PendingLink {
            code: generate_link_code(),
            user: user.unwrap_or_else(|| caller.user.clone()),
            role: Role::Player,
            expires_at: Timestamp::from_second(now.as_second() + ttl).map_err(backend_error)?,
        };
        self.tokens.store_link_code(link.clone(), now).await?;
        Ok(link)
    }

    async fn list_link_codes(&self, user: &UserId) -> Result<Vec<PendingLink>, AuthError> {
        Ok(self.tokens.list_link_codes(user, Timestamp::now()).await?)
    }

    async fn revoke_link_code(&self, user: &UserId, code: &str) -> Result<(), AuthError> {
        self.tokens.delete_link_code(&normalize_link_code(code), user).await?;
        Ok(())
    }

    async fn logout(&self, refresh_token: &str) -> Result<(), AuthError> {
        let claims = self.decode(refresh_token, TYP_REFRESH)?;
        let jti = AuthSessionId(claims.jti.ok_or(AuthError::InvalidToken)?);
        self.tokens.revoke_refresh(&jti).await?;
        Ok(())
    }

    async fn logout_all(&self, user: &UserId) -> Result<(), AuthError> {
        self.tokens.revoke_all_for_user(user).await?;
        Ok(())
    }

    async fn list_devices(&self, user: &UserId) -> Result<Vec<Device>, AuthError> {
        Ok(self.tokens.list_devices(user).await?)
    }

    async fn list_api_tokens(&self, user: &UserId) -> Result<Vec<ApiToken>, AuthError> {
        Ok(self.tokens.list_api_tokens(user).await?)
    }

    async fn revoke_device(&self, user: &UserId, device: &DeviceId) -> Result<(), AuthError> {
        let owned =
            self.tokens.get_device(device).await?.is_some_and(|candidate| &candidate.user == user);
        if !owned {
            return Err(AuthError::NotFound);
        }
        for token in self.tokens.list_api_tokens(user).await? {
            if &token.device == device {
                self.tokens.revoke_api_token(&token.id).await?;
            }
        }
        self.tokens.delete_device(device).await?;
        Ok(())
    }

    async fn revoke_api_token(&self, user: &UserId, token: &ApiTokenId) -> Result<(), AuthError> {
        let owned =
            self.tokens.list_api_tokens(user).await?.iter().any(|candidate| &candidate.id == token);
        if !owned {
            return Err(AuthError::NotFound);
        }
        self.tokens.revoke_api_token(token).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::user::{PendingLink, User};
    use mocks::{MockAuthTokenRepo, MockUserRepo};

    const SECRET: &[u8] = b"shadowmask-test-secret";

    fn device() -> DeviceRegistration {
        DeviceRegistration { name: "Roku".into(), platform: "roku".into() }
    }

    fn user(id: &str, username: &str, role: Role) -> User {
        User {
            id: UserId(id.into()),
            username: username.into(),
            password_hash: password::hash("pw").unwrap(),
            role,
            max_content_rating: None,
            preferred_audio: Vec::new(),
            preferred_subtitle: Vec::new(),
            concurrent_stream_limit: None,
            bitrate_cap: None,
            active: true,
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn service(
        access_ttl: i64,
        refresh_ttl: i64,
    ) -> DefaultAuthService<MockUserRepo, MockAuthTokenRepo> {
        let users = MockUserRepo::new();
        DefaultAuthService::new(users, MockAuthTokenRepo::new(), SECRET, access_ttl, refresh_ttl)
    }

    async fn seeded() -> DefaultAuthService<MockUserRepo, MockAuthTokenRepo> {
        let svc = service(3600, 86_400);
        svc.users.insert(user("u1", "alice", Role::Admin));
        svc
    }

    #[test]
    fn role_claims_round_trip() {
        for role in [Role::Admin, Role::User, Role::Player, Role::Automation] {
            assert_eq!(role_from_claim(role_to_claim(role)).unwrap(), role);
        }
        assert!(matches!(role_from_claim("nope"), Err(AuthError::InvalidToken)));
    }

    #[tokio::test]
    async fn login_authenticate_roundtrip() {
        let svc = seeded().await;
        let pair = svc.login("alice", "pw").await.unwrap();
        let principal = svc.authenticate(&pair.access_token).await.unwrap();
        assert_eq!(principal.user, UserId("u1".into()));
        assert_eq!(principal.role, Role::Admin);
    }

    #[tokio::test]
    async fn a_deleted_user_stops_authenticating_immediately() {
        let svc = seeded().await;
        let pair = svc.login("alice", "pw").await.unwrap();
        assert!(svc.authenticate(&pair.access_token).await.is_ok());

        svc.users.remove(&UserId("u1".into()));

        assert!(matches!(
            svc.authenticate(&pair.access_token).await.unwrap_err(),
            AuthError::InvalidToken
        ));
        assert!(matches!(
            svc.refresh(&pair.refresh_token).await.unwrap_err(),
            AuthError::InvalidToken
        ));
    }

    #[tokio::test]
    async fn a_deactivated_user_is_refused_on_every_credential() {
        let svc = seeded().await;
        let pair = svc.login("alice", "pw").await.unwrap();
        let link = svc
            .create_link_code(
                &Principal { user: UserId("u1".into()), role: Role::Admin },
                None,
                None,
            )
            .await
            .unwrap();

        svc.users.insert(User { active: false, ..user("u1", "alice", Role::Admin) });

        assert!(matches!(svc.login("alice", "pw").await.unwrap_err(), AuthError::AccountDisabled));
        assert!(matches!(
            svc.authenticate(&pair.access_token).await.unwrap_err(),
            AuthError::AccountDisabled
        ));
        assert!(matches!(
            svc.refresh(&pair.refresh_token).await.unwrap_err(),
            AuthError::AccountDisabled
        ));
        assert!(matches!(
            svc.redeem_link_code(&link.code, device()).await.unwrap_err(),
            AuthError::AccountDisabled
        ));
    }

    #[tokio::test]
    async fn an_api_token_dies_with_the_account_that_holds_it() {
        let svc = seeded().await;
        let link = svc
            .create_link_code(
                &Principal { user: UserId("u1".into()), role: Role::Admin },
                None,
                None,
            )
            .await
            .unwrap();
        let issued = svc.redeem_link_code(&link.code, device()).await.unwrap();
        assert!(issued.token.starts_with(API_TOKEN_PREFIX));
        assert!(svc.authenticate(&issued.token).await.is_ok());

        svc.users.insert(User { active: false, ..user("u1", "alice", Role::Admin) });

        assert!(matches!(
            svc.authenticate(&issued.token).await.unwrap_err(),
            AuthError::AccountDisabled
        ));
    }

    #[tokio::test]
    async fn login_rejects_bad_credentials() {
        let svc = seeded().await;
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
    async fn login_refuses_an_empty_password_outright() {
        let svc = seeded().await;
        for blank in ["", "   "] {
            assert!(matches!(
                svc.login("alice", blank).await.unwrap_err(),
                AuthError::InvalidCredentials
            ));
        }
    }

    #[tokio::test]
    async fn login_takes_a_password_with_surrounding_spaces_as_typed() {
        for secret in ["  my-pass", "pass  ", "  my-pass  "] {
            let svc = service(3600, 86_400);
            let mut account = user("u1", "alice", Role::Admin);
            account.password_hash = password::hash(secret).unwrap();
            svc.users.insert(account);

            svc.login("alice", secret).await.unwrap();
            assert!(matches!(
                svc.login("alice", secret.trim()).await.unwrap_err(),
                AuthError::InvalidCredentials
            ));
        }
    }

    #[tokio::test]
    async fn refresh_rotates_and_revokes_old() {
        let svc = seeded().await;
        let pair = svc.login("alice", "pw").await.unwrap();
        let rotated = svc.refresh(&pair.refresh_token).await.unwrap();
        assert!(svc.authenticate(&rotated.access_token).await.is_ok());

        assert!(matches!(
            svc.refresh(&pair.refresh_token).await.unwrap_err(),
            AuthError::InvalidToken
        ));
    }

    #[tokio::test]
    async fn refresh_rejects_access_token_and_garbage() {
        let svc = seeded().await;
        let pair = svc.login("alice", "pw").await.unwrap();
        assert!(matches!(
            svc.refresh(&pair.access_token).await.unwrap_err(),
            AuthError::InvalidToken
        ));
        assert!(matches!(svc.refresh("not.a.jwt").await.unwrap_err(), AuthError::InvalidToken));
    }

    #[tokio::test]
    async fn authenticate_rejects_refresh_and_expired() {
        let svc = seeded().await;
        let pair = svc.login("alice", "pw").await.unwrap();
        assert!(matches!(
            svc.authenticate(&pair.refresh_token).await.unwrap_err(),
            AuthError::InvalidToken
        ));

        let expired = service(-3600, 86_400);
        expired.users.insert(user("u1", "alice", Role::Admin));
        let pair = expired.login("alice", "pw").await.unwrap();
        assert!(matches!(
            expired.authenticate(&pair.access_token).await.unwrap_err(),
            AuthError::TokenExpired
        ));
    }

    #[tokio::test]
    async fn redeem_link_code_binds_device_and_api_token() {
        let svc = seeded().await;
        let future = Timestamp::from_second(4_000_000_000).unwrap();
        svc.tokens
            .store_link_code(
                PendingLink {
                    code: "7G2K9QMP".into(),
                    user: UserId("u1".into()),
                    role: Role::Player,
                    expires_at: future,
                },
                Timestamp::UNIX_EPOCH,
            )
            .await
            .unwrap();
        let issued = svc.redeem_link_code("7G2K9QMP", device()).await.unwrap();
        assert!(issued.token.starts_with("smk_"));
        assert!(issued.expires_at.is_none());

        let principal = svc.authenticate(&issued.token).await.unwrap();
        assert_eq!(principal.user, UserId("u1".into()));
        assert_eq!(principal.role, Role::Player);

        let devices = svc.list_devices(&UserId("u1".into())).await.unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(issued.device, devices[0].id);

        assert!(matches!(
            svc.redeem_link_code("MISSING", device()).await.unwrap_err(),
            AuthError::UnknownLinkCode
        ));
    }

    #[tokio::test]
    async fn logout_all_revokes_linked_devices_not_just_browser_sessions() {
        let svc = seeded().await;
        let future = Timestamp::from_second(4_000_000_000).unwrap();
        svc.tokens
            .store_link_code(
                PendingLink {
                    code: "7G2K9QMP".into(),
                    user: UserId("u1".into()),
                    role: Role::Player,
                    expires_at: future,
                },
                Timestamp::UNIX_EPOCH,
            )
            .await
            .unwrap();
        let issued = svc.redeem_link_code("7G2K9QMP", device()).await.unwrap();
        assert!(svc.authenticate(&issued.token).await.is_ok());

        svc.logout_all(&UserId("u1".into())).await.unwrap();

        assert!(matches!(
            svc.authenticate(&issued.token).await.unwrap_err(),
            AuthError::InvalidToken
        ));
        assert!(svc.list_devices(&UserId("u1".into())).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn authenticate_rejects_unknown_api_token() {
        let svc = seeded().await;
        assert!(matches!(
            svc.authenticate("smk_deadbeef").await.unwrap_err(),
            AuthError::InvalidToken
        ));
    }

    #[tokio::test]
    async fn create_link_code_defaults_to_caller_then_redeems() {
        let svc = seeded().await;
        let caller = Principal { user: UserId("u1".into()), role: Role::Admin };
        let link = svc.create_link_code(&caller, None, None).await.unwrap();
        assert_eq!(link.user, UserId("u1".into()));
        assert_eq!(link.role, Role::Player);
        assert!(!link.code.is_empty());

        let issued = svc.redeem_link_code(&link.code, device()).await.unwrap();
        let principal = svc.authenticate(&issued.token).await.unwrap();
        assert_eq!(principal.role, Role::Player);
    }

    #[tokio::test]
    async fn create_link_code_honors_explicit_user_and_ttl() {
        let svc = seeded().await;
        let caller = Principal { user: UserId("u1".into()), role: Role::Admin };
        let before = Timestamp::now().as_second();
        let link =
            svc.create_link_code(&caller, Some(UserId("u2".into())), Some(60)).await.unwrap();
        assert_eq!(link.user, UserId("u2".into()));
        assert!(link.expires_at.as_second() <= before + 61);
    }

    #[tokio::test]
    async fn create_link_code_rejects_out_of_range_ttl() {
        let svc = seeded().await;
        let caller = Principal { user: UserId("u1".into()), role: Role::Admin };
        assert!(matches!(
            svc.create_link_code(&caller, None, Some(300_000_000_000)).await.unwrap_err(),
            AuthError::Repository(_)
        ));
    }

    #[test]
    fn generated_link_codes_are_short_and_unambiguous() {
        for _ in 0..64 {
            let code = generate_link_code();
            assert_eq!(code.len(), LINK_CODE_LEN);
            assert!(code.bytes().all(|b| LINK_CODE_ALPHABET.contains(&b)));
            assert!(!code.contains(['I', 'L', 'O', 'U']));
        }
    }

    #[test]
    fn normalize_maps_ambiguous_characters_and_strips_separators() {
        assert_eq!(normalize_link_code("7g2k-9qmp"), "7G2K9QMP");
        assert_eq!(normalize_link_code("oil 7"), "0117");
        assert_eq!(normalize_link_code(" a-b_c "), "ABC");
    }

    #[tokio::test]
    async fn redeem_tolerates_separators_and_case() {
        let svc = seeded().await;
        let caller = Principal { user: UserId("u1".into()), role: Role::Admin };
        let link = svc.create_link_code(&caller, None, None).await.unwrap();
        let typed = format!("{}-{}", &link.code[..4], link.code[4..].to_lowercase());
        let issued = svc.redeem_link_code(&typed, device()).await.unwrap();
        assert!(issued.token.starts_with("smk_"));
    }

    #[tokio::test]
    async fn list_and_revoke_link_codes() {
        let svc = seeded().await;
        let caller = Principal { user: UserId("u1".into()), role: Role::Admin };
        let a = svc.create_link_code(&caller, None, None).await.unwrap();
        let b = svc.create_link_code(&caller, None, None).await.unwrap();
        assert_eq!(svc.list_link_codes(&UserId("u1".into())).await.unwrap().len(), 2);

        svc.revoke_link_code(&UserId("u1".into()), &a.code).await.unwrap();
        let after = svc.list_link_codes(&UserId("u1".into())).await.unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].code, b.code);
        assert!(matches!(
            svc.redeem_link_code(&a.code, device()).await.unwrap_err(),
            AuthError::UnknownLinkCode
        ));
    }

    #[tokio::test]
    async fn refresh_with_unknown_jti_is_invalid() {
        let svc = seeded().await;
        let pair = svc.login("alice", "pw").await.unwrap();
        svc.tokens.revoke_all_for_user(&UserId("u1".into())).await.unwrap();
        assert!(matches!(
            svc.refresh(&pair.refresh_token).await.unwrap_err(),
            AuthError::InvalidToken
        ));
    }

    #[tokio::test]
    async fn logout_revokes_single_refresh() {
        let svc = seeded().await;
        let pair = svc.login("alice", "pw").await.unwrap();
        svc.logout(&pair.refresh_token).await.unwrap();
        assert!(matches!(
            svc.refresh(&pair.refresh_token).await.unwrap_err(),
            AuthError::InvalidToken
        ));
    }

    #[tokio::test]
    async fn logout_rejects_garbage_access_and_missing_jti() {
        let svc = seeded().await;
        let pair = svc.login("alice", "pw").await.unwrap();
        assert!(matches!(svc.logout("not.a.jwt").await.unwrap_err(), AuthError::InvalidToken));
        assert!(matches!(
            svc.logout(&pair.access_token).await.unwrap_err(),
            AuthError::InvalidToken
        ));
        let no_jti = raw_token(&Claims {
            sub: "u1".into(),
            rol: "admin".into(),
            aud: AUDIENCE.into(),
            typ: TYP_REFRESH.into(),
            exp: Timestamp::now().as_second() + 3600,
            jti: None,
        });
        assert!(matches!(svc.logout(&no_jti).await.unwrap_err(), AuthError::InvalidToken));
    }

    #[tokio::test]
    async fn logout_all_revokes_every_session() {
        let svc = seeded().await;
        let first = svc.login("alice", "pw").await.unwrap();
        let second = svc.login("alice", "pw").await.unwrap();
        svc.logout_all(&UserId("u1".into())).await.unwrap();
        assert!(svc.refresh(&first.refresh_token).await.is_err());
        assert!(svc.refresh(&second.refresh_token).await.is_err());
    }

    #[tokio::test]
    async fn logout_surfaces_backend_errors() {
        let svc = seeded().await;
        let pair = svc.login("alice", "pw").await.unwrap();
        svc.tokens.set_fail();
        assert!(matches!(
            svc.logout(&pair.refresh_token).await.unwrap_err(),
            AuthError::Repository(_)
        ));
        assert!(matches!(
            svc.logout_all(&UserId("u1".into())).await.unwrap_err(),
            AuthError::Repository(_)
        ));
    }

    #[tokio::test]
    async fn backend_errors_propagate() {
        let users = MockUserRepo::new();
        users.set_fail();
        let svc = DefaultAuthService::new(users, MockAuthTokenRepo::new(), SECRET, 3600, 86_400);
        assert!(matches!(svc.login("alice", "pw").await.unwrap_err(), AuthError::Repository(_)));
    }

    // An unreadable credential store must refuse, never fall through to a decision.
    // These are the arms where a swallowed error is worst: silently granting on a
    // failed revocation check, or silently denying and locking everyone out.
    #[tokio::test]
    async fn an_unreadable_token_store_refuses_every_credential_path() {
        let svc = seeded().await;
        let pair = svc.login("alice", "pw").await.unwrap();
        let issued = {
            let boss = Principal { user: UserId("u1".into()), role: Role::Admin };
            let link = svc.create_link_code(&boss, None, None).await.unwrap();
            svc.redeem_link_code(&link.code, device()).await.unwrap()
        };
        svc.tokens.set_fail();

        let user = UserId("u1".into());
        let boss = Principal { user: user.clone(), role: Role::Admin };

        macro_rules! is_repository_error {
            ($call:expr) => {
                assert!(matches!($call.await.unwrap_err(), AuthError::Repository(_)))
            };
        }

        is_repository_error!(svc.login("alice", "pw"));
        is_repository_error!(svc.refresh(&pair.refresh_token));
        is_repository_error!(svc.authenticate(&issued.token));
        is_repository_error!(svc.redeem_link_code("7G2K9QMP", device()));
        is_repository_error!(svc.create_link_code(&boss, None, None));
        is_repository_error!(svc.list_link_codes(&user));
        is_repository_error!(svc.revoke_link_code(&user, "7G2K9QMP"));
        is_repository_error!(svc.list_devices(&user));
        is_repository_error!(svc.list_api_tokens(&user));
        is_repository_error!(svc.revoke_device(&user, &DeviceId("d1".into())));
        is_repository_error!(svc.revoke_api_token(&user, &ApiTokenId("t1".into())));
    }

    // The account lookup is a separate store from the credentials, and its failure
    // must not read as "this account is inactive", which would deny a valid session.
    #[tokio::test]
    async fn an_unreadable_account_refuses_rather_than_reading_as_inactive() {
        let svc = seeded().await;
        let pair = svc.login("alice", "pw").await.unwrap();
        // A real code, or the redeem below stops at UnknownLinkCode and never
        // reaches the account lookup this test is about.
        let link = svc
            .create_link_code(
                &Principal { user: UserId("u1".into()), role: Role::Admin },
                None,
                None,
            )
            .await
            .unwrap();
        svc.users.set_fail();

        assert!(matches!(
            svc.refresh(&pair.refresh_token).await.unwrap_err(),
            AuthError::Repository(_)
        ));
        assert!(matches!(
            svc.authenticate(&pair.access_token).await.unwrap_err(),
            AuthError::Repository(_)
        ));
        assert!(matches!(
            svc.redeem_link_code(&link.code, device()).await.unwrap_err(),
            AuthError::Repository(_)
        ));
    }

    fn stored_device(id: &str, user: &str) -> Device {
        Device {
            id: DeviceId(id.into()),
            user: UserId(user.into()),
            name: "Roku".into(),
            platform: "roku".into(),
            created_at: Timestamp::UNIX_EPOCH,
            last_seen: None,
        }
    }

    fn stored_token(id: &str, user: &str, device: &str, hash: &str) -> ApiToken {
        ApiToken {
            id: ApiTokenId(id.into()),
            user: UserId(user.into()),
            device: DeviceId(device.into()),
            token_hash: hash.into(),
            created_at: Timestamp::UNIX_EPOCH,
            last_used_at: None,
        }
    }

    #[tokio::test]
    async fn lists_are_user_scoped() {
        let svc = seeded().await;
        svc.tokens.upsert_device(stored_device("d1", "u1")).await.unwrap();
        svc.tokens.upsert_device(stored_device("d2", "u2")).await.unwrap();
        svc.tokens.store_api_token(stored_token("t1", "u1", "d1", "h1")).await.unwrap();
        svc.tokens.store_api_token(stored_token("t2", "u2", "d2", "h2")).await.unwrap();

        let devices = svc.list_devices(&UserId("u1".into())).await.unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, DeviceId("d1".into()));
        let tokens = svc.list_api_tokens(&UserId("u1".into())).await.unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].id, ApiTokenId("t1".into()));
    }

    #[tokio::test]
    async fn revoke_device_cascades_its_tokens() {
        let svc = seeded().await;
        let user = UserId("u1".into());
        svc.tokens.upsert_device(stored_device("d1", "u1")).await.unwrap();
        svc.tokens.upsert_device(stored_device("d2", "u1")).await.unwrap();
        svc.tokens.store_api_token(stored_token("t1", "u1", "d1", "h1")).await.unwrap();
        svc.tokens.store_api_token(stored_token("t2", "u1", "d1", "h2")).await.unwrap();
        svc.tokens.store_api_token(stored_token("t3", "u1", "d2", "h3")).await.unwrap();

        svc.revoke_device(&user, &DeviceId("d1".into())).await.unwrap();

        assert_eq!(svc.list_devices(&user).await.unwrap().len(), 1);
        let tokens = svc.list_api_tokens(&user).await.unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].id, ApiTokenId("t3".into()));
    }

    #[tokio::test]
    async fn revoke_device_absent_or_not_owned_is_not_found() {
        let svc = seeded().await;
        let user = UserId("u1".into());
        svc.tokens.upsert_device(stored_device("foreign", "u2")).await.unwrap();
        assert!(matches!(
            svc.revoke_device(&user, &DeviceId("ghost".into())).await.unwrap_err(),
            AuthError::NotFound
        ));
        assert!(matches!(
            svc.revoke_device(&user, &DeviceId("foreign".into())).await.unwrap_err(),
            AuthError::NotFound
        ));
        assert_eq!(
            svc.list_devices(&UserId("u2".into())).await.unwrap().len(),
            1,
            "another user's device must survive a revoke aimed at it"
        );
    }

    #[tokio::test]
    async fn revoke_api_token_present_and_not_owned() {
        let svc = seeded().await;
        let user = UserId("u1".into());
        svc.tokens.store_api_token(stored_token("t1", "u1", "d1", "h1")).await.unwrap();
        svc.tokens.store_api_token(stored_token("foreign", "u2", "d9", "h9")).await.unwrap();

        assert!(matches!(
            svc.revoke_api_token(&user, &ApiTokenId("foreign".into())).await.unwrap_err(),
            AuthError::NotFound
        ));
        assert!(matches!(
            svc.revoke_api_token(&user, &ApiTokenId("ghost".into())).await.unwrap_err(),
            AuthError::NotFound
        ));
        assert_eq!(
            svc.list_api_tokens(&UserId("u2".into())).await.unwrap().len(),
            1,
            "another user's token must survive a revoke aimed at it"
        );

        svc.revoke_api_token(&user, &ApiTokenId("t1".into())).await.unwrap();
        assert!(svc.list_api_tokens(&user).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn revoke_api_token_disables_authentication() {
        let svc = seeded().await;
        let future = Timestamp::from_second(4_000_000_000).unwrap();
        svc.tokens
            .store_link_code(
                PendingLink {
                    code: "7G2K9QMP".into(),
                    user: UserId("u1".into()),
                    role: Role::Player,
                    expires_at: future,
                },
                Timestamp::UNIX_EPOCH,
            )
            .await
            .unwrap();
        let issued = svc.redeem_link_code("7G2K9QMP", device()).await.unwrap();
        assert!(svc.authenticate(&issued.token).await.is_ok());

        let tokens = svc.list_api_tokens(&UserId("u1".into())).await.unwrap();
        assert_eq!(tokens.len(), 1);
        svc.revoke_api_token(&UserId("u1".into()), &tokens[0].id).await.unwrap();
        assert!(matches!(
            svc.authenticate(&issued.token).await.unwrap_err(),
            AuthError::InvalidToken
        ));
    }

    #[tokio::test]
    async fn device_token_operations_surface_backend_errors() {
        let svc = seeded().await;
        svc.tokens.set_fail();
        let user = UserId("u1".into());
        assert!(svc.list_devices(&user).await.is_err());
        assert!(svc.list_api_tokens(&user).await.is_err());
        assert!(matches!(
            svc.revoke_device(&user, &DeviceId("d1".into())).await.unwrap_err(),
            AuthError::Repository(_)
        ));
        assert!(matches!(
            svc.revoke_api_token(&user, &ApiTokenId("t1".into())).await.unwrap_err(),
            AuthError::Repository(_)
        ));
    }

    fn raw_token(claims: &Claims) -> String {
        encode(&Header::new(Algorithm::HS256), claims, &EncodingKey::from_secret(SECRET)).unwrap()
    }

    #[tokio::test]
    async fn clone_shares_state() {
        let svc = seeded().await;
        let clone = svc.clone();
        let pair = clone.login("alice", "pw").await.unwrap();
        assert!(svc.authenticate(&pair.access_token).await.is_ok());
    }

    #[tokio::test]
    async fn creates_and_reads_user_role() {
        let svc = service(3600, 86_400);
        svc.users.insert(user("u2", "bob", Role::User));
        let pair = svc.login("bob", "pw").await.unwrap();
        assert_eq!(svc.authenticate(&pair.access_token).await.unwrap().role, Role::User);
    }

    #[tokio::test]
    async fn authenticate_rejects_unknown_role() {
        let svc = seeded().await;
        let token = raw_token(&Claims {
            sub: "u1".into(),
            rol: "ghost".into(),
            aud: AUDIENCE.into(),
            typ: TYP_ACCESS.into(),
            exp: Timestamp::now().as_second() + 3600,
            jti: None,
        });
        assert!(matches!(svc.authenticate(&token).await.unwrap_err(), AuthError::InvalidToken));
    }

    #[tokio::test]
    async fn refresh_rejects_hash_mismatch() {
        let svc = seeded().await;
        let exp = Timestamp::now().as_second() + 3600;
        let token = raw_token(&Claims {
            sub: "u1".into(),
            rol: "admin".into(),
            aud: AUDIENCE.into(),
            typ: TYP_REFRESH.into(),
            exp,
            jti: Some("fixed".into()),
        });
        svc.tokens
            .store_refresh(AuthSession {
                id: AuthSessionId("fixed".into()),
                user: UserId("u1".into()),
                refresh_token_hash: "not-the-signature".into(),
                issued_at: Timestamp::UNIX_EPOCH,
                expires_at: Timestamp::from_second(exp).unwrap(),
            })
            .await
            .unwrap();
        assert!(matches!(svc.refresh(&token).await.unwrap_err(), AuthError::InvalidToken));
    }

    #[tokio::test]
    async fn out_of_range_expiry_surfaces_backend_error() {
        let svc = service(3600, 300_000_000_000);
        svc.users.insert(user("u1", "alice", Role::Admin));
        assert!(matches!(svc.login("alice", "pw").await.unwrap_err(), AuthError::Repository(_)));
    }
}
