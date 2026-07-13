use std::collections::{HashMap, HashSet};

use domain::catalog::{Collection, Episode, EpisodeId, Movie, MovieId, Season, SeasonId, SeriesId};

pub fn next_episodes(
    seasons: &[Season],
    episodes: &[Episode],
    watched: &HashSet<EpisodeId>,
) -> Vec<Episode> {
    let season_info: HashMap<&SeasonId, (&SeriesId, u16)> = seasons
        .iter()
        .map(|s| (&s.id, (&s.series, s.number)))
        .collect();

    let mut by_series: HashMap<&SeriesId, Vec<(u16, u16, &Episode)>> = HashMap::new();
    for episode in episodes {
        if let Some((series, season_number)) = season_info.get(&episode.season) {
            by_series
                .entry(*series)
                .or_default()
                .push((*season_number, episode.number, episode));
        }
    }

    let mut next: Vec<(&SeriesId, Episode)> = Vec::new();
    for (series, mut ordered) in by_series {
        ordered.sort_by_key(|&(season, number, _)| (season, number));
        let next_episode = ordered
            .iter()
            .rposition(|&(_, _, episode)| watched.contains(&episode.id))
            .and_then(|index| ordered.get(index + 1))
            .map(|&(_, _, episode)| episode);
        if let Some(episode) = next_episode {
            next.push((series, episode.clone()));
        }
    }
    next.sort_by(|a, b| a.0.0.cmp(&b.0.0));
    next.into_iter().map(|(_, episode)| episode).collect()
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
    use jiff::Timestamp;

    fn season(id: &str, series: &str, number: u16) -> Season {
        Season {
            id: SeasonId(id.to_owned()),
            series: SeriesId(series.to_owned()),
            number,
            title: None,
            overview: None,
            artwork: Vec::new(),
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
            added_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn movie(id: &str) -> Movie {
        Movie {
            id: MovieId(id.to_owned()),
            title: id.to_owned(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn watched_episodes(ids: &[&str]) -> HashSet<EpisodeId> {
        ids.iter().map(|id| EpisodeId((*id).to_owned())).collect()
    }

    fn ids(episodes: &[Episode]) -> Vec<String> {
        episodes.iter().map(|e| e.id.0.clone()).collect()
    }

    #[test]
    fn returns_next_after_last_watched_across_seasons() {
        let seasons = [season("s1", "show", 1), season("s2", "show", 2)];
        let episodes = [
            episode("s1e1", "s1", 1),
            episode("s1e2", "s1", 2),
            episode("s2e1", "s2", 1),
        ];
        let next = next_episodes(&seasons, &episodes, &watched_episodes(&["s1e1", "s1e2"]));
        assert_eq!(ids(&next), vec!["s2e1"]);
    }

    #[test]
    fn crosses_season_boundary_when_season_finished() {
        let seasons = [season("s1", "show", 1), season("s2", "show", 2)];
        let episodes = [episode("s1e1", "s1", 1), episode("s2e1", "s2", 1)];
        let next = next_episodes(&seasons, &episodes, &watched_episodes(&["s1e1"]));
        assert_eq!(ids(&next), vec!["s2e1"]);
    }

    #[test]
    fn nothing_watched_or_all_watched_yields_no_next() {
        let seasons = [season("s1", "show", 1)];
        let episodes = [episode("s1e1", "s1", 1), episode("s1e2", "s1", 2)];
        assert!(next_episodes(&seasons, &episodes, &HashSet::new()).is_empty());
        assert!(
            next_episodes(&seasons, &episodes, &watched_episodes(&["s1e1", "s1e2"])).is_empty()
        );
    }

    #[test]
    fn independent_series_are_sorted_and_orphan_episodes_skipped() {
        let seasons = [season("a1", "alpha", 1), season("b1", "beta", 1)];
        let episodes = [
            episode("a1e1", "a1", 1),
            episode("a1e2", "a1", 2),
            episode("b1e1", "b1", 1),
            episode("b1e2", "b1", 2),
            episode("orphan", "missing", 1),
        ];
        let next = next_episodes(&seasons, &episodes, &watched_episodes(&["a1e1", "b1e1"]));
        assert_eq!(ids(&next), vec!["a1e2", "b1e2"]);
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
            artwork: Vec::new(),
        }];
        let movies = [movie("m1"), movie("m2"), movie("m3")];
        let watched = HashSet::from([MovieId("m1".to_owned())]);
        let next = next_movies(&collections, &movies, &watched);
        assert_eq!(
            next.iter().map(|m| m.id.0.clone()).collect::<Vec<_>>(),
            vec!["m2"]
        );
    }

    #[test]
    fn no_next_movie_when_unstarted_finished_or_missing() {
        let unstarted = [Collection {
            id: domain::catalog::CollectionId("a".to_owned()),
            name: "A".to_owned(),
            overview: None,
            movies: vec![MovieId("m1".to_owned())],
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
            artwork: Vec::new(),
        }];
        assert!(next_movies(&missing_next, &movies, &finished).is_empty());
    }
}
