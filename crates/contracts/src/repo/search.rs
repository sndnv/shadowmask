use domain::catalog::{Episode, EpisodeId, Movie, MovieId, Season, SeasonId, Series, SeriesId};
use domain::common::{Page, PageRequest};
use domain::discovery::{SearchKind, SearchResult, search};
use domain::metadata::{Person, PersonId};
use domain::repository::SearchIndex;
use jiff::Timestamp;

pub struct SearchSeed {
    pub movies: Vec<Movie>,
    pub series: Vec<Series>,
    pub seasons: Vec<Season>,
    pub episodes: Vec<Episode>,
    pub people: Vec<Person>,
}

fn ts() -> Timestamp {
    Timestamp::UNIX_EPOCH
}

fn movie(id: &str, title: &str) -> Movie {
    Movie {
        id: MovieId(id.into()),
        title: title.into(),
        year: None,
        overview: None,
        runtime_minutes: None,
        content_rating: None,
        added_at: ts(),
        updated_at: ts(),
        artwork: Vec::new(),
    }
}

fn series(id: &str, title: &str) -> Series {
    Series {
        id: SeriesId(id.into()),
        title: title.into(),
        year: None,
        overview: None,
        content_rating: None,
        added_at: ts(),
        updated_at: ts(),
        artwork: Vec::new(),
    }
}

fn season(id: &str, series: &str) -> Season {
    Season {
        id: SeasonId(id.into()),
        series: SeriesId(series.into()),
        number: 1,
        title: None,
        overview: None,
        added_at: ts(),
        updated_at: ts(),
        artwork: Vec::new(),
    }
}

fn episode(id: &str, season: &str, title: &str) -> Episode {
    Episode {
        id: EpisodeId(id.into()),
        season: SeasonId(season.into()),
        number: 1,
        title: title.into(),
        overview: None,
        runtime_minutes: None,
        air_date: None,
        added_at: ts(),
        updated_at: ts(),
        artwork: Vec::new(),
    }
}

fn person(id: &str, name: &str) -> Person {
    Person {
        id: PersonId(id.into()),
        name: name.into(),
    }
}

pub fn search_seed() -> SearchSeed {
    SearchSeed {
        movies: vec![
            movie("m1", "Matrix"),
            movie("m2", "Matrix Reloaded"),
            movie("m3", "Rematrix"),
            movie("m4", "Inception"),
        ],
        series: vec![series("sr1", "The Matrix")],
        seasons: vec![season("se1", "sr1")],
        episodes: vec![episode("e1", "se1", "Matrix Origins")],
        people: vec![person("p1", "Neo Anderson")],
    }
}

fn all_results(seed: &SearchSeed) -> Vec<SearchResult> {
    let mut all = Vec::new();
    all.extend(seed.movies.iter().cloned().map(SearchResult::Movie));
    all.extend(seed.series.iter().cloned().map(SearchResult::Series));
    all.extend(seed.episodes.iter().cloned().map(SearchResult::Episode));
    all.extend(seed.people.iter().cloned().map(SearchResult::Person));
    all
}

fn page(offset: u32, limit: u32) -> PageRequest {
    PageRequest { offset, limit }
}

fn title(result: &SearchResult) -> String {
    match result {
        SearchResult::Movie(m) => m.title.clone(),
        SearchResult::Series(s) => s.title.clone(),
        SearchResult::Episode(e) => e.title.clone(),
        SearchResult::Person(p) => p.name.clone(),
    }
}

fn titles(page: &Page<SearchResult>) -> Vec<String> {
    page.items.iter().map(title).collect()
}

fn keys(page: &Page<SearchResult>) -> Vec<String> {
    page.items
        .iter()
        .map(|result| match result {
            SearchResult::Movie(m) => format!("movie:{}", m.id.0),
            SearchResult::Series(s) => format!("series:{}", s.id.0),
            SearchResult::Episode(e) => format!("episode:{}", e.id.0),
            SearchResult::Person(p) => format!("person:{}", p.id.0),
        })
        .collect()
}

pub async fn search_index_contract<R: SearchIndex>(index: R, seed: impl AsyncFn(&R)) {
    let before = index.search("matrix", &[], page(0, 10)).await.unwrap();
    assert_eq!(before.total, 0);
    assert!(before.items.is_empty());

    seed(&index).await;

    let fixture = search_seed();
    let all = all_results(&fixture);

    let blank = index.search("   ", &[], page(0, 10)).await.unwrap();
    assert_eq!(blank.total, 0);
    assert!(blank.items.is_empty());

    let hits = index.search("matrix", &[], page(0, 10)).await.unwrap();
    assert_eq!(
        titles(&hits),
        ["Matrix", "Matrix Origins", "Matrix Reloaded", "The Matrix"]
    );
    assert_eq!(hits.total, 4);
    assert!(
        titles(&hits).iter().all(|t| t != "Rematrix"),
        "substring-only matches are excluded"
    );

    assert_eq!(keys(&hits), keys(&search(&all, "matrix", &[], page(0, 10))));

    let reloaded = index.search("reloaded", &[], page(0, 10)).await.unwrap();
    assert_eq!(titles(&reloaded), ["Matrix Reloaded"]);

    let movies_only = index
        .search("matrix", &[SearchKind::Movie], page(0, 10))
        .await
        .unwrap();
    assert_eq!(titles(&movies_only), ["Matrix", "Matrix Reloaded"]);

    let series_only = index
        .search("matrix", &[SearchKind::Series], page(0, 10))
        .await
        .unwrap();
    assert_eq!(titles(&series_only), ["The Matrix"]);

    let episode_only = index
        .search("matrix", &[SearchKind::Episode], page(0, 10))
        .await
        .unwrap();
    assert_eq!(titles(&episode_only), ["Matrix Origins"]);

    let neo = index.search("neo", &[], page(0, 10)).await.unwrap();
    assert_eq!(titles(&neo), ["Neo Anderson"]);
    assert_eq!(keys(&neo), ["person:p1"]);
    let people_only = index
        .search("neo", &[SearchKind::Person], page(0, 10))
        .await
        .unwrap();
    assert_eq!(titles(&people_only), ["Neo Anderson"]);
    assert!(
        index
            .search("matrix", &[SearchKind::Person], page(0, 10))
            .await
            .unwrap()
            .items
            .is_empty()
    );

    let paged = index.search("matrix", &[], page(1, 2)).await.unwrap();
    assert_eq!(titles(&paged), ["Matrix Origins", "Matrix Reloaded"]);
    assert_eq!(paged.total, 4);

    assert!(
        index
            .search("nothing-here", &[], page(0, 10))
            .await
            .unwrap()
            .items
            .is_empty()
    );

    index.rebuild().await.unwrap();
    let after = index.search("matrix", &[], page(0, 10)).await.unwrap();
    assert_eq!(keys(&after), keys(&hits));
    let neo_after = index.search("neo", &[], page(0, 10)).await.unwrap();
    assert_eq!(keys(&neo_after), ["person:p1"]);
}
