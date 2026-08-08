use domain::catalog::{
    ArtworkId, ArtworkOwner, ArtworkRef, Collection, CollectionId, Episode, EpisodeId, Movie,
    MovieId, Season, SeasonId, Series, SeriesId, SortOrder, TitleId, TitleListFilter, TitleRef,
    TitleSort, Version, VersionDetail, VersionId,
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
            edition: Some("Director's Cut".into()),
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
        }],
        chapters: vec![Chapter {
            title: "Chapter 1".into(),
            start_ms: 0,
        }],
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
                year: Some(2020),
                overview: Some("first".into()),
                runtime_minutes: Some(100),
                content_rating: Some(ContentRating {
                    system: "MPAA".into(),
                    code: "PG-13".into(),
                }),
                added_at: ts(1),
                updated_at: ts(1),
                artwork: Vec::new(),
            },
            Movie {
                id: MovieId("m2".into()),
                title: "Beta".into(),
                year: None,
                overview: None,
                runtime_minutes: None,
                content_rating: None,
                added_at: ts(2),
                updated_at: ts(2),
                artwork: Vec::new(),
            },
        ],
        series: vec![Series {
            id: SeriesId("s1".into()),
            title: "Gamma".into(),
            year: Some(2019),
            overview: None,
            content_rating: None,
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
                edition: None,
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
                edition: None,
                available: true,
                added_at: ts(13),
                updated_at: ts(13),
            },
        ],
        detail,
        people: vec![
            Person {
                id: PersonId("p1".into()),
                name: "Ada".into(),
                ..Person::default()
            },
            Person {
                id: PersonId("p2".into()),
                name: "Bob".into(),
                ..Person::default()
            },
        ],
        enrichment: vec![
            (
                TitleRef::Movie(MovieId("m1".into())),
                TitleEnrichment {
                    genres: vec![
                        Genre {
                            id: GenreId("g-action".into()),
                            name: "Action".into(),
                        },
                        Genre {
                            id: GenreId("g-drama".into()),
                            name: "Drama".into(),
                        },
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
                    ratings: vec![Rating {
                        source: "tmdb".into(),
                        value: 8.5,
                    }],
                    external_ids: vec![
                        ExternalId {
                            source: "tmdb".into(),
                            value: "603".into(),
                        },
                        ExternalId {
                            source: "imdb".into(),
                            value: "tt0133093".into(),
                        },
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
                    genres: vec![Genre {
                        id: GenreId("g-action".into()),
                        name: "Action".into(),
                    }],
                    credits: vec![Credit {
                        person: PersonId("p1".into()),
                        title: TitleRef::Series(SeriesId("s1".into())),
                        role: CreditRole::Actor,
                        character: Some("Lead".into()),
                        order: 0,
                    }],
                    ratings: vec![Rating {
                        source: "tmdb".into(),
                        value: 9.0,
                    }],
                    external_ids: vec![ExternalId {
                        source: "tvdb".into(),
                        value: "81189".into(),
                    }],
                    ..TitleEnrichment::default()
                },
            ),
        ],
    }
}

pub async fn catalog_repository_contract<R: CatalogRepository>(repo: R, seed: impl AsyncFn(&R)) {
    assert!(
        repo.get_movie(&MovieId("nope".into()))
            .await
            .unwrap()
            .is_none()
    );
    let empty = repo.list_movies(page(0, 10)).await.unwrap();
    assert_eq!(empty.total, 0);
    assert!(empty.items.is_empty());
    assert!(
        repo.version_detail(&VersionId("nope".into()))
            .await
            .unwrap()
            .is_none()
    );

    seed(&repo).await;

    let movies = repo.list_movies(page(0, 10)).await.unwrap();
    assert_eq!(movies.total, 2);
    assert_eq!(
        movies
            .items
            .iter()
            .map(|m| m.id.0.as_str())
            .collect::<Vec<_>>(),
        ["m1", "m2"]
    );
    assert_eq!(movies.items[0].title, "Alpha");
    assert_eq!(movies.items[0].year, Some(2020));
    assert_eq!(
        movies.items[0].content_rating,
        Some(ContentRating {
            system: "MPAA".into(),
            code: "PG-13".into(),
        })
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

    assert_eq!(
        repo.get_movie(&MovieId("m1".into()))
            .await
            .unwrap()
            .unwrap()
            .title,
        "Alpha"
    );

    let series = repo.list_series(page(0, 10)).await.unwrap();
    assert_eq!(series.total, 1);
    assert_eq!(series.items[0].id, SeriesId("s1".into()));
    assert!(
        repo.get_series(&SeriesId("s1".into()))
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        repo.get_series(&SeriesId("no".into()))
            .await
            .unwrap()
            .is_none()
    );

    let seasons = repo.list_seasons(&SeriesId("s1".into())).await.unwrap();
    assert_eq!(seasons.len(), 1);
    assert_eq!(seasons[0].number, 1);

    let episodes = repo.list_episodes(&SeasonId("se1".into())).await.unwrap();
    assert_eq!(
        episodes.iter().map(|e| e.id.0.as_str()).collect::<Vec<_>>(),
        ["e1", "e2"]
    );
    assert_eq!(episodes[0].air_date, Some(ts(4)));
    assert_eq!(episodes[1].air_date, None);
    assert!(
        repo.get_episode(&EpisodeId("e1".into()))
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        repo.get_episode(&EpisodeId("no".into()))
            .await
            .unwrap()
            .is_none()
    );

    let all_seasons = repo.list_all_seasons().await.unwrap();
    assert_eq!(
        all_seasons
            .iter()
            .map(|s| s.id.0.as_str())
            .collect::<Vec<_>>(),
        ["se1"]
    );
    let all_episodes = repo.list_all_episodes().await.unwrap();
    let mut all_episode_ids = all_episodes
        .iter()
        .map(|e| e.id.0.as_str())
        .collect::<Vec<_>>();
    all_episode_ids.sort_unstable();
    assert_eq!(all_episode_ids, ["e1", "e2"]);

    let collections = repo.list_collections(page(0, 10)).await.unwrap();
    assert_eq!(collections.total, 1);
    let collection = repo
        .get_collection(&CollectionId("c1".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        collection.movies,
        vec![MovieId("m1".into()), MovieId("m2".into())]
    );
    assert!(
        repo.get_collection(&CollectionId("no".into()))
            .await
            .unwrap()
            .is_none()
    );

    let movie_versions = repo
        .list_versions(&TitleId::Movie(MovieId("m1".into())), page(0, 10))
        .await
        .unwrap();
    assert_eq!(movie_versions.total, 2);
    assert_eq!(
        movie_versions
            .items
            .iter()
            .map(|v| v.id.0.as_str())
            .collect::<Vec<_>>(),
        ["v1", "v2"]
    );
    let episode_versions = repo
        .list_versions(&TitleId::Episode(EpisodeId("e1".into())), page(0, 10))
        .await
        .unwrap();
    assert_eq!(episode_versions.total, 1);
    assert_eq!(episode_versions.items[0].id, VersionId("v3".into()));

    let lib1_versions = repo
        .list_library_versions(&LibraryId("lib1".into()), page(0, 10))
        .await
        .unwrap();
    assert_eq!(lib1_versions.total, 2);
    let lib2_versions = repo
        .list_library_versions(&LibraryId("lib2".into()), page(0, 10))
        .await
        .unwrap();
    assert_eq!(lib2_versions.total, 1);
    assert_eq!(lib2_versions.items[0].id, VersionId("v3".into()));
    assert_eq!(lib2_versions.items[0].edition, None);

    let all_versions = repo.list_all_versions(page(0, 10)).await.unwrap();
    assert_eq!(all_versions.total, 3);
    assert_eq!(
        all_versions
            .items
            .iter()
            .map(|v| v.id.0.as_str())
            .collect::<Vec<_>>(),
        ["v1", "v2", "v3"]
    );

    let in_library = |name: &str| TitleListFilter {
        libraries: Some(vec![LibraryId(name.into())]),
        ..TitleListFilter::default()
    };
    let movie_ids = |page: &domain::common::Page<Movie>| {
        page.items
            .iter()
            .map(|m| m.id.0.clone())
            .collect::<Vec<_>>()
    };
    let series_ids = |page: &domain::common::Page<Series>| {
        page.items
            .iter()
            .map(|s| s.id.0.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        movie_ids(
            &repo
                .list_movies_filtered(&in_library("lib1"), page(0, 10))
                .await
                .unwrap()
        ),
        vec!["m1".to_owned()]
    );
    assert!(
        repo.list_movies_filtered(&in_library("lib2"), page(0, 10))
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(
        series_ids(
            &repo
                .list_series_filtered(&in_library("lib2"), page(0, 10))
                .await
                .unwrap()
        ),
        vec!["s1".to_owned()]
    );
    assert!(
        repo.list_series_filtered(&in_library("lib1"), page(0, 10))
            .await
            .unwrap()
            .items
            .is_empty()
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
        movie_ids(
            &repo
                .list_movies_filtered(&blocked_by("G"), page(0, 10))
                .await
                .unwrap()
        ),
        vec!["m2".to_owned()]
    );
    assert_eq!(
        movie_ids(
            &repo
                .list_movies_filtered(&blocked_by("PG-13"), page(0, 10))
                .await
                .unwrap()
        ),
        vec!["m1".to_owned(), "m2".to_owned()]
    );

    let sorted = |sort: TitleSort, order: SortOrder| TitleListFilter {
        sort,
        order,
        ..TitleListFilter::default()
    };
    assert_eq!(
        movie_ids(
            &repo
                .list_movies_filtered(&TitleListFilter::default(), page(0, 10))
                .await
                .unwrap()
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
            &TitleListFilter {
                libraries: Some(Vec::new()),
                ..TitleListFilter::default()
            },
            page(0, 10)
        )
        .await
        .unwrap()
        .items
        .is_empty()
    );

    let detail = repo
        .version_detail(&VersionId("v1".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(detail.version.quality, Quality::Sd);
    assert_eq!(detail.version.edition, Some("Director's Cut".into()));
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

    let plain = repo
        .version_detail(&VersionId("v2".into()))
        .await
        .unwrap()
        .unwrap();
    assert!(plain.video.is_empty());
    assert!(plain.audio.is_empty());
    assert!(plain.subtitles.is_empty());
    assert!(plain.chapters.is_empty());
    assert!(plain.markers.intros.is_empty());
    assert!(plain.markers.credits.is_empty());
    assert!(plain.trickplay.is_empty());

    let season = repo
        .get_season(&SeasonId("se1".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(season.number, 1);
    assert_eq!(season.title, Some("Season 1".to_owned()));
    assert!(
        repo.get_season(&SeasonId("no".into()))
            .await
            .unwrap()
            .is_none()
    );

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
    let updated = repo
        .get_collection(&CollectionId("c1".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.name, "Saga Remastered");
    assert_eq!(updated.movies, vec![MovieId("m2".into())]);
    assert_eq!(updated.added_at, ts(11));
    assert_eq!(updated.updated_at, ts(20));

    repo.upsert_collection(Collection {
        id: CollectionId("c2".into()),
        name: "New".into(),
        overview: None,
        movies: Vec::new(),
        added_at: ts(21),
        updated_at: ts(21),
        artwork: Vec::new(),
    })
    .await
    .unwrap();
    assert_eq!(repo.list_collections(page(0, 10)).await.unwrap().total, 2);

    repo.delete_collection(&CollectionId("c1".into()))
        .await
        .unwrap();
    assert!(
        repo.get_collection(&CollectionId("c1".into()))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(repo.list_collections(page(0, 10)).await.unwrap().total, 1);
    repo.delete_collection(&CollectionId("missing".into()))
        .await
        .unwrap();

    let poster = ArtworkRef {
        id: ArtworkId("art-poster-m1".into()),
        kind: ArtworkKind::Poster,
        widths: vec![180, 480, 960],
    };
    let backdrop = ArtworkRef {
        id: ArtworkId("art-backdrop-m1".into()),
        kind: ArtworkKind::Backdrop,
        widths: vec![480, 960],
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
            widths: vec![180],
        }],
    )
    .await
    .unwrap();

    assert_eq!(
        repo.list_artwork(&ArtworkOwner::Movie(MovieId("m1".into())))
            .await
            .unwrap(),
        vec![poster.clone(), backdrop.clone()]
    );
    let hydrated_movie = repo
        .get_movie(&MovieId("m1".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        hydrated_movie.artwork,
        vec![poster.clone(), backdrop.clone()]
    );
    let listed_movies = repo.list_movies(page(0, 10)).await.unwrap();
    assert_eq!(
        listed_movies.items[0].artwork,
        vec![poster.clone(), backdrop.clone()]
    );
    assert!(listed_movies.items[1].artwork.is_empty());

    let hydrated_series = repo
        .get_series(&SeriesId("s1".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(hydrated_series.artwork.len(), 1);
    assert_eq!(hydrated_series.artwork[0].kind, ArtworkKind::Poster);
    assert_eq!(hydrated_series.artwork[0].widths, vec![180]);

    repo.set_artwork(
        &ArtworkOwner::Movie(MovieId("m1".into())),
        std::slice::from_ref(&backdrop),
    )
    .await
    .unwrap();
    let replaced = repo
        .get_movie(&MovieId("m1".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(replaced.artwork, vec![backdrop]);

    assert!(
        repo.list_artwork(&ArtworkOwner::Movie(MovieId("m2".into())))
            .await
            .unwrap()
            .is_empty()
    );

    repo.upsert_movie(Movie {
        id: MovieId("um1".into()),
        title: "Ingested".into(),
        year: Some(2001),
        overview: None,
        runtime_minutes: None,
        content_rating: None,
        added_at: ts(7),
        updated_at: ts(7),
        artwork: Vec::new(),
    })
    .await
    .unwrap();
    repo.upsert_movie(Movie {
        id: MovieId("um1".into()),
        title: "Ingested Remux".into(),
        year: Some(2001),
        overview: None,
        runtime_minutes: None,
        content_rating: None,
        added_at: ts(999),
        updated_at: ts(8),
        artwork: Vec::new(),
    })
    .await
    .unwrap();
    let ingested = repo
        .get_movie(&MovieId("um1".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ingested.title, "Ingested Remux");
    assert_eq!(ingested.added_at, ts(7));
    assert_eq!(ingested.updated_at, ts(8));

    repo.upsert_series(Series {
        id: SeriesId("us1".into()),
        title: "Ingested Show".into(),
        year: Some(2002),
        overview: None,
        content_rating: None,
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
        edition: None,
        available: true,
        added_at: ts(24),
        updated_at: ts(24),
    })
    .await
    .unwrap();

    assert!(
        repo.get_series(&SeriesId("us1".into()))
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        repo.list_seasons(&SeriesId("us1".into()))
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        repo.get_episode(&EpisodeId("ue1".into()))
            .await
            .unwrap()
            .is_some()
    );
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
        edition: None,
        available: true,
        added_at: ts(999),
        updated_at: ts(25),
    })
    .await
    .unwrap();
    let reupserted = repo
        .version_detail(&VersionId("uv1".into()))
        .await
        .unwrap()
        .unwrap()
        .version;
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
        &[Chapter {
            title: "Cold Open".into(),
            start_ms: 0,
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
            },
            SubtitleFile {
                id: SubtitleFileId("uv1-sf2".into()),
                version: uv1.clone(),
                language: Some(LanguageCode("en".into())),
                format: SubtitleFormat::Vtt,
                source: SubtitleSource::Generated,
                path: "/subs/uv1.en.generated.vtt".into(),
                translated_from: None,
            },
            SubtitleFile {
                id: SubtitleFileId("uv1-sf3".into()),
                version: uv1.clone(),
                language: Some(LanguageCode("fr".into())),
                format: SubtitleFormat::Vtt,
                source: SubtitleSource::MachineTranslated,
                path: "/subs/uv1.fr.machine.vtt".into(),
                translated_from: Some(SubtitleFileId("uv1-sf1".into())),
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
    assert!(
        uv1_detail
            .subtitle_files
            .iter()
            .any(|f| f.source == SubtitleSource::OpenSubtitles)
    );
    assert!(
        uv1_detail
            .subtitle_files
            .iter()
            .any(|f| f.source == SubtitleSource::Generated)
    );
    assert!(
        uv1_detail
            .subtitle_files
            .iter()
            .any(|f| f.source == SubtitleSource::MachineTranslated)
    );
    assert_eq!(uv1_detail.subtitle_files[0].format, SubtitleFormat::Vtt);

    repo.set_version_tracks(&uv1, &[], &[], &[], &[])
        .await
        .unwrap();
    repo.set_trickplay(&uv1, &[]).await.unwrap();
    repo.set_subtitle_files(&uv1, &[]).await.unwrap();
    let uv1_cleared = repo.version_detail(&uv1).await.unwrap().unwrap();
    assert!(uv1_cleared.video.is_empty());
    assert!(uv1_cleared.audio.is_empty());
    assert!(uv1_cleared.subtitles.is_empty());
    assert!(uv1_cleared.subtitle_files.is_empty());
    assert!(uv1_cleared.chapters.is_empty());
    assert!(uv1_cleared.trickplay.is_empty());

    let m1_detail = repo
        .movie_detail(&MovieId("m1".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(m1_detail.movie.title, "Alpha");
    assert_eq!(
        m1_detail
            .genres
            .iter()
            .map(|g| g.name.as_str())
            .collect::<Vec<_>>(),
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
        m1_detail
            .studios
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["Acme Studios"]
    );
    assert_eq!(
        m1_detail.ratings,
        vec![Rating {
            source: "tmdb".into(),
            value: 8.5,
        }]
    );
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
    assert!(
        repo.movie_detail(&MovieId("missing".into()))
            .await
            .unwrap()
            .is_none()
    );

    let s1_detail = repo
        .series_detail(&SeriesId("s1".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(s1_detail.series.title, "Gamma");
    assert_eq!(s1_detail.genres.len(), 1);
    assert_eq!(s1_detail.credits.len(), 1);
    assert_eq!(s1_detail.credits[0].person.name, "Ada");
    assert_eq!(s1_detail.credits[0].character, Some("Lead".to_owned()));
    assert_eq!(s1_detail.ratings[0].value, 9.0);
    assert_eq!(s1_detail.external_ids[0].source, "tvdb");
    assert!(
        repo.series_detail(&SeriesId("missing".into()))
            .await
            .unwrap()
            .is_none()
    );

    assert_eq!(
        repo.get_person(&PersonId("p1".into()))
            .await
            .unwrap()
            .unwrap()
            .name,
        "Ada"
    );
    assert!(
        repo.get_person(&PersonId("missing".into()))
            .await
            .unwrap()
            .is_none()
    );

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
    let enriched = repo
        .get_person(&PersonId("p1".into()))
        .await
        .unwrap()
        .unwrap();
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
    let preserved = repo
        .get_person(&PersonId("p1".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(preserved.name, "Ada");
    assert_eq!(preserved.biography.as_deref(), Some("A mathematician."));
    assert_eq!(preserved.also_known_as, vec!["Augusta Ada King".to_owned()]);

    let cascade_series = SeriesId("cascade-series".into());
    let cascade_season = SeasonId("cascade-s1".into());
    for n in 1..=3u16 {
        repo.upsert_series(Series {
            id: cascade_series.clone(),
            title: "Cascade".into(),
            year: None,
            overview: None,
            content_rating: None,
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
    assert!(
        repo.filmography(&PersonId("missing".into()))
            .await
            .unwrap()
            .is_empty()
    );

    assert_eq!(
        repo.list_genres()
            .await
            .unwrap()
            .iter()
            .map(|g| g.name.as_str())
            .collect::<Vec<_>>(),
        ["Action", "Drama"]
    );

    let by_genre = |name: &str| TitleListFilter {
        genres: vec![name.into()],
        ..TitleListFilter::default()
    };
    let by_action = repo
        .list_movies_filtered(&by_genre("Action"), page(0, 10))
        .await
        .unwrap();
    assert_eq!(
        by_action
            .items
            .iter()
            .map(|m| m.id.0.as_str())
            .collect::<Vec<_>>(),
        ["m1"]
    );
    assert_eq!(
        repo.list_movies_filtered(&by_genre("Drama"), page(0, 10))
            .await
            .unwrap()
            .total,
        1
    );
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
        repo.list_series_filtered(&by_genre("Drama"), page(0, 10))
            .await
            .unwrap()
            .items
            .is_empty()
    );

    repo.set_title_enrichment(
        &TitleRef::Movie(MovieId("m1".into())),
        &TitleEnrichment::default(),
    )
    .await
    .unwrap();
    let cleared = repo
        .movie_detail(&MovieId("m1".into()))
        .await
        .unwrap()
        .unwrap();
    assert!(cleared.genres.is_empty());
    assert!(cleared.credits.is_empty());
    assert!(cleared.studios.is_empty());
    assert!(cleared.ratings.is_empty());
    assert!(cleared.external_ids.is_empty());
    assert!(cleared.extras.is_empty());

    let p1_after = repo.filmography(&PersonId("p1".into())).await.unwrap();
    assert_eq!(p1_after.len(), 1);
    assert_eq!(p1_after[0].title, TitleRef::Series(SeriesId("s1".into())));
    assert!(
        repo.filmography(&PersonId("p2".into()))
            .await
            .unwrap()
            .is_empty()
    );

    let available = async |repo: &R, id: &str| -> bool {
        repo.version_detail(&VersionId(id.into()))
            .await
            .unwrap()
            .unwrap()
            .version
            .available
    };

    repo.reconcile_library_versions(
        &LibraryId("lib1".into()),
        &["/media/v1.mkv".to_owned(), "/media/uv1.mkv".to_owned()],
    )
    .await
    .unwrap();
    assert_eq!(
        repo.list_library_versions(&LibraryId("lib1".into()), page(0, 10))
            .await
            .unwrap()
            .total,
        3
    );
    assert!(available(&repo, "v1").await);
    assert!(available(&repo, "uv1").await);
    assert!(!available(&repo, "v2").await);
    assert!(available(&repo, "v3").await);

    repo.reconcile_library_versions(
        &LibraryId("lib1".into()),
        &[
            "/media/v1.mkv".to_owned(),
            "/media/v2.mp4".to_owned(),
            "/media/uv1.mkv".to_owned(),
        ],
    )
    .await
    .unwrap();
    assert!(available(&repo, "v2").await);

    repo.reconcile_library_versions(&LibraryId("lib1".into()), &[])
        .await
        .unwrap();
    assert!(!available(&repo, "v1").await);
    assert!(!available(&repo, "v2").await);
    assert!(!available(&repo, "uv1").await);
    assert!(available(&repo, "v3").await);
}
