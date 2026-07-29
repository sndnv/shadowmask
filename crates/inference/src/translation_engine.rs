use std::future::Future;

use domain::error::TranslationError;

pub trait TranslationEngine {
    fn translate(
        &self,
        texts: Vec<String>,
        source_language: Option<String>,
        target_language: String,
    ) -> impl Future<Output = Result<Vec<String>, TranslationError>> + Send;
}
