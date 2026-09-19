use domain::catalog::{
    ArtworkId, ArtworkRef, ArtworkWidth, Episode, EpisodeCard, EpisodeId, Movie, MovieId, Season,
    SeasonId, Series, SeriesId, TitleId, TitleListFilter, Version, VersionId,
};
use domain::common::{Page, PageRequest, Quality};
use domain::discovery::{SearchKind, SearchResult, search};
use domain::library::LibraryId;
use domain::metadata::{ArtworkKind, ContentRating, Person, PersonId};
use domain::repository::SearchIndex;
use domain::text::sort_title;
use jiff::Timestamp;

pub struct SearchSeed {
    pub movies: Vec<Movie>,
    pub series: Vec<Series>,
    pub seasons: Vec<Season>,
    pub episodes: Vec<Episode>,
    pub people: Vec<Person>,
    pub versions: Vec<Version>,
}

fn ts() -> Timestamp {
    Timestamp::UNIX_EPOCH
}

fn movie(id: &str, title: &str) -> Movie {
    Movie {
        id: MovieId(id.into()),
        title: title.into(),
        sort_title: sort_title(title, &[]),
        year: None,
        overview: None,
        runtime_minutes: None,
        content_rating: None,
        manually_edited: false,
        added_at: ts(),
        updated_at: ts(),
        artwork: Vec::new(),
    }
}

fn series(id: &str, title: &str) -> Series {
    Series {
        id: SeriesId(id.into()),
        title: title.into(),
        sort_title: sort_title(title, &[]),
        year: None,
        overview: None,
        content_rating: None,
        manually_edited: false,
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
        manually_edited: false,
        added_at: ts(),
        updated_at: ts(),
        artwork: Vec::new(),
    }
}

fn poster(id: &str) -> ArtworkRef {
    ArtworkRef {
        id: ArtworkId(id.into()),
        kind: ArtworkKind::Poster,
        widths: vec![ArtworkWidth::new(180, format!("/art/{id}/180.jpg"))],
    }
}

fn person(id: &str, name: &str) -> Person {
    Person { id: PersonId(id.into()), name: name.into(), ..Person::default() }
}

fn version(id: &str, title: TitleId, library: &str) -> Version {
    Version {
        id: VersionId(id.into()),
        title,
        library: LibraryId(library.into()),
        quality: Quality::Hd,
        container: "mkv".into(),
        path: format!("/media/{id}.mkv"),
        size_bytes: 1,
        duration_ms: 1000,
        available: true,
        added_at: ts(),
        updated_at: ts(),
    }
}

fn rated(mut movie: Movie) -> Movie {
    movie.content_rating = Some(adult());
    movie
}

pub fn search_seed() -> SearchSeed {
    SearchSeed {
        movies: vec![
            movie("m1", "Matrix"),
            rated(movie("m2", "Matrix Reloaded")),
            movie("m3", "Rematrix"),
            movie("m4", "Inception"),
        ],
        series: vec![Series { artwork: vec![poster("sr1-poster")], ..series("sr1", "The Matrix") }],
        seasons: vec![season("se1", "sr1")],
        episodes: vec![episode("e1", "se1", "Matrix Origins")],
        people: vec![person("p1", "Neo Anderson")],
        versions: vec![
            version("v1", TitleId::Movie(MovieId("m1".into())), "lib1"),
            version("v2", TitleId::Movie(MovieId("m2".into())), "lib2"),
            version("ve1", TitleId::Episode(EpisodeId("e1".into())), "lib1"),
        ],
    }
}

fn all_results(seed: &SearchSeed) -> Vec<SearchResult> {
    let mut all = Vec::new();
    all.extend(seed.movies.iter().cloned().map(SearchResult::Movie));
    all.extend(seed.series.iter().cloned().map(SearchResult::Series));
    all.extend(
        seed.episodes
            .iter()
            .cloned()
            .map(|episode| SearchResult::Episode(Box::new(EpisodeCard::bare(episode)))),
    );
    all.extend(seed.people.iter().cloned().map(SearchResult::Person));
    all
}

fn page(offset: u32, limit: u32) -> PageRequest {
    PageRequest { offset, limit }
}

fn open() -> TitleListFilter {
    TitleListFilter::default()
}

fn adult() -> ContentRating {
    ContentRating { system: "mpaa".into(), code: "r".into() }
}

fn capped() -> TitleListFilter {
    TitleListFilter { blocked_ratings: vec![adult()], ..TitleListFilter::default() }
}

fn without_library_access() -> TitleListFilter {
    TitleListFilter { libraries: Some(Vec::new()), ..TitleListFilter::default() }
}

fn granted(library: &str) -> TitleListFilter {
    TitleListFilter {
        libraries: Some(vec![LibraryId(library.into())]),
        ..TitleListFilter::default()
    }
}

fn title(result: &SearchResult) -> String {
    match result {
        SearchResult::Movie(m) => m.title.clone(),
        SearchResult::Series(s) => s.title.clone(),
        SearchResult::Episode(e) => e.episode.title.clone(),
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
            SearchResult::Episode(e) => format!("episode:{}", e.episode.id.0),
            SearchResult::Person(p) => format!("person:{}", p.id.0),
        })
        .collect()
}

pub async fn search_index_contract<R: SearchIndex>(index: R, seed: impl AsyncFn(&R)) {
    let before = index.search("matrix", &[], &open(), page(0, 10)).await.unwrap();
    assert_eq!(before.total, 0);
    assert!(before.items.is_empty());

    seed(&index).await;

    let fixture = search_seed();
    let all = all_results(&fixture);

    let blank = index.search("   ", &[], &open(), page(0, 10)).await.unwrap();
    assert_eq!(blank.total, 0);
    assert!(blank.items.is_empty());

    let hits = index.search("matrix", &[], &open(), page(0, 10)).await.unwrap();
    assert_eq!(titles(&hits), ["Matrix", "Matrix Origins", "Matrix Reloaded", "The Matrix"]);
    assert_eq!(hits.total, 4);
    assert!(titles(&hits).iter().all(|t| t != "Rematrix"), "substring-only matches are excluded");

    assert_eq!(keys(&hits), keys(&search(&all, "matrix", &[], page(0, 10))));

    let reloaded = index.search("reloaded", &[], &open(), page(0, 10)).await.unwrap();
    assert_eq!(titles(&reloaded), ["Matrix Reloaded"]);

    let movies_only =
        index.search("matrix", &[SearchKind::Movie], &open(), page(0, 10)).await.unwrap();
    assert_eq!(titles(&movies_only), ["Matrix", "Matrix Reloaded"]);

    let series_only =
        index.search("matrix", &[SearchKind::Series], &open(), page(0, 10)).await.unwrap();
    assert_eq!(titles(&series_only), ["The Matrix"]);

    let episode_only =
        index.search("matrix", &[SearchKind::Episode], &open(), page(0, 10)).await.unwrap();
    assert_eq!(titles(&episode_only), ["Matrix Origins"]);

    let SearchResult::Episode(found) = &episode_only.items[0] else {
        panic!("the episode branch must return an episode");
    };
    assert_eq!(
        (
            found.series.as_ref().map(|s| s.0.as_str()),
            found.series_title.as_deref(),
            found.season_number
        ),
        (Some("sr1"), Some("The Matrix"), Some(1)),
        "without the series an episode result cannot be labelled with the show it belongs to"
    );
    assert_eq!(
        found.series_artwork.iter().map(|art| art.id.0.as_str()).collect::<Vec<_>>(),
        ["sr1-poster"],
        "an episode still is landscape, so a portrait card needs the series poster to fall back on"
    );

    let neo = index.search("neo", &[], &open(), page(0, 10)).await.unwrap();
    assert_eq!(titles(&neo), ["Neo Anderson"]);
    assert_eq!(keys(&neo), ["person:p1"]);
    let people_only =
        index.search("neo", &[SearchKind::Person], &open(), page(0, 10)).await.unwrap();
    assert_eq!(titles(&people_only), ["Neo Anderson"]);
    assert!(
        index
            .search("matrix", &[SearchKind::Person], &open(), page(0, 10))
            .await
            .unwrap()
            .items
            .is_empty()
    );

    let paged = index.search("matrix", &[], &open(), page(1, 2)).await.unwrap();
    assert_eq!(titles(&paged), ["Matrix Origins", "Matrix Reloaded"]);
    assert_eq!(paged.total, 4);

    assert!(
        index.search("nothing-here", &[], &open(), page(0, 10)).await.unwrap().items.is_empty()
    );

    let under_cap = index.search("matrix", &[], &capped(), page(0, 10)).await.unwrap();
    assert_eq!(
        titles(&under_cap),
        ["Matrix", "Matrix Origins", "The Matrix"],
        "a blocked rating must leave the search, not just the detail route"
    );
    assert_eq!(
        under_cap.total, 3,
        "the total has to count the filtered set, or paging reports rows the caller cannot see"
    );

    let capped_series = TitleListFilter {
        blocked_ratings: vec![ContentRating { system: "mpaa".into(), code: "nc-17".into() }],
        ..TitleListFilter::default()
    };
    assert_eq!(
        titles(&index.search("matrix", &[], &capped_series, page(0, 10)).await.unwrap()),
        ["Matrix", "Matrix Origins", "Matrix Reloaded", "The Matrix"],
        "blocking a rating nothing carries must not drop anything"
    );

    let no_access =
        index.search("matrix", &[], &without_library_access(), page(0, 10)).await.unwrap();
    assert!(no_access.items.is_empty(), "a user granted no library sees no titles through search");

    let one_library = index.search("matrix", &[], &granted("lib1"), page(0, 10)).await.unwrap();
    assert_eq!(
        titles(&one_library),
        ["Matrix", "Matrix Origins", "The Matrix"],
        "only titles with a version in the granted library survive, and a series rides on its episodes"
    );

    assert_eq!(
        titles(&index.search("neo", &[], &without_library_access(), page(0, 10)).await.unwrap()),
        ["Neo Anderson"],
        "people carry no rating and belong to no library, so they are never gated"
    );

    index.rebuild().await.unwrap();
    let after = index.search("matrix", &[], &open(), page(0, 10)).await.unwrap();
    assert_eq!(keys(&after), keys(&hits));
    let neo_after = index.search("neo", &[], &open(), page(0, 10)).await.unwrap();
    assert_eq!(keys(&neo_after), ["person:p1"]);
}
