use domain::catalog::{TitleId, Version, VersionId};
use domain::error::RepositoryError;
use domain::playback::{ResumeCard, progress_percent};
use domain::repository::CatalogRepository;

pub async fn resume_card(
    catalog: &(impl CatalogRepository + Sync),
    version: &VersionId,
    position_ms: u64,
) -> Result<Option<ResumeCard>, RepositoryError> {
    let Some(played) = catalog.get_version(version).await? else {
        return Ok(None);
    };
    Ok(Some(resume_card_from(catalog, played, position_ms).await?))
}

pub async fn resume_card_from(
    catalog: &(impl CatalogRepository + Sync),
    played: Version,
    position_ms: u64,
) -> Result<ResumeCard, RepositoryError> {
    let duration_ms = played.duration_ms;
    let title = played.title;
    let mut card = ResumeCard {
        title: title.clone(),
        display_title: String::new(),
        artwork: Vec::new(),
        duration_ms,
        progress_percent: progress_percent(position_ms, duration_ms),
        year: None,
        series_title: None,
        series_artwork: Vec::new(),
        season_number: None,
        episode_number: None,
    };
    match &title {
        TitleId::Movie(id) => match catalog.get_movie(id).await? {
            Some(movie) => {
                card.display_title = movie.title;
                card.artwork = movie.artwork;
                card.year = movie.year;
            }
            None => card.display_title = id.0.clone(),
        },
        TitleId::Episode(id) => match catalog.get_episode(id).await? {
            Some(episode) => {
                card.display_title = episode.title;
                card.artwork = episode.artwork;
                card.episode_number = Some(episode.number);
                if let Some(season) = catalog.get_season(&episode.season).await? {
                    card.season_number = Some(season.number);
                    if let Some(series) = catalog.get_series(&season.series).await? {
                        card.series_title = Some(series.title);
                        card.series_artwork = series.artwork;
                    }
                }
            }
            None => card.display_title = id.0.clone(),
        },
    }
    Ok(card)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{
        ArtworkId, ArtworkRef, ArtworkWidth, Episode, EpisodeId, Movie, MovieId, Season, SeasonId,
        Series, SeriesId, Version,
    };
    use domain::common::Quality;
    use domain::library::LibraryId;
    use domain::metadata::ArtworkKind;
    use jiff::Timestamp;
    use mocks::MockCatalogRepo;

    fn version(id: &str, title: TitleId, duration_ms: u64) -> Version {
        Version {
            id: VersionId(id.to_owned()),
            title,
            library: LibraryId("lib1".to_owned()),
            quality: Quality::Hd,
            container: "mkv".to_owned(),
            path: format!("/media/{id}.mkv"),
            size_bytes: 1,
            duration_ms,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn poster() -> ArtworkRef {
        ArtworkRef {
            id: ArtworkId("art-1".to_owned()),
            kind: ArtworkKind::Poster,
            widths: vec![
                ArtworkWidth::new(180, "/art/art-1/180.jpg"),
                ArtworkWidth::new(480, "/art/art-1/480.jpg"),
            ],
        }
    }

    #[tokio::test]
    async fn builds_movie_card_with_title_artwork_and_percent() {
        let repo = MockCatalogRepo::new();
        repo.add_movie(Movie {
            id: MovieId("m1".to_owned()),
            title: "Alpha".to_owned(),
            sort_title: "alpha".to_owned(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        repo.set_artwork(
            &domain::catalog::ArtworkOwner::Movie(MovieId("m1".to_owned())),
            &[poster()],
        )
        .await
        .unwrap();
        repo.add_version(version(
            "v1",
            TitleId::Movie(MovieId("m1".to_owned())),
            1000,
        ));

        let card = resume_card(&repo, &VersionId("v1".to_owned()), 250)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(card.display_title, "Alpha");
        assert_eq!(card.duration_ms, 1000);
        assert_eq!(card.progress_percent, 25);
        assert_eq!(card.artwork, vec![poster()]);
        assert_eq!(card.title, TitleId::Movie(MovieId("m1".to_owned())));
    }

    #[tokio::test]
    async fn builds_episode_card_with_series_context() {
        let repo = MockCatalogRepo::new();
        repo.add_series(Series {
            id: SeriesId("sr1".to_owned()),
            title: "Show ABC".to_owned(),
            sort_title: "show abc".to_owned(),
            year: None,
            overview: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        repo.set_artwork(
            &domain::catalog::ArtworkOwner::Series(SeriesId("sr1".to_owned())),
            &[poster()],
        )
        .await
        .unwrap();
        repo.add_season(Season {
            id: SeasonId("se1".to_owned()),
            series: SeriesId("sr1".to_owned()),
            number: 2,
            title: None,
            overview: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        repo.add_episode(Episode {
            id: EpisodeId("e1".to_owned()),
            season: SeasonId("se1".to_owned()),
            number: 3,
            title: "Pilot".to_owned(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        repo.add_version(version(
            "ev1",
            TitleId::Episode(EpisodeId("e1".to_owned())),
            2000,
        ));

        let card = resume_card(&repo, &VersionId("ev1".to_owned()), 1000)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(card.display_title, "Pilot");
        assert_eq!(card.progress_percent, 50);
        assert_eq!(card.series_title.as_deref(), Some("Show ABC"));
        assert_eq!(card.series_artwork, vec![poster()]);
        assert_eq!(card.season_number, Some(2));
        assert_eq!(card.episode_number, Some(3));
        assert_eq!(card.year, None);
    }

    #[tokio::test]
    async fn episode_card_survives_a_missing_season() {
        let repo = MockCatalogRepo::new();
        repo.add_episode(Episode {
            id: EpisodeId("e1".to_owned()),
            season: SeasonId("gone".to_owned()),
            number: 3,
            title: "Pilot".to_owned(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        repo.add_version(version(
            "ev1",
            TitleId::Episode(EpisodeId("e1".to_owned())),
            2000,
        ));

        let card = resume_card(&repo, &VersionId("ev1".to_owned()), 1000)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(card.episode_number, Some(3));
        assert_eq!(card.season_number, None);
        assert_eq!(card.series_title, None);
        assert!(card.series_artwork.is_empty());
    }

    #[tokio::test]
    async fn falls_back_when_title_row_absent() {
        let repo = MockCatalogRepo::new();
        repo.add_version(version(
            "v1",
            TitleId::Movie(MovieId("gone".to_owned())),
            1000,
        ));

        let card = resume_card(&repo, &VersionId("v1".to_owned()), 0)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(card.display_title, "gone");
        assert!(card.artwork.is_empty());
    }

    #[tokio::test]
    async fn none_when_version_missing() {
        let repo = MockCatalogRepo::new();
        assert!(
            resume_card(&repo, &VersionId("ghost".to_owned()), 0)
                .await
                .unwrap()
                .is_none()
        );
    }
}
