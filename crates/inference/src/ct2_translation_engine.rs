use std::path::PathBuf;

use ct2rs::{Config, TranslationOptions, Translator};
use domain::error::TranslationError;

use crate::TranslationEngine;
use crate::language_prefix::render_language_prefix;

const MAX_BATCH_SIZE: usize = 32;

pub struct Ct2TranslationEngine {
    model_path: PathBuf,
    source_prefix: Option<String>,
    target_prefix: Option<String>,
    threads: usize,
}

impl Ct2TranslationEngine {
    pub fn new(
        model_path: impl Into<PathBuf>,
        source_prefix: Option<String>,
        target_prefix: Option<String>,
        threads: usize,
    ) -> Self {
        Self { model_path: model_path.into(), source_prefix, target_prefix, threads }
    }
}

impl TranslationEngine for Ct2TranslationEngine {
    async fn translate(
        &self,
        texts: Vec<String>,
        source_language: Option<String>,
        target_language: String,
    ) -> Result<Vec<String>, TranslationError> {
        let model_path = self.model_path.clone();
        let threads = self.threads;
        let source_prefix = self.source_prefix.as_deref().map(|template| {
            render_language_prefix(template, source_language.as_deref(), &target_language)
        });
        let target_prefix = self.target_prefix.as_deref().map(|template| {
            render_language_prefix(template, source_language.as_deref(), &target_language)
        });
        #[rustfmt::skip]
        tracing::info!("loading the translation model at [{}] with [{}] thread(s)", model_path.display(), threads);
        tokio::task::spawn_blocking(move || {
            let config = Config { num_threads_per_replica: threads, ..Config::default() };
            let translator = Translator::new(&model_path, &config)
                .map_err(|e| TranslationError::Backend(e.to_string()))?;
            let inputs: Vec<String> = match &source_prefix {
                Some(prefix) => texts.into_iter().map(|text| format!("{prefix} {text}")).collect(),
                None => texts,
            };
            let options: TranslationOptions<String, String> = TranslationOptions {
                max_batch_size: MAX_BATCH_SIZE,
                ..TranslationOptions::default()
            };
            let results = match &target_prefix {
                Some(token) => {
                    let prefixes: Vec<Vec<String>> = vec![vec![token.clone()]; inputs.len()];
                    translator
                        .translate_batch_with_target_prefix(&inputs, &prefixes, &options, None)
                }
                None => translator.translate_batch(&inputs, &options, None),
            }
            .map_err(|e| TranslationError::Backend(e.to_string()))?;
            Ok(results.into_iter().map(|(text, _score)| text).collect())
        })
        .await
        .map_err(|e| TranslationError::Backend(e.to_string()))?
    }
}
