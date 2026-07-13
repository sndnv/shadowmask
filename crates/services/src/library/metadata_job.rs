use domain::catalog::{MovieId, SeriesId, TitleRef};
use domain::metadata::ExternalId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataJobPayload {
    pub title: TitleRef,
    pub external_id: Option<ExternalId>,
}

impl MetadataJobPayload {
    pub fn encode(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&Wire::from(self))
    }

    pub fn decode(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Wire>(raw).map(Self::from)
    }
}

#[derive(Serialize, Deserialize)]
struct Wire {
    title: TitleWire,
    external_id: Option<ExternalIdWire>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TitleWire {
    Movie { id: String },
    Series { id: String },
}

#[derive(Serialize, Deserialize)]
struct ExternalIdWire {
    source: String,
    value: String,
}

impl From<&MetadataJobPayload> for Wire {
    fn from(payload: &MetadataJobPayload) -> Self {
        let title = match &payload.title {
            TitleRef::Movie(id) => TitleWire::Movie { id: id.0.clone() },
            TitleRef::Series(id) => TitleWire::Series { id: id.0.clone() },
        };
        Wire {
            title,
            external_id: payload.external_id.as_ref().map(|id| ExternalIdWire {
                source: id.source.clone(),
                value: id.value.clone(),
            }),
        }
    }
}

impl From<Wire> for MetadataJobPayload {
    fn from(wire: Wire) -> Self {
        let title = match wire.title {
            TitleWire::Movie { id } => TitleRef::Movie(MovieId(id)),
            TitleWire::Series { id } => TitleRef::Series(SeriesId(id)),
        };
        MetadataJobPayload {
            title,
            external_id: wire.external_id.map(|id| ExternalId {
                source: id.source,
                value: id.value,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(payload: MetadataJobPayload) {
        let encoded = payload.encode().unwrap();
        assert_eq!(MetadataJobPayload::decode(&encoded).unwrap(), payload);
    }

    #[test]
    fn round_trips_movie_and_series_with_and_without_external_id() {
        round_trip(MetadataJobPayload {
            title: TitleRef::Movie(MovieId("m1".into())),
            external_id: Some(ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            }),
        });
        round_trip(MetadataJobPayload {
            title: TitleRef::Series(SeriesId("s1".into())),
            external_id: None,
        });
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(MetadataJobPayload::decode("not json").is_err());
    }
}
