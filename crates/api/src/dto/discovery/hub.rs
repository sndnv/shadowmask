use serde::Serialize;

use domain::discovery::{Hub, HubItem};

use crate::dto::catalog::{EpisodeResponse, MovieResponse, SeriesResponse};

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum HubItemResponse {
    Movie(Box<MovieResponse>),
    Series(Box<HubSeriesResponse>),
    Episode(Box<EpisodeResponse>),
}

#[derive(Debug, Serialize)]
pub struct HubSeriesResponse {
    #[serde(flatten)]
    pub series: SeriesResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_count: Option<u32>,
}

impl From<HubItem> for HubItemResponse {
    fn from(i: HubItem) -> Self {
        match i {
            HubItem::Movie(m) => HubItemResponse::Movie(Box::new(m.into())),
            HubItem::Series {
                series,
                episode_count,
            } => HubItemResponse::Series(Box::new(HubSeriesResponse {
                series: series.into(),
                episode_count,
            })),
            HubItem::Episode(card) => HubItemResponse::Episode(Box::new((*card).into())),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HubResponse {
    pub id: String,
    pub title: String,
    pub items: Vec<HubItemResponse>,
}

impl From<Hub> for HubResponse {
    fn from(h: Hub) -> Self {
        HubResponse {
            id: h.id,
            title: h.title,
            items: h.items.into_iter().map(Into::into).collect(),
        }
    }
}
