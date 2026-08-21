pub mod env;
pub mod executor;
pub mod library;
pub mod user;

use std::collections::HashSet;
use std::fmt::Display;
use std::hash::Hash;
use std::path::PathBuf;

use domain::user::{Principal, Role, UserId};
use serde::{Deserialize, Serialize};

pub use env::load_expanded;
pub use executor::{BootstrapEntityProvider, complete, run_one};
pub use library::LibraryBootstrapProvider;
pub use user::UserBootstrapProvider;

pub(crate) fn bootstrap_admin() -> Principal {
    Principal {
        user: UserId("bootstrap".to_owned()),
        role: Role::Admin,
    }
}

pub(crate) fn backend<E: Display>(entity: &'static str, error: E) -> BootstrapError {
    BootstrapError::Backend {
        entity,
        reason: error.to_string(),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BootstrapMode {
    #[default]
    Off,
    Init,
    InitAndStart,
}

impl BootstrapMode {
    pub fn enabled(self) -> bool {
        !matches!(self, BootstrapMode::Off)
    }

    pub fn serves(self) -> bool {
        !matches!(self, BootstrapMode::Init)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Created {
    New,
    Skipped,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BootstrapResult {
    pub found: usize,
    pub created: usize,
    pub skipped: usize,
}

impl BootstrapResult {
    pub fn empty() -> Self {
        Self::default()
    }
}

impl std::ops::Add for BootstrapResult {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            found: self.found + other.found,
            created: self.created + other.created,
            skipped: self.skipped + other.skipped,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    #[error("failed to read bootstrap file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse bootstrap file {path}: {source}")]
    ParseFile {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("failed to expand environment variables in bootstrap config: {0}")]
    EnvExpansion(#[from] subst::Error),
    #[error("invalid {entity} bootstrap entry: {reason}")]
    Invalid {
        entity: &'static str,
        reason: String,
    },
    #[error("duplicate {field} value {value:?} across {entity} bootstrap entries")]
    Duplicate {
        entity: &'static str,
        field: &'static str,
        value: String,
    },
    #[error("bootstrap backend failure creating {entity}: {reason}")]
    Backend {
        entity: &'static str,
        reason: String,
    },
}

pub fn require_unique<T, K, F>(
    entities: &[T],
    entity: &'static str,
    field: &'static str,
    key: F,
) -> Result<(), BootstrapError>
where
    K: Eq + Hash + Display,
    F: Fn(&T) -> K,
{
    let mut seen = HashSet::with_capacity(entities.len());
    for candidate in entities {
        let value = key(candidate);
        if !seen.insert(value.to_string()) {
            return Err(BootstrapError::Duplicate {
                entity,
                field,
                value: value.to_string(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize)]
    struct Wrapper {
        mode: BootstrapMode,
    }

    #[test]
    fn mode_default_is_off() {
        assert_eq!(BootstrapMode::default(), BootstrapMode::Off);
        assert!(!BootstrapMode::Off.enabled());
        assert!(BootstrapMode::Off.serves());
    }

    #[test]
    fn mode_flags() {
        assert!(BootstrapMode::Init.enabled());
        assert!(!BootstrapMode::Init.serves());
        assert!(BootstrapMode::InitAndStart.enabled());
        assert!(BootstrapMode::InitAndStart.serves());
    }

    #[test]
    fn mode_parses_kebab_case() {
        let parsed: Wrapper = toml::from_str(r#"mode = "init-and-start""#).unwrap();
        assert_eq!(parsed.mode, BootstrapMode::InitAndStart);
        let parsed: Wrapper = toml::from_str(r#"mode = "off""#).unwrap();
        assert_eq!(parsed.mode, BootstrapMode::Off);
    }

    #[test]
    fn mode_rejects_unknown() {
        assert!(toml::from_str::<Wrapper>(r#"mode = "bogus""#).is_err());
    }

    #[test]
    fn result_adds_componentwise() {
        let a = BootstrapResult {
            found: 2,
            created: 1,
            skipped: 1,
        };
        let b = BootstrapResult {
            found: 3,
            created: 2,
            skipped: 0,
        };
        assert_eq!(
            a + b,
            BootstrapResult {
                found: 5,
                created: 3,
                skipped: 1,
            }
        );
    }

    #[test]
    fn require_unique_accepts_distinct() {
        let names = ["a", "b", "c"];
        assert!(require_unique(&names, "widget", "name", |n| *n).is_ok());
    }

    #[test]
    fn require_unique_rejects_duplicate() {
        let names = ["a", "b", "a"];
        let err = require_unique(&names, "widget", "name", |n| *n).unwrap_err();
        assert!(matches!(
            err,
            BootstrapError::Duplicate {
                entity: "widget",
                field: "name",
                ..
            }
        ));
    }
}
