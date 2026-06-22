use serde::Serialize;

use domain::user::IssuedToken;

#[derive(Debug, Serialize)]
pub struct IssuedTokenResponse {
    pub token: String,
    pub expires_at: Option<String>,
}

impl From<IssuedToken> for IssuedTokenResponse {
    fn from(t: IssuedToken) -> Self {
        IssuedTokenResponse {
            token: t.token,
            expires_at: t.expires_at.map(|ts| ts.to_string()),
        }
    }
}
