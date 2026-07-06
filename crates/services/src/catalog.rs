use domain::catalog::{
    Collection, CollectionId, CollectionUpdate, Episode, EpisodeId, Movie, MovieId, NewCollection,
    Season, SeasonId, Series, SeriesId, TitleId, Version, VersionDetail, VersionId,
};
use domain::common::{Page, PageRequest};
use domain::error::CatalogError;
use domain::library::LibraryId;
use domain::metadata::ContentRating;
use domain::repository::{CatalogRepository, UserRepository};
use domain::service::CatalogService;
use domain::user::Principal;
use uuid::Uuid;

use crate::acl;
use crate::page::paginate;

const ALL: PageRequest = PageRequest {
    offset: 0,
    limit: u32::MAX,
};

#[derive(Clone)]
pub struct CatalogServiceImpl<C, U> {
    catalog: C,
    users: U,
}

impl<C, U> CatalogServiceImpl<C, U> {
    pub fn new(catalog: C, users: U) -> Self {
        Self { catalog, users }
    }
}

impl<C, U> CatalogServiceImpl<C, U>
where
    C: CatalogRepository + Sync,
    U: UserRepository + Sync,
{
    async fn rating_cap(&self, caller: &Principal) -> Result<Option<ContentRating>, CatalogError> {
        Ok(self
            .users
            .get(&caller.user)
            .await?
            .and_then(|user| user.max_content_rating))
    }

    async fn access_set(&self, caller: &Principal) -> Result<Vec<LibraryId>, CatalogError> {
        Ok(self
            .users
            .list_library_access(&caller.user)
            .await?
            .into_iter()
            .map(|entry| entry.library)
            .collect())
    }
}

impl<C, U> CatalogService for CatalogServiceImpl<C, U>
where
    C: CatalogRepository + Sync,
    U: UserRepository + Sync,
{
    async fn collections(
        &self,
        _caller: &Principal,
        page: PageRequest,
    ) -> Result<Page<Collection>, CatalogError> {
        Ok(self.catalog.list_collections(page).await?)
    }

    async fn collection(
        &self,
        _caller: &Principal,
        id: &CollectionId,
    ) -> Result<Collection, CatalogError> {
        self.catalog
            .get_collection(id)
            .await?
            .ok_or(CatalogError::NotFound)
    }

    async fn create_collection(
        &self,
        _caller: &Principal,
        input: NewCollection,
    ) -> Result<Collection, CatalogError> {
        let collection = Collection {
            id: CollectionId(Uuid::new_v4().to_string()),
            name: input.name,
            overview: input.overview,
            movies: input.movies,
        };
        self.catalog.upsert_collection(collection.clone()).await?;
        Ok(collection)
    }

    async fn update_collection(
        &self,
        _caller: &Principal,
        id: &CollectionId,
        update: CollectionUpdate,
    ) -> Result<Collection, CatalogError> {
        self.catalog
            .get_collection(id)
            .await?
            .ok_or(CatalogError::NotFound)?;
        let collection = Collection {
            id: id.clone(),
            name: update.name,
            overview: update.overview,
            movies: update.movies,
        };
        self.catalog.upsert_collection(collection.clone()).await?;
        Ok(collection)
    }

    async fn delete_collection(
        &self,
        _caller: &Principal,
        id: &CollectionId,
    ) -> Result<(), CatalogError> {
        self.catalog
            .get_collection(id)
            .await?
            .ok_or(CatalogError::NotFound)?;
        self.catalog.delete_collection(id).await?;
        Ok(())
    }

    async fn movies(
        &self,
        caller: &Principal,
        page: PageRequest,
    ) -> Result<Page<Movie>, CatalogError> {
        if acl::is_admin(caller) {
            return Ok(self.catalog.list_movies(page).await?);
        }
        let cap = self.rating_cap(caller).await?;
        let filtered: Vec<Movie> = self
            .catalog
            .list_movies(ALL)
            .await?
            .items
            .into_iter()
            .filter(|movie| acl::rating_permits(cap.as_ref(), movie.content_rating.as_ref()))
            .collect();
        Ok(paginate(&filtered, page))
    }

    async fn movie(&self, caller: &Principal, id: &MovieId) -> Result<Movie, CatalogError> {
        let movie = self
            .catalog
            .get_movie(id)
            .await?
            .ok_or(CatalogError::NotFound)?;
        if !acl::is_admin(caller) {
            let cap = self.rating_cap(caller).await?;
            if !acl::rating_permits(cap.as_ref(), movie.content_rating.as_ref()) {
                return Err(CatalogError::NotFound);
            }
        }
        Ok(movie)
    }

    async fn series(
        &self,
        caller: &Principal,
        page: PageRequest,
    ) -> Result<Page<Series>, CatalogError> {
        if acl::is_admin(caller) {
            return Ok(self.catalog.list_series(page).await?);
        }
        let cap = self.rating_cap(caller).await?;
        let filtered: Vec<Series> = self
            .catalog
            .list_series(ALL)
            .await?
            .items
            .into_iter()
            .filter(|series| acl::rating_permits(cap.as_ref(), series.content_rating.as_ref()))
            .collect();
        Ok(paginate(&filtered, page))
    }

    async fn series_detail(
        &self,
        caller: &Principal,
        id: &SeriesId,
    ) -> Result<Series, CatalogError> {
        let series = self
            .catalog
            .get_series(id)
            .await?
            .ok_or(CatalogError::NotFound)?;
        if !acl::is_admin(caller) {
            let cap = self.rating_cap(caller).await?;
            if !acl::rating_permits(cap.as_ref(), series.content_rating.as_ref()) {
                return Err(CatalogError::NotFound);
            }
        }
        Ok(series)
    }

    async fn seasons(
        &self,
        _caller: &Principal,
        series: &SeriesId,
    ) -> Result<Vec<Season>, CatalogError> {
        Ok(self.catalog.list_seasons(series).await?)
    }

    async fn season(&self, _caller: &Principal, id: &SeasonId) -> Result<Season, CatalogError> {
        self.catalog
            .get_season(id)
            .await?
            .ok_or(CatalogError::NotFound)
    }

    async fn episodes(
        &self,
        _caller: &Principal,
        season: &SeasonId,
    ) -> Result<Vec<Episode>, CatalogError> {
        Ok(self.catalog.list_episodes(season).await?)
    }

    async fn episode(&self, _caller: &Principal, id: &EpisodeId) -> Result<Episode, CatalogError> {
        self.catalog
            .get_episode(id)
            .await?
            .ok_or(CatalogError::NotFound)
    }

    async fn versions(
        &self,
        caller: &Principal,
        title: &TitleId,
        page: PageRequest,
    ) -> Result<Page<Version>, CatalogError> {
        if acl::is_admin(caller) {
            return Ok(self.catalog.list_versions(title, page).await?);
        }
        let access = self.access_set(caller).await?;
        let filtered: Vec<Version> = self
            .catalog
            .list_versions(title, ALL)
            .await?
            .items
            .into_iter()
            .filter(|version| acl::can_access_library(&access, &version.library))
            .collect();
        Ok(paginate(&filtered, page))
    }

    async fn library_versions(
        &self,
        caller: &Principal,
        library: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<Version>, CatalogError> {
        if !acl::is_admin(caller) {
            let access = self.access_set(caller).await?;
            if !acl::can_access_library(&access, library) {
                return Ok(paginate(&Vec::<Version>::new(), page));
            }
        }
        Ok(self.catalog.list_library_versions(library, page).await?)
    }

    async fn version(
        &self,
        caller: &Principal,
        id: &VersionId,
    ) -> Result<VersionDetail, CatalogError> {
        let detail = self
            .catalog
            .version_detail(id)
            .await?
            .ok_or(CatalogError::NotFound)?;
        if !acl::is_admin(caller) {
            let access = self.access_set(caller).await?;
            if !acl::can_access_library(&access, &detail.version.library) {
                return Err(CatalogError::NotFound);
            }
        }
        Ok(detail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockCatalogRepo, MockUserRepo};
    use domain::common::Quality;
    use domain::user::{Role, User, UserId};
    use jiff::Timestamp;

    fn admin() -> Principal {
        Principal {
            user: UserId("admin".into()),
            role: Role::Admin,
        }
    }

    fn member() -> Principal {
        Principal {
            user: UserId("u1".into()),
            role: Role::User,
        }
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    fn rating(code: &str) -> ContentRating {
        ContentRating {
            system: "MPAA".into(),
            code: code.into(),
        }
    }

    fn movie(id: &str, code: Option<&str>) -> Movie {
        Movie {
            id: MovieId(id.into()),
            title: id.into(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: code.map(rating),
            added_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn version(id: &str, lib: &str) -> Version {
        Version {
            id: VersionId(id.into()),
            title: TitleId::Movie(MovieId("m1".into())),
            library: LibraryId(lib.into()),
            quality: Quality::Hd,
            container: "mkv".into(),
            path: format!("/media/{id}.mkv"),
            size_bytes: 1,
            duration_ms: 1000,
            edition: None,
        }
    }

    fn capped_user() -> User {
        User {
            id: UserId("u1".into()),
            username: "u1".into(),
            password_hash: "hash".into(),
            role: Role::User,
            max_content_rating: Some(rating("PG-13")),
            preferred_audio: Vec::new(),
            preferred_subtitle: Vec::new(),
            concurrent_stream_limit: None,
            bitrate_cap: None,
            created_at: Timestamp::UNIX_EPOCH,
        }
    }

    async fn seeded() -> CatalogServiceImpl<MockCatalogRepo, MockUserRepo> {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", Some("PG-13")));
        catalog.add_movie(movie("m2", Some("R")));
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "s1".into(),
            year: None,
            overview: None,
            content_rating: Some(ContentRating {
                system: "US-TV".into(),
                code: "TV-MA".into(),
            }),
            added_at: Timestamp::UNIX_EPOCH,
        });
        catalog.add_series(Series {
            id: SeriesId("s2".into()),
            title: "s2".into(),
            year: None,
            overview: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH,
        });
        catalog.add_season(Season {
            id: SeasonId("se1".into()),
            series: SeriesId("s1".into()),
            number: 1,
            title: None,
            overview: None,
        });
        catalog.add_episode(Episode {
            id: EpisodeId("e1".into()),
            season: SeasonId("se1".into()),
            number: 1,
            title: "e1".into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            added_at: Timestamp::UNIX_EPOCH,
        });
        catalog.add_version(version("v1", "lib1"));
        catalog.add_version(version("v2", "lib2"));

        let users = MockUserRepo::new();
        users.insert(capped_user());
        users
            .set_library_access(&UserId("u1".into()), &[LibraryId("lib1".into())])
            .await
            .unwrap();
        CatalogServiceImpl::new(catalog, users)
    }

    #[tokio::test]
    async fn admin_bypasses_rating_and_library_gates() {
        let svc = seeded().await;
        assert_eq!(svc.movies(&admin(), page()).await.unwrap().total, 2);
        assert!(svc.movie(&admin(), &MovieId("m2".into())).await.is_ok());
        assert_eq!(svc.series(&admin(), page()).await.unwrap().total, 2);
        assert!(
            svc.series_detail(&admin(), &SeriesId("s1".into()))
                .await
                .is_ok()
        );
        assert_eq!(
            svc.versions(&admin(), &TitleId::Movie(MovieId("m1".into())), page())
                .await
                .unwrap()
                .total,
            2
        );
        assert_eq!(
            svc.library_versions(&admin(), &LibraryId("lib2".into()), page())
                .await
                .unwrap()
                .total,
            1
        );
        assert!(svc.version(&admin(), &VersionId("v2".into())).await.is_ok());
    }

    #[tokio::test]
    async fn member_rating_gate_filters_and_hides() {
        let svc = seeded().await;
        let movies = svc.movies(&member(), page()).await.unwrap();
        assert_eq!(movies.total, 1);
        assert_eq!(movies.items[0].id, MovieId("m1".into()));
        assert!(svc.movie(&member(), &MovieId("m1".into())).await.is_ok());
        assert!(matches!(
            svc.movie(&member(), &MovieId("m2".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));

        let series = svc.series(&member(), page()).await.unwrap();
        assert_eq!(series.total, 1);
        assert_eq!(series.items[0].id, SeriesId("s2".into()));
        assert!(
            svc.series_detail(&member(), &SeriesId("s2".into()))
                .await
                .is_ok()
        );
        assert!(matches!(
            svc.series_detail(&member(), &SeriesId("s1".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn member_library_gate_filters_versions() {
        let svc = seeded().await;
        let title = TitleId::Movie(MovieId("m1".into()));
        let versions = svc.versions(&member(), &title, page()).await.unwrap();
        assert_eq!(versions.total, 1);
        assert_eq!(versions.items[0].id, VersionId("v1".into()));

        assert_eq!(
            svc.library_versions(&member(), &LibraryId("lib1".into()), page())
                .await
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            svc.library_versions(&member(), &LibraryId("lib2".into()), page())
                .await
                .unwrap()
                .total,
            0
        );

        assert!(
            svc.version(&member(), &VersionId("v1".into()))
                .await
                .is_ok()
        );
        assert!(matches!(
            svc.version(&member(), &VersionId("v2".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn seasons_episodes_and_missing() {
        let svc = seeded().await;
        assert_eq!(
            svc.seasons(&member(), &SeriesId("s1".into()))
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(svc.season(&member(), &SeasonId("se1".into())).await.is_ok());
        assert!(matches!(
            svc.season(&member(), &SeasonId("x".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
        assert_eq!(
            svc.episodes(&member(), &SeasonId("se1".into()))
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(
            svc.episode(&member(), &EpisodeId("e1".into()))
                .await
                .is_ok()
        );
        assert!(matches!(
            svc.episode(&member(), &EpisodeId("x".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
        assert!(matches!(
            svc.movie(&member(), &MovieId("x".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
        assert!(matches!(
            svc.version(&member(), &VersionId("x".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn collection_create_update_delete() {
        let svc = seeded().await;
        let created = svc
            .create_collection(
                &admin(),
                NewCollection {
                    name: "Saga".into(),
                    overview: Some("epic".into()),
                    movies: vec![MovieId("m1".into())],
                },
            )
            .await
            .unwrap();
        assert!(Uuid::parse_str(&created.id.0).is_ok());
        assert_eq!(svc.collections(&admin(), page()).await.unwrap().total, 1);
        assert!(svc.collection(&admin(), &created.id).await.is_ok());
        assert!(matches!(
            svc.collection(&admin(), &CollectionId("x".into()))
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));

        let updated = svc
            .update_collection(
                &admin(),
                &created.id,
                CollectionUpdate {
                    name: "Saga II".into(),
                    overview: None,
                    movies: Vec::new(),
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "Saga II");
        assert!(matches!(
            svc.update_collection(
                &admin(),
                &CollectionId("x".into()),
                CollectionUpdate {
                    name: "x".into(),
                    overview: None,
                    movies: Vec::new(),
                },
            )
            .await
            .unwrap_err(),
            CatalogError::NotFound
        ));

        svc.delete_collection(&admin(), &created.id).await.unwrap();
        assert!(matches!(
            svc.delete_collection(&admin(), &created.id)
                .await
                .unwrap_err(),
            CatalogError::NotFound
        ));
    }

    #[tokio::test]
    async fn backend_errors_propagate() {
        let catalog = MockCatalogRepo::new();
        catalog.set_fail();
        let svc = CatalogServiceImpl::new(catalog, MockUserRepo::new());
        assert!(matches!(
            svc.movies(&admin(), page()).await.unwrap_err(),
            CatalogError::Repository(_)
        ));
        assert!(matches!(
            svc.collections(&admin(), page()).await.unwrap_err(),
            CatalogError::Repository(_)
        ));
    }
}
