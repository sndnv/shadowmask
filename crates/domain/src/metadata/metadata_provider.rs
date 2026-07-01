use std::future::Future;

use crate::error::MetadataError;
use crate::metadata::{ExternalId, MetadataMatch, MetadataQuery, TitleMetadata};

pub trait MetadataProvider {
    fn search(
        &self,
        query: &MetadataQuery,
    ) -> impl Future<Output = Result<Vec<MetadataMatch>, MetadataError>> + Send;

    fn fetch(
        &self,
        id: &ExternalId,
    ) -> impl Future<Output = Result<TitleMetadata, MetadataError>> + Send;
}
