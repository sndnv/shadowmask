use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: Option<String>,
    pub new_password: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_with_and_without_current() {
        let full: ChangePasswordRequest =
            serde_json::from_str(r#"{"current_password": "old", "new_password": "new"}"#).unwrap();
        assert_eq!(full.current_password.as_deref(), Some("old"));
        assert_eq!(full.new_password, "new");

        let reset: ChangePasswordRequest =
            serde_json::from_str(r#"{"new_password": "new"}"#).unwrap();
        assert!(reset.current_password.is_none());
        assert_eq!(reset.new_password, "new");
    }
}
