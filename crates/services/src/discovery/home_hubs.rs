use std::collections::HashMap;

use domain::catalog::{EpisodeCard, EpisodeContext, EpisodeId, Movie, MovieId, SeriesId, TitleId};
use domain::discovery::{Hub, HubItem};
use domain::playback::WatchlistItem;

const HUB_LIMIT: usize = 20;

pub const RECENT_ROW: usize = 10;

pub const EPISODE_WINDOW: u32 = 2_000;

pub fn recently_added_movies(movies: &[Movie], limit: usize) -> Vec<HubItem> {
    let mut ordered: Vec<&Movie> = movies.iter().collect();
    ordered.sort_by(|a, b| {
        b.added_at
            .cmp(&a.added_at)
            .then_with(|| a.id.0.cmp(&b.id.0))
    });
    ordered.truncate(limit);
    ordered.into_iter().cloned().map(HubItem::Movie).collect()
}

pub fn recently_added_shows(window: &[EpisodeContext], limit: usize) -> Vec<(SeriesId, u32)> {
    let mut ordered: Vec<&EpisodeContext> = window.iter().collect();
    ordered.sort_by(|a, b| {
        b.episode
            .added_at
            .cmp(&a.episode.added_at)
            .then_with(|| a.episode.id.0.cmp(&b.episode.id.0))
    });

    let mut counted: HashMap<&SeriesId, u32> = HashMap::new();
    let mut order: Vec<&SeriesId> = Vec::new();
    for entry in ordered {
        match counted.get_mut(&entry.series) {
            Some(count) => *count += 1,
            None => {
                if order.len() == limit {
                    break;
                }
                order.push(&entry.series);
                counted.insert(&entry.series, 1);
            }
        }
    }

    order
        .into_iter()
        .map(|id| (id.clone(), counted[id]))
        .collect()
}

pub fn watchlist_row(
    items: &[WatchlistItem],
    movies: &[Movie],
    episodes: &[EpisodeCard],
    limit: usize,
) -> Vec<HubItem> {
    let movie_by_id: HashMap<&MovieId, &Movie> = movies.iter().map(|m| (&m.id, m)).collect();
    let card_by_id: HashMap<&EpisodeId, &EpisodeCard> =
        episodes.iter().map(|c| (&c.episode.id, c)).collect();

    let mut ordered: Vec<&WatchlistItem> = items.iter().collect();
    ordered.sort_by(|a, b| {
        b.added_at
            .cmp(&a.added_at)
            .then_with(|| a.title.id().cmp(b.title.id()))
    });

    let mut row: Vec<HubItem> = Vec::new();
    for item in ordered {
        if row.len() == limit {
            break;
        }
        match &item.title {
            TitleId::Movie(id) => {
                if let Some(movie) = movie_by_id.get(id) {
                    row.push(HubItem::Movie((*movie).clone()));
                }
            }
            TitleId::Episode(id) => {
                if let Some(card) = card_by_id.get(id) {
                    row.push(HubItem::Episode(Box::new((*card).clone())));
                }
            }
        }
    }
    row
}

pub fn home_hubs(
    watchlist: Vec<HubItem>,
    recent_movies: Vec<HubItem>,
    recent_shows: Vec<HubItem>,
    on_deck: Vec<HubItem>,
    continue_watching: Vec<HubItem>,
) -> Vec<Hub> {
    [
        ("watchlist", "On Your Watchlist", watchlist),
        (
            "recently_added_movies",
            "Recently Added Movies",
            recent_movies,
        ),
        (
            "recently_added_shows",
            "Recently Added Series",
            recent_shows,
        ),
        ("on_deck", "On Deck", on_deck),
        ("continue_watching", "Continue Watching", continue_watching),
    ]
    .into_iter()
    .filter(|(_, _, items)| !items.is_empty())
    .map(|(id, title, mut items)| {
        items.truncate(HUB_LIMIT);
        Hub {
            id: id.to_owned(),
            title: title.to_owned(),
            items,
        }
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{Episode, SeasonId, Series};
    use domain::user::UserId;
    use jiff::{SignedDuration, Timestamp};

    fn at(seconds: i64) -> Timestamp {
        Timestamp::UNIX_EPOCH + SignedDuration::from_secs(seconds)
    }

    fn movie(id: &str, seconds: i64) -> Movie {
        Movie {
            id: MovieId(id.to_owned()),
            title: id.to_owned(),
            sort_title: id.to_owned(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: at(seconds),
            updated_at: at(seconds),
            artwork: Vec::new(),
        }
    }

    fn context(id: &str, series: &str, seconds: i64) -> EpisodeContext {
        EpisodeContext {
            episode: episode(id, "se1", seconds),
            season: SeasonId("se1".to_owned()),
            season_number: 1,
            season_title: None,
            series: SeriesId(series.to_owned()),
        }
    }

    fn show(id: &str) -> Series {
        Series {
            id: SeriesId(id.to_owned()),
            title: id.to_owned(),
            sort_title: id.to_owned(),
            year: None,
            overview: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn card(id: &str, series: Option<&str>) -> EpisodeCard {
        EpisodeCard {
            episode: episode(id, "se1", 0),
            series: series.map(|s| SeriesId(s.to_owned())),
            series_title: series.map(str::to_owned),
            series_artwork: Vec::new(),
            season_number: series.map(|_| 1),
            season_title: None,
        }
    }

    fn episode(id: &str, season: &str, seconds: i64) -> Episode {
        Episode {
            id: EpisodeId(id.to_owned()),
            season: SeasonId(season.to_owned()),
            number: 1,
            title: id.to_owned(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: at(seconds),
            updated_at: at(seconds),
            artwork: Vec::new(),
        }
    }

    fn saved(title: TitleId, seconds: i64) -> WatchlistItem {
        WatchlistItem {
            user: UserId("u1".to_owned()),
            title,
            added_at: at(seconds),
        }
    }

    fn saved_movie(id: &str, seconds: i64) -> WatchlistItem {
        saved(TitleId::Movie(MovieId(id.to_owned())), seconds)
    }

    fn saved_episode(id: &str, seconds: i64) -> WatchlistItem {
        saved(TitleId::Episode(EpisodeId(id.to_owned())), seconds)
    }

    fn shown(items: &[HubItem]) -> Vec<(String, Option<u32>)> {
        items
            .iter()
            .map(|i| match i {
                HubItem::Movie(m) => (m.title.clone(), None),
                HubItem::Series {
                    series,
                    episode_count,
                } => (series.title.clone(), *episode_count),
                HubItem::Episode(card) => (card.episode.title.clone(), None),
            })
            .collect()
    }

    #[test]
    fn movies_are_newest_first_and_capped() {
        let movies = [movie("old", 10), movie("new", 30), movie("mid", 20)];
        let items = recently_added_movies(&movies, 2);
        assert_eq!(
            shown(&items),
            vec![("new".to_owned(), None), ("mid".to_owned(), None)]
        );
    }

    fn listed(shows: &[(SeriesId, u32)]) -> Vec<(String, u32)> {
        shows
            .iter()
            .map(|(id, count)| (id.0.clone(), *count))
            .collect()
    }

    #[test]
    fn shows_are_driven_by_episodes_not_by_the_series_row() {
        let window = [
            context("e1", "ongoing", 500),
            context("e2", "ongoing", 400),
            context("e3", "fresh", 300),
        ];
        assert_eq!(
            listed(&recently_added_shows(&window, 10)),
            vec![("ongoing".to_owned(), 2), ("fresh".to_owned(), 1)]
        );
    }

    #[test]
    fn the_scan_finishes_the_run_of_the_last_show_it_admits() {
        let window = [
            context("a1", "a", 900),
            context("b1", "b", 800),
            context("b2", "b", 700),
            context("c1", "c", 600),
            context("b3", "b", 500),
        ];
        assert_eq!(
            listed(&recently_added_shows(&window, 2)),
            vec![("a".to_owned(), 1), ("b".to_owned(), 2)],
            "the count is what the recency scan admitted, not the series total"
        );
    }

    #[test]
    fn home_hubs_labels_orders_and_drops_empty_rows() {
        let hubs = home_hubs(
            Vec::new(),
            vec![HubItem::Movie(movie("m", 10))],
            vec![HubItem::Series {
                series: show("s1"),
                episode_count: Some(3),
            }],
            Vec::new(),
            vec![HubItem::Movie(movie("c", 10))],
        );
        assert_eq!(hubs.len(), 3);
        assert_eq!(hubs[0].id, "recently_added_movies");
        assert_eq!(hubs[0].title, "Recently Added Movies");
        assert_eq!(hubs[1].id, "recently_added_shows");
        assert_eq!(
            shown(&hubs[1].items),
            vec![("s1".to_owned(), Some(3))],
            "the shows rail is the only one whose cards carry a count"
        );
        assert_eq!(hubs[2].id, "continue_watching");
    }

    #[test]
    fn home_hubs_caps_each_row() {
        let big: Vec<HubItem> = (0..HUB_LIMIT + 5)
            .map(|n| HubItem::Movie(movie(&format!("m{n}"), 0)))
            .collect();
        let hubs = home_hubs(Vec::new(), big, Vec::new(), Vec::new(), Vec::new());
        assert_eq!(hubs.len(), 1);
        assert_eq!(hubs[0].items.len(), HUB_LIMIT);
    }

    #[test]
    fn the_watchlist_rail_sits_above_recently_added() {
        let hubs = home_hubs(
            vec![HubItem::Movie(movie("w", 10))],
            vec![HubItem::Movie(movie("m", 10))],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(hubs[0].id, "watchlist");
        assert_eq!(hubs[0].title, "On Your Watchlist");
        assert_eq!(hubs[1].id, "recently_added_movies");
    }

    #[test]
    fn the_watchlist_row_is_newest_saved_first_and_capped() {
        let movies = [movie("old", 0), movie("new", 0), movie("mid", 0)];
        let items = [
            saved_movie("old", 10),
            saved_movie("new", 30),
            saved_movie("mid", 20),
        ];
        let row = watchlist_row(&items, &movies, &[], 2);
        assert_eq!(
            shown(&row),
            vec![("new".to_owned(), None), ("mid".to_owned(), None)]
        );
    }

    #[test]
    fn every_saved_episode_keeps_its_own_card() {
        let cards = [
            card("e1", Some("s1")),
            card("e2", Some("s1")),
            card("e3", Some("s1")),
        ];
        let items = [
            saved_episode("e1", 30),
            saved_episode("e2", 20),
            saved_episode("e3", 10),
        ];

        let row = watchlist_row(&items, &[], &cards, 10);

        assert_eq!(
            shown(&row),
            vec![
                ("e1".to_owned(), None),
                ("e2".to_owned(), None),
                ("e3".to_owned(), None)
            ],
            "one card per saved row, so dismissing one cannot take the others"
        );
    }

    #[test]
    fn a_saved_episode_carries_the_context_its_card_needs() {
        let row = watchlist_row(
            &[saved_episode("e1", 10)],
            &[],
            &[card("e1", Some("s1"))],
            10,
        );

        match &row[..] {
            [HubItem::Episode(card)] => {
                assert_eq!(card.series, Some(SeriesId("s1".to_owned())));
                assert_eq!(card.series_title.as_deref(), Some("s1"));
                assert_eq!(card.season_number, Some(1));
            }
            other => panic!("unexpected watchlist row: {other:?}"),
        }
    }

    #[test]
    fn the_watchlist_row_skips_entries_whose_title_is_gone() {
        let movies = [movie("kept", 0)];
        let cards = [card("stray", None), card("e1", Some("known"))];
        let items = [
            saved_movie("kept", 60),
            saved_movie("deleted", 50),
            saved_episode("gone", 40),
            saved_episode("stray", 30),
            saved_episode("e1", 20),
        ];

        let row = watchlist_row(&items, &movies, &cards, 10);

        assert_eq!(
            shown(&row),
            vec![
                ("kept".to_owned(), None),
                ("stray".to_owned(), None),
                ("e1".to_owned(), None)
            ],
            "a title the viewer cannot see drops out, the rest keep their place"
        );
        match &row[1] {
            HubItem::Episode(card) => assert!(
                card.series_title.is_none(),
                "the card simply carries no series context"
            ),
            other => panic!("unexpected row entry: {other:?}"),
        }
    }
}
