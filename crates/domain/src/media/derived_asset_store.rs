use std::future::Future;

use crate::error::CacheError;
use crate::media::{DerivedAssetDir, DerivedAssetFile};

pub trait DerivedAssetStore: Send + Sync {
    fn label(&self) -> &'static str;
    fn list_dirs(&self) -> impl Future<Output = Result<Vec<DerivedAssetDir>, CacheError>> + Send;
    fn remove_dir(&self, owner: &str) -> impl Future<Output = Result<(), CacheError>> + Send;
    fn list_files(
        &self,
        owner: &str,
    ) -> impl Future<Output = Result<Vec<DerivedAssetFile>, CacheError>> + Send;
    fn remove_file(&self, path: &str) -> impl Future<Output = Result<(), CacheError>> + Send;
}
