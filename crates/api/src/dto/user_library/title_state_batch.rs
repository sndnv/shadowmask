use serde::{Deserialize, Serialize};

use domain::playback::TitleState;

use crate::dto::common::TitleRefDto;

#[derive(Debug, Deserialize)]
pub struct TitleStateBatchRequest {
    pub titles: Vec<TitleRefDto>,
}

#[derive(Debug, Serialize)]
pub struct TitleStateResponse {
    pub title: TitleRefDto,
    pub favorite: bool,
    pub watchlisted: bool,
    pub watched: bool,
    pub completed: bool,
}

impl From<TitleState> for TitleStateResponse {
    fn from(s: TitleState) -> Self {
        TitleStateResponse {
            title: s.title.into(),
            favorite: s.favorite,
            watchlisted: s.watchlisted,
            watched: s.watched,
            completed: s.completed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId};

    #[test]
    fn deserializes_title_list() {
        let req: TitleStateBatchRequest =
            serde_json::from_str(r#"{"titles":[{"type":"movie","id":"m1"}]}"#).unwrap();
        assert_eq!(req.titles.len(), 1);
        assert_eq!(
            TitleId::from(req.titles[0].clone()),
            TitleId::Movie(MovieId("m1".into()))
        );
    }

    #[test]
    fn serializes_state() {
        let response = TitleStateResponse::from(TitleState {
            title: TitleId::Movie(MovieId("m1".into())),
            favorite: true,
            watchlisted: false,
            watched: true,
            completed: false,
        });
        let value = serde_json::to_value(response).unwrap();
        assert_eq!(
            value["title"],
            serde_json::json!({"type": "movie", "id": "m1"})
        );
        assert_eq!(value["favorite"], true);
        assert_eq!(value["watchlisted"], false);
        assert_eq!(value["watched"], true);
        assert_eq!(value["completed"], false);
    }
}
