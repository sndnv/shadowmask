use serde::Serialize;

use domain::user::IssuedToken;

#[derive(Debug, Serialize)]
pub struct IssuedTokenResponse {
    pub token: String,
    pub expires_at: Option<String>,
}

impl From<IssuedToken> for IssuedTokenResponse {
    fn from(t: IssuedToken) -> Self {
        IssuedTokenResponse { token: t.token, expires_at: t.expires_at.map(|ts| ts.to_string()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::Timestamp;

    #[test]
    fn maps_some_expiry_to_string() {
        let out = IssuedTokenResponse::from(IssuedToken {
            token: "tok".to_owned(),
            expires_at: Some(Timestamp::from_second(1_700_000_000).unwrap()),
        });
        assert_eq!(out.token, "tok");
        assert_eq!(out.expires_at.as_deref(), Some("2023-11-14T22:13:20Z"));
    }

    #[test]
    fn maps_none_expiry_to_none() {
        let out =
            IssuedTokenResponse::from(IssuedToken { token: "tok".to_owned(), expires_at: None });
        assert!(out.expires_at.is_none());
    }
}
