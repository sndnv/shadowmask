use serde::Deserialize;

use domain::library::ResolveTarget;
use domain::metadata::ExternalId;

use crate::dto::common::TitleRefDto;

#[derive(Debug, Deserialize)]
pub struct ResolveUnmatchedRequest {
    pub target: ResolveTargetInput,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResolveTargetInput {
    Existing { title: TitleRefDto },
    Provider { source: String, value: String },
}

impl From<ResolveTargetInput> for ResolveTarget {
    fn from(input: ResolveTargetInput) -> Self {
        match input {
            ResolveTargetInput::Existing { title } => ResolveTarget::Existing(title.into()),
            ResolveTargetInput::Provider { source, value } => {
                ResolveTarget::Provider(ExternalId { source, value })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId};

    #[test]
    fn deserializes_and_maps_both_targets() {
        let existing: ResolveUnmatchedRequest =
            serde_json::from_value(serde_json::json!({"target": {"kind": "existing", "title": {"type": "movie", "id": "m1"}}}))
                .unwrap();
        assert_eq!(
            ResolveTarget::from(existing.target),
            ResolveTarget::Existing(TitleId::Movie(MovieId("m1".into())))
        );

        let provider: ResolveUnmatchedRequest = serde_json::from_value(
            serde_json::json!({"target": {"kind": "provider", "source": "tmdb", "value": "movie/603"}}),
        )
        .unwrap();
        assert_eq!(
            ResolveTarget::from(provider.target),
            ResolveTarget::Provider(ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            })
        );
    }
}
