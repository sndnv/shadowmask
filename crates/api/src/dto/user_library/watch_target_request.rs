use serde::Deserialize;

use domain::catalog::{EpisodeId, MovieId, SeasonId, SeriesId};
use domain::playback::WatchTarget;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchTargetKind {
    Movie,
    Episode,
    Season,
    Series,
}

#[derive(Debug, Deserialize)]
pub struct WatchTargetRequest {
    #[serde(rename = "type")]
    pub kind: WatchTargetKind,
    pub watched: bool,
}

impl WatchTargetRequest {
    pub fn into_target(&self, id: String) -> WatchTarget {
        match self.kind {
            WatchTargetKind::Movie => WatchTarget::Movie(MovieId(id)),
            WatchTargetKind::Episode => WatchTarget::Episode(EpisodeId(id)),
            WatchTargetKind::Season => WatchTarget::Season(SeasonId(id)),
            WatchTargetKind::Series => WatchTarget::Series(SeriesId(id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(kind: WatchTargetKind) -> WatchTargetRequest {
        WatchTargetRequest {
            kind,
            watched: true,
        }
    }

    #[test]
    fn into_target_maps_every_kind() {
        assert_eq!(
            request(WatchTargetKind::Movie).into_target("m1".into()),
            WatchTarget::Movie(MovieId("m1".into()))
        );
        assert_eq!(
            request(WatchTargetKind::Episode).into_target("e1".into()),
            WatchTarget::Episode(EpisodeId("e1".into()))
        );
        assert_eq!(
            request(WatchTargetKind::Season).into_target("se1".into()),
            WatchTarget::Season(SeasonId("se1".into()))
        );
        assert_eq!(
            request(WatchTargetKind::Series).into_target("sr1".into()),
            WatchTarget::Series(SeriesId("sr1".into()))
        );
    }

    #[test]
    fn deserializes_type_and_watched() {
        let req: WatchTargetRequest =
            serde_json::from_str(r#"{"type":"series","watched":false}"#).unwrap();
        assert!(!req.watched);
        assert_eq!(
            req.into_target("sr1".into()),
            WatchTarget::Series(SeriesId("sr1".into()))
        );
    }
}
