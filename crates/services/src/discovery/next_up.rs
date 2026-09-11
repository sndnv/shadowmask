use std::collections::{HashMap, HashSet};

use domain::catalog::{Collection, EpisodeContext, EpisodeId, Movie, MovieId, SeriesId, TitleId};
use domain::playback::WatchHistory;

pub fn watched_movie_ids(history: &[WatchHistory]) -> HashSet<MovieId> {
    history
        .iter()
        .filter(|h| h.watched)
        .filter_map(|h| match &h.title {
            TitleId::Movie(id) => Some(id.clone()),
            TitleId::Episode(_) => None,
        })
        .collect()
}

pub fn watched_episode_ids(history: &[WatchHistory]) -> HashSet<EpisodeId> {
    history
        .iter()
        .filter(|h| h.watched)
        .filter_map(|h| match &h.title {
            TitleId::Episode(id) => Some(id.clone()),
            TitleId::Movie(_) => None,
        })
        .collect()
}

pub fn furthest_watched(seen: &[EpisodeContext]) -> Vec<(SeriesId, u16, u16)> {
    let mut best: HashMap<&SeriesId, (u16, u16)> = HashMap::new();
    for entry in seen {
        let slot = (entry.season_number, entry.episode.number);
        best.entry(&entry.series)
            .and_modify(|found| {
                if slot > *found {
                    *found = slot;
                }
            })
            .or_insert(slot);
    }
    let mut out: Vec<(SeriesId, u16, u16)> = best
        .into_iter()
        .map(|(series, (season, number))| (series.clone(), season, number))
        .collect();
    out.sort_by(|a, b| a.0.0.cmp(&b.0.0));
    out
}

pub fn next_movie_candidates(
    collections: &[Collection],
    watched: &HashSet<MovieId>,
) -> Vec<MovieId> {
    collections
        .iter()
        .filter_map(|collection| {
            collection
                .movies
                .iter()
                .rposition(|id| watched.contains(id))
                .and_then(|index| collection.movies.get(index + 1))
                .cloned()
        })
        .collect()
}

pub fn next_movies(
    collections: &[Collection],
    movies: &[Movie],
    watched: &HashSet<MovieId>,
) -> Vec<Movie> {
    let by_id: HashMap<&MovieId, &Movie> = movies.iter().map(|m| (&m.id, m)).collect();
    let mut next = Vec::new();
    for collection in collections {
        let next_movie = collection
            .movies
            .iter()
            .rposition(|id| watched.contains(id))
            .and_then(|index| collection.movies.get(index + 1))
            .and_then(|id| by_id.get(id));
        if let Some(movie) = next_movie {
            next.push((*movie).clone());
        }
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{Episode, SeasonId};
    use jiff::Timestamp;

    fn seen(id: &str, series: &str, season_number: u16, number: u16) -> EpisodeContext {
        EpisodeContext {
            episode: episode(id, "se1", number),
            season: SeasonId("se1".to_owned()),
            season_number,
            season_title: None,
            series: SeriesId(series.to_owned()),
        }
    }

    fn episode(id: &str, season: &str, number: u16) -> Episode {
        Episode {
            id: EpisodeId(id.to_owned()),
            season: SeasonId(season.to_owned()),
            number,
            title: id.to_owned(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn movie(id: &str) -> Movie {
        Movie {
            id: MovieId(id.to_owned()),
            title: id.to_owned(),
            sort_title: id.to_owned(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn resume(marks: &[(SeriesId, u16, u16)]) -> Vec<(String, u16, u16)> {
        marks.iter().map(|(series, season, number)| (series.0.clone(), *season, *number)).collect()
    }

    #[test]
    fn the_furthest_watched_slot_wins_even_when_an_earlier_one_comes_last() {
        let watched =
            [seen("s2e1", "show", 2, 1), seen("s1e1", "show", 1, 1), seen("s1e2", "show", 1, 2)];
        assert_eq!(
            resume(&furthest_watched(&watched)),
            vec![("show".to_owned(), 2, 1)],
            "a rewatch of an early episode must not drag up next backwards"
        );
    }

    #[test]
    fn nothing_watched_leaves_nothing_to_resume() {
        assert!(furthest_watched(&[]).is_empty());
    }

    #[test]
    fn each_series_resumes_independently_and_in_id_order() {
        let watched =
            [seen("b1e1", "beta", 1, 1), seen("a1e1", "alpha", 1, 1), seen("a1e2", "alpha", 1, 2)];
        assert_eq!(
            resume(&furthest_watched(&watched)),
            vec![("alpha".to_owned(), 1, 2), ("beta".to_owned(), 1, 1)]
        );
    }

    #[test]
    fn a_collection_offers_the_movie_after_the_last_watched_one() {
        let collections = [Collection {
            id: domain::catalog::CollectionId("mtx".to_owned()),
            name: "Matrix".to_owned(),
            overview: None,
            movies: vec![
                MovieId("m1".to_owned()),
                MovieId("m2".to_owned()),
                MovieId("m3".to_owned()),
            ],
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }];
        let watched = HashSet::from([MovieId("m1".to_owned())]);
        assert_eq!(
            next_movie_candidates(&collections, &watched),
            vec![MovieId("m2".to_owned())],
            "only the candidate is fetched, never the whole catalog"
        );
        assert!(next_movie_candidates(&collections, &HashSet::new()).is_empty());
    }

    #[test]
    fn next_movie_in_collection_after_last_watched() {
        let collections = [Collection {
            id: domain::catalog::CollectionId("mtx".to_owned()),
            name: "Matrix".to_owned(),
            overview: None,
            movies: vec![
                MovieId("m1".to_owned()),
                MovieId("m2".to_owned()),
                MovieId("m3".to_owned()),
            ],
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }];
        let movies = [movie("m1"), movie("m2"), movie("m3")];
        let watched = HashSet::from([MovieId("m1".to_owned())]);
        let next = next_movies(&collections, &movies, &watched);
        assert_eq!(next.iter().map(|m| m.id.0.clone()).collect::<Vec<_>>(), vec!["m2"]);
    }

    #[test]
    fn no_next_movie_when_unstarted_finished_or_missing() {
        let unstarted = [Collection {
            id: domain::catalog::CollectionId("a".to_owned()),
            name: "A".to_owned(),
            overview: None,
            movies: vec![MovieId("m1".to_owned())],
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }];
        let movies = [movie("m1")];
        assert!(next_movies(&unstarted, &movies, &HashSet::new()).is_empty());

        let finished = HashSet::from([MovieId("m1".to_owned())]);
        assert!(next_movies(&unstarted, &movies, &finished).is_empty());

        let missing_next = [Collection {
            id: domain::catalog::CollectionId("b".to_owned()),
            name: "B".to_owned(),
            overview: None,
            movies: vec![MovieId("m1".to_owned()), MovieId("gone".to_owned())],
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }];
        assert!(next_movies(&missing_next, &movies, &finished).is_empty());
    }
}
