use domain::catalog::{MovieId, SeriesId, TitleRef};
use domain::metadata::{ExternalId, PersonId};
use serde::{Deserialize, Serialize};

use crate::job::encode_payload;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataJobPayload {
    Title {
        title: TitleRef,
        external_id: Option<ExternalId>,
        force: bool,
    },
    People {
        ids: Vec<PersonId>,
        force: bool,
    },
}

impl MetadataJobPayload {
    pub fn encode(&self) -> String {
        encode_payload(&Wire::from(self))
    }

    pub fn decode(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Wire>(raw).map(Self::from)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "snake_case")]
enum Wire {
    Title {
        title: TitleWire,
        external_id: Option<ExternalIdWire>,
        force: bool,
    },
    People {
        ids: Vec<String>,
        force: bool,
    },
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
        match payload {
            MetadataJobPayload::Title {
                title,
                external_id,
                force,
            } => {
                let title = match title {
                    TitleRef::Movie(id) => TitleWire::Movie { id: id.0.clone() },
                    TitleRef::Series(id) => TitleWire::Series { id: id.0.clone() },
                };
                Wire::Title {
                    title,
                    external_id: external_id.as_ref().map(|id| ExternalIdWire {
                        source: id.source.clone(),
                        value: id.value.clone(),
                    }),
                    force: *force,
                }
            }
            MetadataJobPayload::People { ids, force } => Wire::People {
                ids: ids.iter().map(|id| id.0.clone()).collect(),
                force: *force,
            },
        }
    }
}

impl From<Wire> for MetadataJobPayload {
    fn from(wire: Wire) -> Self {
        match wire {
            Wire::Title {
                title,
                external_id,
                force,
            } => {
                let title = match title {
                    TitleWire::Movie { id } => TitleRef::Movie(MovieId(id)),
                    TitleWire::Series { id } => TitleRef::Series(SeriesId(id)),
                };
                MetadataJobPayload::Title {
                    title,
                    external_id: external_id.map(|id| ExternalId {
                        source: id.source,
                        value: id.value,
                    }),
                    force,
                }
            }
            Wire::People { ids, force } => MetadataJobPayload::People {
                ids: ids.into_iter().map(PersonId).collect(),
                force,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(payload: MetadataJobPayload) {
        let encoded = payload.encode();
        assert_eq!(MetadataJobPayload::decode(&encoded).unwrap(), payload);
    }

    #[test]
    fn round_trips_movie_and_series_with_and_without_external_id() {
        round_trip(MetadataJobPayload::Title {
            title: TitleRef::Movie(MovieId("m1".into())),
            external_id: Some(ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            }),
            force: false,
        });
        round_trip(MetadataJobPayload::Title {
            title: TitleRef::Series(SeriesId("s1".into())),
            external_id: None,
            force: false,
        });
    }

    #[test]
    fn round_trips_a_forced_title_refresh() {
        round_trip(MetadataJobPayload::Title {
            title: TitleRef::Movie(MovieId("m1".into())),
            external_id: None,
            force: true,
        });
        round_trip(MetadataJobPayload::Title {
            title: TitleRef::Series(SeriesId("s1".into())),
            external_id: None,
            force: true,
        });
    }

    #[test]
    fn round_trips_people() {
        round_trip(MetadataJobPayload::People {
            ids: vec![PersonId("p1".into()), PersonId("p2".into())],
            force: true,
        });
        round_trip(MetadataJobPayload::People {
            ids: Vec::new(),
            force: false,
        });
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(MetadataJobPayload::decode("not json").is_err());
    }
}
