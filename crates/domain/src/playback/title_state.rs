use crate::catalog::TitleId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleState {
    pub title: TitleId,
    pub favorite: bool,
    pub watchlisted: bool,
    pub watched: bool,
    pub completed: bool,
    pub progress_percent: u8,
}
