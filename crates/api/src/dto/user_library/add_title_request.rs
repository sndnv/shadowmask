use serde::Deserialize;

use domain::catalog::{EpisodeId, MovieId, TitleId};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TitleKind {
    Movie,
    Episode,
}

#[derive(Debug, Deserialize)]
pub struct AddTitleRequest {
    #[serde(rename = "type")]
    pub kind: TitleKind,
}

impl AddTitleRequest {
    pub fn into_title(self, id: String) -> TitleId {
        match self.kind {
            TitleKind::Movie => TitleId::Movie(MovieId(id)),
            TitleKind::Episode => TitleId::Episode(EpisodeId(id)),
        }
    }
}
