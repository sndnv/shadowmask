use crate::common::{Page, PageRequest, paginate};
use crate::discovery::{SearchKind, SearchResult};
use crate::text::{is_typo_of, joined_terms, search_text};

const TYPO_TIER: u8 = 3;

struct Searched<'a> {
    candidate: &'a SearchResult,
    id: &'a str,
    haystack: String,
    terms: Vec<String>,
}

pub fn search(
    candidates: &[SearchResult],
    query: &str,
    types: &[SearchKind],
    page: PageRequest,
) -> Page<SearchResult> {
    let needle = search_text(query);
    let words: Vec<&str> = needle.split_whitespace().collect();
    let searched: Vec<Searched> = candidates
        .iter()
        .filter(|_| !words.is_empty())
        .filter(|candidate| types.is_empty() || types.contains(&candidate.kind()))
        .map(|candidate| {
            let text = searchable(candidate);
            let haystack = search_text(text);
            let mut terms: Vec<String> = haystack.split_whitespace().map(str::to_owned).collect();
            terms.extend(joined_terms(text));
            Searched { candidate, id: id(candidate), haystack, terms }
        })
        .collect();
    let mut ranked: Vec<(u8, &Searched)> = searched
        .iter()
        .filter_map(|entry| rank(&needle, &words, entry).map(|tier| (tier, entry)))
        .collect();
    if ranked.is_empty() {
        ranked = searched
            .iter()
            .filter(|entry| every_word(&words, entry, is_typo_of))
            .map(|entry| (TYPO_TIER, entry))
            .collect();
    }
    ranked.sort_by(|a, b| {
        a.0.cmp(&b.0).then_with(|| a.1.haystack.cmp(&b.1.haystack)).then_with(|| a.1.id.cmp(b.1.id))
    });
    let matches: Vec<SearchResult> =
        ranked.into_iter().map(|(_, entry)| entry.candidate.clone()).collect();
    paginate(&matches, page)
}

fn rank(needle: &str, words: &[&str], entry: &Searched) -> Option<u8> {
    if entry.haystack == needle {
        Some(0)
    } else if entry.haystack.starts_with(needle) {
        Some(1)
    } else if every_word(words, entry, |_, _| false) {
        Some(2)
    } else {
        None
    }
}

fn every_word(words: &[&str], entry: &Searched, close: impl Fn(&str, &str) -> bool) -> bool {
    words.iter().all(|word| {
        entry.terms.iter().any(|term| term.starts_with(word) || close(word, term.as_str()))
    })
}

fn searchable(result: &SearchResult) -> &str {
    match result {
        SearchResult::Movie(m) => &m.title,
        SearchResult::Series(s) => &s.title,
        SearchResult::Episode(e) => &e.episode.title,
        SearchResult::Person(p) => &p.name,
    }
}

fn id(result: &SearchResult) -> &str {
    match result {
        SearchResult::Movie(m) => &m.id.0,
        SearchResult::Series(s) => &s.id.0,
        SearchResult::Episode(e) => &e.episode.id.0,
        SearchResult::Person(p) => &p.id.0,
    }
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;

    use super::*;
    use crate::catalog::{
        Episode, EpisodeCard, EpisodeId, Movie, MovieId, SeasonId, Series, SeriesId,
    };
    use crate::metadata::{Person, PersonId};

    fn movie(title: &str) -> SearchResult {
        SearchResult::Movie(Movie {
            id: MovieId(title.to_owned()),
            title: title.to_owned(),
            sort_title: crate::text::sort_title(title, &[]),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        })
    }

    fn series(title: &str) -> SearchResult {
        SearchResult::Series(Series {
            id: SeriesId(title.to_owned()),
            title: title.to_owned(),
            sort_title: crate::text::sort_title(title, &[]),
            year: None,
            overview: None,
            content_rating: None,
            manually_edited: false,
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
        SearchResult::Episode(Box::new(EpisodeCard::bare(Episode {
            id: EpisodeId(title.to_owned()),
            season: SeasonId("s".to_owned()),
            number: 1,
            title: title.to_owned(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        })))
    }

    fn titles(page: &Page<SearchResult>) -> Vec<String> {
        page.items.iter().map(|r| searchable(r).to_owned()).collect()
    }

    fn all() -> PageRequest {
        PageRequest { offset: 0, limit: 100 }
    }

    fn search(candidates: &[SearchResult], query: &str, page: PageRequest) -> Page<SearchResult> {
        super::search(candidates, query, &[], page)
    }

    #[test]
    fn mapping_the_series_leaves_every_other_kind_alone() {
        let mapped: Vec<SearchResult<String>> =
            [movie("Drift"), series("Static Bloom"), episode("Pilot Light"), person("Ada")]
                .into_iter()
                .map(|result| result.map_series(|series| series.title))
                .collect();

        assert_eq!(
            mapped.iter().map(SearchResult::kind).collect::<Vec<_>>(),
            [SearchKind::Movie, SearchKind::Series, SearchKind::Episode, SearchKind::Person]
        );
        assert!(matches!(&mapped[1], SearchResult::Series(title) if title == "Static Bloom"));
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

        let two =
            super::search(&candidates, "matrix", &[SearchKind::Series, SearchKind::Episode], all());
        assert_eq!(two.total, 2);

        let unfiltered = super::search(&candidates, "matrix", &[], all());
        assert_eq!(unfiltered.total, 4);
    }

    #[test]
    fn ranks_exact_then_title_prefix_then_word_prefix() {
        let candidates = [
            series("The Matrix"),
            movie("Matrix"),
            movie("Rematrix"),
            movie("Matrix Reloaded"),
            movie("Inception"),
        ];
        let hits = search(&candidates, "matrix", all());
        assert_eq!(hits.total, 3);
        assert_eq!(titles(&hits), vec!["Matrix", "Matrix Reloaded", "The Matrix"]);
    }

    #[test]
    fn excludes_substring_only_matches() {
        let candidates = [movie("Rematrix"), movie("Matrix")];
        let hits = search(&candidates, "matrix", all());
        assert_eq!(hits.total, 1);
        assert_eq!(titles(&hits), vec!["Matrix"]);
    }

    #[test]
    fn a_partial_word_matches_the_word_it_starts() {
        let candidates =
            [series("The Matrix"), episode("Children of the Gods"), movie("Inception")];
        assert_eq!(titles(&search(&candidates, "mat", all())), vec!["The Matrix"]);
        assert_eq!(titles(&search(&candidates, "god", all())), vec!["Children of the Gods"]);
        assert!(
            search(&candidates, "trix", all()).items.is_empty(),
            "a partial word still has to start one, or every query becomes a substring search"
        );
    }

    #[test]
    fn every_word_of_the_query_may_start_a_word_anywhere_in_the_title() {
        let candidates = [
            movie("Harry Potter and the Prisoner of Azkaban"),
            movie("Star Wars: The Empire Strikes Back"),
            movie("Star Trek"),
        ];
        assert_eq!(
            titles(&search(&candidates, "harry potter prisoner", all())),
            ["Harry Potter and the Prisoner of Azkaban"],
            "a query of several words had to match the start of the title"
        );
        assert_eq!(
            titles(&search(&candidates, "empire star", all())),
            ["Star Wars: The Empire Strikes Back"]
        );
        assert_eq!(
            titles(&search(&candidates, "star", all())),
            ["Star Trek", "Star Wars: The Empire Strikes Back"]
        );
    }

    #[test]
    fn a_title_written_with_punctuation_or_accents_is_found_without_them() {
        let candidates = [
            movie("Spider-Man: Far From Home"),
            movie("Ocean's Eleven"),
            series("Marvel's Agents of S.H.I.E.L.D."),
            movie("Amélie"),
            movie("Fast & Furious"),
            movie("Брат 2"),
        ];
        let found = |query: &str| titles(&search(&candidates, query, all()));

        assert_eq!(found("spiderman"), ["Spider-Man: Far From Home"]);
        assert_eq!(found("spider-man"), ["Spider-Man: Far From Home"]);
        assert_eq!(found("oceans"), ["Ocean's Eleven"]);
        assert_eq!(found("shield"), ["Marvel's Agents of S.H.I.E.L.D."]);
        assert_eq!(found("amelie"), ["Amélie"]);
        assert_eq!(found("fast and furious"), ["Fast & Furious"]);
        assert_eq!(found("fast & furious"), ["Fast & Furious"]);
        assert_eq!(found("брат"), ["Брат 2"]);
    }

    #[test]
    fn a_word_typed_with_a_typo_is_found_when_nothing_matches_it_as_typed() {
        let candidates = [
            series("The Matrix"),
            movie("Harry Potter and the Chamber of Secrets"),
            movie("House"),
            movie("Horse Feathers"),
        ];
        let found = |query: &str| titles(&search(&candidates, query, all()));

        assert_eq!(found("matirx"), ["The Matrix"]);
        assert_eq!(found("harry pottr chamber"), ["Harry Potter and the Chamber of Secrets"]);
        assert_eq!(
            found("house"),
            ["House"],
            "a typo match would bury what was typed right, so it only answers a query nothing else does"
        );
        assert_eq!(found("hosue"), ["House"]);
        assert!(found("hpuse feathrs xyz").is_empty());
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
        let candidates =
            [series("The Matrix"), movie("Matrix"), movie("Rematrix"), movie("Matrix Reloaded")];
        let page = search(&candidates, "matrix", PageRequest { offset: 1, limit: 2 });
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
            sort_title: crate::text::sort_title(title, &[]),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        })
    }

    fn all_pages() -> PageRequest {
        PageRequest { offset: 0, limit: 1000 }
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
        fn every_hit_has_each_query_word_start_or_nearly_spell_one_of_its_words(
            titles in prop::collection::vec("[a-z ]{0,10}", 0..16),
            query in "[a-z ]{1,6}",
        ) {
            let candidates: Vec<SearchResult> = titles.iter().map(|t| movie(t)).collect();
            let needle = search_text(&query);
            let hits = search(&candidates, &query, all_pages());
            prop_assert!(hits.total as usize <= candidates.len());
            for item in &hits.items {
                let hay = search_text(searchable(item));
                prop_assert!(needle.split_whitespace().all(|word| hay
                    .split_whitespace()
                    .any(|term| term.starts_with(word) || is_typo_of(word, term))));
            }
        }

        #[test]
        fn a_typo_match_never_joins_a_match_as_typed(
            titles in prop::collection::vec("[a-z ]{0,10}", 0..16),
            query in "[a-z ]{1,9}",
        ) {
            let candidates: Vec<SearchResult> = titles.iter().map(|t| movie(t)).collect();
            let needle = search_text(&query);
            let as_typed = |title: &str| needle.split_whitespace().all(|word| {
                search_text(title).split_whitespace().any(|term| term.starts_with(word))
            });
            let hits = search(&candidates, &query, all_pages());
            if titles.iter().any(|title| as_typed(title)) {
                for item in &hits.items {
                    prop_assert!(as_typed(searchable(item)));
                }
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
