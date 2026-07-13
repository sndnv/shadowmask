use crate::metadata::{Credit, ExternalId, Extra, Genre, Rating, Studio};

#[derive(Debug, Clone, Default)]
pub struct TitleEnrichment {
    pub genres: Vec<Genre>,
    pub credits: Vec<Credit>,
    pub studios: Vec<Studio>,
    pub ratings: Vec<Rating>,
    pub external_ids: Vec<ExternalId>,
    pub extras: Vec<Extra>,
}
