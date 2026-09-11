use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RandomPickResponse {
    pub version_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pick_carries_only_the_version_to_play() {
        let response = RandomPickResponse { version_id: "v1".into() };
        assert_eq!(serde_json::to_string(&response).unwrap(), r#"{"version_id":"v1"}"#);
    }
}
