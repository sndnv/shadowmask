use std::collections::HashSet;

use domain::catalog::{
    ArtworkOwner, Collection, CollectionId, Episode, EpisodeId, LibraryPurge, MovieId, Season,
    SeasonId, SeriesId, TitleId, TitleRef, Version, VersionId,
};
use domain::common::Quality;
use domain::library::LibraryId;
use domain::media::{SubtitleFile, SubtitleFileId, SubtitleFormat, SubtitleSource, TrickplayAsset};
use domain::metadata::{ExternalId, Genre, GenreId, TitleEnrichment};
use domain::repository::CatalogRepository;

use crate::fixture;

const OWN: &str = "purge-a";
const OTHER: &str = "purge-b";

fn movie(id: &str) -> TitleId {
    TitleId::Movie(MovieId(id.into()))
}

fn episode(id: &str) -> TitleId {
    TitleId::Episode(EpisodeId(id.into()))
}

fn versions(ids: &[&str]) -> HashSet<VersionId> {
    ids.iter().map(|id| VersionId((*id).into())).collect()
}

fn subtitle(id: &str, version: &str, source: SubtitleSource, from: Option<&str>) -> SubtitleFile {
    SubtitleFile {
        id: SubtitleFileId(id.into()),
        version: VersionId(version.into()),
        language: None,
        format: SubtitleFormat::Vtt,
        source,
        path: format!("/subs/{id}.vtt"),
        translated_from: from.map(|from| SubtitleFileId(from.into())),
        label: None,
        pinned: false,
    }
}

fn enrichment() -> TitleEnrichment {
    TitleEnrichment {
        genres: vec![Genre { id: GenreId("purge-genre".into()), name: "Drama".into() }],
        external_ids: vec![ExternalId { source: "tmdb".into(), value: "1".into() }],
        ..TitleEnrichment::default()
    }
}

async fn seed<R: CatalogRepository>(repo: &R) {
    for id in ["pm-own", "pm-shared"] {
        repo.upsert_movie(fixture::movie(id)).await.unwrap();
        repo.set_title_enrichment(&TitleRef::Movie(MovieId(id.into())), &enrichment())
            .await
            .unwrap();
        repo.set_artwork(&ArtworkOwner::Movie(MovieId(id.into())), &fixture::artwork_set(id))
            .await
            .unwrap();
    }
    repo.upsert_collection(Collection {
        id: CollectionId("purge-saga".into()),
        movies: vec![MovieId("pm-own".into()), MovieId("pm-shared".into())],
        ..fixture::saga_collection()
    })
    .await
    .unwrap();
    for id in ["ps-gone", "ps-kept"] {
        repo.upsert_series(fixture::series(id)).await.unwrap();
        repo.set_title_enrichment(&TitleRef::Series(SeriesId(id.into())), &enrichment())
            .await
            .unwrap();
    }
    for (id, series, number) in [
        ("pse-gone-1", "ps-gone", 1),
        ("pse-empty", "ps-gone", 2),
        ("pse-kept-1", "ps-kept", 1),
        ("pse-kept-2", "ps-kept", 2),
    ] {
        repo.upsert_season(Season { number, ..fixture::season(id, series) }).await.unwrap();
        repo.set_artwork(&ArtworkOwner::Season(SeasonId(id.into())), &fixture::artwork_set(id))
            .await
            .unwrap();
    }
    for (id, season, number) in [
        ("pe-1", "pse-gone-1", 1),
        ("pe-2", "pse-kept-1", 1),
        ("pe-3", "pse-kept-1", 2),
        ("pe-4", "pse-kept-2", 1),
    ] {
        repo.upsert_episode(Episode { number, ..fixture::episode(id, season) }).await.unwrap();
    }
    for (id, title, library) in [
        ("pv-own", movie("pm-own"), OWN),
        ("pv-shared-a", movie("pm-shared"), OWN),
        ("pv-shared-b", movie("pm-shared"), OTHER),
        ("pv-ghost", movie("pm-ghost"), OWN),
        ("pv-e-ghost", episode("pe-ghost"), OWN),
        ("pv-e1", episode("pe-1"), OWN),
        ("pv-e2", episode("pe-2"), OWN),
        ("pv-e3", episode("pe-3"), OTHER),
        ("pv-e4", episode("pe-4"), OWN),
    ] {
        repo.upsert_version(fixture::version(id, title, library, Quality::Hd)).await.unwrap();
    }
    repo.set_subtitle_files(
        &VersionId("pv-own".into()),
        &[
            subtitle("pv-own-os", "pv-own", SubtitleSource::OpenSubtitles, None),
            subtitle(
                "pv-own-os-fr",
                "pv-own",
                SubtitleSource::MachineTranslated,
                Some("pv-own-os"),
            ),
            subtitle("pv-own-ext", "pv-own", SubtitleSource::External, None),
            subtitle(
                "pv-own-ext-fr",
                "pv-own",
                SubtitleSource::MachineTranslated,
                Some("pv-own-ext"),
            ),
            subtitle("pv-own-gen", "pv-own", SubtitleSource::Generated, None),
            subtitle("pv-own-both", "pv-own", SubtitleSource::Combined, None),
        ],
    )
    .await
    .unwrap();
    repo.set_subtitle_files(
        &VersionId("pv-shared-a".into()),
        &[subtitle("pv-shared-os", "pv-shared-a", SubtitleSource::OpenSubtitles, None)],
    )
    .await
    .unwrap();
    repo.set_trickplay(
        &VersionId("pv-own".into()),
        &[TrickplayAsset {
            version: VersionId("pv-own".into()),
            interval_ms: 5_000,
            columns: 8,
            rows: 8,
            tile_width: 320,
            tile_height: 180,
            sheet_paths: vec!["/tp/pv-own/sheet-001.jpg".into()],
        }],
    )
    .await
    .unwrap();
}

async fn untitled<R: CatalogRepository>(repo: &R, library: &LibraryId) -> HashSet<VersionId> {
    let found = repo.untitled_versions(library).await.unwrap();
    found.into_iter().filter(|id| id.0.starts_with("pv-")).collect()
}

fn ids<T>(items: &HashSet<T>, id: impl Fn(&T) -> &str) -> Vec<String> {
    let mut ids: Vec<String> = items.iter().map(|item| id(item).to_owned()).collect();
    ids.sort();
    ids
}

pub(super) async fn library_purge_takes_what_the_scan_derived<R: CatalogRepository>(repo: &R) {
    seed(repo).await;
    let own = LibraryId(OWN.into());
    for (id, available) in [("pv-left", false), ("pv-held", true)] {
        let version = fixture::version(id, movie(&format!("pm-{id}")), OTHER, Quality::Hd);
        repo.upsert_version(Version { available, ..version }).await.unwrap();
    }

    let footprint = repo.library_footprint(&own).await.unwrap();
    assert_eq!(
        footprint.versions,
        versions(&["pv-own", "pv-shared-a", "pv-ghost", "pv-e-ghost", "pv-e1", "pv-e2", "pv-e4"])
    );
    assert_eq!(ids(&footprint.titles.movies, |id| &id.0), ["pm-ghost", "pm-own", "pm-shared"]);
    assert_eq!(ids(&footprint.titles.episodes, |id| &id.0), ["pe-1", "pe-2", "pe-4", "pe-ghost"]);
    assert_eq!(
        ids(&footprint.titles.seasons, |id| &id.0),
        ["pse-gone-1", "pse-kept-1", "pse-kept-2"]
    );
    assert_eq!(ids(&footprint.titles.series, |id| &id.0), ["ps-gone", "ps-kept"]);
    assert_eq!(
        untitled(repo, &own).await,
        versions(&["pv-ghost", "pv-e-ghost", "pv-left"]),
        "another library's version is a scan's to take over only when unavailable"
    );

    let purged = repo.purge_library(&own).await.unwrap();

    assert_eq!(ids(&purged.titles.movies, |id| &id.0), ["pm-own"], "a shared title stays");
    assert_eq!(ids(&purged.titles.episodes, |id| &id.0), ["pe-1", "pe-2", "pe-4"]);
    assert_eq!(
        ids(&purged.titles.seasons, |id| &id.0),
        ["pse-empty", "pse-gone-1", "pse-kept-2"],
        "an emptied season goes, and every season of a series that goes"
    );
    assert_eq!(ids(&purged.titles.series, |id| &id.0), ["ps-gone"]);
    assert_eq!(
        purged.versions,
        versions(&["pv-own", "pv-ghost", "pv-e-ghost", "pv-e1", "pv-e2", "pv-e4"]),
        "a version on a shared title keeps its title"
    );
    assert_eq!(purged.subtitles, 2);

    assert!(repo.get_movie(&MovieId("pm-own".into())).await.unwrap().is_none());
    assert!(repo.get_movie(&MovieId("pm-shared".into())).await.unwrap().is_some());
    assert!(repo.get_series(&SeriesId("ps-gone".into())).await.unwrap().is_none());
    assert!(repo.get_season(&SeasonId("pse-empty".into())).await.unwrap().is_none());
    let kept_seasons = repo.list_seasons(&SeriesId("ps-kept".into())).await.unwrap();
    assert_eq!(kept_seasons.iter().map(|s| s.id.0.as_str()).collect::<Vec<_>>(), ["pse-kept-1"]);
    let kept_episodes = repo.list_episodes(&SeasonId("pse-kept-1".into())).await.unwrap();
    assert_eq!(kept_episodes.iter().map(|e| e.id.0.as_str()).collect::<Vec<_>>(), ["pe-3"]);

    let own_detail = repo.version_detail(&VersionId("pv-own".into())).await.unwrap().unwrap();
    assert_eq!(own_detail.trickplay.len(), 1, "what belongs to the file stays");
    let mut kept: Vec<&str> = own_detail.subtitle_files.iter().map(|f| f.id.0.as_str()).collect();
    kept.sort();
    assert_eq!(kept, ["pv-own-both", "pv-own-ext", "pv-own-ext-fr", "pv-own-gen"]);
    let shared = repo.version_detail(&VersionId("pv-shared-a".into())).await.unwrap().unwrap();
    assert_eq!(shared.subtitle_files.len(), 1);
    assert_eq!(
        untitled(repo, &own).await,
        versions(&["pv-own", "pv-ghost", "pv-e-ghost", "pv-e1", "pv-e2", "pv-e4", "pv-left"])
    );

    repo.upsert_movie(fixture::movie("pm-own")).await.unwrap();
    repo.upsert_series(fixture::series("ps-gone")).await.unwrap();
    let again = repo.movie_detail(&MovieId("pm-own".into())).await.unwrap().unwrap();
    assert!(again.genres.is_empty() && again.external_ids.is_empty(), "no attribute survives");
    let series = repo.series_detail(&SeriesId("ps-gone".into())).await.unwrap().unwrap();
    assert!(series.genres.is_empty() && series.external_ids.is_empty());
    assert!(
        repo.list_artwork(&ArtworkOwner::Movie(MovieId("pm-own".into()))).await.unwrap().is_empty()
    );
    assert!(
        repo.list_artwork(&ArtworkOwner::Season(SeasonId("pse-kept-2".into())))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(repo.collections_of_movie(&MovieId("pm-own".into())).await.unwrap().is_empty());
    let saga = repo.get_collection(&CollectionId("purge-saga".into())).await.unwrap().unwrap();
    assert_eq!(saga.movies, [MovieId("pm-shared".into())]);
    let shared_detail = repo.movie_detail(&MovieId("pm-shared".into())).await.unwrap().unwrap();
    assert_eq!(shared_detail.genres.len(), 1);

    assert_eq!(
        repo.purge_library(&LibraryId("purge-none".into())).await.unwrap(),
        LibraryPurge::default()
    );
}
