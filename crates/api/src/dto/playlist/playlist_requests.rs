use serde::Deserialize;

use domain::catalog::CollectionId;
use domain::playback::{PlaylistId, PlaylistTarget};

use crate::dto::user_library::WatchTargetKind;

#[derive(Debug, Deserialize)]
pub struct PlaylistNameRequest {
    pub name: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaylistItemKind {
    Movie,
    Episode,
    Season,
    Series,
    Collection,
}

#[derive(Debug, Deserialize)]
pub struct PlaylistItemRequest {
    #[serde(rename = "type")]
    pub kind: PlaylistItemKind,
    pub id: String,
}

impl PlaylistItemRequest {
    pub fn into_target(self) -> PlaylistTarget {
        let title = match self.kind {
            PlaylistItemKind::Movie => WatchTargetKind::Movie,
            PlaylistItemKind::Episode => WatchTargetKind::Episode,
            PlaylistItemKind::Season => WatchTargetKind::Season,
            PlaylistItemKind::Series => WatchTargetKind::Series,
            PlaylistItemKind::Collection => {
                return PlaylistTarget::Collection(CollectionId(self.id));
            }
        };
        PlaylistTarget::Title(title.target(self.id))
    }
}

#[derive(Debug, Deserialize)]
pub struct PlaylistPositionRequest {
    pub position: u32,
}

#[derive(Debug, Deserialize)]
pub struct QueuePlaylistRequest {
    pub playlist_id: String,
    #[serde(default)]
    pub shuffle: bool,
}

impl QueuePlaylistRequest {
    pub fn playlist(&self) -> PlaylistId {
        PlaylistId(self.playlist_id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{EpisodeId, MovieId, SeasonId, SeriesId};
    use domain::playback::WatchTarget;

    fn target(json: &str) -> PlaylistTarget {
        serde_json::from_str::<PlaylistItemRequest>(json).unwrap().into_target()
    }

    #[test]
    fn an_item_request_names_what_to_add() {
        let title = PlaylistTarget::Title;
        assert_eq!(
            target(r#"{"type":"movie","id":"m1"}"#),
            title(WatchTarget::Movie(MovieId("m1".into())))
        );
        assert_eq!(
            target(r#"{"type":"episode","id":"e1"}"#),
            title(WatchTarget::Episode(EpisodeId("e1".into())))
        );
        assert_eq!(
            target(r#"{"type":"season","id":"se1"}"#),
            title(WatchTarget::Season(SeasonId("se1".into())))
        );
        assert_eq!(
            target(r#"{"type":"series","id":"sr1"}"#),
            title(WatchTarget::Series(SeriesId("sr1".into())))
        );
        assert_eq!(
            target(r#"{"type":"collection","id":"c1"}"#),
            PlaylistTarget::Collection(CollectionId("c1".into()))
        );
    }

    #[test]
    fn an_item_request_refuses_a_kind_it_cannot_add() {
        let person = serde_json::from_str::<PlaylistItemRequest>(r#"{"type":"person","id":"p1"}"#);
        assert!(person.is_err());
    }

    #[test]
    fn a_queue_request_plays_in_order_unless_asked_to_shuffle() {
        let req: QueuePlaylistRequest = serde_json::from_str(r#"{"playlist_id":"pl1"}"#).unwrap();
        assert!(!req.shuffle);
        assert_eq!(req.playlist(), PlaylistId("pl1".into()));
    }
}
