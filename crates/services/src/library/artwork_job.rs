use domain::catalog::{
    ArtworkId, ArtworkOwner, CollectionId, EpisodeId, MovieId, SeasonId, SeriesId,
};
use domain::metadata::{ArtworkKind, PersonId};
use serde::{Deserialize, Serialize};

use crate::job::encode_payload;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtworkJobPayload {
    pub owner: ArtworkOwner,
    pub items: Vec<ArtworkJobItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtworkJobItem {
    pub id: ArtworkId,
    pub kind: ArtworkKind,
    pub sources: Vec<ArtworkSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtworkSource {
    Provider(String),
    Thumbnail(String),
    Frame { path: String, offset_ms: u64 },
}

impl ArtworkJobPayload {
    pub fn encode(&self) -> String {
        encode_payload(&Wire::from(self))
    }

    pub fn decode(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Wire>(raw).map(Self::from)
    }
}

#[derive(Serialize, Deserialize)]
struct Wire {
    owner: WireOwner,
    items: Vec<WireItem>,
}

#[derive(Serialize, Deserialize)]
struct WireOwner {
    kind: WireOwnerKind,
    id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireOwnerKind {
    Movie,
    Series,
    Season,
    Episode,
    Collection,
    Person,
}

#[derive(Serialize, Deserialize)]
struct WireItem {
    id: String,
    kind: WireArtworkKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(default)]
    sources: Vec<WireSource>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WireSource {
    Provider { url: String },
    Thumbnail { url: String },
    Frame { path: String, offset_ms: u64 },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireArtworkKind {
    Poster,
    Backdrop,
    Banner,
    Logo,
    ClearArt,
}

impl From<&ArtworkJobPayload> for Wire {
    fn from(payload: &ArtworkJobPayload) -> Self {
        Wire {
            owner: WireOwner::from(&payload.owner),
            items: payload.items.iter().map(WireItem::from).collect(),
        }
    }
}

impl From<Wire> for ArtworkJobPayload {
    fn from(wire: Wire) -> Self {
        ArtworkJobPayload {
            owner: wire.owner.into(),
            items: wire.items.into_iter().map(ArtworkJobItem::from).collect(),
        }
    }
}

impl From<&ArtworkOwner> for WireOwner {
    fn from(owner: &ArtworkOwner) -> Self {
        let (kind, id) = match owner {
            ArtworkOwner::Movie(id) => (WireOwnerKind::Movie, id.0.clone()),
            ArtworkOwner::Series(id) => (WireOwnerKind::Series, id.0.clone()),
            ArtworkOwner::Season(id) => (WireOwnerKind::Season, id.0.clone()),
            ArtworkOwner::Episode(id) => (WireOwnerKind::Episode, id.0.clone()),
            ArtworkOwner::Collection(id) => (WireOwnerKind::Collection, id.0.clone()),
            ArtworkOwner::Person(id) => (WireOwnerKind::Person, id.0.clone()),
        };
        WireOwner { kind, id }
    }
}

impl From<WireOwner> for ArtworkOwner {
    fn from(owner: WireOwner) -> Self {
        match owner.kind {
            WireOwnerKind::Movie => ArtworkOwner::Movie(MovieId(owner.id)),
            WireOwnerKind::Series => ArtworkOwner::Series(SeriesId(owner.id)),
            WireOwnerKind::Season => ArtworkOwner::Season(SeasonId(owner.id)),
            WireOwnerKind::Episode => ArtworkOwner::Episode(EpisodeId(owner.id)),
            WireOwnerKind::Collection => ArtworkOwner::Collection(CollectionId(owner.id)),
            WireOwnerKind::Person => ArtworkOwner::Person(PersonId(owner.id)),
        }
    }
}

impl From<&ArtworkJobItem> for WireItem {
    fn from(item: &ArtworkJobItem) -> Self {
        WireItem {
            id: item.id.0.clone(),
            kind: WireArtworkKind::from(item.kind),
            url: None,
            sources: item.sources.iter().map(WireSource::from).collect(),
        }
    }
}

impl From<WireItem> for ArtworkJobItem {
    fn from(item: WireItem) -> Self {
        ArtworkJobItem {
            id: ArtworkId(item.id),
            kind: item.kind.into(),
            sources: item
                .url
                .map(ArtworkSource::Provider)
                .into_iter()
                .chain(item.sources.into_iter().map(ArtworkSource::from))
                .collect(),
        }
    }
}

impl From<&ArtworkSource> for WireSource {
    fn from(source: &ArtworkSource) -> Self {
        match source {
            ArtworkSource::Provider(url) => WireSource::Provider { url: url.clone() },
            ArtworkSource::Thumbnail(url) => WireSource::Thumbnail { url: url.clone() },
            ArtworkSource::Frame { path, offset_ms } => {
                WireSource::Frame { path: path.clone(), offset_ms: *offset_ms }
            }
        }
    }
}

impl From<WireSource> for ArtworkSource {
    fn from(source: WireSource) -> Self {
        match source {
            WireSource::Provider { url } => ArtworkSource::Provider(url),
            WireSource::Thumbnail { url } => ArtworkSource::Thumbnail(url),
            WireSource::Frame { path, offset_ms } => ArtworkSource::Frame { path, offset_ms },
        }
    }
}

impl From<ArtworkKind> for WireArtworkKind {
    fn from(kind: ArtworkKind) -> Self {
        match kind {
            ArtworkKind::Poster => WireArtworkKind::Poster,
            ArtworkKind::Backdrop => WireArtworkKind::Backdrop,
            ArtworkKind::Banner => WireArtworkKind::Banner,
            ArtworkKind::Logo => WireArtworkKind::Logo,
            ArtworkKind::ClearArt => WireArtworkKind::ClearArt,
        }
    }
}

impl From<WireArtworkKind> for ArtworkKind {
    fn from(kind: WireArtworkKind) -> Self {
        match kind {
            WireArtworkKind::Poster => ArtworkKind::Poster,
            WireArtworkKind::Backdrop => ArtworkKind::Backdrop,
            WireArtworkKind::Banner => ArtworkKind::Banner,
            WireArtworkKind::Logo => ArtworkKind::Logo,
            WireArtworkKind::ClearArt => ArtworkKind::ClearArt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, kind: ArtworkKind) -> ArtworkJobItem {
        ArtworkJobItem {
            id: ArtworkId(id.to_owned()),
            kind,
            sources: vec![ArtworkSource::Provider(format!("https://cdn/{id}.jpg"))],
        }
    }

    #[test]
    fn round_trips_every_source_in_order() {
        let payload = ArtworkJobPayload {
            owner: ArtworkOwner::Episode(EpisodeId("e1".into())),
            items: vec![ArtworkJobItem {
                id: ArtworkId("a".into()),
                kind: ArtworkKind::Backdrop,
                sources: vec![
                    ArtworkSource::Provider("https://tmdb/still.jpg".into()),
                    ArtworkSource::Thumbnail("https://i.ytimg.com/vi/abc/maxresdefault.jpg".into()),
                    ArtworkSource::Frame {
                        path: "/ext/Show/Season 01/x.mkv".into(),
                        offset_ms: 61_500,
                    },
                ],
            }],
        };
        let encoded = payload.encode();
        let raw: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert!(raw["items"][0].get("url").is_none(), "a new payload writes sources only");
        assert_eq!(ArtworkJobPayload::decode(&encoded).unwrap(), payload);
    }

    #[test]
    fn a_payload_queued_before_sources_decodes_its_url_as_the_provider_source() {
        let raw = r#"{"owner":{"kind":"movie","id":"m1"},"items":[{"id":"a","kind":"poster","url":"https://cdn/a.jpg"}]}"#;
        let payload = ArtworkJobPayload::decode(raw).unwrap();
        assert_eq!(payload.items, vec![item("a", ArtworkKind::Poster)]);
    }

    #[test]
    fn round_trips_every_owner_and_artwork_kind() {
        let owners = [
            ArtworkOwner::Movie(MovieId("m1".into())),
            ArtworkOwner::Series(SeriesId("s1".into())),
            ArtworkOwner::Season(SeasonId("se1".into())),
            ArtworkOwner::Episode(EpisodeId("e1".into())),
            ArtworkOwner::Collection(CollectionId("c1".into())),
            ArtworkOwner::Person(PersonId("p1".into())),
        ];
        for owner in owners {
            let payload = ArtworkJobPayload {
                owner,
                items: vec![
                    item("a", ArtworkKind::Poster),
                    item("b", ArtworkKind::Backdrop),
                    item("c", ArtworkKind::Banner),
                    item("d", ArtworkKind::Logo),
                    item("e", ArtworkKind::ClearArt),
                ],
            };
            let encoded = payload.encode();
            assert_eq!(ArtworkJobPayload::decode(&encoded).unwrap(), payload);
        }
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(ArtworkJobPayload::decode("not json").is_err());
    }
}
