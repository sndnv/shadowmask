use domain::catalog::{EpisodeId, MovieId, TitleId};
use domain::library::{LibraryId, ResolveTarget, UnmatchedFileId};
use domain::metadata::ExternalId;
use serde::{Deserialize, Serialize};

use crate::job::encode_payload;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestJobPayload {
    pub library: LibraryId,
    pub unmatched: UnmatchedFileId,
    pub path: String,
    pub target: ResolveTarget,
}

impl IngestJobPayload {
    pub fn encode(&self) -> String {
        encode_payload(&Wire::from(self))
    }

    pub fn decode(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Wire>(raw).map(Self::from)
    }
}

#[derive(Serialize, Deserialize)]
struct Wire {
    library: String,
    unmatched: String,
    path: String,
    target: TargetWire,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TargetWire {
    Existing {
        title_kind: String,
        title_id: String,
    },
    Provider {
        source: String,
        value: String,
    },
}

impl From<&IngestJobPayload> for Wire {
    fn from(payload: &IngestJobPayload) -> Self {
        let target = match &payload.target {
            ResolveTarget::Existing(title) => TargetWire::Existing {
                title_kind: match title {
                    TitleId::Movie(_) => "movie".to_owned(),
                    TitleId::Episode(_) => "episode".to_owned(),
                },
                title_id: title.id().to_owned(),
            },
            ResolveTarget::Provider(id) => TargetWire::Provider {
                source: id.source.clone(),
                value: id.value.clone(),
            },
        };
        Wire {
            library: payload.library.0.clone(),
            unmatched: payload.unmatched.0.clone(),
            path: payload.path.clone(),
            target,
        }
    }
}

impl From<Wire> for IngestJobPayload {
    fn from(wire: Wire) -> Self {
        let target = match wire.target {
            TargetWire::Existing {
                title_kind,
                title_id,
            } => ResolveTarget::Existing(match title_kind.as_str() {
                "episode" => TitleId::Episode(EpisodeId(title_id)),
                _ => TitleId::Movie(MovieId(title_id)),
            }),
            TargetWire::Provider { source, value } => {
                ResolveTarget::Provider(ExternalId { source, value })
            }
        };
        IngestJobPayload {
            library: LibraryId(wire.library),
            unmatched: UnmatchedFileId(wire.unmatched),
            path: wire.path,
            target,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(payload: IngestJobPayload) {
        let encoded = payload.encode();
        assert_eq!(IngestJobPayload::decode(&encoded).unwrap(), payload);
    }

    #[test]
    fn round_trips_existing_movie_and_episode() {
        round_trip(IngestJobPayload {
            library: LibraryId("lib1".into()),
            unmatched: UnmatchedFileId("uf1".into()),
            path: "/media/x.mkv".into(),
            target: ResolveTarget::Existing(TitleId::Movie(MovieId("m1".into()))),
        });
        round_trip(IngestJobPayload {
            library: LibraryId("lib1".into()),
            unmatched: UnmatchedFileId("uf1".into()),
            path: "/media/x.mkv".into(),
            target: ResolveTarget::Existing(TitleId::Episode(EpisodeId("e1".into()))),
        });
    }

    #[test]
    fn round_trips_provider() {
        round_trip(IngestJobPayload {
            library: LibraryId("lib1".into()),
            unmatched: UnmatchedFileId("uf1".into()),
            path: "/media/x.mkv".into(),
            target: ResolveTarget::Provider(ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            }),
        });
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(IngestJobPayload::decode("not json").is_err());
    }
}
