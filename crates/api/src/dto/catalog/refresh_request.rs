use serde::Deserialize;

use domain::metadata::ExternalId;

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub external_id: Option<ExternalIdInput>,
}

#[derive(Debug, Deserialize)]
pub struct ExternalIdInput {
    pub source: String,
    pub value: String,
}

impl From<ExternalIdInput> for ExternalId {
    fn from(input: ExternalIdInput) -> Self {
        ExternalId {
            source: input.source,
            value: input.value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_optional_external_id() {
        let empty: RefreshRequest = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(empty.external_id.is_none());

        let forced: RefreshRequest = serde_json::from_value(
            serde_json::json!({"external_id": {"source": "tmdb", "value": "movie/603"}}),
        )
        .unwrap();
        assert_eq!(
            ExternalId::from(forced.external_id.unwrap()),
            ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            }
        );
    }
}
