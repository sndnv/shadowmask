use std::path::PathBuf;
use std::sync::Arc;

use ct2rs::tokenizers::auto::Tokenizer;
use ct2rs::{Config, TranslationOptions, Translator};
use domain::error::TranslationError;
use tokio::sync::OnceCell;

use crate::TranslationEngine;
use crate::language_prefix::render_language_prefix;

pub struct Ct2TranslationEngine {
    model_path: PathBuf,
    source_prefix: Option<String>,
    target_prefix: Option<String>,
    threads: usize,
    translator: OnceCell<Arc<Translator<Tokenizer>>>,
}

impl Ct2TranslationEngine {
    pub fn new(
        model_path: impl Into<PathBuf>,
        source_prefix: Option<String>,
        target_prefix: Option<String>,
        threads: usize,
    ) -> Self {
        Self {
            model_path: model_path.into(),
            source_prefix,
            target_prefix,
            threads,
            translator: OnceCell::new(),
        }
    }

    async fn translator(&self) -> Result<Arc<Translator<Tokenizer>>, TranslationError> {
        self.translator
            .get_or_try_init(|| async {
                let model_path = self.model_path.clone();
                let config = Config { num_threads_per_replica: self.threads, ..Config::default() };
                #[rustfmt::skip]
                tracing::info!("loading the translation model at [{}] with [{}] thread(s)", model_path.display(), self.threads);
                tokio::task::spawn_blocking(move || {
                    Translator::new(&model_path, &config)
                        .map(Arc::new)
                        .map_err(|e| TranslationError::Backend(e.to_string()))
                })
                .await
                .map_err(|e| TranslationError::Backend(e.to_string()))?
            })
            .await
            .map(Arc::clone)
    }
}

impl TranslationEngine for Ct2TranslationEngine {
    async fn translate(
        &self,
        texts: Vec<String>,
        source_language: Option<String>,
        target_language: String,
    ) -> Result<Vec<String>, TranslationError> {
        let translator = self.translator().await?;
        let source_prefix = self.source_prefix.as_deref().map(|template| {
            render_language_prefix(template, source_language.as_deref(), &target_language)
        });
        let target_prefix = self.target_prefix.as_deref().map(|template| {
            render_language_prefix(template, source_language.as_deref(), &target_language)
        });
        tokio::task::spawn_blocking(move || {
            let inputs: Vec<String> = match &source_prefix {
                Some(prefix) => texts.into_iter().map(|text| format!("{prefix} {text}")).collect(),
                None => texts,
            };
            let options: TranslationOptions<String, String> = TranslationOptions::default();
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
