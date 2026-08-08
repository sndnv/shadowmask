use std::future::Future;

use crate::error::TranslationError;
use crate::media::{FetchedSubtitle, TranslationSpec};

pub trait TranslationProvider {
    fn translate(
        &self,
        request: &TranslationSpec,
    ) -> impl Future<Output = Result<FetchedSubtitle, TranslationError>> + Send;
}
