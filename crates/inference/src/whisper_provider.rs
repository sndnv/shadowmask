use domain::error::TranscriptionError;
use domain::media::{
    FetchedSubtitle, SubtitleFormat, TranscriptionProvider, TranscriptionRequest,
    is_hallucinated_text,
};
use media::transcode::ProcessSpawner;

use crate::WhisperEngine;
use crate::audio::decode_audio;
use crate::vtt::segments_to_vtt;

pub struct WhisperProvider<E, S> {
    engine: E,
    spawner: S,
}

impl<E, S> WhisperProvider<E, S> {
    pub fn new(engine: E, spawner: S) -> Self {
        Self { engine, spawner }
    }
}

impl<E, S> TranscriptionProvider for WhisperProvider<E, S>
where
    E: WhisperEngine + Send + Sync,
    S: ProcessSpawner,
{
    async fn transcribe(
        &self,
        request: &TranscriptionRequest,
    ) -> Result<FetchedSubtitle, TranscriptionError> {
        let samples = decode_audio(
            &self.spawner,
            "ffmpeg",
            &request.audio_path,
            request.audio_track_index,
        )
        .await?;
        let language = request.source_language.as_ref().map(|code| code.0.clone());
        let mut segments = self.engine.transcribe(samples, language).await?;
        segments.retain(|segment| !is_hallucinated_text(&segment.text));
        Ok(FetchedSubtitle {
            content: segments_to_vtt(&segments),
            format: SubtitleFormat::Vtt,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use crate::Segment;

    use super::*;

    struct MockSpawner {
        ok: bool,
    }

    impl ProcessSpawner for MockSpawner {
        async fn run(&self, _program: &str, args: &[String]) -> io::Result<bool> {
            if self.ok {
                let path = args.last().expect("output path arg");
                std::fs::write(path, [0x00, 0x40]).expect("write mock pcm");
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    struct MockEngine {
        result: Result<Vec<Segment>, String>,
    }

    impl WhisperEngine for MockEngine {
        async fn transcribe(
            &self,
            _samples: Vec<f32>,
            _language: Option<String>,
        ) -> Result<Vec<Segment>, TranscriptionError> {
            self.result.clone().map_err(TranscriptionError::Backend)
        }
    }

    fn request() -> TranscriptionRequest {
        TranscriptionRequest {
            audio_path: "/media/v1.mkv".to_owned(),
            source_language: None,
            audio_track_index: None,
        }
    }

    #[tokio::test]
    async fn produces_vtt_from_engine_segments() {
        let provider = WhisperProvider::new(
            MockEngine {
                result: Ok(vec![Segment {
                    start_ms: 0,
                    end_ms: 1000,
                    text: "hello".to_owned(),
                }]),
            },
            MockSpawner { ok: true },
        );

        let subtitle = provider.transcribe(&request()).await.unwrap();

        assert_eq!(subtitle.format, SubtitleFormat::Vtt);
        assert!(subtitle.content.starts_with("WEBVTT"));
        assert!(subtitle.content.contains("hello"));
    }

    #[tokio::test]
    async fn drops_hallucinated_segments() {
        let provider = WhisperProvider::new(
            MockEngine {
                result: Ok(vec![
                    Segment {
                        start_ms: 0,
                        end_ms: 1000,
                        text: "Real dialogue here".to_owned(),
                    },
                    Segment {
                        start_ms: 1000,
                        end_ms: 2000,
                        text: "Thanks for watching!".to_owned(),
                    },
                    Segment {
                        start_ms: 2000,
                        end_ms: 3000,
                        text: "the the the the the the the".to_owned(),
                    },
                ]),
            },
            MockSpawner { ok: true },
        );

        let subtitle = provider.transcribe(&request()).await.unwrap();

        assert!(subtitle.content.contains("Real dialogue here"));
        assert!(!subtitle.content.contains("Thanks for watching"));
        assert!(!subtitle.content.contains("the the the"));
    }

    #[tokio::test]
    async fn all_hallucinated_segments_yield_empty_subtitle() {
        let provider = WhisperProvider::new(
            MockEngine {
                result: Ok(vec![Segment {
                    start_ms: 0,
                    end_ms: 1000,
                    text: "Subtitles by TeamWhatever".to_owned(),
                }]),
            },
            MockSpawner { ok: true },
        );

        let subtitle = provider.transcribe(&request()).await.unwrap();

        assert!(!subtitle.has_text());
    }

    #[tokio::test]
    async fn propagates_decode_error() {
        let provider =
            WhisperProvider::new(MockEngine { result: Ok(vec![]) }, MockSpawner { ok: false });
        assert!(matches!(
            provider.transcribe(&request()).await.unwrap_err(),
            TranscriptionError::Backend(_)
        ));
    }

    #[tokio::test]
    async fn propagates_engine_error() {
        let provider = WhisperProvider::new(
            MockEngine {
                result: Err("boom".to_owned()),
            },
            MockSpawner { ok: true },
        );
        assert!(matches!(
            provider.transcribe(&request()).await.unwrap_err(),
            TranscriptionError::Backend(_)
        ));
    }
}
