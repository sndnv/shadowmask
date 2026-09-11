use domain::error::TranslationError;
use domain::media::{FetchedSubtitle, TranslationProvider, TranslationSpec};

#[derive(Debug, Default, Clone, Copy)]
pub struct DisabledTranslationProvider;

impl TranslationProvider for DisabledTranslationProvider {
    async fn translate(
        &self,
        _request: &TranslationSpec,
    ) -> Result<FetchedSubtitle, TranslationError> {
        Err(TranslationError::Unsupported("translation feature not built".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use domain::common::LanguageCode;
    use domain::media::SubtitleFormat;

    use super::*;

    #[tokio::test]
    async fn translate_is_unsupported() {
        let request = TranslationSpec {
            content: "WEBVTT\n".to_owned(),
            format: SubtitleFormat::Vtt,
            source_language: Some(LanguageCode("en".into())),
            target_language: LanguageCode("fr".into()),
        };
        let err = DisabledTranslationProvider.translate(&request).await.unwrap_err();
        assert!(matches!(err, TranslationError::Unsupported(_)));
    }
}
