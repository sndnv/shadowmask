use serde::Serialize;

use domain::discovery::{Hub, HubItem};

use crate::dto::catalog::{MovieResponse, SeriesResponse};

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum HubItemResponse {
    Movie(Box<MovieResponse>),
    Series(Box<SeriesResponse>),
}

impl From<HubItem> for HubItemResponse {
    fn from(i: HubItem) -> Self {
        match i {
            HubItem::Movie(m) => HubItemResponse::Movie(Box::new(m.into())),
            HubItem::Series(s) => HubItemResponse::Series(Box::new(s.into())),
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
