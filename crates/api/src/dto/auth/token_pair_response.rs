use serde::Serialize;

use domain::user::TokenPair;

#[derive(Debug, Serialize)]
pub struct TokenPairResponse {
    pub access_token: String,
    pub refresh_token: String,
}

impl From<TokenPair> for TokenPairResponse {
    fn from(t: TokenPair) -> Self {
        TokenPairResponse {
            access_token: t.access_token,
            refresh_token: t.refresh_token,
        }
    }
}
