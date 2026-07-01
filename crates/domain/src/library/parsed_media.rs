use crate::common::Quality;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMedia {
    pub title: String,
    pub year: Option<u16>,
    pub season: Option<u16>,
    pub episode: Option<u16>,
    pub quality: Option<Quality>,
}

impl ParsedMedia {
    pub fn is_episodic(&self) -> bool {
        self.season.is_some() && self.episode.is_some()
    }
}
