use crate::common::{Page, PageRequest, paginate};
use crate::discovery::{SearchKind, SearchResult};
use crate::text::normalize_title;

pub fn search(
    candidates: &[SearchResult],
    query: &str,
    types: &[SearchKind],
    page: PageRequest,
) -> Page<SearchResult> {
    let needle = normalize_title(query);
    let mut ranked: Vec<(u8, String, &str, &SearchResult)> = Vec::new();
    if !needle.is_empty() {
        for candidate in candidates {
            if !types.is_empty() && !types.contains(&candidate.kind()) {
                continue;
            }
            let haystack = normalize_title(searchable(candidate));
            if let Some(tier) = rank(&needle, &haystack) {
                ranked.push((tier, haystack, id(candidate), candidate));
            }
        }
        ranked.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.cmp(b.2))
        });
    }
    let matches: Vec<SearchResult> = ranked.into_iter().map(|(_, _, _, c)| c.clone()).collect();
    paginate(&matches, page)
}

fn rank(needle: &str, haystack: &str) -> Option<u8> {
    if haystack == needle {
        Some(0)
    } else if haystack.starts_with(needle) {
        Some(1)
    } else if haystack.split(' ').any(|word| word == needle) {
        Some(2)
    } else {
        None
    }
}

fn searchable(result: &SearchResult) -> &str {
    match result {
        SearchResult::Movie(m) => &m.title,
        SearchResult::Series(s) => &s.title,
        SearchResult::Episode(e) => &e.title,
        SearchResult::Person(p) => &p.name,
    }
}

fn id(result: &SearchResult) -> &str {
    match result {
        SearchResult::Movie(m) => &m.id.0,
        SearchResult::Series(s) => &s.id.0,
        SearchResult::Episode(e) => &e.id.0,
        SearchResult::Person(p) => &p.id.0,
    }
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;

    use super::*;
    use crate::catalog::{Episode, EpisodeId, Movie, MovieId, SeasonId, Series, SeriesId};
    use crate::metadata::{Person, PersonId};

    fn movie(title: &str) -> SearchResult {
        SearchResult::Movie(Movie {
            id: MovieId(title.to_owned()),
            title: title.to_owned(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        })
    }

    fn series(title: &str) -> SearchResult {
        SearchResult::Series(Series {
            id: SeriesId(title.to_owned()),
            title: title.to_owned(),
            year: None,
            overview: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        })
    }

    fn person(name: &str) -> SearchResult {
        SearchResult::Person(Person {
            id: PersonId(name.to_owned()),
            name: name.to_owned(),
            ..Person::default()
        })
    }

    fn episode(title: &str) -> SearchResult {
        SearchResult::Episode(Episode {
            id: EpisodeId(title.to_owned()),
            season: SeasonId("s".to_owned()),
            number: 1,
            title: title.to_owned(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        })
    }

    fn titles(page: &Page<SearchResult>) -> Vec<String> {
        page.items
            .iter()
            .map(|r| searchable(r).to_owned())
            .collect()
    }

    fn all() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 100,
        }
    }

    fn search(candidates: &[SearchResult], query: &str, page: PageRequest) -> Page<SearchResult> {
        super::search(candidates, query, &[], page)
    }

    #[test]
    fn filters_by_kind() {
        let candidates = [
            movie("Matrix"),
            series("The Matrix"),
            episode("Matrix Origins"),
            person("Matrix Guy"),
        ];
        let movies = super::search(&candidates, "matrix", &[SearchKind::Movie], all());
        assert_eq!(titles(&movies), vec!["Matrix"]);

        let people = super::search(&candidates, "matrix", &[SearchKind::Person], all());
        assert_eq!(titles(&people), vec!["Matrix Guy"]);

        let two = super::search(
            &candidates,
            "matrix",
            &[SearchKind::Series, SearchKind::Episode],
            all(),
        );
        assert_eq!(two.total, 2);

        let unfiltered = super::search(&candidates, "matrix", &[], all());
        assert_eq!(unfiltered.total, 4);
    }

    #[test]
    fn ranks_exact_then_prefix_then_token() {
        let candidates = [
            series("The Matrix"),
            movie("Matrix"),
            movie("Rematrix"),
            movie("Matrix Reloaded"),
            movie("Inception"),
        ];
        let hits = search(&candidates, "matrix", all());
        assert_eq!(hits.total, 3);
        assert_eq!(
            titles(&hits),
            vec!["Matrix", "Matrix Reloaded", "The Matrix"]
        );
    }

    #[test]
    fn excludes_substring_only_matches() {
        let candidates = [movie("Rematrix"), movie("Matrix")];
        let hits = search(&candidates, "matrix", all());
        assert_eq!(hits.total, 1);
        assert_eq!(titles(&hits), vec!["Matrix"]);
    }

    #[test]
    fn matches_people_and_episodes_case_and_punctuation_insensitive() {
        let candidates = [person("Keanu Reeves"), episode("The Matrix.")];
        let hits = search(&candidates, "  the MATRIX!! ", all());
        assert_eq!(titles(&hits), vec!["The Matrix."]);

        let people = search(&candidates, "keanu", all());
        assert_eq!(titles(&people), vec!["Keanu Reeves"]);
    }

    #[test]
    fn excludes_non_matches_and_empty_query() {
        let candidates = [movie("Matrix"), movie("Inception")];
        assert!(search(&candidates, "nothing", all()).items.is_empty());
        assert_eq!(search(&candidates, "nothing", all()).total, 0);

        let blank = search(&candidates, "   ", all());
        assert!(blank.items.is_empty());
        assert_eq!(blank.total, 0);
    }

    #[test]
    fn paginates_ranked_matches() {
        let candidates = [
            series("The Matrix"),
            movie("Matrix"),
            movie("Rematrix"),
            movie("Matrix Reloaded"),
        ];
        let page = search(
            &candidates,
            "matrix",
            PageRequest {
                offset: 1,
                limit: 2,
            },
        );
        assert_eq!(page.total, 3);
        assert_eq!(titles(&page), vec!["Matrix Reloaded", "The Matrix"]);
    }
}

#[cfg(test)]
mod prop_tests {
    use jiff::Timestamp;
    use proptest::prelude::*;

    use super::*;
    use crate::catalog::{Movie, MovieId};

    fn movie(title: &str) -> SearchResult {
        SearchResult::Movie(Movie {
            id: MovieId(title.to_owned()),
            title: title.to_owned(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        })
    }

    fn all_pages() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 1000,
        }
    }

    fn search(candidates: &[SearchResult], query: &str, page: PageRequest) -> Page<SearchResult> {
        super::search(candidates, query, &[], page)
    }

    proptest! {
        #[test]
        fn whitespace_query_matches_nothing(
            titles in prop::collection::vec("[a-z ]{0,10}", 0..16),
            blank in r"[ \t]{0,5}",
        ) {
            let candidates: Vec<SearchResult> = titles.iter().map(|t| movie(t)).collect();
            let hits = search(&candidates, &blank, all_pages());
            prop_assert!(hits.items.is_empty());
            prop_assert_eq!(hits.total, 0);
        }

        #[test]
        fn every_hit_contains_query(
            titles in prop::collection::vec("[a-z ]{0,10}", 0..16),
            query in "[a-z ]{1,6}",
        ) {
            let candidates: Vec<SearchResult> = titles.iter().map(|t| movie(t)).collect();
            let needle = normalize_title(&query);
            let hits = search(&candidates, &query, all_pages());
            prop_assert!(hits.total as usize <= candidates.len());
            for item in &hits.items {
                let hay = normalize_title(searchable(item));
                prop_assert!(needle.is_empty() || hay.contains(&needle));
            }
        }

        #[test]
        fn total_is_case_insensitive(
            titles in prop::collection::vec("[a-z ]{0,10}", 0..16),
            query in "[a-z ]{1,6}",
        ) {
            let candidates: Vec<SearchResult> = titles.iter().map(|t| movie(t)).collect();
            let lower = search(&candidates, &query, all_pages()).total;
            let upper = search(&candidates, &query.to_uppercase(), all_pages()).total;
            prop_assert_eq!(lower, upper);
        }

        #[test]
        fn respects_page_limit(
            titles in prop::collection::vec("[a-z ]{0,10}", 0..16),
            query in "[a-z ]{1,6}",
            limit in 0u32..8,
        ) {
            let candidates: Vec<SearchResult> = titles.iter().map(|t| movie(t)).collect();
            let hits = search(&candidates, &query, PageRequest { offset: 0, limit });
            prop_assert!(hits.items.len() as u64 <= u64::from(limit));
        }
    }
}
