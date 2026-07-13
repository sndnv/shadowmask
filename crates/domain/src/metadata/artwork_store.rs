use std::future::Future;
use std::path::PathBuf;

use crate::catalog::ArtworkId;
use crate::error::ArtworkError;

pub trait ArtworkStore {
    fn store(
        &self,
        id: &ArtworkId,
        width: u32,
        bytes: &[u8],
    ) -> impl Future<Output = Result<(), ArtworkError>> + Send;

    fn path_for(&self, id: &ArtworkId, width: u32) -> PathBuf;
}
