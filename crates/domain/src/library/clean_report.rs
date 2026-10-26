#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CleanReport {
    pub versions: u64,
    pub movies: u64,
    pub series: u64,
    pub seasons: u64,
    pub episodes: u64,
    pub wiped_users: u64,
}
