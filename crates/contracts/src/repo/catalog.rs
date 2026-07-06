use domain::catalog::{
    Collection, CollectionId, Episode, EpisodeId, Movie, MovieId, Season, SeasonId, Series,
    SeriesId, TitleId, Version, VersionDetail, VersionId,
};
use domain::common::{LanguageCode, PageRequest, Quality};
use domain::library::LibraryId;
use domain::media::{
    AudioTrack, Chapter, CreditsMarker, DetectedMarkers, EmbeddedSubtitleTrack, HdrFormat,
    IntroMarker, SubtitleFormat, TrickplayAsset, VideoTrack,
};
use domain::metadata::ContentRating;
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
            },
            Movie {
                id: MovieId("m2".into()),
                title: "Beta".into(),
                year: None,
                overview: None,
                runtime_minutes: None,
                content_rating: None,
                added_at: ts(2),
            },
        ],
        series: vec![Series {
            id: SeriesId("s1".into()),
            title: "Gamma".into(),
            year: Some(2019),
            overview: None,
            content_rating: None,
            added_at: ts(3),
        }],
        seasons: vec![Season {
            id: SeasonId("se1".into()),
            series: SeriesId("s1".into()),
            number: 1,
            title: Some("Season 1".into()),
            overview: None,
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
            },
        ],
        collections: vec![Collection {
            id: CollectionId("c1".into()),
            name: "Saga".into(),
            overview: Some("epic".into()),
            movies: vec![MovieId("m1".into()), MovieId("m2".into())],
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
            },
        ],
        detail,
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

    repo.upsert_collection(Collection {
        id: CollectionId("c2".into()),
        name: "New".into(),
        overview: None,
        movies: Vec::new(),
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
}
