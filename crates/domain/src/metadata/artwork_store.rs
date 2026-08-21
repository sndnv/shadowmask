use std::future::Future;
use std::path::PathBuf;

use crate::catalog::ArtworkId;
use crate::error::ArtworkError;
use crate::metadata::{ArtworkFormat, ProcessedArtwork};

pub trait ArtworkStore {
    fn store(
        &self,
        id: &ArtworkId,
        width: u32,
        art: &ProcessedArtwork,
    ) -> impl Future<Output = Result<(), ArtworkError>> + Send;

    fn path_for(&self, id: &ArtworkId, width: u32, format: ArtworkFormat) -> PathBuf;
}
