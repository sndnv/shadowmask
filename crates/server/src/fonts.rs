use std::path::Path;

use ::api::{FontFile, FontSet};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const MANIFEST: &str = "fonts.json";
const ROUTE: &str = "/fonts";

#[derive(Deserialize)]
struct Manifest {
    name: String,
    regular: String,
    medium: String,
    license: String,
}

#[derive(Debug, thiserror::Error)]
enum FontsError {
    #[error("{0}: {1}")]
    Io(String, std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0} is not a plain file name")]
    Name(String),
}

pub fn load(dir: &Path) -> Option<FontSet> {
    if !dir.join(MANIFEST).is_file() {
        return None;
    }
    match read(dir) {
        Ok(fonts) => {
            tracing::info!("Serving the fonts [{}] from [{}]", fonts.name, dir.display());
            Some(fonts)
        }
        Err(error) => {
            tracing::warn!("Not serving the fonts in [{}]: {error}", dir.display());
            None
        }
    }
}

fn read(dir: &Path) -> Result<FontSet, FontsError> {
    let text = std::fs::read_to_string(dir.join(MANIFEST))
        .map_err(|error| FontsError::Io(MANIFEST.to_owned(), error))?;
    let manifest: Manifest = serde_json::from_str(&text)?;
    Ok(FontSet {
        name: manifest.name,
        regular: file(dir, &manifest.regular)?,
        medium: file(dir, &manifest.medium)?,
        license: file(dir, &manifest.license)?,
    })
}

fn file(dir: &Path, name: &str) -> Result<FontFile, FontsError> {
    let plain = !name.starts_with('.')
        && !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if !plain {
        return Err(FontsError::Name(name.to_owned()));
    }
    let bytes =
        std::fs::read(dir.join(name)).map_err(|error| FontsError::Io(name.to_owned(), error))?;
    Ok(FontFile {
        url: format!("{ROUTE}/{name}"),
        size: bytes.len() as u64,
        sha256: hex::encode(Sha256::digest(&bytes)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    const SHA256_OF_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn folder(manifest: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(MANIFEST), manifest).unwrap();
        for name in ["Regular.otf", "Medium.otf", "LICENSE"] {
            std::fs::write(dir.path().join(name), "abc").unwrap();
        }
        dir
    }

    const VALID: &str = r#"{"name":"Noto Sans CJK SC","regular":"Regular.otf","medium":"Medium.otf","license":"LICENSE"}"#;

    #[test]
    #[traced_test]
    fn names_each_file_with_its_url_size_and_hash() {
        let dir = folder(VALID);

        let fonts = load(dir.path()).unwrap();

        assert_eq!(fonts.name, "Noto Sans CJK SC");
        assert_eq!(fonts.regular.url, "/fonts/Regular.otf");
        assert_eq!(fonts.medium.url, "/fonts/Medium.otf");
        assert_eq!(fonts.license.url, "/fonts/LICENSE");
        assert_eq!(fonts.regular.size, 3);
        assert_eq!(fonts.regular.sha256, SHA256_OF_ABC);
        assert!(logs_contain("Serving the fonts [Noto Sans CJK SC]"));
    }

    #[test]
    fn serves_nothing_without_a_folder_or_a_manifest() {
        assert!(load(Path::new("does-not-exist-shadowmask-fonts")).is_none());
        assert!(load(tempfile::tempdir().unwrap().path()).is_none());
    }

    #[test]
    #[traced_test]
    fn serves_nothing_when_a_listed_file_is_missing() {
        let dir = folder(VALID);
        std::fs::remove_file(dir.path().join("Medium.otf")).unwrap();

        assert!(load(dir.path()).is_none());
        assert!(logs_contain("Medium.otf"));
    }

    #[test]
    #[traced_test]
    fn serves_nothing_for_a_malformed_manifest() {
        assert!(load(folder("{").path()).is_none());
        assert!(logs_contain("Not serving the fonts"));
    }

    #[test]
    fn refuses_a_name_that_reaches_outside_the_folder() {
        let dir = folder(VALID);
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub").join("Regular.otf"), "abc").unwrap();
        std::fs::write(dir.path().join(".hidden"), "abc").unwrap();
        let inside = dir.path().join("sub");

        for (base, name) in [
            (inside.as_path(), "../Regular.otf"),
            (dir.path(), "sub/Regular.otf"),
            (dir.path(), ".hidden"),
            (dir.path(), ""),
            (dir.path(), "Regular .otf"),
        ] {
            assert!(matches!(file(base, name), Err(FontsError::Name(_))), "{name}");
        }
    }

    #[test]
    #[traced_test]
    fn serves_nothing_when_the_manifest_names_a_path() {
        let dir = folder(&VALID.replace("\"Regular.otf\"", "\"sub/Regular.otf\""));

        assert!(load(dir.path()).is_none());
        assert!(logs_contain("is not a plain file name"));
    }

    #[test]
    #[traced_test]
    fn serves_nothing_when_the_manifest_is_not_text() {
        let dir = folder(VALID);
        std::fs::write(dir.path().join(MANIFEST), [0xff, 0xfe, 0xfd]).unwrap();

        assert!(load(dir.path()).is_none());
        assert!(logs_contain("fonts.json"));
    }
}
