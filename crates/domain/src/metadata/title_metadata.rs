use crate::metadata::{Artwork, CollectionMeta, ContentRating, CreditInfo, ExternalId, Rating};

#[derive(Debug, Clone, Default)]
pub struct TitleMetadata {
    pub title: String,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub content_rating: Option<ContentRating>,
    pub ratings: Vec<Rating>,
    pub genres: Vec<String>,
    pub cast: Vec<CreditInfo>,
    pub studios: Vec<String>,
    pub artwork: Vec<Artwork>,
    pub external_ids: Vec<ExternalId>,
    pub collection: Option<CollectionMeta>,
}
