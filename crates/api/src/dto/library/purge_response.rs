use serde::Serialize;

use domain::library::PurgeReport;

#[derive(Debug, Serialize)]
pub struct PurgeResponse {
    pub movies: u64,
    pub series: u64,
    pub seasons: u64,
    pub episodes: u64,
    pub subtitles: u64,
    pub wiped_users: u64,
}

impl From<PurgeReport> for PurgeResponse {
    fn from(report: PurgeReport) -> Self {
        PurgeResponse {
            movies: report.movies,
            series: report.series,
            seasons: report.seasons,
            episodes: report.episodes,
            subtitles: report.subtitles,
            wiped_users: report.wiped_users,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_count_is_carried_under_its_own_name() {
        let report = PurgeReport {
            movies: 1,
            series: 2,
            seasons: 3,
            episodes: 4,
            subtitles: 5,
            wiped_users: 6,
        };

        assert_eq!(
            serde_json::to_value(PurgeResponse::from(report)).unwrap(),
            serde_json::json!({
                "movies": 1,
                "series": 2,
                "seasons": 3,
                "episodes": 4,
                "subtitles": 5,
                "wiped_users": 6
            })
        );
    }
}
