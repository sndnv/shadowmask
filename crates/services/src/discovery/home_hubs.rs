use jiff::Timestamp;

use domain::catalog::{Movie, Series};
use domain::discovery::{Hub, HubItem};

const HUB_LIMIT: usize = 20;

pub fn recently_added(movies: &[Movie], series: &[Series], limit: usize) -> Vec<HubItem> {
    let mut items: Vec<HubItem> = movies
        .iter()
        .cloned()
        .map(HubItem::Movie)
        .chain(series.iter().cloned().map(HubItem::Series))
        .collect();
    items.sort_by(|a, b| added_at(b).cmp(added_at(a)));
    items.truncate(limit);
    items
}

pub fn home_hubs(
    recently_added: Vec<HubItem>,
    on_deck: Vec<HubItem>,
    continue_watching: Vec<HubItem>,
) -> Vec<Hub> {
    [
        ("recently_added", "Recently Added", recently_added),
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

fn added_at(item: &HubItem) -> &Timestamp {
    match item {
        HubItem::Movie(m) => &m.added_at,
        HubItem::Series(s) => &s.added_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, SeriesId};
    use jiff::SignedDuration;

    fn movie(id: &str, seconds: i64) -> Movie {
        Movie {
            id: MovieId(id.to_owned()),
            title: id.to_owned(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH + SignedDuration::from_secs(seconds),
        }
    }

    fn series(id: &str, seconds: i64) -> Series {
        Series {
            id: SeriesId(id.to_owned()),
            title: id.to_owned(),
            year: None,
            overview: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH + SignedDuration::from_secs(seconds),
        }
    }

    fn item_titles(items: &[HubItem]) -> Vec<String> {
        items
            .iter()
            .map(|i| match i {
                HubItem::Movie(m) => m.title.clone(),
                HubItem::Series(s) => s.title.clone(),
            })
            .collect()
    }

    #[test]
    fn recently_added_orders_newest_first_and_caps() {
        let movies = [movie("old", 10), movie("new", 30)];
        let series = [series("mid", 20)];
        let items = recently_added(&movies, &series, 2);
        assert_eq!(item_titles(&items), vec!["new", "mid"]);
    }

    #[test]
    fn home_hubs_labels_orders_and_drops_empty_rows() {
        let recently = vec![HubItem::Movie(movie("m", 10))];
        let on_deck = Vec::new();
        let continue_watching = vec![HubItem::Series(series("s", 10))];
        let hubs = home_hubs(recently, on_deck, continue_watching);
        assert_eq!(hubs.len(), 2);
        assert_eq!(hubs[0].id, "recently_added");
        assert_eq!(hubs[0].title, "Recently Added");
        assert_eq!(hubs[1].id, "continue_watching");
    }

    #[test]
    fn home_hubs_caps_each_row() {
        let big: Vec<HubItem> = (0..HUB_LIMIT + 5)
            .map(|n| HubItem::Movie(movie(&format!("m{n}"), 0)))
            .collect();
        let hubs = home_hubs(big, Vec::new(), Vec::new());
        assert_eq!(hubs.len(), 1);
        assert_eq!(hubs[0].items.len(), HUB_LIMIT);
    }
}
