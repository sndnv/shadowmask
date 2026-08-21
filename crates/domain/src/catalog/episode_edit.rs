use jiff::Timestamp;

#[derive(Debug, Clone)]
pub struct EpisodeEdit {
    pub title: String,
    pub overview: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub air_date: Option<Timestamp>,
}
