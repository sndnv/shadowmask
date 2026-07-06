use domain::error::{AuthError, RepositoryError};
use domain::repository::{AuthTokenRepository, UserRepository};
use domain::service::AuthService;
use domain::user::{
    AuthSession, AuthSessionId, DeviceRegistration, IssuedToken, Principal, Role, TokenPair, UserId,
};
use jiff::Timestamp;
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::password;

const AUDIENCE: &str = "shadowmask-api";
const TYP_ACCESS: &str = "access";
const TYP_REFRESH: &str = "refresh";

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
    }
}

fn role_from_claim(value: &str) -> Result<Role, AuthError> {
    match value {
        "admin" => Ok(Role::Admin),
        "user" => Ok(Role::User),
        "player" => Ok(Role::Player),
        _ => Err(AuthError::InvalidToken),
    }
}

fn mint_error(error: impl ToString) -> AuthError {
    AuthError::Repository(RepositoryError::Backend(error.to_string()))
}

fn signature_of(token: &str) -> String {
    token.rsplit('.').next().unwrap_or_default().to_owned()
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
        encode(&self.header, &claims, &self.encoding_key).map_err(mint_error)
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
                issued_at: Timestamp::from_second(now).map_err(mint_error)?,
                expires_at: Timestamp::from_second(refresh_exp).map_err(mint_error)?,
            })
            .await?;
        Ok(TokenPair {
            access_token,
            refresh_token,
        })
    }
}

impl<U, T> AuthService for DefaultAuthService<U, T>
where
    U: UserRepository + Sync,
    T: AuthTokenRepository + Sync,
{
    async fn login(&self, username: &str, password: &str) -> Result<TokenPair, AuthError> {
        let user = self
            .users
            .find_by_username(username)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
        if !password::verify(password, &user.password_hash).unwrap_or(false) {
            return Err(AuthError::InvalidCredentials);
        }
        self.issue_pair(&user.id, user.role).await
    }

    async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, AuthError> {
        let claims = self.decode(refresh_token, TYP_REFRESH)?;
        let jti = AuthSessionId(claims.jti.ok_or(AuthError::InvalidToken)?);
        let stored = self
            .tokens
            .find_refresh(&jti)
            .await?
            .ok_or(AuthError::InvalidToken)?;
        if stored.refresh_token_hash != signature_of(refresh_token) {
            return Err(AuthError::InvalidToken);
        }
        let role = role_from_claim(&claims.rol)?;
        self.tokens.revoke_refresh(&jti).await?;
        self.issue_pair(&UserId(claims.sub), role).await
    }

    async fn redeem_link_code(
        &self,
        code: &str,
        _device: DeviceRegistration,
    ) -> Result<IssuedToken, AuthError> {
        let now = Timestamp::now().as_second();
        let link = self
            .tokens
            .redeem_link_code(code, Timestamp::from_second(now).map_err(mint_error)?)
            .await?
            .ok_or(AuthError::UnknownLinkCode)?;
        let exp = now + self.access_ttl_secs;
        let token = self.encode(&link.user, link.role, TYP_ACCESS, exp, None)?;
        Ok(IssuedToken {
            token,
            expires_at: Some(Timestamp::from_second(exp).map_err(mint_error)?),
        })
    }

    async fn authenticate(&self, access_token: &str) -> Result<Principal, AuthError> {
        let claims = self.decode(access_token, TYP_ACCESS)?;
        Ok(Principal {
            user: UserId(claims.sub),
            role: role_from_claim(&claims.rol)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockAuthTokenRepo, MockUserRepo};
    use domain::user::{PendingLink, User};

    const SECRET: &[u8] = b"shadowmask-test-secret";

    fn device() -> DeviceRegistration {
        DeviceRegistration {
            name: "Roku".into(),
            platform: "roku".into(),
        }
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
            created_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn service(
        access_ttl: i64,
        refresh_ttl: i64,
    ) -> DefaultAuthService<MockUserRepo, MockAuthTokenRepo> {
        let users = MockUserRepo::new();
        DefaultAuthService::new(
            users,
            MockAuthTokenRepo::new(),
            SECRET,
            access_ttl,
            refresh_ttl,
        )
    }

    async fn seeded() -> DefaultAuthService<MockUserRepo, MockAuthTokenRepo> {
        let svc = service(3600, 86_400);
        svc.users.insert(user("u1", "alice", Role::Admin));
        svc
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
        assert!(matches!(
            svc.refresh("not.a.jwt").await.unwrap_err(),
            AuthError::InvalidToken
        ));
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
    async fn redeem_link_code_issues_access_token() {
        let svc = seeded().await;
        let future = Timestamp::from_second(4_000_000_000).unwrap();
        svc.tokens
            .store_link_code(PendingLink {
                code: "CODE".into(),
                user: UserId("u1".into()),
                role: Role::Player,
                expires_at: future,
            })
            .await
            .unwrap();
        let issued = svc.redeem_link_code("CODE", device()).await.unwrap();
        let principal = svc.authenticate(&issued.token).await.unwrap();
        assert_eq!(principal.role, Role::Player);
        assert!(issued.expires_at.is_some());

        assert!(matches!(
            svc.redeem_link_code("MISSING", device()).await.unwrap_err(),
            AuthError::UnknownLinkCode
        ));
    }

    #[tokio::test]
    async fn refresh_with_unknown_jti_is_invalid() {
        let svc = seeded().await;
        let pair = svc.login("alice", "pw").await.unwrap();
        svc.tokens
            .revoke_all_for_user(&UserId("u1".into()))
            .await
            .unwrap();
        assert!(matches!(
            svc.refresh(&pair.refresh_token).await.unwrap_err(),
            AuthError::InvalidToken
        ));
    }

    #[tokio::test]
    async fn backend_errors_propagate() {
        let users = MockUserRepo::new();
        users.set_fail();
        let svc = DefaultAuthService::new(users, MockAuthTokenRepo::new(), SECRET, 3600, 86_400);
        assert!(matches!(
            svc.login("alice", "pw").await.unwrap_err(),
            AuthError::Repository(_)
        ));
    }

    fn raw_token(claims: &Claims) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(SECRET),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn clone_shares_state() {
        let svc = seeded().await;
        let clone = svc.clone();
        let pair = clone.login("alice", "pw").await.unwrap();
        assert!(svc.authenticate(&pair.access_token).await.is_ok());
    }

    #[tokio::test]
    async fn mints_and_reads_user_role() {
        let svc = service(3600, 86_400);
        svc.users.insert(user("u2", "bob", Role::User));
        let pair = svc.login("bob", "pw").await.unwrap();
        assert_eq!(
            svc.authenticate(&pair.access_token).await.unwrap().role,
            Role::User
        );
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
        assert!(matches!(
            svc.authenticate(&token).await.unwrap_err(),
            AuthError::InvalidToken
        ));
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
        assert!(matches!(
            svc.refresh(&token).await.unwrap_err(),
            AuthError::InvalidToken
        ));
    }

    #[tokio::test]
    async fn out_of_range_expiry_surfaces_backend_error() {
        let svc = service(3600, 300_000_000_000);
        svc.users.insert(user("u1", "alice", Role::Admin));
        assert!(matches!(
            svc.login("alice", "pw").await.unwrap_err(),
            AuthError::Repository(_)
        ));
    }
}
