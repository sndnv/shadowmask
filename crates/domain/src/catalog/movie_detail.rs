use crate::catalog::Movie;
use crate::metadata::{CreditedPerson, ExternalId, Extra, Genre, Rating, Studio};

#[derive(Debug, Clone)]
pub struct MovieDetail {
    pub movie: Movie,
    pub genres: Vec<Genre>,
    pub credits: Vec<CreditedPerson>,
    pub studios: Vec<Studio>,
    pub ratings: Vec<Rating>,
    pub external_ids: Vec<ExternalId>,
    pub extras: Vec<Extra>,
}
