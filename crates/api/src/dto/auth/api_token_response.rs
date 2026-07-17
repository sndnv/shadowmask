use serde::Serialize;

use domain::user::ApiToken;

#[derive(Debug, Serialize)]
pub struct ApiTokenResponse {
    pub id: String,
    pub user_id: String,
    pub device_id: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

impl From<ApiToken> for ApiTokenResponse {
    fn from(t: ApiToken) -> Self {
        ApiTokenResponse {
            id: t.id.0,
            user_id: t.user.0,
            device_id: t.device.0,
            created_at: t.created_at.to_string(),
            last_used_at: t.last_used_at.map(|t| t.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::user::{ApiTokenId, DeviceId, UserId};
    use jiff::Timestamp;

    #[test]
    fn maps_fields_and_never_exposes_hash() {
        let response = ApiTokenResponse::from(ApiToken {
            id: ApiTokenId("t1".into()),
            user: UserId("u1".into()),
            device: DeviceId("d1".into()),
            token_hash: "secret-hash".into(),
            created_at: Timestamp::UNIX_EPOCH,
            last_used_at: Some(Timestamp::UNIX_EPOCH),
        });
        assert_eq!(response.id, "t1");
        assert_eq!(response.user_id, "u1");
        assert_eq!(response.device_id, "d1");
        assert_eq!(
            response.last_used_at,
            Some(Timestamp::UNIX_EPOCH.to_string())
        );

        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("token_hash"));
        assert!(!json.contains("secret-hash"));
    }
}
