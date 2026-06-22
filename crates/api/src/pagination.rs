use serde::{Deserialize, Serialize};

use domain::common::{Page, PageRequest};

const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 200;

#[derive(Debug, Deserialize)]
pub struct PageParams {
    #[serde(default)]
    pub offset: u32,
    pub limit: Option<u32>,
}

impl PageParams {
    pub fn to_request(&self) -> PageRequest {
        PageRequest {
            offset: self.offset,
            limit: self.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub offset: u32,
    pub limit: u32,
}

impl<T> PageResponse<T> {
    pub fn from_page<S>(page: Page<S>, map: impl Fn(S) -> T) -> Self {
        PageResponse {
            items: page.items.into_iter().map(map).collect(),
            total: page.total,
            offset: page.offset,
            limit: page.limit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_default_limit() {
        let params = PageParams {
            offset: 0,
            limit: None,
        };
        let req = params.to_request();
        assert_eq!(req.offset, 0);
        assert_eq!(req.limit, DEFAULT_LIMIT);
    }

    #[test]
    fn clamps_limit_to_max() {
        let params = PageParams {
            offset: 7,
            limit: Some(10_000),
        };
        let req = params.to_request();
        assert_eq!(req.offset, 7);
        assert_eq!(req.limit, MAX_LIMIT);
    }

    #[test]
    fn from_page_maps_items_and_meta() {
        let page = Page {
            items: vec![1u32, 2, 3],
            total: 3,
            offset: 0,
            limit: 50,
        };
        let response = PageResponse::from_page(page, |n| n.to_string());
        assert_eq!(response.items, vec!["1", "2", "3"]);
        assert_eq!(response.total, 3);
        assert_eq!(response.offset, 0);
        assert_eq!(response.limit, 50);
    }
}
