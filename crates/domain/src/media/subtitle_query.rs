use crate::common::LanguageCode;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubtitleQuery {
    pub imdb_id: Option<String>,
    pub query: Option<String>,
    pub languages: Vec<LanguageCode>,
    pub season: Option<u16>,
    pub episode: Option<u16>,
}
