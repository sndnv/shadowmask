use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PurgeRequest {
    #[serde(default)]
    pub wipe_user_data: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_data_is_wiped_only_when_asked() {
        let wipe: PurgeRequest =
            serde_json::from_value(serde_json::json!({"wipe_user_data": true})).unwrap();
        let empty: PurgeRequest = serde_json::from_value(serde_json::json!({})).unwrap();

        assert!(wipe.wipe_user_data);
        assert!(!empty.wipe_user_data);
    }
}
