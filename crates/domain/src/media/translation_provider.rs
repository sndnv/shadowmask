use std::future::Future;

use crate::error::TranslationError;
use crate::media::{FetchedSubtitle, TranslationRequest};

pub trait TranslationProvider {
    fn translate(
        &self,
        request: &TranslationRequest,
    ) -> impl Future<Output = Result<FetchedSubtitle, TranslationError>> + Send;
}
