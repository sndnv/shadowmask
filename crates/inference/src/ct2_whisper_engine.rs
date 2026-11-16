use std::path::PathBuf;

use ct2rs::Config;
use ct2rs::tokenizers::hf::Tokenizer;
use ct2rs_sys::{StorageView, Whisper, WhisperOptions};
use domain::error::TranscriptionError;

use crate::whisper_loop::{
    Attempt, Vocabulary, WINDOW_FRAMES, WindowDecoder, timestamp_token, transcribe,
};
use crate::whisper_mel::Features;
use crate::{Segment, WhisperEngine};

const SAMPLED_HYPOTHESES: usize = 5;

pub struct Ct2WhisperEngine {
    model_path: PathBuf,
    threads: usize,
}

impl Ct2WhisperEngine {
    pub fn new(model_path: impl Into<PathBuf>, threads: usize) -> Self {
        Self { model_path: model_path.into(), threads }
    }
}

impl WhisperEngine for Ct2WhisperEngine {
    async fn transcribe(
        &self,
        samples: Vec<f32>,
        language: Option<String>,
    ) -> Result<Vec<Segment>, TranscriptionError> {
        let model_path = self.model_path.clone();
        let threads = self.threads;
        #[rustfmt::skip]
        tracing::info!("loading the transcription model at [{}] with [{}] thread(s)", model_path.display(), threads);
        tokio::task::spawn_blocking(move || {
            let config = Config { num_threads_per_replica: threads, ..Config::default() };
            let whisper = Whisper::new(&model_path, config).map_err(backend)?;
            let tokenizer = Tokenizer::new(&model_path).map_err(backend)?;
            let vocabulary = Vocabulary {
                start_of_prev: token_id(&tokenizer, "<|startofprev|>")?,
                start_of_transcript: token_id(&tokenizer, "<|startoftranscript|>")?,
                transcribe: token_id(&tokenizer, "<|transcribe|>")?,
                end_of_text: token_id(&tokenizer, "<|endoftext|>")?,
                timestamp_begin: token_id(&tokenizer, "<|notimestamps|>")? + 1,
                multilingual: whisper.is_multilingual(),
            };
            let language = match language {
                Some(code) if vocabulary.multilingual => {
                    Some(token_id(&tokenizer, &format!("<|{code}|>"))?)
                }
                _ => None,
            };
            let features = Features::new(&samples, whisper.n_mels());
            let mut decoder = Ct2Decoder {
                whisper: &whisper,
                tokenizer: &tokenizer,
                features: &features,
                timestamp_begin: vocabulary.timestamp_begin,
                encoded: None,
            };
            transcribe(&mut decoder, &vocabulary, features.frames(), language)
        })
        .await
        .map_err(backend)?
    }
}

struct Ct2Decoder<'a> {
    whisper: &'a Whisper,
    tokenizer: &'a Tokenizer,
    features: &'a Features<'a>,
    timestamp_begin: u32,
    encoded: Option<StorageView<'static>>,
}

impl Ct2Decoder<'_> {
    fn encoded(&self) -> Result<&StorageView<'static>, TranscriptionError> {
        self.encoded.as_ref().ok_or_else(|| backend("no audio window is loaded"))
    }

    fn token(&self, id: u32) -> Result<String, TranscriptionError> {
        match id.checked_sub(self.timestamp_begin) {
            Some(position) => Ok(timestamp_token(position)),
            None => self
                .tokenizer
                .id_to_token(id)
                .ok_or_else(|| backend(format!("the transcription model has no token [{id}]"))),
        }
    }
}

impl WindowDecoder for Ct2Decoder<'_> {
    type Error = TranscriptionError;

    fn load(&mut self, start_frame: usize) -> Result<(), TranscriptionError> {
        let mut window = self.features.window(start_frame);
        let view = window_view(&mut window, self.whisper.n_mels())?;
        self.encoded = Some(self.whisper.encode(&view, false).map_err(backend)?);
        Ok(())
    }

    fn detect_language(&mut self) -> Result<u32, TranscriptionError> {
        let detected = self.whisper.detect_language(self.encoded()?).map_err(backend)?;
        let best = detected
            .into_iter()
            .next()
            .and_then(|languages| languages.into_iter().next())
            .ok_or_else(|| backend("the transcription model detected no language"))?;
        token_id(self.tokenizer, &best.language)
    }

    fn decode(&mut self, prompt: &[u32], temperature: f32) -> Result<Attempt, TranscriptionError> {
        let prompt = prompt.iter().map(|id| self.token(*id)).collect::<Result<Vec<_>, _>>()?;
        let result = self
            .whisper
            .generate(self.encoded()?, &[prompt], &options(temperature))
            .map_err(backend)?
            .into_iter()
            .next()
            .ok_or_else(|| backend("the transcription model returned no result"))?;
        let no_hypothesis = || backend("the transcription model returned no hypothesis");
        let tokens = result.sequences_ids.first().ok_or_else(no_hypothesis)?;
        let score = result.scores.first().ok_or_else(no_hypothesis)?;
        Ok(Attempt {
            tokens: tokens.iter().map(|id| *id as u32).collect(),
            score: *score,
            no_speech_prob: result.no_speech_prob,
        })
    }

    fn text(&self, tokens: &[u32]) -> Result<String, TranscriptionError> {
        self.tokenizer.decode(tokens, false).map_err(backend)
    }
}

fn options(temperature: f32) -> WhisperOptions {
    let beam_search = WhisperOptions {
        return_scores: true,
        return_no_speech_prob: true,
        ..WhisperOptions::default()
    };
    if temperature > 0.0 {
        WhisperOptions {
            beam_size: 1,
            num_hypotheses: SAMPLED_HYPOTHESES,
            sampling_topk: 0,
            sampling_temperature: temperature,
            ..beam_search
        }
    } else {
        beam_search
    }
}

fn window_view(window: &mut [f32], mels: usize) -> Result<StorageView<'_>, TranscriptionError> {
    StorageView::new(&[1, mels, WINDOW_FRAMES], window, Default::default()).map_err(backend)
}

fn token_id(tokenizer: &Tokenizer, token: &str) -> Result<u32, TranscriptionError> {
    tokenizer
        .token_to_id(token)
        .ok_or_else(|| backend(format!("the transcription model has no token [{token}]")))
}

fn backend(error: impl ToString) -> TranscriptionError {
    TranscriptionError::Backend(error.to_string())
}
