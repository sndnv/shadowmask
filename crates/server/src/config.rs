use std::net::SocketAddr;
use std::path::PathBuf;

use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use serde::{Deserialize, Serialize};

use crate::bootstrap::BootstrapMode;
use crate::enrichment::EnrichmentConfig;
use crate::fetch_providers::FetchProvidersConfig;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub fn transcription_model_dir(enrichment: &EnrichmentConfig) -> PathBuf {
    enrichment
        .transcription
        .model_path
        .clone()
        .unwrap_or_else(|| enrichment.model_cache.join("transcription"))
}

pub fn translation_model_dir(enrichment: &EnrichmentConfig) -> PathBuf {
    enrichment
        .translation
        .model_path
        .clone()
        .unwrap_or_else(|| enrichment.model_cache.join("translation"))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HardwareAccelerationMode {
    Off,
    #[default]
    Auto,
    Vaapi,
}

pub fn resolve_vaapi_device(
    mode: HardwareAccelerationMode,
    device: &str,
    present: bool,
) -> Option<String> {
    match mode {
        HardwareAccelerationMode::Off => None,
        HardwareAccelerationMode::Auto => present.then(|| device.to_owned()),
        HardwareAccelerationMode::Vaapi => {
            if present {
                Some(device.to_owned())
            } else {
                tracing::warn!(
                    device,
                    "hardware_acceleration=vaapi requested but no render node found; using software"
                );
                None
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub db_root: PathBuf,
    pub bind: SocketAddr,
    pub jwt_secret: String,
    pub stream_secret: String,
    pub access_ttl_secs: i64,
    pub refresh_ttl_secs: i64,
    pub transcode_cache: PathBuf,
    pub artwork_cache: PathBuf,
    pub trickplay_cache: PathBuf,
    pub subtitle_cache: PathBuf,
    pub job_log_dir: PathBuf,
    pub tmdb_api_key: Option<String>,
    pub omdb_api_key: Option<String>,
    pub opensubtitles_api_key: Option<String>,
    pub tmdb_min_interval_ms: u64,
    pub omdb_min_interval_ms: u64,
    pub opensubtitles_min_interval_ms: u64,
    pub target_languages: Vec<String>,
    pub worker_concurrency: usize,
    pub worker_period_secs: u64,
    pub scheduler_period_secs: u64,
    pub reaper_period_secs: u64,
    pub shutdown_timeout_secs: u64,
    pub reindex_every_secs: i64,
    pub transcode_cache_cap_bytes: u64,
    pub cache_eviction_every_secs: i64,
    pub log_level: String,
    pub sqlx_log_level: String,
    pub bootstrap_mode: BootstrapMode,
    pub bootstrap_dir: PathBuf,
    pub webhook_clients: Vec<::api::WebhookClient>,
    pub basic_client_dir: PathBuf,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub hardware_acceleration: HardwareAccelerationMode,
    pub vaapi_device: PathBuf,
    pub enrichment: EnrichmentConfig,
    pub fetch_providers: FetchProvidersConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_root: PathBuf::from("data"),
            bind: SocketAddr::from(([0, 0, 0, 0], 8080)),
            jwt_secret: "change-me-jwt-secret".to_owned(),
            stream_secret: "change-me-stream-secret".to_owned(),
            access_ttl_secs: 3600,
            refresh_ttl_secs: 86_400,
            transcode_cache: PathBuf::from("data/transcode"),
            artwork_cache: PathBuf::from("data/artwork"),
            trickplay_cache: PathBuf::from("data/trickplay"),
            subtitle_cache: PathBuf::from("data/subtitles"),
            job_log_dir: PathBuf::from("data/job-logs"),
            tmdb_api_key: None,
            omdb_api_key: None,
            opensubtitles_api_key: None,
            tmdb_min_interval_ms: 100,
            omdb_min_interval_ms: 500,
            opensubtitles_min_interval_ms: 1000,
            target_languages: vec!["en".to_owned()],
            worker_concurrency: 4,
            worker_period_secs: 5,
            scheduler_period_secs: 30,
            reaper_period_secs: 30,
            shutdown_timeout_secs: 30,
            reindex_every_secs: 3600,
            transcode_cache_cap_bytes: 10 * 1024 * 1024 * 1024,
            cache_eviction_every_secs: 3600,
            log_level: "info".to_owned(),
            sqlx_log_level: "warn".to_owned(),
            bootstrap_mode: BootstrapMode::Off,
            bootstrap_dir: PathBuf::from("bootstrap"),
            webhook_clients: Vec::new(),
            basic_client_dir: PathBuf::from("clients/basic"),
            tls_cert: None,
            tls_key: None,
            hardware_acceleration: HardwareAccelerationMode::default(),
            vaapi_device: PathBuf::from("/dev/dri/renderD128"),
            enrichment: EnrichmentConfig::default(),
            fetch_providers: FetchProvidersConfig::default(),
        }
    }
}

fn nest_section_key(key: &str) -> String {
    let key = key.to_ascii_lowercase();
    for section in ["transcription", "translation", "upscaling"] {
        if let Some(leaf) = key.strip_prefix(&format!("enrichment_{section}_")) {
            return format!("enrichment.{section}.{leaf}");
        }
    }
    if let Some(leaf) = key.strip_prefix("enrichment_") {
        return format!("enrichment.{leaf}");
    }
    if let Some(leaf) = key.strip_prefix("fetch_providers_") {
        return format!("fetch_providers.{leaf}");
    }
    key
}

impl Config {
    pub fn load() -> Result<Self, BoxError> {
        let mut config: Config = Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Toml::file("shadowmask.toml"))
            .merge(
                Env::prefixed("SHADOWMASK_")
                    .map(|key| nest_section_key(key.as_str()).into())
                    .split(".")
                    .ignore(&["target_languages"]),
            )
            .extract()?;
        config.tmdb_api_key = trim_key(config.tmdb_api_key);
        config.omdb_api_key = trim_key(config.omdb_api_key);
        config.opensubtitles_api_key = trim_key(config.opensubtitles_api_key);
        if let Ok(raw) = std::env::var("SHADOWMASK_TARGET_LANGUAGES") {
            let languages: Vec<String> = raw
                .split(',')
                .map(|language| language.trim().to_owned())
                .filter(|language| !language.is_empty())
                .collect();
            if !languages.is_empty() {
                config.target_languages = languages;
            }
        }
        Ok(config)
    }

    pub fn describe(&self) -> String {
        use std::fmt::Write as _;
        let provided = |value: &str| {
            if value.trim().is_empty() {
                "none"
            } else {
                "<provided>"
            }
        };
        let opt_secret = |value: &Option<String>| match value {
            Some(v) if !v.trim().is_empty() => "<provided>",
            _ => "none",
        };
        let opt_path = |value: &Option<PathBuf>| {
            value
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "none".to_owned())
        };
        let e = &self.enrichment;
        let mut out = String::new();
        let _ = writeln!(out, "\nConfig(");
        let _ = writeln!(out, "  server:");
        let _ = writeln!(out, "    bind:             {}", self.bind);
        let _ = writeln!(out, "    db_root:          {}", self.db_root.display());
        let _ = writeln!(out, "    access_ttl_secs:  {}", self.access_ttl_secs);
        let _ = writeln!(out, "    refresh_ttl_secs: {}", self.refresh_ttl_secs);
        let _ = writeln!(
            out,
            "    shutdown_timeout: {} s",
            self.shutdown_timeout_secs
        );
        let _ = writeln!(out, "    reindex_every:    {} s", self.reindex_every_secs);
        let _ = writeln!(
            out,
            "    cache_evict_every: {} s",
            self.cache_eviction_every_secs
        );
        let _ = writeln!(out, "    log_level:        {}", self.log_level);
        let _ = writeln!(out, "    sqlx_log_level:   {}", self.sqlx_log_level);
        let _ = writeln!(out, "  caches:");
        let _ = writeln!(out, "    transcode: {}", self.transcode_cache.display());
        let _ = writeln!(
            out,
            "    transcode_cap: {} bytes",
            self.transcode_cache_cap_bytes
        );
        let _ = writeln!(out, "    artwork:   {}", self.artwork_cache.display());
        let _ = writeln!(out, "    trickplay: {}", self.trickplay_cache.display());
        let _ = writeln!(out, "    subtitles: {}", self.subtitle_cache.display());
        let _ = writeln!(out, "    job_logs:  {}", self.job_log_dir.display());
        let _ = writeln!(out, "  tls:");
        let _ = writeln!(out, "    cert: {}", opt_path(&self.tls_cert));
        let _ = writeln!(out, "    key:  {}", opt_path(&self.tls_key));
        let _ = writeln!(out, "  transcoding:");
        let _ = writeln!(
            out,
            "    hardware_acceleration: {:?}",
            self.hardware_acceleration
        );
        let _ = writeln!(
            out,
            "    vaapi_device:          {}",
            self.vaapi_device.display()
        );
        let _ = writeln!(out, "  metadata:");
        let _ = writeln!(
            out,
            "    tmdb_api_key:          {}",
            opt_secret(&self.tmdb_api_key)
        );
        let _ = writeln!(
            out,
            "    omdb_api_key:          {}",
            opt_secret(&self.omdb_api_key)
        );
        let _ = writeln!(
            out,
            "    opensubtitles_api_key: {}",
            opt_secret(&self.opensubtitles_api_key)
        );
        let _ = writeln!(
            out,
            "    min_interval_ms:       tmdb {} / omdb {} / opensubtitles {}",
            self.tmdb_min_interval_ms,
            self.omdb_min_interval_ms,
            self.opensubtitles_min_interval_ms
        );
        let _ = writeln!(
            out,
            "    target_languages:      {}",
            self.target_languages.join(", ")
        );
        let _ = writeln!(out, "  workers:");
        let _ = writeln!(out, "    concurrency:      {}", self.worker_concurrency);
        let _ = writeln!(out, "    worker_period:    {} s", self.worker_period_secs);
        let _ = writeln!(
            out,
            "    scheduler_period: {} s",
            self.scheduler_period_secs
        );
        let _ = writeln!(out, "    reaper_period:    {} s", self.reaper_period_secs);
        let _ = writeln!(out, "  bootstrap:");
        let _ = writeln!(out, "    mode: {:?}", self.bootstrap_mode);
        let _ = writeln!(out, "    dir:  {}", self.bootstrap_dir.display());
        let _ = writeln!(out, "  webhooks:");
        let _ = writeln!(out, "    clients: {}", self.webhook_clients.len());
        let _ = writeln!(out, "  enrichment:");
        let _ = writeln!(out, "    model_cache:   {}", e.model_cache.display());
        let _ = writeln!(out, "    concurrency:   {}", e.concurrency);
        let _ = writeln!(
            out,
            "    transcription: enabled={} provider={:?} models_dir={}",
            e.transcription.enabled,
            e.transcription.provider,
            transcription_model_dir(e).display()
        );
        let _ = writeln!(
            out,
            "    translation:   enabled={} provider={:?} models_dir={} source_prefix={} target_prefix={}",
            e.translation.enabled,
            e.translation.provider,
            translation_model_dir(e).display(),
            e.translation.source_prefix.as_deref().unwrap_or("none"),
            e.translation.target_prefix.as_deref().unwrap_or("none")
        );
        let _ = writeln!(
            out,
            "    upscaling:     enabled={} provider={:?} target_height={} model_path={}",
            e.upscaling.enabled,
            e.upscaling.provider,
            e.upscaling.target_height,
            opt_path(&e.upscaling.model_path)
        );
        let _ = writeln!(out, "  fetch_providers:");
        let _ = writeln!(out, "    enabled:       {}", self.fetch_providers.enabled);
        let _ = writeln!(
            out,
            "    concurrency:   {}",
            self.fetch_providers.concurrency
        );
        let _ = writeln!(
            out,
            "    yt_dlp_binary: {}",
            self.fetch_providers.yt_dlp_binary
        );
        let _ = writeln!(
            out,
            "    plugin_dir:    {}",
            opt_path(&self.fetch_providers.yt_dlp_plugin_dir)
        );
        let _ = writeln!(
            out,
            "    max_height:    {}",
            self.fetch_providers
                .max_height
                .map(|h| h.to_string())
                .unwrap_or_else(|| "none".to_owned())
        );
        let _ = writeln!(
            out,
            "    cookies_file:  {}",
            opt_path(&self.fetch_providers.cookies_file)
        );
        let _ = writeln!(out, "  secrets:");
        let _ = writeln!(out, "    jwt_secret:    {}", provided(&self.jwt_secret));
        let _ = writeln!(out, "    stream_secret: {}", provided(&self.stream_secret));
        let _ = write!(out, ")");
        out
    }
}

fn trim_key(key: Option<String>) -> Option<String> {
    key.map(|key| key.trim().to_owned())
        .filter(|key| !key.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::result_large_err)]
    fn load_uses_defaults_when_unset() {
        figment::Jail::expect_with(|_jail| {
            let config = Config::load().unwrap();
            assert_eq!(config.bind, SocketAddr::from(([0, 0, 0, 0], 8080)));
            assert_eq!(config.worker_concurrency, 4);
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn bootstrap_defaults_to_off() {
        figment::Jail::expect_with(|_jail| {
            let config = Config::load().unwrap();
            assert_eq!(config.bootstrap_mode, BootstrapMode::Off);
            assert_eq!(config.bootstrap_dir, PathBuf::from("bootstrap"));
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn bootstrap_mode_read_from_env() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("SHADOWMASK_BOOTSTRAP_MODE", "init-and-start");
            jail.set_env("SHADOWMASK_BOOTSTRAP_DIR", "/etc/shadowmask/bootstrap");
            let config = Config::load().unwrap();
            assert_eq!(config.bootstrap_mode, BootstrapMode::InitAndStart);
            assert_eq!(
                config.bootstrap_dir,
                PathBuf::from("/etc/shadowmask/bootstrap")
            );
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn load_merges_file_then_env() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "shadowmask.toml",
                "access_ttl_secs = 7\nworker_concurrency = 2\n",
            )?;
            jail.set_env("SHADOWMASK_WORKER_CONCURRENCY", "9");
            let config = Config::load().unwrap();
            assert_eq!(config.access_ttl_secs, 7);
            assert_eq!(config.worker_concurrency, 9);
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn blank_tmdb_api_key_is_treated_as_unset() {
        figment::Jail::expect_with(|jail| {
            assert_eq!(Config::load().unwrap().tmdb_api_key, None);
            jail.set_env("SHADOWMASK_TMDB_API_KEY", "");
            assert_eq!(Config::load().unwrap().tmdb_api_key, None);
            jail.set_env("SHADOWMASK_TMDB_API_KEY", "   ");
            assert_eq!(Config::load().unwrap().tmdb_api_key, None);
            jail.set_env("SHADOWMASK_TMDB_API_KEY", "  real-key  ");
            assert_eq!(
                Config::load().unwrap().tmdb_api_key,
                Some("real-key".to_owned())
            );
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn blank_omdb_api_key_is_treated_as_unset() {
        figment::Jail::expect_with(|jail| {
            assert_eq!(Config::load().unwrap().omdb_api_key, None);
            jail.set_env("SHADOWMASK_OMDB_API_KEY", "   ");
            assert_eq!(Config::load().unwrap().omdb_api_key, None);
            jail.set_env("SHADOWMASK_OMDB_API_KEY", "  real-key  ");
            assert_eq!(
                Config::load().unwrap().omdb_api_key,
                Some("real-key".to_owned())
            );
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn blank_opensubtitles_api_key_is_treated_as_unset() {
        figment::Jail::expect_with(|jail| {
            assert_eq!(Config::load().unwrap().opensubtitles_api_key, None);
            jail.set_env("SHADOWMASK_OPENSUBTITLES_API_KEY", "   ");
            assert_eq!(Config::load().unwrap().opensubtitles_api_key, None);
            jail.set_env("SHADOWMASK_OPENSUBTITLES_API_KEY", "  os-key  ");
            assert_eq!(
                Config::load().unwrap().opensubtitles_api_key,
                Some("os-key".to_owned())
            );
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn target_languages_default_then_env_comma_split() {
        figment::Jail::expect_with(|jail| {
            assert_eq!(
                Config::load().unwrap().target_languages,
                vec!["en".to_owned()]
            );
            jail.set_env("SHADOWMASK_TARGET_LANGUAGES", "en, es ,, fr");
            assert_eq!(
                Config::load().unwrap().target_languages,
                vec!["en".to_owned(), "es".to_owned(), "fr".to_owned()]
            );
            jail.set_env("SHADOWMASK_TARGET_LANGUAGES", "   ");
            assert_eq!(
                Config::load().unwrap().target_languages,
                vec!["en".to_owned()]
            );
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn enrichment_defaults_all_off() {
        figment::Jail::expect_with(|_jail| {
            let enrichment = Config::load().unwrap().enrichment;
            assert_eq!(
                enrichment.model_cache,
                PathBuf::from("data/enrichment-models")
            );
            assert!(!enrichment.transcription.enabled);
            assert!(!enrichment.translation.enabled);
            assert!(!enrichment.upscaling.enabled);
            assert_eq!(
                enrichment.transcription.provider,
                crate::enrichment::ProviderChoice::None
            );
            assert_eq!(
                enrichment.translation.provider,
                crate::enrichment::ProviderChoice::None
            );
            assert_eq!(
                enrichment.upscaling.provider,
                crate::enrichment::ProviderChoice::None
            );
            assert_eq!(enrichment.transcription.model_path, None);
            assert_eq!(enrichment.upscaling.target_height, 1080);
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn enrichment_config_round_trips_from_toml() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "shadowmask.toml",
                "[enrichment.transcription]\nenabled = true\nprovider = \"built-in\"\nmodel_path = \"/models/transcription\"\n",
            )?;
            let enrichment = Config::load().unwrap().enrichment;
            assert!(enrichment.transcription.enabled);
            assert_eq!(
                enrichment.transcription.provider,
                crate::enrichment::ProviderChoice::BuiltIn
            );
            assert_eq!(
                enrichment.transcription.model_path,
                Some(PathBuf::from("/models/transcription"))
            );
            assert!(!enrichment.translation.enabled);
            assert!(!enrichment.upscaling.enabled);
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn nested_env_keys_apply_without_regressing_flat_keys() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("SHADOWMASK_JWT_SECRET", "flat-secret");
            jail.set_env("SHADOWMASK_ENRICHMENT_MODEL_CACHE", "/models");
            jail.set_env("SHADOWMASK_ENRICHMENT_UPSCALING_ENABLED", "true");
            jail.set_env("SHADOWMASK_ENRICHMENT_UPSCALING_PROVIDER", "built-in");
            jail.set_env("SHADOWMASK_ENRICHMENT_UPSCALING_TARGET_HEIGHT", "2160");
            let config = Config::load().unwrap();
            assert_eq!(config.jwt_secret, "flat-secret");
            assert_eq!(config.enrichment.model_cache, PathBuf::from("/models"));
            assert!(config.enrichment.upscaling.enabled);
            assert_eq!(
                config.enrichment.upscaling.provider,
                crate::enrichment::ProviderChoice::BuiltIn
            );
            assert_eq!(config.enrichment.upscaling.target_height, 2160);
            Ok(())
        });
    }

    #[test]
    fn nest_section_key_maps_only_boundaries() {
        assert_eq!(super::nest_section_key("jwt_secret"), "jwt_secret");
        assert_eq!(
            super::nest_section_key("enrichment_model_cache"),
            "enrichment.model_cache"
        );
        assert_eq!(
            super::nest_section_key("enrichment_transcription_enabled"),
            "enrichment.transcription.enabled"
        );
        assert_eq!(
            super::nest_section_key("ENRICHMENT_UPSCALING_TARGET_HEIGHT"),
            "enrichment.upscaling.target_height"
        );
        assert_eq!(
            super::nest_section_key("FETCH_PROVIDERS_YT_DLP_BINARY"),
            "fetch_providers.yt_dlp_binary"
        );
        assert_eq!(
            super::nest_section_key("fetch_providers_enabled"),
            "fetch_providers.enabled"
        );
    }

    #[test]
    fn describe_redacts_secrets_and_lists_config() {
        let config = Config {
            jwt_secret: "super-secret-jwt-value".to_owned(),
            stream_secret: String::new(),
            tmdb_api_key: Some("tmdb-key-123".to_owned()),
            omdb_api_key: Some("omdb-key-456".to_owned()),
            opensubtitles_api_key: None,
            ..Config::default()
        };
        let described = config.describe();
        assert!(described.contains("Config("));
        assert!(!described.contains("super-secret-jwt-value"));
        assert!(!described.contains("tmdb-key-123"));
        assert!(!described.contains("omdb-key-456"));
        assert!(described.contains("jwt_secret:    <provided>"));
        assert!(described.contains("stream_secret: none"));
        assert!(described.contains("tmdb_api_key:          <provided>"));
        assert!(described.contains("omdb_api_key:          <provided>"));
        assert!(described.contains("opensubtitles_api_key: none"));
        assert!(described.contains("hardware_acceleration: Auto"));
        assert!(described.contains("max_height:    none"));
        assert!(described.contains("cookies_file:  none"));
        assert!(described.contains("cache_evict_every: 3600 s"));
        assert!(described.contains("transcode_cap: 10737418240 bytes"));

        let capped = Config {
            fetch_providers: FetchProvidersConfig {
                max_height: Some(1080),
                cookies_file: Some(PathBuf::from("/secrets/cookies.txt")),
                ..FetchProvidersConfig::default()
            },
            ..Config::default()
        };
        let capped = capped.describe();
        assert!(capped.contains("max_height:    1080"));
        assert!(capped.contains("cookies_file:  /secrets/cookies.txt"));
    }

    #[test]
    fn model_dirs_default_under_cache_or_override() {
        let defaults = crate::enrichment::EnrichmentConfig::default();
        assert_eq!(
            transcription_model_dir(&defaults),
            defaults.model_cache.join("transcription")
        );
        assert_eq!(
            translation_model_dir(&defaults),
            defaults.model_cache.join("translation")
        );
        let overridden = crate::enrichment::EnrichmentConfig {
            transcription: crate::enrichment::TranscriptionConfig {
                model_path: Some(PathBuf::from("/models/w")),
                ..Default::default()
            },
            translation: crate::enrichment::TranslationConfig {
                model_path: Some(PathBuf::from("/models/t")),
                ..Default::default()
            },
            ..crate::enrichment::EnrichmentConfig::default()
        };
        assert_eq!(
            transcription_model_dir(&overridden),
            PathBuf::from("/models/w")
        );
        assert_eq!(
            translation_model_dir(&overridden),
            PathBuf::from("/models/t")
        );
    }

    #[test]
    fn resolve_vaapi_off_is_always_software() {
        assert_eq!(
            resolve_vaapi_device(HardwareAccelerationMode::Off, "/dev/dri/renderD128", true),
            None
        );
    }

    #[test]
    fn resolve_vaapi_auto_follows_device_presence() {
        assert_eq!(
            resolve_vaapi_device(HardwareAccelerationMode::Auto, "/dev/dri/renderD128", true),
            Some("/dev/dri/renderD128".to_owned())
        );
        assert_eq!(
            resolve_vaapi_device(HardwareAccelerationMode::Auto, "/dev/dri/renderD128", false),
            None
        );
    }

    #[test]
    fn resolve_vaapi_forced_falls_back_when_absent() {
        assert_eq!(
            resolve_vaapi_device(HardwareAccelerationMode::Vaapi, "/dev/dri/renderD128", true),
            Some("/dev/dri/renderD128".to_owned())
        );
        assert_eq!(
            resolve_vaapi_device(
                HardwareAccelerationMode::Vaapi,
                "/dev/dri/renderD128",
                false
            ),
            None
        );
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn webhook_clients_default_empty_and_load_from_toml() {
        figment::Jail::expect_with(|jail| {
            assert!(Config::load().unwrap().webhook_clients.is_empty());
            jail.create_file(
                "shadowmask.toml",
                "[[webhook_clients]]\nname = \"sonarr\"\nsecret = \"abc\"\nlibraries = [\"lib1\"]\n",
            )?;
            let config = Config::load().unwrap();
            assert_eq!(config.webhook_clients.len(), 1);
            assert_eq!(config.webhook_clients[0].name, "sonarr");
            assert_eq!(config.webhook_clients[0].secret, "abc");
            assert_eq!(config.webhook_clients[0].libraries, vec!["lib1".to_owned()]);
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn shutdown_timeout_read_from_env() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("SHADOWMASK_SHUTDOWN_TIMEOUT_SECS", "7");
            let config = Config::load().unwrap();
            assert_eq!(config.shutdown_timeout_secs, 7);
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn log_level_read_from_env() {
        figment::Jail::expect_with(|jail| {
            let config = Config::load().unwrap();
            assert_eq!(config.log_level, "info");
            assert_eq!(config.sqlx_log_level, "warn");
            jail.set_env("SHADOWMASK_LOG_LEVEL", "debug");
            jail.set_env("SHADOWMASK_SQLX_LOG_LEVEL", "trace");
            let config = Config::load().unwrap();
            assert_eq!(config.log_level, "debug");
            assert_eq!(config.sqlx_log_level, "trace");
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn tls_paths_default_none_and_load_from_env() {
        figment::Jail::expect_with(|jail| {
            let config = Config::load().unwrap();
            assert!(config.tls_cert.is_none());
            assert!(config.tls_key.is_none());
            jail.set_env("SHADOWMASK_TLS_CERT", "/etc/shadowmask/cert.pem");
            jail.set_env("SHADOWMASK_TLS_KEY", "/etc/shadowmask/key.pem");
            let config = Config::load().unwrap();
            assert_eq!(
                config.tls_cert,
                Some(PathBuf::from("/etc/shadowmask/cert.pem"))
            );
            assert_eq!(
                config.tls_key,
                Some(PathBuf::from("/etc/shadowmask/key.pem"))
            );
            Ok(())
        });
    }
}
