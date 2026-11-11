use std::future::Future;

use crate::error::FetchError;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FetchOptions {
    pub format: Option<String>,
    pub skip_note: Option<String>,
    pub prefer_h264: bool,
    pub site_subtitles: bool,
    pub chapters: bool,
}

#[derive(Debug, Clone)]
pub struct FetchSpec {
    pub url: String,
    pub dest_dir: String,
    pub filename_stem: String,
    pub options: FetchOptions,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SiteDetails {
    pub thumbnails: Vec<String>,
    pub upload_date: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FetchedMedia {
    pub path: String,
    pub size_bytes: u64,
    pub site: SiteDetails,
}

pub trait MediaFetcher {
    fn fetch(
        &self,
        request: &FetchSpec,
    ) -> impl Future<Output = Result<FetchedMedia, FetchError>> + Send;
}
