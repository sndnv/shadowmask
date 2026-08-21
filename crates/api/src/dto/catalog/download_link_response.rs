use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DownloadLinkResponse {
    pub url: String,
    pub filename: String,
    pub size_bytes: u64,
    pub expires_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_serializes_without_the_source_path() {
        let response = DownloadLinkResponse {
            url: "/download/tok".into(),
            filename: "clip.mkv".into(),
            size_bytes: 42,
            expires_at: "2026-08-24T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("/download/tok"));
        assert!(json.contains("clip.mkv"));
        assert!(!json.contains("path"));
    }
}
