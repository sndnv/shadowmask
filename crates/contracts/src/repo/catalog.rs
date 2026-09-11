use domain::catalog::{
    ArtworkId, ArtworkOwner, ArtworkRef, ArtworkWidth, Collection, CollectionId, Episode,
    EpisodeId, Movie, MovieId, RandomScope, Season, SeasonId, Series, SeriesId, SortOrder, TitleId,
    TitleKind, TitleListFilter, TitleRef, TitleSort, Version, VersionDetail, VersionId,
};
use domain::common::{LanguageCode, PageRequest, Quality};
use domain::library::LibraryId;
use domain::media::{
    AudioTrack, Chapter, CreditsMarker, DetectedMarkers, EmbeddedSubtitleTrack, HdrFormat,
    IntroMarker, SubtitleFile, SubtitleFileId, SubtitleFormat, SubtitleSource, TrickplayAsset,
    VideoTrack,
};
use domain::metadata::{
    ArtworkKind, ContentRating, Credit, CreditRole, ExternalId, Extra, ExtraKind, Genre, GenreId,
    Person, PersonId, Rating, Studio, StudioId, TitleEnrichment,
};
use domain::repository::CatalogRepository;
use jiff::Timestamp;

fn ts(second: i64) -> Timestamp {
    Timestamp::from_second(second).expect("valid timestamp")
}

fn page(offset: u32, limit: u32) -> PageRequest {
    PageRequest { offset, limit }
}

/// Deterministic catalog fixture applied by the seed closure and used to derive
/// the contract's expectations. A single source of truth for both sides.
pub struct CatalogSeed {
    pub movies: Vec<Movie>,
    pub series: Vec<Series>,
    pub seasons: Vec<Season>,
    pub episodes: Vec<Episode>,
    pub collections: Vec<Collection>,
    pub versions: Vec<Version>,
    pub detail: VersionDetail,
    pub people: Vec<Person>,
    pub enrichment: Vec<(TitleRef, TitleEnrichment)>,
}

pub fn catalog_seed() -> CatalogSeed {
    let detail = VersionDetail {
        version: Version {
            id: VersionId("v1".into()),
            title: TitleId::Movie(MovieId("m1".into())),
            library: LibraryId("lib1".into()),
            quality: Quality::Sd,
            container: "mkv".into(),
            path: "/media/v1.mkv".into(),
            size_bytes: 1_000_000,
            duration_ms: 120_000,
            available: true,
            added_at: ts(14),
            updated_at: ts(14),
        },
        video: vec![VideoTrack {
            index: 0,
            codec: "h264".into(),
            width: 1920,
            height: 1080,
            bit_depth: 8,
            hdr: Some(HdrFormat::Hdr10),
            frame_rate: 24.0,
            bitrate: Some(8_000_000),
        }],
        audio: vec![AudioTrack {
            index: 1,
            codec: "aac".into(),
            channels: 6,
            language: Some(LanguageCode("en".into())),
            bitrate: Some(640_000),
        }],
        subtitles: vec![EmbeddedSubtitleTrack {
            index: 2,
            language: Some(LanguageCode("en".into())),
            format: SubtitleFormat::Srt,
            forced: false,
            default: true,
        }],
        subtitle_files: vec![SubtitleFile {
            id: SubtitleFileId("sf1".into()),
            version: VersionId("v1".into()),
            language: Some(LanguageCode("en".into())),
            format: SubtitleFormat::Srt,
            source: SubtitleSource::External,
            path: "/media/v1.en.srt".into(),
            translated_from: None,
            label: None,
            pinned: false,
        }],
        chapters: vec![Chapter { title: "Chapter 1".into(), start_ms: 0 }],
        markers: DetectedMarkers {
            intros: vec![IntroMarker {
                version: VersionId("v1".into()),
                start_ms: 0,
                end_ms: 5_000,
            }],
            credits: vec![CreditsMarker {
                version: VersionId("v1".into()),
                start_ms: 90_000,
                end_ms: 95_000,
            }],
        },
        trickplay: vec![TrickplayAsset {
            version: VersionId("v1".into()),
            interval_ms: 10_000,
            columns: 10,
            rows: 10,
            tile_width: 320,
            tile_height: 180,
            sheet_paths: vec!["/tp/1.jpg".into(), "/tp/2.jpg".into()],
        }],
    };
    CatalogSeed {
        movies: vec![
            Movie {
                id: MovieId("m1".into()),
                title: "Alpha".into(),
                sort_title: "alpha".into(),
                year: Some(2020),
                overview: Some("first".into()),
                runtime_minutes: Some(100),
                content_rating: Some(ContentRating { system: "MPAA".into(), code: "PG-13".into() }),
                manually_edited: false,
                added_at: ts(1),
                updated_at: ts(1),
                artwork: Vec::new(),
            },
            Movie {
                id: MovieId("m2".into()),
                title: "Beta".into(),
                sort_title: "beta".into(),
                year: None,
                overview: None,
                runtime_minutes: None,
                content_rating: None,
                manually_edited: false,
                added_at: ts(2),
                updated_at: ts(2),
                artwork: Vec::new(),
            },
        ],
        series: vec![Series {
            id: SeriesId("s1".into()),
            title: "Gamma".into(),
            sort_title: "gamma".into(),
            year: Some(2019),
            overview: None,
            content_rating: None,
            manually_edited: false,
            added_at: ts(3),
            updated_at: ts(3),
            artwork: Vec::new(),
        }],
        seasons: vec![Season {
            id: SeasonId("se1".into()),
            series: SeriesId("s1".into()),
            number: 1,
            title: Some("Season 1".into()),
            overview: None,
            added_at: ts(10),
            updated_at: ts(10),
            artwork: Vec::new(),
        }],
        episodes: vec![
            Episode {
                id: EpisodeId("e1".into()),
                season: SeasonId("se1".into()),
                number: 1,
                title: "Pilot".into(),
                overview: None,
                runtime_minutes: Some(42),
                air_date: Some(ts(4)),
                manually_edited: false,
                added_at: ts(5),
                updated_at: ts(5),
                artwork: Vec::new(),
            },
            Episode {
                id: EpisodeId("e2".into()),
                season: SeasonId("se1".into()),
                number: 2,
                title: "Second".into(),
                overview: None,
                runtime_minutes: None,
                air_date: None,
                manually_edited: false,
                added_at: ts(6),
                updated_at: ts(6),
                artwork: Vec::new(),
            },
        ],
        collections: vec![Collection {
            id: CollectionId("c1".into()),
            name: "Saga".into(),
            overview: Some("epic".into()),
            movies: vec![MovieId("m1".into()), MovieId("m2".into())],
            added_at: ts(11),
            updated_at: ts(11),
            artwork: Vec::new(),
        }],
        versions: vec![
            Version {
                id: VersionId("v2".into()),
                title: TitleId::Movie(MovieId("m1".into())),
                library: LibraryId("lib1".into()),
                quality: Quality::Hd,
                container: "mp4".into(),
                path: "/media/v2.mp4".into(),
                size_bytes: 2_000_000,
                duration_ms: 120_000,
                available: true,
                added_at: ts(12),
                updated_at: ts(12),
            },
            Version {
                id: VersionId("v3".into()),
                title: TitleId::Episode(EpisodeId("e1".into())),
                library: LibraryId("lib2".into()),
                quality: Quality::Hd,
                container: "mkv".into(),
                path: "/media/v3.mkv".into(),
                size_bytes: 3_000_000,
                duration_ms: 2_520_000,
                available: true,
                added_at: ts(13),
                updated_at: ts(13),
            },
        ],
        detail,
        people: vec![
            Person { id: PersonId("p1".into()), name: "Ada".into(), ..Person::default() },
            Person { id: PersonId("p2".into()), name: "Bob".into(), ..Person::default() },
        ],
        enrichment: vec![
            (
                TitleRef::Movie(MovieId("m1".into())),
                TitleEnrichment {
                    genres: vec![
                        Genre { id: GenreId("g-action".into()), name: "Action".into() },
                        Genre { id: GenreId("g-drama".into()), name: "Drama".into() },
                    ],
                    credits: vec![
                        Credit {
                            person: PersonId("p1".into()),
                            title: TitleRef::Movie(MovieId("m1".into())),
                            role: CreditRole::Actor,
                            character: Some("Hero".into()),
                            order: 0,
                        },
                        Credit {
                            person: PersonId("p2".into()),
                            title: TitleRef::Movie(MovieId("m1".into())),
                            role: CreditRole::Director,
                            character: None,
                            order: 1,
                        },
                    ],
                    studios: vec![Studio {
                        id: StudioId("st-acme".into()),
                        name: "Acme Studios".into(),
                    }],
                    ratings: vec![Rating { source: "tmdb".into(), value: 8.5 }],
                    external_ids: vec![
                        ExternalId { source: "tmdb".into(), value: "603".into() },
                        ExternalId { source: "imdb".into(), value: "tt0133093".into() },
                    ],
                    extras: vec![Extra {
                        kind: ExtraKind::Trailer,
                        title: "Teaser".into(),
                        path: "/extras/m1/teaser.mkv".into(),
                    }],
                },
            ),
            (
                TitleRef::Series(SeriesId("s1".into())),
                TitleEnrichment {
                    genres: vec![Genre { id: GenreId("g-action".into()), name: "Action".into() }],
                    credits: vec![Credit {
                        person: PersonId("p1".into()),
                        title: TitleRef::Series(SeriesId("s1".into())),
                        role: CreditRole::Actor,
                        character: Some("Lead".into()),
                        order: 0,
                    }],
                    ratings: vec![Rating { source: "tmdb".into(), value: 9.0 }],
                    external_ids: vec![ExternalId { source: "tvdb".into(), value: "81189".into() }],
                    ..TitleEnrichment::default()
                },
            ),
        ],
    }
}

fn in_libraries(libraries: &[&str]) -> TitleListFilter {
    TitleListFilter {
        libraries: Some(libraries.iter().map(|id| LibraryId((*id).into())).collect()),
        ..TitleListFilter::default()
    }
}

fn with_genres(genres: &[&str]) -> TitleListFilter {
    TitleListFilter {
        genres: genres.iter().map(|name| (*name).to_string()).collect(),
        ..TitleListFilter::default()
    }
}

async fn random_picks_stay_inside_their_scope<R: CatalogRepository>(repo: &R) {
    let any = TitleListFilter::default();
    let movie = TitleId::Movie(MovieId("m1".into()));
    let episode = TitleId::Episode(EpisodeId("e1".into()));

    assert_eq!(
        repo.random_playable_title(&RandomScope::Movies, &any).await.unwrap(),
        Some(movie.clone()),
        "m2 has no version at all, so m1 is the only playable movie"
    );
    assert_eq!(
        repo.random_playable_title(&RandomScope::Episodes, &any).await.unwrap(),
        Some(episode.clone()),
        "e2 has no version at all, so e1 is the only playable episode"
    );
    assert_eq!(
        repo.random_playable_title(&RandomScope::Series(SeriesId("s1".into())), &any)
            .await
            .unwrap(),
        Some(episode.clone())
    );
    assert_eq!(
        repo.random_playable_title(&RandomScope::Season(SeasonId("se1".into())), &any)
            .await
            .unwrap(),
        Some(episode.clone())
    );
    assert_eq!(
        repo.random_playable_title(&RandomScope::Collection(CollectionId("c1".into())), &any)
            .await
            .unwrap(),
        Some(movie.clone())
    );

    assert!(
        repo.random_playable_title(&RandomScope::Series(SeriesId("nope".into())), &any)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.random_playable_title(&RandomScope::Season(SeasonId("nope".into())), &any)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.random_playable_title(&RandomScope::Collection(CollectionId("nope".into())), &any)
            .await
            .unwrap()
            .is_none()
    );

    assert_eq!(
        repo.random_playable_title(&RandomScope::Movies, &in_libraries(&["lib1"])).await.unwrap(),
        Some(movie.clone())
    );
    assert!(
        repo.random_playable_title(&RandomScope::Movies, &in_libraries(&["lib2"]))
            .await
            .unwrap()
            .is_none(),
        "m1 lives only in lib1, so a lib2 reader gets nothing"
    );
    assert_eq!(
        repo.random_playable_title(&RandomScope::Episodes, &in_libraries(&["lib2"])).await.unwrap(),
        Some(episode.clone())
    );
    assert!(
        repo.random_playable_title(&RandomScope::Episodes, &in_libraries(&["lib1"]))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.random_playable_title(&RandomScope::Movies, &in_libraries(&[]))
            .await
            .unwrap()
            .is_none(),
        "a reader with no library access can never be handed a title"
    );

    assert_eq!(
        repo.random_playable_title(&RandomScope::Movies, &with_genres(&["Drama"])).await.unwrap(),
        Some(movie.clone())
    );
    assert!(
        repo.random_playable_title(&RandomScope::Movies, &with_genres(&["Comedy"]))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.random_playable_title(&RandomScope::Episodes, &with_genres(&["Drama"]))
            .await
            .unwrap()
            .is_none(),
        "the episode genre gate reads the series, and s1 is Action only"
    );
    assert_eq!(
        repo.random_playable_title(&RandomScope::Episodes, &with_genres(&["Action"]))
            .await
            .unwrap(),
        Some(episode.clone())
    );

    let blocked = |code: &str| TitleListFilter {
        blocked_ratings: vec![ContentRating { system: "MPAA".into(), code: code.into() }],
        ..TitleListFilter::default()
    };
    assert_eq!(
        repo.random_playable_title(&RandomScope::Movies, &blocked("R")).await.unwrap(),
        Some(movie),
        "blocking a rating m1 does not carry must not filter it out"
    );
    assert!(
        repo.random_playable_title(&RandomScope::Movies, &blocked("PG-13"))
            .await
            .unwrap()
            .is_none(),
        "m1 is PG-13, so a PG-13 block leaves nothing playable"
    );
}

pub async fn catalog_repository_contract<R: CatalogRepository>(repo: R, seed: impl AsyncFn(&R)) {
    assert!(repo.get_movie(&MovieId("nope".into())).await.unwrap().is_none());
    let empty = repo.list_movies(page(0, 10)).await.unwrap();
    assert_eq!(empty.total, 0);
    assert!(empty.items.is_empty());
    assert!(repo.version_detail(&VersionId("nope".into())).await.unwrap().is_none());

    seed(&repo).await;

    random_picks_stay_inside_their_scope(&repo).await;

    let movies = repo.list_movies(page(0, 10)).await.unwrap();
    assert_eq!(movies.total, 2);
    assert_eq!(movies.items.iter().map(|m| m.id.0.as_str()).collect::<Vec<_>>(), ["m1", "m2"]);
    assert_eq!(movies.items[0].title, "Alpha");
    assert_eq!(movies.items[0].year, Some(2020));
    assert_eq!(
        movies.items[0].content_rating,
        Some(ContentRating { system: "MPAA".into(), code: "PG-13".into() })
    );
    assert_eq!(movies.items[1].year, None);
    assert_eq!(movies.items[1].content_rating, None);

    let page_two = repo.list_movies(page(1, 1)).await.unwrap();
    assert_eq!(page_two.total, 2);
    assert_eq!(page_two.items.len(), 1);
    assert_eq!(page_two.items[0].id, MovieId("m2".into()));
    assert_eq!(page_two.offset, 1);

    let past_end = repo.list_movies(page(9, 10)).await.unwrap();
    assert_eq!(past_end.total, 2);
    assert!(past_end.items.is_empty());

    assert_eq!(repo.get_movie(&MovieId("m1".into())).await.unwrap().unwrap().title, "Alpha");

    let batched = repo
        .movies_by_ids(&[MovieId("m2".into()), MovieId("nope".into()), MovieId("m1".into())])
        .await
        .unwrap();
    assert_eq!(
        batched.iter().map(|m| m.id.0.as_str()).collect::<Vec<_>>(),
        vec!["m2", "m1"],
        "movies come back in the order asked for, and unknown ids are dropped"
    );
    assert_eq!(
        batched[1].artwork,
        repo.get_movie(&MovieId("m1".into())).await.unwrap().unwrap().artwork,
        "a batched movie is hydrated exactly like a single one"
    );
    assert!(repo.movies_by_ids(&[]).await.unwrap().is_empty());

    let series = repo.list_series(page(0, 10)).await.unwrap();
    assert_eq!(series.total, 1);
    assert_eq!(series.items[0].id, SeriesId("s1".into()));
    assert!(repo.get_series(&SeriesId("s1".into())).await.unwrap().is_some());
    assert!(repo.get_series(&SeriesId("no".into())).await.unwrap().is_none());

    let seasons = repo.list_seasons(&SeriesId("s1".into())).await.unwrap();
    assert_eq!(seasons.len(), 1);
    assert_eq!(seasons[0].number, 1);

    let episodes = repo.list_episodes(&SeasonId("se1".into())).await.unwrap();
    assert_eq!(episodes.iter().map(|e| e.id.0.as_str()).collect::<Vec<_>>(), ["e1", "e2"]);
    assert_eq!(episodes[0].air_date, Some(ts(4)));
    assert_eq!(episodes[1].air_date, None);

    let batched_series =
        repo.series_by_ids(&[SeriesId("nope".into()), SeriesId("s1".into())]).await.unwrap();
    assert_eq!(
        batched_series.iter().map(|s| s.id.0.as_str()).collect::<Vec<_>>(),
        vec!["s1"],
        "series come back in the order asked for, and unknown ids are dropped"
    );
    assert_eq!(
        batched_series[0].artwork,
        repo.get_series(&SeriesId("s1".into())).await.unwrap().unwrap().artwork,
        "a batched series is hydrated exactly like a single one"
    );
    assert!(repo.series_by_ids(&[]).await.unwrap().is_empty());

    assert_eq!(
        repo.episode_ids_for_series(&[SeriesId("s1".into())]).await.unwrap()
            [&SeriesId("s1".into())],
        vec![EpisodeId("e1".into()), EpisodeId("e2".into())],
        "a rollup counts a series without walking its seasons one at a time"
    );
    assert_eq!(
        repo.episode_ids_for_seasons(&[SeasonId("se1".into())]).await.unwrap()
            [&SeasonId("se1".into())],
        vec![EpisodeId("e1".into()), EpisodeId("e2".into())]
    );
    assert!(
        repo.episode_ids_for_series(&[SeriesId("no".into())]).await.unwrap().is_empty(),
        "a target with no episodes is absent from the map rather than an error, \
         so the rollup reports zero of zero"
    );
    assert!(
        repo.episode_ids_for_series(&[]).await.unwrap().is_empty(),
        "a grid page of only movies asks for no series at all, and an empty \
         IN list is not valid SQL"
    );
    assert!(repo.episode_ids_for_seasons(&[]).await.unwrap().is_empty());
    assert!(repo.get_episode(&EpisodeId("e1".into())).await.unwrap().is_some());
    assert!(repo.get_episode(&EpisodeId("no".into())).await.unwrap().is_none());

    let ungated = TitleListFilter::default();
    let recent = repo.recent_episodes(&ungated, 10).await.unwrap();
    assert_eq!(
        recent.iter().map(|c| c.episode.id.0.as_str()).collect::<Vec<_>>(),
        ["e2", "e1"],
        "the rail window is newest first, so the caller never sorts the catalog"
    );
    assert_eq!(recent[0].series, SeriesId("s1".into()));
    assert_eq!(recent[0].season, SeasonId("se1".into()));
    assert_eq!(recent[0].season_number, 1);
    assert_eq!(
        repo.recent_episodes(&ungated, 1).await.unwrap().len(),
        1,
        "the limit is applied by the query, not by the caller"
    );
    assert_eq!(
        repo.recent_episodes(&in_libraries(&["lib2"]), 10)
            .await
            .unwrap()
            .iter()
            .map(|c| c.episode.id.0.as_str())
            .collect::<Vec<_>>(),
        ["e1"],
        "e2 has no version at all, so no library grants it"
    );

    assert_eq!(
        repo.visible_movies(&[MovieId("m1".into()), MovieId("ghost".into())], &ungated)
            .await
            .unwrap()
            .iter()
            .map(|m| m.id.0.as_str())
            .collect::<Vec<_>>(),
        ["m1"],
        "an id with no row drops out rather than erroring"
    );
    assert!(repo.visible_movies(&[], &ungated).await.unwrap().is_empty());
    assert!(
        repo.visible_movies(&[MovieId("m1".into())], &in_libraries(&["lib2"]))
            .await
            .unwrap()
            .is_empty(),
        "the watchlist cannot route around the library gate"
    );

    let seen = repo
        .visible_episodes(&[EpisodeId("e1".into()), EpisodeId("ghost".into())], &ungated)
        .await
        .unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].episode.id, EpisodeId("e1".into()));
    assert_eq!(seen[0].season_number, 1);
    assert!(
        repo.visible_episodes(&[EpisodeId("e1".into())], &in_libraries(&["lib1"]))
            .await
            .unwrap()
            .is_empty()
    );

    let next = repo
        .next_episode_in_series(&SeriesId("s1".into()), 1, 1, &ungated)
        .await
        .unwrap()
        .expect("e2 follows e1 in the same season");
    assert_eq!(next.episode.id, EpisodeId("e2".into()));
    assert_eq!(
        repo.next_episode_in_series(&SeriesId("s1".into()), 0, 0, &ungated)
            .await
            .unwrap()
            .expect("both episodes sit after season zero")
            .episode
            .id,
        EpisodeId("e1".into()),
        "with more than one episode still ahead, up next resumes at the earliest, \
         never an arbitrary later one"
    );
    assert!(
        repo.next_episode_in_series(&SeriesId("s1".into()), 1, 2, &ungated)
            .await
            .unwrap()
            .is_none(),
        "the last episode of a series has no successor"
    );
    assert!(
        repo.next_episode_in_series(&SeriesId("s1".into()), 0, 0, &in_libraries(&["lib1"]))
            .await
            .unwrap()
            .is_none(),
        "up next must not surface an episode the viewer cannot play"
    );

    let collections = repo.list_collections(page(0, 10)).await.unwrap();
    assert_eq!(collections.total, 1);
    let collection = repo.get_collection(&CollectionId("c1".into())).await.unwrap().unwrap();
    assert_eq!(collection.movies, vec![MovieId("m1".into()), MovieId("m2".into())]);
    assert!(repo.get_collection(&CollectionId("no".into())).await.unwrap().is_none());

    let movie_versions =
        repo.list_versions(&TitleId::Movie(MovieId("m1".into())), page(0, 10)).await.unwrap();
    assert_eq!(movie_versions.total, 2);
    assert_eq!(
        movie_versions.items.iter().map(|v| v.id.0.as_str()).collect::<Vec<_>>(),
        ["v1", "v2"]
    );
    let episode_versions =
        repo.list_versions(&TitleId::Episode(EpisodeId("e1".into())), page(0, 10)).await.unwrap();
    assert_eq!(episode_versions.total, 1);
    assert_eq!(episode_versions.items[0].id, VersionId("v3".into()));

    assert_eq!(
        repo.series_versions(&SeriesId("s1".into()))
            .await
            .unwrap()
            .iter()
            .map(|v| v.id.0.as_str())
            .collect::<Vec<_>>(),
        vec!["v3"],
        "every version under a series arrives in one read, not one per episode"
    );
    assert!(repo.series_versions(&SeriesId("nope".into())).await.unwrap().is_empty());

    let lib1_versions =
        repo.list_library_versions(&LibraryId("lib1".into()), page(0, 10)).await.unwrap();
    assert_eq!(lib1_versions.total, 2);
    let lib2_versions =
        repo.list_library_versions(&LibraryId("lib2".into()), page(0, 10)).await.unwrap();
    assert_eq!(lib2_versions.total, 1);
    assert_eq!(lib2_versions.items[0].id, VersionId("v3".into()));

    let all_versions = repo.list_all_versions(page(0, 10)).await.unwrap();
    assert_eq!(all_versions.total, 3);
    assert_eq!(
        all_versions.items.iter().map(|v| v.id.0.as_str()).collect::<Vec<_>>(),
        ["v1", "v2", "v3"]
    );

    let in_library = |name: &str| TitleListFilter {
        libraries: Some(vec![LibraryId(name.into())]),
        ..TitleListFilter::default()
    };
    let movie_ids = |page: &domain::common::Page<Movie>| {
        page.items.iter().map(|m| m.id.0.clone()).collect::<Vec<_>>()
    };
    let series_ids = |page: &domain::common::Page<Series>| {
        page.items.iter().map(|s| s.id.0.clone()).collect::<Vec<_>>()
    };
    assert_eq!(
        movie_ids(&repo.list_movies_filtered(&in_library("lib1"), page(0, 10)).await.unwrap()),
        vec!["m1".to_owned()]
    );
    assert!(
        repo.list_movies_filtered(&in_library("lib2"), page(0, 10)).await.unwrap().items.is_empty()
    );
    assert_eq!(
        series_ids(&repo.list_series_filtered(&in_library("lib2"), page(0, 10)).await.unwrap()),
        vec!["s1".to_owned()]
    );
    assert!(
        repo.list_series_filtered(&in_library("lib1"), page(0, 10)).await.unwrap().items.is_empty()
    );
    assert!(
        repo.list_movies_filtered(&in_library("ghost"), page(0, 10))
            .await
            .unwrap()
            .items
            .is_empty()
    );

    let blocked_by = |code: &str| TitleListFilter {
        blocked_ratings: ContentRating::blocked_by(Some(&ContentRating {
            system: "MPAA".into(),
            code: code.into(),
        })),
        ..TitleListFilter::default()
    };
    assert_eq!(
        movie_ids(&repo.list_movies_filtered(&blocked_by("G"), page(0, 10)).await.unwrap()),
        vec!["m2".to_owned()]
    );
    assert_eq!(
        movie_ids(&repo.list_movies_filtered(&blocked_by("PG-13"), page(0, 10)).await.unwrap()),
        vec!["m1".to_owned(), "m2".to_owned()]
    );
    // A cap is applied to shows and episodes through the series rating, and an unrated series
    // is not something a cap can block.
    assert_eq!(
        series_ids(&repo.list_series_filtered(&blocked_by("G"), page(0, 10)).await.unwrap()),
        vec!["s1".to_owned()]
    );
    assert_eq!(
        repo.visible_episodes(&[EpisodeId("e1".into())], &blocked_by("G")).await.unwrap().len(),
        1
    );

    let sorted = |sort: TitleSort, order: SortOrder| TitleListFilter {
        sort,
        order,
        ..TitleListFilter::default()
    };
    assert_eq!(
        movie_ids(
            &repo.list_movies_filtered(&TitleListFilter::default(), page(0, 10)).await.unwrap()
        ),
        vec!["m1".to_owned(), "m2".to_owned()]
    );
    assert_eq!(
        movie_ids(
            &repo
                .list_movies_filtered(&sorted(TitleSort::Title, SortOrder::Desc), page(0, 10))
                .await
                .unwrap()
        ),
        vec!["m2".to_owned(), "m1".to_owned()]
    );
    assert_eq!(
        movie_ids(
            &repo
                .list_movies_filtered(&sorted(TitleSort::Year, SortOrder::Asc), page(0, 10))
                .await
                .unwrap()
        ),
        vec!["m2".to_owned(), "m1".to_owned()]
    );
    assert_eq!(
        movie_ids(
            &repo
                .list_movies_filtered(&sorted(TitleSort::Year, SortOrder::Desc), page(0, 10))
                .await
                .unwrap()
        ),
        vec!["m1".to_owned(), "m2".to_owned()]
    );
    assert!(
        repo.list_movies_filtered(
            &TitleListFilter { libraries: Some(Vec::new()), ..TitleListFilter::default() },
            page(0, 10)
        )
        .await
        .unwrap()
        .items
        .is_empty()
    );

    let detail = repo.version_detail(&VersionId("v1".into())).await.unwrap().unwrap();
    assert_eq!(detail.version.quality, Quality::Sd);
    assert_eq!(detail.video.len(), 1);
    assert_eq!(detail.video[0].codec, "h264");
    assert_eq!(detail.video[0].width, 1920);
    assert_eq!(detail.video[0].hdr, Some(HdrFormat::Hdr10));
    assert_eq!(detail.audio.len(), 1);
    assert_eq!(detail.audio[0].channels, 6);
    assert_eq!(detail.audio[0].language, Some(LanguageCode("en".into())));
    assert_eq!(detail.subtitles.len(), 1);
    assert_eq!(detail.subtitles[0].format, SubtitleFormat::Srt);
    assert!(detail.subtitles[0].default);
    assert!(!detail.subtitles[0].forced);
    assert_eq!(detail.subtitle_files.len(), 1);
    assert_eq!(detail.subtitle_files[0].id, SubtitleFileId("sf1".into()));
    assert_eq!(detail.subtitle_files[0].source, SubtitleSource::External);
    assert_eq!(detail.subtitle_files[0].format, SubtitleFormat::Srt);
    assert_eq!(detail.chapters.len(), 1);
    assert_eq!(detail.chapters[0].start_ms, 0);
    assert_eq!(detail.markers.intros.len(), 1);
    assert_eq!(detail.markers.credits.len(), 1);
    assert_eq!(detail.markers.credits[0].start_ms, 90_000);
    assert_eq!(detail.trickplay.len(), 1);
    assert_eq!(
        detail.trickplay[0].sheet_paths,
        vec!["/tp/1.jpg".to_owned(), "/tp/2.jpg".to_owned()]
    );

    let plain = repo.version_detail(&VersionId("v2".into())).await.unwrap().unwrap();
    assert!(plain.video.is_empty());
    assert!(plain.audio.is_empty());
    assert!(plain.subtitles.is_empty());
    assert!(plain.chapters.is_empty());
    assert!(plain.markers.intros.is_empty());
    assert!(plain.markers.credits.is_empty());
    assert!(plain.trickplay.is_empty());

    let season = repo.get_season(&SeasonId("se1".into())).await.unwrap().unwrap();
    assert_eq!(season.number, 1);
    assert_eq!(season.title, Some("Season 1".to_owned()));
    assert!(repo.get_season(&SeasonId("no".into())).await.unwrap().is_none());

    repo.upsert_collection(Collection {
        id: CollectionId("c1".into()),
        name: "Saga Remastered".into(),
        overview: None,
        movies: vec![MovieId("m2".into())],
        added_at: ts(999),
        updated_at: ts(20),
        artwork: Vec::new(),
    })
    .await
    .unwrap();
    let updated = repo.get_collection(&CollectionId("c1".into())).await.unwrap().unwrap();
    assert_eq!(updated.name, "Saga Remastered");
    assert_eq!(updated.movies, vec![MovieId("m2".into())]);
    assert_eq!(updated.added_at, ts(11));
    assert_eq!(updated.updated_at, ts(20));

    assert_eq!(
        repo.collections_of_movie(&MovieId("m2".into())).await.unwrap(),
        vec![CollectionId("c1".into())],
        "a movie can be found from the collection side"
    );
    assert!(
        repo.collections_of_movie(&MovieId("m1".into())).await.unwrap().is_empty(),
        "a movie dropped from a collection is no longer a member"
    );

    repo.upsert_collection(Collection {
        id: CollectionId("c2".into()),
        name: "New".into(),
        overview: None,
        movies: vec![MovieId("m2".into())],
        added_at: ts(21),
        updated_at: ts(21),
        artwork: Vec::new(),
    })
    .await
    .unwrap();
    assert_eq!(
        repo.collections_of_movie(&MovieId("m2".into())).await.unwrap(),
        vec![CollectionId("c1".into()), CollectionId("c2".into())],
        "a movie in two collections reports both, in a stable order"
    );
    assert_eq!(repo.list_collections(page(0, 10)).await.unwrap().total, 2);

    repo.upsert_collection(Collection {
        id: CollectionId("c3".into()),
        name: "alpha pack".into(),
        overview: None,
        movies: Vec::new(),
        added_at: ts(22),
        updated_at: ts(22),
        artwork: Vec::new(),
    })
    .await
    .unwrap();
    let listed = repo.list_collections(page(0, 10)).await.unwrap();
    assert_eq!(
        listed.items.iter().map(|c| c.id.0.as_str()).collect::<Vec<_>>(),
        ["c3", "c2", "c1"],
        "collections list by name, case-insensitively, not by id"
    );
    repo.delete_collection(&CollectionId("c3".into())).await.unwrap();

    repo.delete_collection(&CollectionId("c1".into())).await.unwrap();
    assert!(repo.get_collection(&CollectionId("c1".into())).await.unwrap().is_none());
    assert_eq!(repo.list_collections(page(0, 10)).await.unwrap().total, 1);
    repo.delete_collection(&CollectionId("missing".into())).await.unwrap();
    assert_eq!(
        repo.collections_of_movie(&MovieId("m2".into())).await.unwrap(),
        vec![CollectionId("c2".into())],
        "deleting one collection leaves the movie's other memberships alone"
    );

    let poster = ArtworkRef {
        id: ArtworkId("art-poster-m1".into()),
        kind: ArtworkKind::Poster,
        widths: vec![
            ArtworkWidth::new(180, "/art/art-poster-m1/180.jpg"),
            ArtworkWidth::new(480, "/art/art-poster-m1/480.jpg"),
            ArtworkWidth::new(960, "/art/art-poster-m1/960.jpg"),
        ],
    };
    let backdrop = ArtworkRef {
        id: ArtworkId("art-backdrop-m1".into()),
        kind: ArtworkKind::Backdrop,
        widths: vec![
            ArtworkWidth::new(480, "/art/art-backdrop-m1/480.jpg"),
            ArtworkWidth::new(960, "/art/art-backdrop-m1/960.jpg"),
        ],
    };
    repo.set_artwork(
        &ArtworkOwner::Movie(MovieId("m1".into())),
        &[poster.clone(), backdrop.clone()],
    )
    .await
    .unwrap();
    repo.set_artwork(
        &ArtworkOwner::Series(SeriesId("s1".into())),
        &[ArtworkRef {
            id: ArtworkId("art-poster-s1".into()),
            kind: ArtworkKind::Poster,
            widths: vec![ArtworkWidth::new(180, "/art/art-poster-s1/180.jpg")],
        }],
    )
    .await
    .unwrap();

    assert_eq!(
        repo.list_artwork(&ArtworkOwner::Movie(MovieId("m1".into()))).await.unwrap(),
        vec![poster.clone(), backdrop.clone()]
    );
    let hydrated_movie = repo.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
    assert_eq!(hydrated_movie.artwork, vec![poster.clone(), backdrop.clone()]);
    let listed_movies = repo.list_movies(page(0, 10)).await.unwrap();
    assert_eq!(listed_movies.items[0].artwork, vec![poster.clone(), backdrop.clone()]);
    assert!(listed_movies.items[1].artwork.is_empty());

    let hydrated_series = repo.get_series(&SeriesId("s1".into())).await.unwrap().unwrap();
    assert_eq!(hydrated_series.artwork.len(), 1);
    assert_eq!(hydrated_series.artwork[0].kind, ArtworkKind::Poster);
    assert_eq!(hydrated_series.artwork[0].sizes(), vec![180]);
    assert_eq!(
        hydrated_series.artwork[0].widths[0].path, "/art/art-poster-s1/180.jpg",
        "the stored path is what the orphan sweep protects"
    );

    repo.set_artwork(&ArtworkOwner::Movie(MovieId("m1".into())), std::slice::from_ref(&backdrop))
        .await
        .unwrap();
    let replaced = repo.get_movie(&MovieId("m1".into())).await.unwrap().unwrap();
    assert_eq!(replaced.artwork, vec![backdrop]);

    assert!(
        repo.list_artwork(&ArtworkOwner::Movie(MovieId("m2".into()))).await.unwrap().is_empty()
    );

    let artwork_ids = repo.all_artwork_ids().await.unwrap();
    assert!(artwork_ids.contains(&ArtworkId("art-backdrop-m1".into())));
    assert!(artwork_ids.contains(&ArtworkId("art-poster-s1".into())));
    assert!(
        !artwork_ids.contains(&ArtworkId("art-poster-m1".into())),
        "artwork replaced on m1 is no longer owned by anything"
    );
    assert_eq!(
        repo.live_artwork_ids(&[
            "art-backdrop-m1".to_owned(),
            "art-poster-m1".to_owned(),
            "never-existed".to_owned(),
        ])
        .await
        .unwrap(),
        std::collections::HashSet::from(["art-backdrop-m1".to_owned()]),
        "the sweep asks about owners on disk and only live ones come back"
    );
    assert!(repo.live_artwork_ids(&[]).await.unwrap().is_empty());
    assert_eq!(
        repo.live_artwork_paths(&["art-poster-s1".to_owned()]).await.unwrap(),
        std::collections::HashSet::from(["/art/art-poster-s1/180.jpg".to_owned()]),
        "the per-file sweep asks which files under a live owner are still referenced"
    );
    assert!(repo.live_artwork_paths(&[]).await.unwrap().is_empty());

    repo.upsert_movie(Movie {
        id: MovieId("um1".into()),
        title: "Ingested".into(),
        sort_title: "ingested".into(),
        year: Some(2001),
        overview: None,
        runtime_minutes: None,
        content_rating: None,
        manually_edited: false,
        added_at: ts(7),
        updated_at: ts(7),
        artwork: Vec::new(),
    })
    .await
    .unwrap();
    repo.upsert_movie(Movie {
        id: MovieId("um1".into()),
        title: "Ingested Remux".into(),
        sort_title: "ingested remux".into(),
        year: Some(2001),
        overview: None,
        runtime_minutes: None,
        content_rating: None,
        manually_edited: false,
        added_at: ts(999),
        updated_at: ts(8),
        artwork: Vec::new(),
    })
    .await
    .unwrap();
    let ingested = repo.get_movie(&MovieId("um1".into())).await.unwrap().unwrap();
    assert_eq!(ingested.title, "Ingested Remux");
    assert_eq!(ingested.sort_title, "ingested remux");
    assert_eq!(ingested.added_at, ts(7));
    assert_eq!(ingested.updated_at, ts(8));

    repo.upsert_movie(Movie {
        id: MovieId("ua1".into()),
        title: "The Aardvark".into(),
        sort_title: "aardvark, the".into(),
        year: Some(2003),
        overview: None,
        runtime_minutes: None,
        content_rating: None,
        manually_edited: false,
        added_at: ts(9),
        updated_at: ts(9),
        artwork: Vec::new(),
    })
    .await
    .unwrap();
    assert_eq!(
        movie_ids(
            &repo
                .list_movies_filtered(
                    &TitleListFilter { sort: TitleSort::Title, ..TitleListFilter::default() },
                    page(0, 10)
                )
                .await
                .unwrap()
        ),
        vec!["ua1".to_owned(), "m1".to_owned(), "m2".to_owned(), "um1".to_owned()]
    );

    repo.upsert_series(Series {
        id: SeriesId("us1".into()),
        title: "Ingested Show".into(),
        sort_title: "ingested show".into(),
        year: Some(2002),
        overview: None,
        content_rating: None,
        manually_edited: false,
        added_at: ts(8),
        updated_at: ts(8),
        artwork: Vec::new(),
    })
    .await
    .unwrap();
    repo.upsert_season(Season {
        id: SeasonId("use1".into()),
        series: SeriesId("us1".into()),
        number: 1,
        title: Some("Season 1".into()),
        overview: None,
        added_at: ts(23),
        updated_at: ts(23),
        artwork: Vec::new(),
    })
    .await
    .unwrap();
    repo.upsert_episode(Episode {
        id: EpisodeId("ue1".into()),
        season: SeasonId("use1".into()),
        number: 1,
        title: "Episode 1".into(),
        overview: None,
        runtime_minutes: None,
        air_date: None,
        manually_edited: false,
        added_at: ts(9),
        updated_at: ts(9),
        artwork: Vec::new(),
    })
    .await
    .unwrap();
    repo.upsert_version(Version {
        id: VersionId("uv1".into()),
        title: TitleId::Episode(EpisodeId("ue1".into())),
        library: LibraryId("lib1".into()),
        quality: Quality::Hd,
        container: "mkv".into(),
        path: "/media/uv1.mkv".into(),
        size_bytes: 4_000_000,
        duration_ms: 1_500_000,
        available: true,
        added_at: ts(24),
        updated_at: ts(24),
    })
    .await
    .unwrap();

    assert!(repo.get_series(&SeriesId("us1".into())).await.unwrap().is_some());
    assert_eq!(repo.list_seasons(&SeriesId("us1".into())).await.unwrap().len(), 1);
    assert!(repo.get_episode(&EpisodeId("ue1".into())).await.unwrap().is_some());
    assert_eq!(
        repo.list_versions(&TitleId::Episode(EpisodeId("ue1".into())), page(0, 10))
            .await
            .unwrap()
            .total,
        1
    );

    repo.upsert_version(Version {
        id: VersionId("uv1".into()),
        title: TitleId::Episode(EpisodeId("ue1".into())),
        library: LibraryId("lib1".into()),
        quality: Quality::Uhd,
        container: "mkv".into(),
        path: "/media/uv1.mkv".into(),
        size_bytes: 4_000_000,
        duration_ms: 1_500_000,
        available: true,
        added_at: ts(999),
        updated_at: ts(25),
    })
    .await
    .unwrap();
    let reupserted = repo.version_detail(&VersionId("uv1".into())).await.unwrap().unwrap().version;
    assert_eq!(reupserted.quality, Quality::Uhd);
    assert_eq!(reupserted.added_at, ts(24));
    assert_eq!(reupserted.updated_at, ts(25));

    let uv1 = VersionId("uv1".into());
    repo.set_version_tracks(
        &uv1,
        &[VideoTrack {
            index: 0,
            codec: "hevc".into(),
            width: 3840,
            height: 2160,
            bit_depth: 10,
            hdr: Some(HdrFormat::DolbyVision),
            frame_rate: 24.0,
            bitrate: Some(20_000_000),
        }],
        &[AudioTrack {
            index: 1,
            codec: "eac3".into(),
            channels: 8,
            language: Some(LanguageCode("de".into())),
            bitrate: Some(768_000),
        }],
        &[
            EmbeddedSubtitleTrack {
                index: 2,
                language: Some(LanguageCode("en".into())),
                format: SubtitleFormat::Ass,
                forced: true,
                default: false,
            },
            EmbeddedSubtitleTrack {
                index: 3,
                language: Some(LanguageCode("de".into())),
                format: SubtitleFormat::Srt,
                forced: false,
                default: true,
            },
        ],
        &[Chapter { title: "Cold Open".into(), start_ms: 0 }],
    )
    .await
    .unwrap();
    repo.set_trickplay(
        &uv1,
        &[TrickplayAsset {
            version: uv1.clone(),
            interval_ms: 5_000,
            columns: 8,
            rows: 8,
            tile_width: 320,
            tile_height: 180,
            sheet_paths: vec!["/tp/uv1/sheet-001.jpg".into()],
        }],
    )
    .await
    .unwrap();
    repo.set_subtitle_files(
        &uv1,
        &[
            SubtitleFile {
                id: SubtitleFileId("uv1-sf1".into()),
                version: uv1.clone(),
                language: Some(LanguageCode("de".into())),
                format: SubtitleFormat::Vtt,
                source: SubtitleSource::OpenSubtitles,
                path: "/subs/uv1.de.vtt".into(),
                translated_from: None,
                label: Some("The.Matrix.1999.BluRay".into()),
                pinned: true,
            },
            SubtitleFile {
                id: SubtitleFileId("uv1-sf2".into()),
                version: uv1.clone(),
                language: Some(LanguageCode("en".into())),
                format: SubtitleFormat::Vtt,
                source: SubtitleSource::Generated,
                path: "/subs/uv1.en.generated.vtt".into(),
                translated_from: None,
                label: None,
                pinned: false,
            },
            SubtitleFile {
                id: SubtitleFileId("uv1-sf3".into()),
                version: uv1.clone(),
                language: Some(LanguageCode("fr".into())),
                format: SubtitleFormat::Vtt,
                source: SubtitleSource::MachineTranslated,
                path: "/subs/uv1.fr.machine.vtt".into(),
                translated_from: Some(SubtitleFileId("uv1-sf1".into())),
                label: None,
                pinned: false,
            },
        ],
    )
    .await
    .unwrap();

    let uv1_detail = repo.version_detail(&uv1).await.unwrap().unwrap();
    assert_eq!(uv1_detail.video.len(), 1);
    assert_eq!(uv1_detail.video[0].codec, "hevc");
    assert_eq!(uv1_detail.video[0].hdr, Some(HdrFormat::DolbyVision));
    assert_eq!(uv1_detail.audio.len(), 1);
    assert_eq!(uv1_detail.audio[0].channels, 8);
    assert_eq!(uv1_detail.subtitles.len(), 2);
    assert!(uv1_detail.subtitles[0].forced);
    assert_eq!(uv1_detail.chapters.len(), 1);
    assert_eq!(uv1_detail.trickplay.len(), 1);
    assert_eq!(uv1_detail.trickplay[0].columns, 8);
    assert_eq!(uv1_detail.trickplay[0].rows, 8);
    assert_eq!(uv1_detail.trickplay[0].sheet_paths.len(), 1);
    assert_eq!(uv1_detail.subtitle_files.len(), 3);
    assert!(uv1_detail.subtitle_files.iter().any(|f| f.source == SubtitleSource::OpenSubtitles));
    assert!(uv1_detail.subtitle_files.iter().any(|f| f.source == SubtitleSource::Generated));
    assert!(
        uv1_detail.subtitle_files.iter().any(|f| f.source == SubtitleSource::MachineTranslated)
    );
    assert_eq!(uv1_detail.subtitle_files[0].format, SubtitleFormat::Vtt);
    let pinned = uv1_detail
        .subtitle_files
        .iter()
        .find(|f| f.source == SubtitleSource::OpenSubtitles)
        .unwrap();
    assert_eq!(pinned.label.as_deref(), Some("The.Matrix.1999.BluRay"));
    assert!(pinned.pinned);
    let generated =
        uv1_detail.subtitle_files.iter().find(|f| f.source == SubtitleSource::Generated).unwrap();
    assert_eq!(generated.label, None);
    assert!(!generated.pinned);

    repo.set_version_tracks(&uv1, &[], &[], &[], &[]).await.unwrap();
    repo.set_trickplay(&uv1, &[]).await.unwrap();
    repo.set_subtitle_files(&uv1, &[]).await.unwrap();
    let uv1_cleared = repo.version_detail(&uv1).await.unwrap().unwrap();
    assert!(uv1_cleared.video.is_empty());
    assert!(uv1_cleared.audio.is_empty());
    assert!(uv1_cleared.subtitles.is_empty());
    assert!(uv1_cleared.subtitle_files.is_empty());
    assert!(uv1_cleared.chapters.is_empty());
    assert!(uv1_cleared.trickplay.is_empty());

    // adding appends without reading or rewriting the rows already there, so two
    // concurrent writers cannot erase each other
    let added = |id: &str, language: &str, path: &str, pinned: bool| SubtitleFile {
        id: SubtitleFileId(id.into()),
        version: uv1.clone(),
        language: Some(LanguageCode(language.into())),
        format: SubtitleFormat::Srt,
        source: SubtitleSource::OpenSubtitles,
        path: path.into(),
        translated_from: None,
        label: None,
        pinned,
    };
    repo.add_subtitle_file(&uv1, &added("uv1-add1", "pt", "/subs/uv1.pt.srt", true)).await.unwrap();
    repo.add_subtitle_file(&uv1, &added("uv1-add2", "fr", "/subs/uv1.fr.srt", true)).await.unwrap();
    repo.add_subtitle_file(&uv1, &added("uv1-add3", "es", "/subs/uv1.es.srt", true)).await.unwrap();
    let uv1_added = repo.version_detail(&uv1).await.unwrap().unwrap();
    assert_eq!(
        uv1_added.subtitle_files.iter().map(|f| f.id.0.as_str()).collect::<Vec<_>>(),
        vec!["uv1-add1", "uv1-add2", "uv1-add3"]
    );

    // re-adding the same id updates that row in place instead of failing on the primary key
    repo.add_subtitle_file(&uv1, &added("uv1-add2", "fr", "/subs/uv1.fr.v2.srt", false))
        .await
        .unwrap();
    let uv1_readded = repo.version_detail(&uv1).await.unwrap().unwrap();
    assert_eq!(uv1_readded.subtitle_files.len(), 3);
    let replaced = uv1_readded.subtitle_files.iter().find(|f| f.id.0 == "uv1-add2").unwrap();
    assert_eq!(replaced.path, "/subs/uv1.fr.v2.srt");
    assert!(!replaced.pinned);

    repo.set_subtitle_files(&uv1, &[]).await.unwrap();

    let m1_detail = repo.movie_detail(&MovieId("m1".into())).await.unwrap().unwrap();
    assert_eq!(m1_detail.movie.title, "Alpha");
    assert_eq!(
        m1_detail.genres.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(),
        ["Action", "Drama"]
    );
    assert_eq!(m1_detail.credits.len(), 2);
    assert_eq!(m1_detail.credits[0].person.name, "Ada");
    assert_eq!(m1_detail.credits[0].role, CreditRole::Actor);
    assert_eq!(m1_detail.credits[0].character, Some("Hero".to_owned()));
    assert_eq!(m1_detail.credits[0].order, 0);
    assert_eq!(m1_detail.credits[1].person.name, "Bob");
    assert_eq!(m1_detail.credits[1].role, CreditRole::Director);
    assert_eq!(m1_detail.credits[1].character, None);
    assert_eq!(
        m1_detail.studios.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["Acme Studios"]
    );
    assert_eq!(m1_detail.ratings, vec![Rating { source: "tmdb".into(), value: 8.5 }]);
    assert_eq!(
        m1_detail
            .external_ids
            .iter()
            .map(|e| (e.source.as_str(), e.value.as_str()))
            .collect::<Vec<_>>(),
        [("tmdb", "603"), ("imdb", "tt0133093")]
    );
    assert_eq!(m1_detail.extras.len(), 1);
    assert_eq!(m1_detail.extras[0].kind, ExtraKind::Trailer);
    assert_eq!(m1_detail.extras[0].title, "Teaser");
    assert!(repo.movie_detail(&MovieId("missing".into())).await.unwrap().is_none());

    let s1_detail = repo.series_detail(&SeriesId("s1".into())).await.unwrap().unwrap();
    assert_eq!(s1_detail.series.title, "Gamma");
    assert_eq!(s1_detail.genres.len(), 1);
    assert_eq!(s1_detail.credits.len(), 1);
    assert_eq!(s1_detail.credits[0].person.name, "Ada");
    assert_eq!(s1_detail.credits[0].character, Some("Lead".to_owned()));
    assert_eq!(s1_detail.ratings[0].value, 9.0);
    assert_eq!(s1_detail.external_ids[0].source, "tvdb");
    assert!(repo.series_detail(&SeriesId("missing".into())).await.unwrap().is_none());

    assert_eq!(repo.get_person(&PersonId("p1".into())).await.unwrap().unwrap().name, "Ada");
    assert!(repo.get_person(&PersonId("missing".into())).await.unwrap().is_none());

    repo.upsert_person(Person {
        id: PersonId("p1".into()),
        name: "Ada Lovelace".into(),
        biography: Some("A mathematician.".into()),
        birthday: Some("1815-12-10".into()),
        deathday: Some("1852-11-27".into()),
        place_of_birth: Some("London".into()),
        also_known_as: vec!["Augusta Ada King".into()],
        external_id: Some("person/1".into()),
        ..Person::default()
    })
    .await
    .unwrap();
    let enriched = repo.get_person(&PersonId("p1".into())).await.unwrap().unwrap();
    assert_eq!(enriched.name, "Ada Lovelace");
    assert_eq!(enriched.biography.as_deref(), Some("A mathematician."));
    assert_eq!(enriched.birthday.as_deref(), Some("1815-12-10"));
    assert_eq!(enriched.deathday.as_deref(), Some("1852-11-27"));
    assert_eq!(enriched.place_of_birth.as_deref(), Some("London"));
    assert_eq!(enriched.also_known_as, vec!["Augusta Ada King".to_owned()]);
    assert_eq!(enriched.external_id.as_deref(), Some("person/1"));

    repo.upsert_person(Person {
        id: PersonId("p1".into()),
        name: "Ada".into(),
        ..Person::default()
    })
    .await
    .unwrap();
    let preserved = repo.get_person(&PersonId("p1".into())).await.unwrap().unwrap();
    assert_eq!(preserved.name, "Ada");
    assert_eq!(preserved.biography.as_deref(), Some("A mathematician."));
    assert_eq!(preserved.also_known_as, vec!["Augusta Ada King".to_owned()]);

    let cascade_series = SeriesId("cascade-series".into());
    let cascade_season = SeasonId("cascade-s1".into());
    for n in 1..=3u16 {
        repo.upsert_series(Series {
            id: cascade_series.clone(),
            title: "Cascade".into(),
            sort_title: "cascade".into(),
            year: None,
            overview: None,
            content_rating: None,
            manually_edited: false,
            added_at: ts(0),
            updated_at: ts(0),
            artwork: Vec::new(),
        })
        .await
        .unwrap();
        repo.upsert_season(Season {
            id: cascade_season.clone(),
            series: cascade_series.clone(),
            number: 1,
            title: Some("Season 1".into()),
            overview: None,
            added_at: ts(0),
            updated_at: ts(0),
            artwork: Vec::new(),
        })
        .await
        .unwrap();
        repo.upsert_episode(Episode {
            id: EpisodeId(format!("cascade-e{n}")),
            season: cascade_season.clone(),
            number: n,
            title: format!("Episode {n}"),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: ts(0),
            updated_at: ts(0),
            artwork: Vec::new(),
        })
        .await
        .unwrap();
    }
    assert_eq!(repo.list_seasons(&cascade_series).await.unwrap().len(), 1);
    assert_eq!(
        repo.list_episodes(&cascade_season).await.unwrap().len(),
        3,
        "re-upserting a series/season must not cascade-delete its episodes"
    );

    let p1_film = repo.filmography(&PersonId("p1".into())).await.unwrap();
    assert_eq!(p1_film.len(), 2);
    assert_eq!(p1_film[0].title, TitleRef::Movie(MovieId("m1".into())));
    assert_eq!(p1_film[0].role, CreditRole::Actor);
    assert_eq!(p1_film[1].title, TitleRef::Series(SeriesId("s1".into())));
    let p2_film = repo.filmography(&PersonId("p2".into())).await.unwrap();
    assert_eq!(p2_film.len(), 1);
    assert_eq!(p2_film[0].role, CreditRole::Director);
    assert!(repo.filmography(&PersonId("missing".into())).await.unwrap().is_empty());

    let genre_names = async |kind| {
        repo.list_genres(kind).await.unwrap().iter().map(|g| g.name.clone()).collect::<Vec<_>>()
    };
    assert_eq!(genre_names(None).await, ["Action", "Drama"]);
    assert_eq!(genre_names(Some(TitleKind::Movie)).await, ["Action", "Drama"]);
    assert_eq!(genre_names(Some(TitleKind::Series)).await, ["Action"]);

    let by_genre =
        |name: &str| TitleListFilter { genres: vec![name.into()], ..TitleListFilter::default() };
    let by_action = repo.list_movies_filtered(&by_genre("Action"), page(0, 10)).await.unwrap();
    assert_eq!(by_action.items.iter().map(|m| m.id.0.as_str()).collect::<Vec<_>>(), ["m1"]);
    assert_eq!(repo.list_movies_filtered(&by_genre("Drama"), page(0, 10)).await.unwrap().total, 1);
    assert!(
        repo.list_movies_filtered(&by_genre("missing"), page(0, 10))
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(
        repo.list_series_filtered(&by_genre("Action"), page(0, 10))
            .await
            .unwrap()
            .items
            .iter()
            .map(|s| s.id.0.as_str())
            .collect::<Vec<_>>(),
        ["s1"]
    );
    assert!(
        repo.list_series_filtered(&by_genre("Drama"), page(0, 10)).await.unwrap().items.is_empty()
    );

    repo.set_title_enrichment(&TitleRef::Movie(MovieId("m1".into())), &TitleEnrichment::default())
        .await
        .unwrap();
    let cleared = repo.movie_detail(&MovieId("m1".into())).await.unwrap().unwrap();
    assert!(cleared.genres.is_empty());
    assert!(cleared.credits.is_empty());
    assert!(cleared.studios.is_empty());
    assert!(cleared.ratings.is_empty());
    assert!(cleared.external_ids.is_empty());
    assert!(cleared.extras.is_empty());

    let p1_after = repo.filmography(&PersonId("p1".into())).await.unwrap();
    assert_eq!(p1_after.len(), 1);
    assert_eq!(p1_after[0].title, TitleRef::Series(SeriesId("s1".into())));
    assert!(repo.filmography(&PersonId("p2".into())).await.unwrap().is_empty());

    let available = async |repo: &R, id: &str| -> bool {
        repo.version_detail(&VersionId(id.into())).await.unwrap().unwrap().version.available
    };

    repo.reconcile_library_versions(
        &LibraryId("lib1".into()),
        &["/media/v1.mkv".to_owned(), "/media/uv1.mkv".to_owned()],
    )
    .await
    .unwrap();
    assert_eq!(
        repo.list_library_versions(&LibraryId("lib1".into()), page(0, 10)).await.unwrap().total,
        3
    );
    assert!(available(&repo, "v1").await);
    assert!(available(&repo, "uv1").await);
    assert!(!available(&repo, "v2").await);
    assert!(available(&repo, "v3").await);

    repo.reconcile_library_versions(
        &LibraryId("lib1".into()),
        &["/media/v1.mkv".to_owned(), "/media/v2.mp4".to_owned(), "/media/uv1.mkv".to_owned()],
    )
    .await
    .unwrap();
    assert!(available(&repo, "v2").await);

    repo.reconcile_library_versions(&LibraryId("lib1".into()), &[]).await.unwrap();
    assert!(!available(&repo, "v1").await);
    assert!(!available(&repo, "v2").await);
    assert!(!available(&repo, "uv1").await);
    assert!(available(&repo, "v3").await);

    version_rows_and_deletes(&repo).await;
    manually_edited_survives_a_round_trip(&repo).await;
}

async fn manually_edited_survives_a_round_trip<R: CatalogRepository>(repo: &R) {
    let movie_id = MovieId("m1".into());
    let series_id = SeriesId("s1".into());
    let episode_id = EpisodeId("e1".into());

    let movie = repo.get_movie(&movie_id).await.unwrap().unwrap();
    assert!(!movie.manually_edited);
    let series = repo.get_series(&series_id).await.unwrap().unwrap();
    assert!(!series.manually_edited);
    let episode = repo.get_episode(&episode_id).await.unwrap().unwrap();
    assert!(!episode.manually_edited);

    repo.upsert_movie(Movie { manually_edited: true, ..movie.clone() }).await.unwrap();
    repo.upsert_series(Series { manually_edited: true, ..series.clone() }).await.unwrap();
    repo.upsert_episode(Episode { manually_edited: true, ..episode.clone() }).await.unwrap();

    let edited_movie = repo.get_movie(&movie_id).await.unwrap().unwrap();
    assert!(edited_movie.manually_edited);
    assert_eq!(edited_movie.title, movie.title);
    assert_eq!(edited_movie.sort_title, movie.sort_title);
    assert!(repo.get_series(&series_id).await.unwrap().unwrap().manually_edited);
    assert!(repo.get_episode(&episode_id).await.unwrap().unwrap().manually_edited);

    repo.upsert_movie(movie).await.unwrap();
    repo.upsert_series(series).await.unwrap();
    repo.upsert_episode(episode).await.unwrap();

    assert!(!repo.get_movie(&movie_id).await.unwrap().unwrap().manually_edited);
    assert!(!repo.get_series(&series_id).await.unwrap().unwrap().manually_edited);
    assert!(!repo.get_episode(&episode_id).await.unwrap().unwrap().manually_edited);
}

async fn version_rows_and_deletes<R: CatalogRepository>(repo: &R) {
    let uv1 = VersionId("uv1".into());
    assert!(repo.get_version(&VersionId("nope".into())).await.unwrap().is_none());
    let stored = repo.get_version(&uv1).await.unwrap().unwrap();
    assert_eq!(stored.quality, Quality::Uhd);
    assert_eq!(stored.size_bytes, 4_000_000);
    assert_eq!(stored.duration_ms, 1_500_000);
    assert_eq!(stored.path, "/media/uv1.mkv");

    repo.set_version_tracks(
        &uv1,
        &[VideoTrack {
            index: 0,
            codec: "hevc".into(),
            width: 3840,
            height: 2160,
            bit_depth: 10,
            hdr: None,
            frame_rate: 24.0,
            bitrate: None,
        }],
        &[],
        &[],
        &[Chapter { title: "Cold Open".into(), start_ms: 0 }],
    )
    .await
    .unwrap();
    repo.set_subtitle_files(
        &uv1,
        &[SubtitleFile {
            id: SubtitleFileId("uv1-doomed".into()),
            version: uv1.clone(),
            language: Some(LanguageCode("fr".into())),
            format: SubtitleFormat::Vtt,
            source: SubtitleSource::MachineTranslated,
            path: "/subs/uv1.fr.machine.vtt".into(),
            translated_from: None,
            label: None,
            pinned: false,
        }],
    )
    .await
    .unwrap();
    repo.set_trickplay(
        &uv1,
        &[TrickplayAsset {
            version: uv1.clone(),
            interval_ms: 5_000,
            columns: 8,
            rows: 8,
            tile_width: 320,
            tile_height: 180,
            sheet_paths: vec!["/tp/uv1/sheet-001.jpg".into()],
        }],
    )
    .await
    .unwrap();
    let before = repo.version_detail(&uv1).await.unwrap().unwrap();
    assert_eq!(before.video.len(), 1);
    assert_eq!(before.chapters.len(), 1);
    assert_eq!(before.subtitle_files.len(), 1);
    assert_eq!(before.trickplay.len(), 1);

    assert!(repo.all_version_ids().await.unwrap().contains(&uv1));
    assert_eq!(
        repo.live_version_ids(&[uv1.0.clone(), "ghost".to_owned()]).await.unwrap(),
        std::collections::HashSet::from([uv1.0.clone()]),
        "only ids that still exist come back, and nothing else is loaded"
    );
    assert!(repo.live_version_ids(&[]).await.unwrap().is_empty());
    assert_eq!(
        repo.live_subtitle_paths(&[uv1.0.clone(), "ghost".to_owned()]).await.unwrap(),
        std::collections::HashSet::from(["/subs/uv1.fr.machine.vtt".to_owned()]),
        "a subtitle file the catalog no longer lists is what the per-file sweep collects"
    );
    assert!(repo.live_subtitle_paths(&[]).await.unwrap().is_empty());
    assert_eq!(
        repo.live_trickplay_paths(&[uv1.0.clone(), "ghost".to_owned()]).await.unwrap(),
        std::collections::HashSet::from(["/tp/uv1/sheet-001.jpg".to_owned()]),
        "a regenerate with fewer sheets leaves the tail behind unless this is exact"
    );
    assert!(repo.live_trickplay_paths(&[]).await.unwrap().is_empty());

    repo.delete_version(&uv1).await.unwrap();
    assert!(repo.get_version(&uv1).await.unwrap().is_none());
    assert!(repo.version_detail(&uv1).await.unwrap().is_none());
    let live = repo.all_version_ids().await.unwrap();
    assert!(!live.contains(&uv1), "a deleted version stops counting as live");
    assert!(live.contains(&VersionId("v1".into())));
    assert!(repo.get_version(&VersionId("v1".into())).await.unwrap().is_some());
    repo.delete_version(&uv1).await.unwrap();

    repo.upsert_version(Version {
        id: uv1.clone(),
        title: TitleId::Episode(EpisodeId("ue1".into())),
        library: LibraryId("lib1".into()),
        quality: Quality::Uhd,
        container: "mkv".into(),
        path: "/media/uv1.mkv".into(),
        size_bytes: 4_000_000,
        duration_ms: 1_500_000,
        available: true,
        added_at: ts(26),
        updated_at: ts(26),
    })
    .await
    .unwrap();
    let reborn = repo.version_detail(&uv1).await.unwrap().unwrap();
    assert!(reborn.video.is_empty());
    assert!(reborn.chapters.is_empty());
    assert!(reborn.subtitle_files.is_empty());
    assert!(reborn.trickplay.is_empty());

    let us1 = SeriesId("us1".into());
    let use1 = SeasonId("use1".into());
    let ue1 = EpisodeId("ue1".into());

    let counted = repo.series_detail(&us1).await.unwrap().unwrap();
    assert_eq!(counted.episodes_total, 1);
    assert_eq!(
        counted.episodes_with_available_version, 1,
        "an episode whose version is available counts as playable"
    );

    assert!(!repo.delete_series(&us1).await.unwrap(), "a series that still has seasons is refused");
    assert!(
        !repo.delete_season(&use1).await.unwrap(),
        "a season that still has episodes is refused"
    );
    assert!(
        !repo.delete_episode(&ue1).await.unwrap(),
        "an episode that still has versions is refused"
    );
    assert!(
        !repo.delete_movie(&MovieId("m1".into())).await.unwrap(),
        "a movie that still has versions is refused"
    );
    assert!(repo.get_series(&us1).await.unwrap().is_some(), "a refused delete writes nothing");

    repo.delete_version(&uv1).await.unwrap();
    let stranded = repo.series_detail(&us1).await.unwrap().unwrap();
    assert_eq!(stranded.episodes_total, 1);
    assert_eq!(
        stranded.episodes_with_available_version, 0,
        "an episode left without a version stops counting as playable"
    );

    assert!(repo.delete_episode(&ue1).await.unwrap());
    assert!(repo.get_episode(&ue1).await.unwrap().is_none());
    assert!(repo.delete_season(&use1).await.unwrap());
    assert!(repo.get_season(&use1).await.unwrap().is_none());
    assert!(repo.delete_series(&us1).await.unwrap());
    assert!(repo.get_series(&us1).await.unwrap().is_none());

    let orphan = MovieId("um9".into());
    repo.upsert_movie(Movie {
        id: orphan.clone(),
        title: "No Files".into(),
        sort_title: "no files".into(),
        year: None,
        overview: None,
        runtime_minutes: None,
        content_rating: None,
        manually_edited: false,
        added_at: ts(27),
        updated_at: ts(27),
        artwork: Vec::new(),
    })
    .await
    .unwrap();
    repo.set_artwork(
        &ArtworkOwner::Movie(orphan.clone()),
        &[ArtworkRef {
            id: ArtworkId("orphan-art".into()),
            kind: ArtworkKind::Poster,
            widths: Vec::new(),
        }],
    )
    .await
    .unwrap();
    assert!(repo.delete_movie(&orphan).await.unwrap());
    assert!(repo.get_movie(&orphan).await.unwrap().is_none());

    repo.upsert_movie(Movie {
        id: orphan.clone(),
        title: "No Files".into(),
        sort_title: "no files".into(),
        year: None,
        overview: None,
        runtime_minutes: None,
        content_rating: None,
        manually_edited: false,
        added_at: ts(28),
        updated_at: ts(28),
        artwork: Vec::new(),
    })
    .await
    .unwrap();
    assert!(
        repo.get_movie(&orphan).await.unwrap().unwrap().artwork.is_empty(),
        "a deleted movie takes its artwork rows with it rather than leaving them for a reused id"
    );
}
