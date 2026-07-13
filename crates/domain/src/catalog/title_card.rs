use crate::catalog::{Episode, Movie};

#[derive(Debug, Clone)]
pub enum TitleCard {
    Movie(Movie),
    Episode(Episode),
}
