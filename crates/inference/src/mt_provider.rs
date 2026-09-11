use domain::error::TranslationError;
use domain::media::{
    FetchedSubtitle, SubtitleFormat, TranslationProvider, TranslationSpec, is_hallucinated_text,
};
use subtp::srt::SubRip;
use subtp::vtt::{VttBlock, WebVtt};

use crate::TranslationEngine;

pub struct MtProvider<E> {
    engine: E,
}

impl<E> MtProvider<E> {
    pub fn new(engine: E) -> Self {
        Self { engine }
    }
}

impl<E> MtProvider<E>
where
    E: TranslationEngine + Send + Sync,
{
    async fn translate_texts(
        &self,
        texts: Vec<String>,
        request: &TranslationSpec,
    ) -> Result<Vec<String>, TranslationError> {
        let source = request.source_language.as_ref().map(|code| code.0.clone());
        let target = request.target_language.0.clone();
        self.engine.translate(texts, source, target).await
    }

    async fn translate_vtt(
        &self,
        request: &TranslationSpec,
    ) -> Result<FetchedSubtitle, TranslationError> {
        let mut vtt = WebVtt::parse(request.content.as_str())
            .map_err(|e| TranslationError::Backend(format!("parse vtt: {e}")))?;
        let texts: Vec<String> = vtt
            .blocks
            .iter()
            .filter_map(|block| match block {
                VttBlock::Que(cue) => Some(cue.payload.join("\n")),
                _ => None,
            })
            .collect();
        let translated = self.translate_texts(texts, request).await?;
        let cues = vtt.blocks.iter_mut().filter_map(|block| match block {
            VttBlock::Que(cue) => Some(cue),
            _ => None,
        });
        for (cue, text) in cues.zip(translated) {
            cue.payload = split_lines(&text);
        }
        vtt.blocks.retain(|block| match block {
            VttBlock::Que(cue) => !is_hallucinated_text(&cue.payload.join(" ")),
            _ => true,
        });
        Ok(FetchedSubtitle { content: vtt.render(), format: SubtitleFormat::Vtt })
    }

    async fn translate_srt(
        &self,
        request: &TranslationSpec,
    ) -> Result<FetchedSubtitle, TranslationError> {
        let mut srt = SubRip::parse(request.content.as_str())
            .map_err(|e| TranslationError::Backend(format!("parse srt: {e}")))?;
        let texts: Vec<String> = srt.subtitles.iter().map(|s| s.text.join("\n")).collect();
        let translated = self.translate_texts(texts, request).await?;
        for (subtitle, text) in srt.subtitles.iter_mut().zip(translated) {
            subtitle.text = split_lines(&text);
        }
        srt.subtitles.retain(|subtitle| !is_hallucinated_text(&subtitle.text.join(" ")));
        Ok(FetchedSubtitle { content: srt.render(), format: SubtitleFormat::Srt })
    }
}

impl<E> TranslationProvider for MtProvider<E>
where
    E: TranslationEngine + Send + Sync,
{
    async fn translate(
        &self,
        request: &TranslationSpec,
    ) -> Result<FetchedSubtitle, TranslationError> {
        match request.format {
            SubtitleFormat::Vtt => self.translate_vtt(request).await,
            SubtitleFormat::Srt => self.translate_srt(request).await,
            other => Err(TranslationError::Unsupported(format!(
                "unsupported subtitle format: {other:?}"
            ))),
        }
    }
}

fn split_lines(text: &str) -> Vec<String> {
    text.lines().map(|line| line.to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use domain::common::LanguageCode;

    use super::*;

    enum EngineMode {
        Echo,
        Hallucinate,
        Fail,
    }

    struct MockEngine {
        mode: EngineMode,
    }

    impl TranslationEngine for MockEngine {
        async fn translate(
            &self,
            texts: Vec<String>,
            _source_language: Option<String>,
            target_language: String,
        ) -> Result<Vec<String>, TranslationError> {
            match self.mode {
                EngineMode::Echo => {
                    Ok(texts.into_iter().map(|text| format!("{target_language}:{text}")).collect())
                }
                EngineMode::Hallucinate => {
                    Ok(texts.into_iter().map(|_| "Thanks for watching".into()).collect())
                }
                EngineMode::Fail => Err(TranslationError::Backend("boom".into())),
            }
        }
    }

    fn request(content: &str, format: SubtitleFormat, source: Option<&str>) -> TranslationSpec {
        TranslationSpec {
            content: content.to_owned(),
            format,
            source_language: source.map(|code| LanguageCode(code.into())),
            target_language: LanguageCode("fr".into()),
        }
    }

    #[tokio::test]
    async fn translates_vtt_cue_text_and_preserves_structure() {
        let content = "WEBVTT\n\nNOTE a comment\n\n00:00:00.000 --> 00:00:01.000\nhello\n";
        let provider = MtProvider::new(MockEngine { mode: EngineMode::Echo });

        let subtitle =
            provider.translate(&request(content, SubtitleFormat::Vtt, Some("en"))).await.unwrap();

        assert_eq!(subtitle.format, SubtitleFormat::Vtt);
        assert!(subtitle.content.contains("WEBVTT"));
        assert!(subtitle.content.contains("fr:hello"));
        assert!(subtitle.content.contains("00:00:00.000 --> 00:00:01.000"));
    }

    #[tokio::test]
    async fn translates_srt_cue_text() {
        let content = "1\n00:00:00,000 --> 00:00:01,000\nhello\n";
        let provider = MtProvider::new(MockEngine { mode: EngineMode::Echo });

        let subtitle =
            provider.translate(&request(content, SubtitleFormat::Srt, None)).await.unwrap();

        assert_eq!(subtitle.format, SubtitleFormat::Srt);
        assert!(subtitle.content.contains("fr:hello"));
    }

    #[tokio::test]
    async fn drops_hallucinated_translated_vtt_cues() {
        let content = "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nhello\n";
        let provider = MtProvider::new(MockEngine { mode: EngineMode::Hallucinate });

        let subtitle =
            provider.translate(&request(content, SubtitleFormat::Vtt, Some("en"))).await.unwrap();

        assert!(!subtitle.has_text());
    }

    #[tokio::test]
    async fn drops_hallucinated_translated_srt_cues() {
        let content = "1\n00:00:00,000 --> 00:00:01,000\nhello\n";
        let provider = MtProvider::new(MockEngine { mode: EngineMode::Hallucinate });

        let subtitle =
            provider.translate(&request(content, SubtitleFormat::Srt, None)).await.unwrap();

        assert!(!subtitle.has_text());
    }

    #[tokio::test]
    async fn engine_failure_is_backend() {
        let content = "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nhello\n";
        let provider = MtProvider::new(MockEngine { mode: EngineMode::Fail });

        assert!(matches!(
            provider
                .translate(&request(content, SubtitleFormat::Vtt, Some("en")))
                .await
                .unwrap_err(),
            TranslationError::Backend(_)
        ));
    }

    #[tokio::test]
    async fn unsupported_format_is_unsupported() {
        let provider = MtProvider::new(MockEngine { mode: EngineMode::Echo });

        assert!(matches!(
            provider.translate(&request("garbage", SubtitleFormat::Ass, None)).await.unwrap_err(),
            TranslationError::Unsupported(_)
        ));
    }

    #[tokio::test]
    async fn malformed_vtt_is_backend() {
        let provider = MtProvider::new(MockEngine { mode: EngineMode::Echo });

        assert!(matches!(
            provider
                .translate(&request("@@@ not a vtt @@@", SubtitleFormat::Vtt, None))
                .await
                .unwrap_err(),
            TranslationError::Backend(_)
        ));
    }

    #[tokio::test]
    async fn malformed_srt_is_backend() {
        let provider = MtProvider::new(MockEngine { mode: EngineMode::Echo });

        assert!(matches!(
            provider
                .translate(&request("@@@ not an srt @@@", SubtitleFormat::Srt, None))
                .await
                .unwrap_err(),
            TranslationError::Backend(_)
        ));
    }
}
