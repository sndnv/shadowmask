use std::collections::HashMap;

use jiff::Timestamp;

use crate::catalog::{
    Episode, Movie, Season, SeasonId, Series, SeriesId, TitleId, TitleOutline, VersionCounts,
};
use crate::metadata::ContentRating;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TitleRowKind {
    Movie,
    Series,
    Season,
    Episode,
}

impl TitleRowKind {
    pub fn parse(value: &str) -> Option<TitleRowKind> {
        match value {
            "movie" => Some(TitleRowKind::Movie),
            "series" => Some(TitleRowKind::Series),
            "season" => Some(TitleRowKind::Season),
            "episode" => Some(TitleRowKind::Episode),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            TitleRowKind::Movie => "movie",
            TitleRowKind::Series => "series",
            TitleRowKind::Season => "season",
            TitleRowKind::Episode => "episode",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleRow {
    pub kind: TitleRowKind,
    pub id: String,
    pub series: Option<SeriesId>,
    pub season: Option<SeasonId>,
    pub title: Option<String>,
    pub series_title: Option<String>,
    pub season_number: Option<u16>,
    pub episode_number: Option<u16>,
    pub year: Option<u16>,
    pub files: u32,
    pub episodes: Option<u32>,
    pub seasons: Option<u32>,
    pub available: bool,
    pub readable: bool,
    pub added_at: Timestamp,
}

enum Top<'a> {
    Movie(&'a Movie),
    Series(&'a Series),
}

#[derive(Default)]
struct Tally {
    files: u32,
    episodes: u32,
    available: bool,
}

impl Tally {
    fn count(&mut self, counts: VersionCounts) {
        self.episodes += 1;
        self.files += u32::from(counts.granted > 0);
        self.available |= counts.present > 0;
    }

    fn add(&mut self, other: &Tally) {
        self.files += other.files;
        self.episodes += other.episodes;
        self.available |= other.available;
    }
}

struct Rows<'a> {
    outline: &'a TitleOutline,
    permits: &'a dyn Fn(Option<&ContentRating>) -> bool,
    seasons: HashMap<&'a SeriesId, Vec<&'a Season>>,
    episodes: HashMap<&'a SeasonId, Vec<&'a Episode>>,
}

impl<'a> Rows<'a> {
    fn new(outline: &'a TitleOutline, permits: &'a dyn Fn(Option<&ContentRating>) -> bool) -> Self {
        let mut seasons: HashMap<&SeriesId, Vec<&Season>> = HashMap::new();
        for season in &outline.seasons {
            seasons.entry(&season.series).or_default().push(season);
        }
        for group in seasons.values_mut() {
            group.sort_by(|a, b| a.number.cmp(&b.number).then_with(|| a.id.0.cmp(&b.id.0)));
        }
        let mut episodes: HashMap<&SeasonId, Vec<&Episode>> = HashMap::new();
        for episode in &outline.episodes {
            episodes.entry(&episode.season).or_default().push(episode);
        }
        for group in episodes.values_mut() {
            group.sort_by(|a, b| a.number.cmp(&b.number).then_with(|| a.id.0.cmp(&b.id.0)));
        }
        Self { outline, permits, seasons, episodes }
    }

    fn counts(&self, title: TitleId) -> VersionCounts {
        self.outline.versions.get(&title).copied().unwrap_or_default()
    }

    fn movie(&self, movie: &Movie, rows: &mut Vec<TitleRow>) {
        let counts = self.counts(TitleId::Movie(movie.id.clone()));
        if !counts.listed() {
            return;
        }
        rows.push(TitleRow {
            kind: TitleRowKind::Movie,
            id: movie.id.0.clone(),
            series: None,
            season: None,
            title: Some(movie.title.clone()),
            series_title: None,
            season_number: None,
            episode_number: None,
            year: movie.year,
            files: counts.granted,
            episodes: None,
            seasons: None,
            available: counts.present > 0,
            readable: counts.granted > 0 && (self.permits)(movie.content_rating.as_ref()),
            added_at: movie.added_at,
        });
    }

    fn season(
        &self,
        series: &Series,
        season: &Season,
        allowed: bool,
        tally: &mut Tally,
    ) -> Vec<TitleRow> {
        let episodes = self.episodes.get(&season.id).map_or(&[][..], Vec::as_slice);
        let mut own = Tally::default();
        let mut listed = Vec::new();
        for episode in episodes {
            let counts = self.counts(TitleId::Episode(episode.id.clone()));
            own.count(counts);
            if counts.listed() {
                listed.push(TitleRow {
                    kind: TitleRowKind::Episode,
                    id: episode.id.0.clone(),
                    series: Some(series.id.clone()),
                    season: Some(season.id.clone()),
                    title: Some(episode.title.clone()),
                    series_title: Some(series.title.clone()),
                    season_number: Some(season.number),
                    episode_number: Some(episode.number),
                    year: None,
                    files: counts.granted,
                    episodes: None,
                    seasons: None,
                    available: counts.present > 0,
                    readable: counts.granted > 0 && allowed,
                    added_at: episode.added_at,
                });
            }
        }
        tally.add(&own);
        if !episodes.is_empty() && listed.is_empty() {
            return Vec::new();
        }
        let mut rows = vec![TitleRow {
            kind: TitleRowKind::Season,
            id: season.id.0.clone(),
            series: Some(series.id.clone()),
            season: None,
            title: season.title.clone(),
            series_title: Some(series.title.clone()),
            season_number: Some(season.number),
            episode_number: None,
            year: None,
            files: own.files,
            episodes: Some(own.episodes),
            seasons: None,
            available: own.available,
            readable: own.files > 0 && allowed,
            added_at: season.added_at,
        }];
        rows.append(&mut listed);
        rows
    }

    fn series(&self, series: &Series, rows: &mut Vec<TitleRow>) {
        let seasons = self.seasons.get(&series.id).map_or(&[][..], Vec::as_slice);
        let allowed = (self.permits)(series.content_rating.as_ref());
        let mut tally = Tally::default();
        let mut listed = Vec::new();
        for season in seasons {
            listed.extend(self.season(series, season, allowed, &mut tally));
        }
        if !seasons.is_empty() && listed.is_empty() {
            return;
        }
        rows.push(TitleRow {
            kind: TitleRowKind::Series,
            id: series.id.0.clone(),
            series: None,
            season: None,
            title: Some(series.title.clone()),
            series_title: None,
            season_number: None,
            episode_number: None,
            year: series.year,
            files: tally.files,
            episodes: Some(tally.episodes),
            seasons: Some(seasons.len() as u32),
            available: tally.available,
            readable: tally.files > 0 && allowed,
            added_at: series.added_at,
        });
        rows.append(&mut listed);
    }
}

pub fn title_rows(
    outline: &TitleOutline,
    permits: &dyn Fn(Option<&ContentRating>) -> bool,
) -> Vec<TitleRow> {
    let builder = Rows::new(outline, permits);
    let mut top: Vec<(&str, &str, Top)> =
        outline
            .movies
            .iter()
            .map(|movie| (movie.sort_title.as_str(), movie.id.0.as_str(), Top::Movie(movie)))
            .chain(outline.series.iter().map(|series| {
                (series.sort_title.as_str(), series.id.0.as_str(), Top::Series(series))
            }))
            .collect();
    top.sort_by(|a, b| a.0.cmp(b.0).then_with(|| a.1.cmp(b.1)));
    let mut rows = Vec::new();
    for (_, _, title) in top {
        match title {
            Top::Movie(movie) => builder.movie(movie, &mut rows),
            Top::Series(series) => builder.series(series, &mut rows),
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{EpisodeId, MovieId};

    fn at(second: i64) -> Timestamp {
        Timestamp::from_second(second).unwrap()
    }

    fn everything(_: Option<&ContentRating>) -> bool {
        true
    }

    fn movie(id: &str, sort: &str) -> Movie {
        Movie {
            id: MovieId(id.into()),
            title: format!("Title {id}"),
            sort_title: sort.into(),
            year: Some(2001),
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: at(1),
            updated_at: at(1),
            artwork: Vec::new(),
        }
    }

    fn series(id: &str, sort: &str) -> Series {
        Series {
            id: SeriesId(id.into()),
            title: format!("Title {id}"),
            sort_title: sort.into(),
            year: Some(2010),
            overview: None,
            content_rating: None,
            manually_edited: false,
            added_at: at(2),
            updated_at: at(2),
            artwork: Vec::new(),
        }
    }

    fn season(id: &str, series: &str, number: u16) -> Season {
        Season {
            id: SeasonId(id.into()),
            series: SeriesId(series.into()),
            number,
            title: None,
            overview: None,
            added_at: at(3),
            updated_at: at(3),
            artwork: Vec::new(),
        }
    }

    fn episode(id: &str, season: &str, number: u16) -> Episode {
        Episode {
            id: EpisodeId(id.into()),
            season: SeasonId(season.into()),
            number,
            title: format!("Title {id}"),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: at(4),
            updated_at: at(4),
            artwork: Vec::new(),
        }
    }

    fn counts(total: u32, granted: u32, present: u32) -> VersionCounts {
        VersionCounts { total, granted, present }
    }

    fn outline() -> TitleOutline {
        TitleOutline {
            movies: vec![
                movie("m-held", "bravo"),
                movie("m-gone", "alpha"),
                movie("m-elsewhere", "delta"),
                movie("m-empty", "echo"),
            ],
            series: vec![
                series("s-elsewhere", "golf"),
                series("s-bare", "foxtrot"),
                series("s-show", "charlie"),
            ],
            seasons: vec![
                season("show-2", "s-show", 2),
                season("show-1", "s-show", 1),
                season("show-0", "s-show", 0),
                season("away-1", "s-elsewhere", 1),
            ],
            episodes: vec![
                episode("e2", "show-1", 2),
                episode("e1", "show-1", 1),
                episode("e0", "show-0", 1),
                episode("away", "away-1", 1),
            ],
            versions: HashMap::from([
                (TitleId::Movie(MovieId("m-held".into())), counts(2, 1, 1)),
                (TitleId::Movie(MovieId("m-gone".into())), counts(1, 1, 0)),
                (TitleId::Movie(MovieId("m-elsewhere".into())), counts(1, 0, 0)),
                (TitleId::Episode(EpisodeId("e1".into())), counts(3, 2, 1)),
                (TitleId::Episode(EpisodeId("e0".into())), counts(1, 0, 0)),
                (TitleId::Episode(EpisodeId("away".into())), counts(1, 0, 0)),
            ]),
        }
    }

    fn listed(rows: &[TitleRow]) -> Vec<&str> {
        rows.iter().map(|row| row.id.as_str()).collect()
    }

    fn row<'a>(rows: &'a [TitleRow], id: &str) -> &'a TitleRow {
        rows.iter().find(|row| row.id == id).unwrap()
    }

    #[test]
    fn a_series_leads_its_seasons_and_episodes_between_movies_by_sort_title() {
        let rows = title_rows(&outline(), &everything);
        assert_eq!(
            listed(&rows),
            ["m-gone", "m-held", "s-show", "show-1", "e1", "e2", "show-2", "m-empty", "s-bare"]
        );
    }

    #[test]
    fn titles_only_in_libraries_out_of_reach_are_left_out_with_their_empty_parents() {
        let rows = title_rows(&outline(), &everything);
        for hidden in ["m-elsewhere", "show-0", "e0", "s-elsewhere", "away-1", "away"] {
            assert!(rows.iter().all(|row| row.id != hidden), "{hidden}");
        }
    }

    #[test]
    fn a_leaf_counts_its_files_in_reach_and_is_available_when_one_is_present() {
        let rows = title_rows(&outline(), &everything);
        let held = row(&rows, "m-held");
        assert_eq!((held.kind, held.files, held.available), (TitleRowKind::Movie, 1, true));
        assert_eq!((held.title.as_deref(), held.year), (Some("Title m-held"), Some(2001)));
        assert_eq!(held.added_at, at(1));
        let gone = row(&rows, "m-gone");
        assert_eq!((gone.files, gone.available), (1, false));
        let empty = row(&rows, "m-empty");
        assert_eq!((empty.files, empty.available, empty.episodes), (0, false, None));
        let e2 = row(&rows, "e2");
        assert_eq!((e2.kind, e2.files, e2.available), (TitleRowKind::Episode, 0, false));
        let e1 = row(&rows, "e1");
        assert_eq!(e1.series, Some(SeriesId("s-show".into())));
        assert_eq!(e1.season, Some(SeasonId("show-1".into())));
        assert_eq!(e1.series_title.as_deref(), Some("Title s-show"));
        assert_eq!((e1.season_number, e1.episode_number, e1.year), (Some(1), Some(1), None));
        assert_eq!((e1.files, e1.available, e1.added_at), (2, true, at(4)));
    }

    #[test]
    fn a_season_and_its_series_count_episodes_with_a_file_out_of_all_of_them() {
        let rows = title_rows(&outline(), &everything);
        let first = row(&rows, "show-1");
        assert_eq!(first.kind, TitleRowKind::Season);
        assert_eq!((first.files, first.episodes, first.available), (1, Some(2), true));
        assert_eq!(
            (first.series.as_ref(), first.season.as_ref()),
            (Some(&SeriesId("s-show".into())), None)
        );
        assert_eq!((first.title.as_deref(), first.season_number), (None, Some(1)));
        assert_eq!(first.added_at, at(3));
        let empty = row(&rows, "show-2");
        assert_eq!((empty.files, empty.episodes, empty.available), (0, Some(0), false));
        let show = row(&rows, "s-show");
        assert_eq!(show.kind, TitleRowKind::Series);
        assert_eq!((show.files, show.episodes, show.seasons), (1, Some(3), Some(3)));
        assert_eq!((show.available, show.year, show.added_at), (true, Some(2010), at(2)));
        let bare = row(&rows, "s-bare");
        assert_eq!(
            (bare.files, bare.episodes, bare.seasons, bare.available),
            (0, Some(0), Some(0), false)
        );
    }

    #[test]
    fn equal_sort_titles_fall_back_to_the_id() {
        let outline = TitleOutline {
            movies: vec![movie("m-b", "same"), movie("m-a", "same")],
            series: vec![series("m-aa", "same")],
            ..TitleOutline::default()
        };
        assert_eq!(listed(&title_rows(&outline, &everything)), ["m-a", "m-aa", "m-b"]);
    }

    #[test]
    fn seasons_and_episodes_with_the_same_number_fall_back_to_the_id() {
        let outline = TitleOutline {
            series: vec![series("s", "show")],
            seasons: vec![season("se-b", "s", 1), season("se-a", "s", 1)],
            episodes: vec![episode("e-b", "se-a", 1), episode("e-a", "se-a", 1)],
            ..TitleOutline::default()
        };
        assert_eq!(listed(&title_rows(&outline, &everything)), ["s", "se-a", "e-a", "e-b", "se-b"]);
    }

    fn rated(code: &str) -> Option<ContentRating> {
        Some(ContentRating { system: "MPAA".into(), code: code.into() })
    }

    fn below_r(rating: Option<&ContentRating>) -> bool {
        rating.is_none_or(|rating| rating.code != "R")
    }

    #[test]
    fn a_row_is_readable_with_a_file_in_reach_and_a_rating_the_admin_may_see() {
        let mut capped = outline();
        capped.movies[0].content_rating = rated("R");
        let rows = title_rows(&capped, &below_r);
        let readable = |id: &str| row(&rows, id).readable;
        assert!(!readable("m-held"), "its title page would refuse the admin's rating cap");
        assert!(readable("m-gone"), "a file that went missing still opens its page");
        assert!(!readable("m-empty"), "a title with no version has no page");
        assert!(readable("s-show") && readable("show-1") && readable("e1"));
        assert!(!readable("e2") && !readable("show-2") && !readable("s-bare"));

        capped.series[2].content_rating = rated("R");
        let rows = title_rows(&capped, &below_r);
        for id in ["s-show", "show-1", "e1"] {
            assert!(!row(&rows, id).readable, "{id} carries its series' rating");
        }
    }

    #[test]
    fn kinds_parse_from_their_names() {
        for kind in
            [TitleRowKind::Movie, TitleRowKind::Series, TitleRowKind::Season, TitleRowKind::Episode]
        {
            assert_eq!(TitleRowKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(TitleRowKind::parse("all"), None);
    }
}
