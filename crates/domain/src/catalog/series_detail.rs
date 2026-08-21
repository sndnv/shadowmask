use crate::catalog::Series;
use crate::metadata::{CreditedPerson, ExternalId, Extra, Genre, Rating, Studio};

#[derive(Debug, Clone)]
pub struct SeriesDetail {
    pub series: Series,
    pub genres: Vec<Genre>,
    pub credits: Vec<CreditedPerson>,
    pub studios: Vec<Studio>,
    pub ratings: Vec<Rating>,
    pub external_ids: Vec<ExternalId>,
    pub extras: Vec<Extra>,
    pub episodes_total: u32,
    pub episodes_with_available_version: u32,
}
