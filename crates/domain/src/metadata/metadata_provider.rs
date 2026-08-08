use std::future::Future;

use crate::error::MetadataError;
use crate::metadata::{
    ExternalId, MetadataMatch, MetadataQuery, PersonMetadata, SeasonArtwork, TitleMetadata,
};

pub trait MetadataProvider {
    fn search(
        &self,
        query: &MetadataQuery,
    ) -> impl Future<Output = Result<Vec<MetadataMatch>, MetadataError>> + Send;

    fn fetch(
        &self,
        id: &ExternalId,
    ) -> impl Future<Output = Result<TitleMetadata, MetadataError>> + Send;

    fn fetch_season(
        &self,
        _id: &ExternalId,
        _season: u16,
    ) -> impl Future<Output = Result<SeasonArtwork, MetadataError>> + Send {
        async { Err(MetadataError::NotFound) }
    }

    fn fetch_person(
        &self,
        _id: &ExternalId,
    ) -> impl Future<Output = Result<PersonMetadata, MetadataError>> + Send {
        async { Err(MetadataError::NotFound) }
    }
}
