use serde::Deserialize;

use domain::playback::{PlaylistId, WatchTarget};

use crate::dto::user_library::WatchTargetKind;

#[derive(Debug, Deserialize)]
pub struct PlaylistNameRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct PlaylistItemRequest {
    #[serde(rename = "type")]
    pub kind: WatchTargetKind,
    pub id: String,
}

impl PlaylistItemRequest {
    pub fn into_target(self) -> WatchTarget {
        self.kind.target(self.id)
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
    use domain::catalog::SeasonId;

    #[test]
    fn an_item_request_names_what_to_add() {
        let req: PlaylistItemRequest =
            serde_json::from_str(r#"{"type":"season","id":"se1"}"#).unwrap();
        assert_eq!(req.into_target(), WatchTarget::Season(SeasonId("se1".into())));
    }

    #[test]
    fn a_queue_request_plays_in_order_unless_asked_to_shuffle() {
        let req: QueuePlaylistRequest = serde_json::from_str(r#"{"playlist_id":"pl1"}"#).unwrap();
        assert!(!req.shuffle);
        assert_eq!(req.playlist(), PlaylistId("pl1".into()));
    }
}
