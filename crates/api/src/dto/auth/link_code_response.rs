use serde::Serialize;

use domain::user::PendingLink;

#[derive(Debug, Serialize)]
pub struct LinkCodeResponse {
    pub code: String,
    pub expires_at: String,
}

impl From<PendingLink> for LinkCodeResponse {
    fn from(link: PendingLink) -> Self {
        LinkCodeResponse {
            code: link.code,
            expires_at: link.expires_at.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::user::{Role, UserId};
    use jiff::Timestamp;

    #[test]
    fn maps_pending_link_to_code_and_expiry() {
        let out = LinkCodeResponse::from(PendingLink {
            code: "7G2K9QMP".into(),
            user: UserId("u1".into()),
            role: Role::Player,
            expires_at: Timestamp::from_second(1_700_000_000).unwrap(),
        });
        assert_eq!(out.code, "7G2K9QMP");
        assert_eq!(out.expires_at, "2023-11-14T22:13:20Z");
    }
}
