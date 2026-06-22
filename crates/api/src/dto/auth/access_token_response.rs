use serde::Serialize;

use domain::user::AccessToken;

#[derive(Debug, Serialize)]
pub struct AccessTokenResponse {
    pub access_token: String,
    pub expires_in_s: u32,
}

impl From<AccessToken> for AccessTokenResponse {
    fn from(t: AccessToken) -> Self {
        AccessTokenResponse {
            access_token: t.access_token,
            expires_in_s: t.expires_in_s,
        }
    }
}
