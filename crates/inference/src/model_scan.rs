use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedModel {
    pub dir: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelScan {
    pub total: usize,
    pub chosen: Option<PathBuf>,
    pub rejected: Vec<RejectedModel>,
}

pub fn scan_transcription_models(base: &Path) -> ModelScan {
    scan(base, validate_transcription_dir)
}

pub fn scan_translation_models(base: &Path) -> ModelScan {
    scan(base, validate_translation_dir)
}

fn scan(base: &Path, validate: impl Fn(&Path) -> Result<(), String>) -> ModelScan {
    let mut dirs: Vec<PathBuf> = match std::fs::read_dir(base) {
        Ok(entries) => entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect(),
        Err(_) => Vec::new(),
    };
    dirs.sort();

    let total = dirs.len();
    let mut chosen = None;
    let mut rejected = Vec::new();
    for dir in dirs {
        match validate(&dir) {
            Ok(()) if chosen.is_none() => chosen = Some(dir),
            Ok(()) => {}
            Err(reason) => rejected.push(RejectedModel { dir, reason }),
        }
    }
    ModelScan {
        total,
        chosen,
        rejected,
    }
}

fn has(dir: &Path, file: &str) -> bool {
    dir.join(file).is_file()
}

fn require(dir: &Path, file: &str) -> Result<(), String> {
    if has(dir, file) {
        Ok(())
    } else {
        Err(format!("missing {file}"))
    }
}

fn validate_transcription_dir(dir: &Path) -> Result<(), String> {
    require(dir, "model.bin")?;
    require(dir, "config.json")?;
    require(dir, "tokenizer.json")?;
    require(dir, "preprocessor_config.json")?;
    Ok(())
}

fn validate_translation_dir(dir: &Path) -> Result<(), String> {
    require(dir, "model.bin")?;
    require(dir, "config.json")?;
    let has_tokenizer = has(dir, "tokenizer.json")
        || (has(dir, "source.spm") && has(dir, "target.spm"))
        || (has(dir, "vocab.json") && has(dir, "merges.txt"));
    if has_tokenizer {
        Ok(())
    } else {
        Err(
            "no tokenizer (need tokenizer.json, or source.spm and target.spm, or vocab.json and merges.txt)"
                .to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_files(dir: &Path, files: &[&str]) {
        fs::create_dir_all(dir).unwrap();
        for file in files {
            fs::write(dir.join(file), b"x").unwrap();
        }
    }

    #[test]
    fn missing_base_dir_yields_empty_scan() {
        let base = TempDir::new().unwrap();
        let scan = scan_transcription_models(&base.path().join("nope"));
        assert_eq!(scan.total, 0);
        assert!(scan.chosen.is_none());
        assert!(scan.rejected.is_empty());
    }

    #[test]
    fn picks_first_valid_alphanumeric_and_rejects_invalid() {
        let base = TempDir::new().unwrap();
        let whisper = &[
            "model.bin",
            "config.json",
            "tokenizer.json",
            "preprocessor_config.json",
        ];
        write_files(&base.path().join("b-good"), whisper);
        write_files(&base.path().join("a-bad"), &["model.bin"]);
        write_files(&base.path().join("c-good"), whisper);

        let scan = scan_transcription_models(base.path());
        assert_eq!(scan.total, 3);
        assert_eq!(scan.chosen, Some(base.path().join("b-good")));
        assert_eq!(scan.rejected.len(), 1);
        assert_eq!(scan.rejected[0].dir, base.path().join("a-bad"));
        assert!(scan.rejected[0].reason.contains("config.json"));
    }

    #[test]
    fn all_invalid_yields_no_choice_but_counts_folders() {
        let base = TempDir::new().unwrap();
        write_files(&base.path().join("x"), &["model.bin"]);
        write_files(&base.path().join("y"), &["config.json"]);
        let scan = scan_transcription_models(base.path());
        assert_eq!(scan.total, 2);
        assert!(scan.chosen.is_none());
        assert_eq!(scan.rejected.len(), 2);
    }

    #[test]
    fn translation_accepts_each_tokenizer_shape() {
        let base = TempDir::new().unwrap();
        write_files(
            &base.path().join("a-hf"),
            &["model.bin", "config.json", "tokenizer.json"],
        );
        write_files(
            &base.path().join("b-spm"),
            &["model.bin", "config.json", "source.spm", "target.spm"],
        );
        write_files(
            &base.path().join("c-bpe"),
            &["model.bin", "config.json", "vocab.json", "merges.txt"],
        );
        for name in ["a-hf", "b-spm", "c-bpe"] {
            let one = TempDir::new().unwrap();
            let src = base.path().join(name);
            let dst = one.path().join(name);
            fs::create_dir_all(&dst).unwrap();
            for entry in fs::read_dir(&src).unwrap() {
                let entry = entry.unwrap();
                fs::copy(entry.path(), dst.join(entry.file_name())).unwrap();
            }
            let scan = scan_translation_models(one.path());
            assert_eq!(scan.chosen, Some(dst), "shape {name} should be valid");
        }
    }

    #[test]
    fn translation_without_tokenizer_is_rejected() {
        let base = TempDir::new().unwrap();
        write_files(
            &base.path().join("madlad"),
            &["model.bin", "config.json", "shared_vocabulary.json"],
        );
        let scan = scan_translation_models(base.path());
        assert_eq!(scan.total, 1);
        assert!(scan.chosen.is_none());
        assert_eq!(scan.rejected.len(), 1);
        assert!(scan.rejected[0].reason.contains("no tokenizer"));
    }

    #[test]
    fn files_in_base_are_not_candidates() {
        let base = TempDir::new().unwrap();
        fs::write(base.path().join("model.bin"), b"x").unwrap();
        let scan = scan_translation_models(base.path());
        assert_eq!(scan.total, 0);
        assert!(scan.chosen.is_none());
    }
}
