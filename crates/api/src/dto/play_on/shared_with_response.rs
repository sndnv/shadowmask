use serde::Serialize;

use domain::user::SharedWith;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct SharedWithResponse {
    pub user_id: String,
    pub username: String,
}

impl From<SharedWith> for SharedWithResponse {
    fn from(s: SharedWith) -> Self {
        SharedWithResponse { user_id: s.user.0, username: s.username }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::user::{DeviceId, UserId};
    use serde_json::json;

    #[test]
    fn a_share_names_the_user_by_id_and_username() {
        let response = SharedWithResponse::from(SharedWith {
            device: DeviceId("roku-1".into()),
            user: UserId("u2".into()),
            username: "kid".into(),
        });
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            json!({ "user_id": "u2", "username": "kid" })
        );
    }
}
