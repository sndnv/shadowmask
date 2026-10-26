use serde::Serialize;

use domain::library::CleanReport;

#[derive(Debug, Serialize)]
pub struct CleanResponse {
    pub versions: u64,
    pub movies: u64,
    pub series: u64,
    pub seasons: u64,
    pub episodes: u64,
    pub wiped_users: u64,
}

impl From<CleanReport> for CleanResponse {
    fn from(report: CleanReport) -> Self {
        CleanResponse {
            versions: report.versions,
            movies: report.movies,
            series: report.series,
            seasons: report.seasons,
            episodes: report.episodes,
            wiped_users: report.wiped_users,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_count_is_carried_under_its_own_name() {
        let report = CleanReport {
            versions: 1,
            movies: 2,
            series: 3,
            seasons: 4,
            episodes: 5,
            wiped_users: 6,
        };

        assert_eq!(
            serde_json::to_value(CleanResponse::from(report)).unwrap(),
            serde_json::json!({
                "versions": 1,
                "movies": 2,
                "series": 3,
                "seasons": 4,
                "episodes": 5,
                "wiped_users": 6
            })
        );
    }
}
