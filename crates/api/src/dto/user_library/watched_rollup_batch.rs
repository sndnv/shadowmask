use serde::{Deserialize, Serialize};

use domain::catalog::{EpisodeId, MovieId, SeasonId, SeriesId};
use domain::playback::{WatchTarget, WatchedRollup};

use super::WatchTargetKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchTargetRefDto {
    #[serde(rename = "type")]
    pub kind: WatchTargetKind,
    pub id: String,
}

impl WatchTargetRefDto {
    pub fn into_target(self) -> WatchTarget {
        match self.kind {
            WatchTargetKind::Movie => WatchTarget::Movie(MovieId(self.id)),
            WatchTargetKind::Episode => WatchTarget::Episode(EpisodeId(self.id)),
            WatchTargetKind::Season => WatchTarget::Season(SeasonId(self.id)),
            WatchTargetKind::Series => WatchTarget::Series(SeriesId(self.id)),
        }
    }
}

impl From<WatchTarget> for WatchTargetRefDto {
    fn from(target: WatchTarget) -> Self {
        let (kind, id) = match target {
            WatchTarget::Movie(id) => (WatchTargetKind::Movie, id.0),
            WatchTarget::Episode(id) => (WatchTargetKind::Episode, id.0),
            WatchTarget::Season(id) => (WatchTargetKind::Season, id.0),
            WatchTarget::Series(id) => (WatchTargetKind::Series, id.0),
        };
        WatchTargetRefDto { kind, id }
    }
}

#[derive(Debug, Deserialize)]
pub struct WatchedRollupBatchRequest {
    pub targets: Vec<WatchTargetRefDto>,
}

#[derive(Debug, Serialize)]
pub struct WatchedRollupResponse {
    pub target: WatchTargetRefDto,
    pub watched: bool,
    pub completed: bool,
    pub watched_episodes: u32,
    pub total_episodes: u32,
}

impl From<WatchedRollup> for WatchedRollupResponse {
    fn from(r: WatchedRollup) -> Self {
        WatchedRollupResponse {
            target: r.target.into(),
            watched: r.watched,
            completed: r.completed,
            watched_episodes: r.watched_episodes,
            total_episodes: r.total_episodes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ref_dto(kind: WatchTargetKind, id: &str) -> WatchTargetRefDto {
        WatchTargetRefDto {
            kind,
            id: id.into(),
        }
    }

    #[test]
    fn into_target_maps_all_kinds() {
        assert_eq!(
            ref_dto(WatchTargetKind::Movie, "m1").into_target(),
            WatchTarget::Movie(MovieId("m1".into()))
        );
        assert_eq!(
            ref_dto(WatchTargetKind::Episode, "e1").into_target(),
            WatchTarget::Episode(EpisodeId("e1".into()))
        );
        assert_eq!(
            ref_dto(WatchTargetKind::Season, "se1").into_target(),
            WatchTarget::Season(SeasonId("se1".into()))
        );
        assert_eq!(
            ref_dto(WatchTargetKind::Series, "sr1").into_target(),
            WatchTarget::Series(SeriesId("sr1".into()))
        );
    }

    #[test]
    fn from_target_round_trips_all_kinds() {
        for target in [
            WatchTarget::Movie(MovieId("m1".into())),
            WatchTarget::Episode(EpisodeId("e1".into())),
            WatchTarget::Season(SeasonId("se1".into())),
            WatchTarget::Series(SeriesId("sr1".into())),
        ] {
            let dto = WatchTargetRefDto::from(target.clone());
            assert_eq!(dto.into_target(), target);
        }
    }

    #[test]
    fn deserializes_request() {
        let req: WatchedRollupBatchRequest = serde_json::from_str(
            r#"{"targets":[{"type":"series","id":"s1"},{"type":"season","id":"se1"}]}"#,
        )
        .unwrap();
        assert_eq!(req.targets.len(), 2);
        assert_eq!(
            req.targets[0].clone().into_target(),
            WatchTarget::Series(SeriesId("s1".into()))
        );
    }

    #[test]
    fn serializes_response() {
        let response = WatchedRollupResponse::from(WatchedRollup {
            target: WatchTarget::Series(SeriesId("s1".into())),
            watched: false,
            completed: false,
            watched_episodes: 3,
            total_episodes: 10,
        });
        let value = serde_json::to_value(response).unwrap();
        assert_eq!(
            value["target"],
            serde_json::json!({"type": "series", "id": "s1"})
        );
        assert_eq!(value["watched"], false);
        assert_eq!(value["completed"], false);
        assert_eq!(value["watched_episodes"], 3);
        assert_eq!(value["total_episodes"], 10);
    }
}
