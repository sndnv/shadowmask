use std::future::Future;

use crate::error::FetchError;

#[derive(Debug, Clone)]
pub struct FetchRequest {
    pub url: String,
    pub dest_dir: String,
    pub filename_stem: String,
}

#[derive(Debug, Clone)]
pub struct FetchedMedia {
    pub path: String,
    pub size_bytes: u64,
}

pub trait MediaFetcher {
    fn fetch(
        &self,
        request: &FetchRequest,
    ) -> impl Future<Output = Result<FetchedMedia, FetchError>> + Send;
}
