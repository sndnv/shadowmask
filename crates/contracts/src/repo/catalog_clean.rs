use std::collections::HashSet;

use domain::catalog::{
    ArtworkOwner, CatalogClean, CleanScope, Collection, CollectionId, Episode, EpisodeId, MovieId,
    Season, SeasonId, SeriesId, TitleId, TitleRef, TitleSet, VersionId,
};
use domain::common::Quality;
use domain::library::LibraryId;
use domain::media::{SubtitleFile, SubtitleFileId, SubtitleFormat, SubtitleSource, TrickplayAsset};
use domain::metadata::{ExternalId, Genre, GenreId, TitleEnrichment};
use domain::repository::CatalogRepository;

use crate::fixture;

const OWN: &str = "clean-a";
const OTHER: &str = "clean-b";
const DELETED: &str = "clean-gone";

fn movie(id: &str) -> TitleId {
    TitleId::Movie(MovieId(id.into()))
}

fn episode(id: &str) -> TitleId {
    TitleId::Episode(EpisodeId(id.into()))
}

fn enrichment() -> TitleEnrichment {
    TitleEnrichment {
        genres: vec![Genre { id: GenreId("clean-genre".into()), name: "Drama".into() }],
        external_ids: vec![ExternalId { source: "tmdb".into(), value: "1".into() }],
        ..TitleEnrichment::default()
    }
}

fn trickplay(version: &str) -> TrickplayAsset {
    TrickplayAsset {
        version: VersionId(version.into()),
        interval_ms: 5_000,
        columns: 8,
        rows: 8,
        tile_width: 320,
        tile_height: 180,
        sheet_paths: vec![format!("/tp/{version}/sheet-001.jpg")],
    }
}

fn subtitle(version: &str) -> SubtitleFile {
    SubtitleFile {
        id: SubtitleFileId(format!("{version}-os")),
        version: VersionId(version.into()),
        language: None,
        format: SubtitleFormat::Vtt,
        source: SubtitleSource::OpenSubtitles,
        path: format!("/subs/{version}.vtt"),
        translated_from: None,
        label: None,
        pinned: false,
    }
}

async fn seed<R: CatalogRepository>(repo: &R) {
    for id in ["cm-gone", "cm-kept", "cm-other", "cm-empty", "cm-deleted", "cm-live"] {
        repo.upsert_movie(fixture::movie(id)).await.unwrap();
    }
    repo.set_title_enrichment(&TitleRef::Movie(MovieId("cm-gone".into())), &enrichment())
        .await
        .unwrap();
    repo.set_artwork(&ArtworkOwner::Movie(MovieId("cm-gone".into())), &fixture::artwork_set("cg"))
        .await
        .unwrap();
    repo.upsert_collection(Collection {
        id: CollectionId("clean-saga".into()),
        movies: vec![MovieId("cm-gone".into()), MovieId("cm-live".into())],
        ..fixture::saga_collection()
    })
    .await
    .unwrap();
    for id in ["cs-gone", "cs-kept", "cs-empty"] {
        repo.upsert_series(fixture::series(id)).await.unwrap();
    }
    repo.set_title_enrichment(&TitleRef::Series(SeriesId("cs-gone".into())), &enrichment())
        .await
        .unwrap();
    for (id, series, number) in [
        ("cse-gone-1", "cs-gone", 1),
        ("cse-gone-2", "cs-gone", 2),
        ("cse-kept-1", "cs-kept", 1),
        ("cse-kept-2", "cs-kept", 2),
        ("cse-empty", "cs-empty", 1),
    ] {
        repo.upsert_season(Season { number, ..fixture::season(id, series) }).await.unwrap();
    }
    repo.set_artwork(
        &ArtworkOwner::Season(SeasonId("cse-kept-2".into())),
        &fixture::artwork_set("ck2"),
    )
    .await
    .unwrap();
    for (id, season, number) in [
        ("ce-1", "cse-gone-1", 1),
        ("ce-2", "cse-kept-1", 1),
        ("ce-3", "cse-kept-1", 2),
        ("ce-4", "cse-kept-2", 1),
    ] {
        repo.upsert_episode(Episode { number, ..fixture::episode(id, season) }).await.unwrap();
    }
    for (id, title, library, available) in [
        ("cv-gone", movie("cm-gone"), OWN, false),
        ("cv-kept-gone", movie("cm-kept"), OWN, false),
        ("cv-kept", movie("cm-kept"), OWN, true),
        ("cv-other-own", movie("cm-other"), OWN, false),
        ("cv-other", movie("cm-other"), OTHER, false),
        ("cv-deleted", movie("cm-deleted"), DELETED, false),
        ("cv-live", movie("cm-live"), OTHER, true),
        ("cv-ghost", movie("cm-ghost"), OWN, false),
        ("cv-untitled", movie("cm-untitled"), OWN, true),
        ("cve-1", episode("ce-1"), OWN, false),
        ("cve-2", episode("ce-2"), OWN, false),
        ("cve-3", episode("ce-3"), OTHER, true),
        ("cve-4", episode("ce-4"), OWN, false),
    ] {
        let version = fixture::version(id, title, library, Quality::Hd);
        repo.upsert_version(domain::catalog::Version { available, ..version }).await.unwrap();
    }
    for id in ["cv-gone", "cv-kept"] {
        repo.set_trickplay(&VersionId(id.into()), &[trickplay(id)]).await.unwrap();
        repo.set_subtitle_files(&VersionId(id.into()), &[subtitle(id)]).await.unwrap();
    }
}

fn ids<T>(items: &HashSet<T>, id: impl Fn(&T) -> &str) -> Vec<String> {
    let mut ids: Vec<String> = items.iter().map(|item| id(item).to_owned()).collect();
    ids.sort();
    ids
}

fn versions(ids: &[&str]) -> HashSet<VersionId> {
    ids.iter().map(|id| VersionId((*id).into())).collect()
}

fn titles(cleaned: &TitleSet) -> [Vec<String>; 4] {
    [
        ids(&cleaned.movies, |id| &id.0),
        ids(&cleaned.episodes, |id| &id.0),
        ids(&cleaned.seasons, |id| &id.0),
        ids(&cleaned.series, |id| &id.0),
    ]
}

async fn library_scope<R: CatalogRepository>(repo: &R) {
    let cleaned = repo.clean_catalog(&CleanScope::Library(LibraryId(OWN.into()))).await.unwrap();

    assert_eq!(
        cleaned.versions,
        versions(&[
            "cv-gone",
            "cv-kept-gone",
            "cv-other-own",
            "cv-ghost",
            "cve-1",
            "cve-2",
            "cve-4"
        ]),
        "only this library's unavailable versions"
    );
    assert_eq!(
        titles(&cleaned.titles),
        [
            vec!["cm-gone".to_owned()],
            vec!["ce-1".to_owned(), "ce-2".to_owned(), "ce-4".to_owned()],
            vec!["cse-gone-1".to_owned(), "cse-gone-2".to_owned(), "cse-kept-2".to_owned()],
            vec!["cs-gone".to_owned()],
        ],
        "a title with any version left stays, and so do titles this clean did not empty"
    );
    assert!(repo.get_version(&VersionId("cv-gone".into())).await.unwrap().is_none());
    assert!(repo.version_detail(&VersionId("cv-gone".into())).await.unwrap().is_none());
    let kept = repo.version_detail(&VersionId("cv-kept".into())).await.unwrap().unwrap();
    assert_eq!((kept.trickplay.len(), kept.subtitle_files.len()), (1, 1));
    for id in ["cm-kept", "cm-other", "cm-empty", "cm-deleted", "cm-live"] {
        assert!(repo.get_movie(&MovieId(id.into())).await.unwrap().is_some(), "{id}");
    }
    assert!(repo.get_movie(&MovieId("cm-gone".into())).await.unwrap().is_none());
    assert!(repo.get_series(&SeriesId("cs-gone".into())).await.unwrap().is_none());
    assert!(repo.get_series(&SeriesId("cs-empty".into())).await.unwrap().is_some());
    let seasons = repo.list_seasons(&SeriesId("cs-kept".into())).await.unwrap();
    assert_eq!(seasons.iter().map(|s| s.id.0.as_str()).collect::<Vec<_>>(), ["cse-kept-1"]);
    let episodes = repo.list_episodes(&SeasonId("cse-kept-1".into())).await.unwrap();
    assert_eq!(episodes.iter().map(|e| e.id.0.as_str()).collect::<Vec<_>>(), ["ce-3"]);

    repo.upsert_movie(fixture::movie("cm-gone")).await.unwrap();
    let again = repo.movie_detail(&MovieId("cm-gone".into())).await.unwrap().unwrap();
    assert!(again.genres.is_empty() && again.external_ids.is_empty(), "no attribute survives");
    assert!(
        repo.list_artwork(&ArtworkOwner::Movie(MovieId("cm-gone".into())))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        repo.list_artwork(&ArtworkOwner::Season(SeasonId("cse-kept-2".into())))
            .await
            .unwrap()
            .is_empty()
    );
    let saga = repo.get_collection(&CollectionId("clean-saga".into())).await.unwrap().unwrap();
    assert_eq!(saga.movies, [MovieId("cm-live".into())]);
    repo.upsert_series(fixture::series("cs-gone")).await.unwrap();
    let series = repo.series_detail(&SeriesId("cs-gone".into())).await.unwrap().unwrap();
    assert!(series.genres.is_empty());
}

async fn every_library<R: CatalogRepository>(repo: &R) {
    let cleaned = repo.clean_catalog(&CleanScope::All).await.unwrap();

    assert_eq!(cleaned.versions, versions(&["cv-other", "cv-deleted"]));
    assert_eq!(
        titles(&cleaned.titles),
        [
            vec![
                "cm-deleted".to_owned(),
                "cm-empty".to_owned(),
                "cm-gone".to_owned(),
                "cm-other".to_owned(),
            ],
            Vec::new(),
            vec!["cse-empty".to_owned()],
            vec!["cs-empty".to_owned(), "cs-gone".to_owned()],
        ],
        "every title with nothing under it, whenever it was left"
    );
    for id in ["cm-kept", "cm-live"] {
        assert!(repo.get_movie(&MovieId(id.into())).await.unwrap().is_some(), "{id}");
    }
    for id in ["cv-kept", "cv-live", "cv-untitled", "cve-3"] {
        assert!(repo.get_version(&VersionId(id.into())).await.unwrap().is_some(), "{id}");
    }
    assert!(repo.get_series(&SeriesId("cs-kept".into())).await.unwrap().is_some());
    assert_eq!(repo.clean_catalog(&CleanScope::All).await.unwrap(), CatalogClean::default());
}

pub async fn catalog_clean_contract<R: CatalogRepository>(repo: R) {
    seed(&repo).await;
    let unknown = CleanScope::Library(LibraryId("clean-none".into()));
    assert_eq!(repo.clean_catalog(&unknown).await.unwrap(), CatalogClean::default());

    library_scope(&repo).await;
    every_library(&repo).await;
}
