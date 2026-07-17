use domain::catalog::{TitleId, VersionId};
use domain::error::RepositoryError;
use domain::playback::{ResumeCard, progress_percent};
use domain::repository::CatalogRepository;

pub async fn resume_card(
    catalog: &(impl CatalogRepository + Sync),
    version: &VersionId,
    position_ms: u64,
) -> Result<Option<ResumeCard>, RepositoryError> {
    let Some(detail) = catalog.version_detail(version).await? else {
        return Ok(None);
    };
    let duration_ms = detail.version.duration_ms;
    let title = detail.version.title;
    let (display_title, artwork) = match &title {
        TitleId::Movie(id) => match catalog.get_movie(id).await? {
            Some(movie) => (movie.title, movie.artwork),
            None => (id.0.clone(), Vec::new()),
        },
        TitleId::Episode(id) => match catalog.get_episode(id).await? {
            Some(episode) => (episode.title, episode.artwork),
            None => (id.0.clone(), Vec::new()),
        },
    };
    Ok(Some(ResumeCard {
        title,
        display_title,
        artwork,
        duration_ms,
        progress_percent: progress_percent(position_ms, duration_ms),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockCatalogRepo;
    use domain::catalog::{ArtworkId, ArtworkRef, Episode, EpisodeId, Movie, MovieId, Version};
    use domain::common::Quality;
    use domain::library::LibraryId;
    use domain::metadata::ArtworkKind;
    use jiff::Timestamp;

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
            edition: None,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn poster() -> ArtworkRef {
        ArtworkRef {
            id: ArtworkId("art-1".to_owned()),
            kind: ArtworkKind::Poster,
            widths: vec![180, 480],
        }
    }

    #[tokio::test]
    async fn builds_movie_card_with_title_artwork_and_percent() {
        let repo = MockCatalogRepo::new();
        repo.add_movie(Movie {
            id: MovieId("m1".to_owned()),
            title: "Alpha".to_owned(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
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
    async fn builds_episode_card() {
        let repo = MockCatalogRepo::new();
        repo.add_episode(Episode {
            id: EpisodeId("e1".to_owned()),
            season: domain::catalog::SeasonId("se1".to_owned()),
            number: 3,
            title: "Pilot".to_owned(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
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
