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
    pub url: String,
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
    url: String,
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
            url: item.url.clone(),
        }
    }
}

impl From<WireItem> for ArtworkJobItem {
    fn from(item: WireItem) -> Self {
        ArtworkJobItem {
            id: ArtworkId(item.id),
            kind: item.kind.into(),
            url: item.url,
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
            url: format!("https://cdn/{id}.jpg"),
        }
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
