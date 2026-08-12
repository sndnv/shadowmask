use serde::Deserialize;

use domain::catalog::{EpisodeId, MovieId, TitleId};

use crate::dto::common::TitleRefKind;

#[derive(Debug, Deserialize)]
pub struct AddTitleRequest {
    #[serde(rename = "type")]
    pub kind: TitleRefKind,
}

impl AddTitleRequest {
    pub fn into_title(self, id: String) -> TitleId {
        match self.kind {
            TitleRefKind::Movie => TitleId::Movie(MovieId(id)),
            TitleRefKind::Episode => TitleId::Episode(EpisodeId(id)),
        }
    }
}
