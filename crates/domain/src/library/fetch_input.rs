use crate::media::FetchOptions;

#[derive(Debug, Clone)]
pub struct FetchInput {
    pub source_url: String,
    pub title: String,
    pub year: Option<u16>,
    pub external_id: Option<String>,
    pub season: Option<u16>,
    pub episode: Option<u16>,
    pub episode_title: Option<String>,
    pub options: FetchOptions,
}
