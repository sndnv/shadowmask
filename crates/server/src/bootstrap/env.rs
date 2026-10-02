use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::Path;

use super::BootstrapError;

const LOADED_FILES: [&str; 2] = [super::user::FILE, super::library::FILE];

#[derive(Default)]
struct Recorder {
    seen: RefCell<BTreeSet<String>>,
    present: BTreeSet<String>,
}

impl<'a> subst::VariableMap<'a> for Recorder {
    type Value = &'a str;

    fn get(&'a self, key: &str) -> Option<Self::Value> {
        self.seen.borrow_mut().insert(key.to_owned());
        self.present.contains(key).then_some("")
    }
}

pub fn referenced_vars(dir: &Path) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for file in LOADED_FILES {
        let Ok(text) = std::fs::read_to_string(dir.join(format!("{file}.toml"))) else {
            continue;
        };
        let Ok(value) = toml::from_str::<toml::Value>(&text) else {
            continue;
        };
        record(&value, &mut names);
    }
    names
}

fn record(value: &toml::Value, names: &mut BTreeSet<String>) {
    match value {
        toml::Value::String(text) => {
            let mut recorder = Recorder::default();
            while let Err(subst::Error::NoSuchVariable(missing)) =
                subst::substitute(text, &recorder)
            {
                recorder.present.insert(missing.name);
            }
            names.extend(recorder.seen.into_inner());
        }
        toml::Value::Array(items) => {
            for item in items {
                record(item, names);
            }
        }
        toml::Value::Table(table) => {
            for entry in table.values() {
                record(entry, names);
            }
        }
        _ => {}
    }
}

pub fn load_expanded(path: &Path) -> Result<toml::Value, BootstrapError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(toml::Value::Table(toml::value::Table::new()));
        }
        Err(source) => {
            return Err(BootstrapError::ReadFile { path: path.to_path_buf(), source });
        }
    };

    let mut value: toml::Value = toml::from_str(&text)
        .map_err(|source| BootstrapError::ParseFile { path: path.to_path_buf(), source })?;
    expand(&mut value)?;
    Ok(value)
}

fn expand(value: &mut toml::Value) -> Result<(), BootstrapError> {
    match value {
        toml::Value::String(text) => {
            *text = subst::substitute(text, &subst::Env)?;
        }
        toml::Value::Array(items) => {
            for item in items {
                expand(item)?;
            }
        }
        toml::Value::Table(table) => {
            for (_key, entry) in table.iter_mut() {
                expand(entry)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::result_large_err)]

    use super::*;

    #[test]
    fn missing_file_is_empty_table() {
        let value = load_expanded(Path::new("does/not/exist.toml")).unwrap();
        assert!(value.as_table().is_some_and(toml::map::Map::is_empty));
    }

    #[test]
    fn a_file_that_exists_but_cannot_be_read_is_an_error() {
        let dir = tempfile::tempdir().unwrap();

        let error = load_expanded(dir.path()).unwrap_err();

        assert!(
            matches!(error, BootstrapError::ReadFile { .. }),
            "only a missing file is allowed to read as an empty config"
        );
    }

    #[test]
    fn expands_env_reference() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("SHADOWMASK_TEST_SECRET", "hunter2");
            let toml = "[[users]]\npassword = \"${SHADOWMASK_TEST_SECRET}\"\n";
            jail.create_file("users.toml", toml)?;
            let value = load_expanded(Path::new("users.toml")).unwrap();
            let password = value["users"][0]["password"].as_str().unwrap();
            assert_eq!(password, "hunter2");
            Ok(())
        });
    }

    #[test]
    fn uses_default_when_var_unset() {
        figment::Jail::expect_with(|jail| {
            jail.create_file("t.toml", "value = \"${SHADOWMASK_UNSET_VAR:fallback}\"\n")?;
            let value = load_expanded(Path::new("t.toml")).unwrap();
            assert_eq!(value["value"].as_str().unwrap(), "fallback");
            Ok(())
        });
    }

    #[test]
    fn missing_required_var_is_error() {
        figment::Jail::expect_with(|jail| {
            jail.create_file("t.toml", "value = \"${SHADOWMASK_STILL_UNSET}\"\n")?;
            let err = load_expanded(Path::new("t.toml")).unwrap_err();
            assert!(matches!(err, BootstrapError::EnvExpansion(_)));
            Ok(())
        });
    }

    #[test]
    fn non_string_leaves_are_untouched() {
        figment::Jail::expect_with(|jail| {
            jail.create_file("t.toml", "count = 3\nflag = true\n")?;
            let value = load_expanded(Path::new("t.toml")).unwrap();
            assert_eq!(value["count"].as_integer(), Some(3));
            assert_eq!(value["flag"].as_bool(), Some(true));
            Ok(())
        });
    }

    #[test]
    fn referenced_vars_are_collected_from_the_files_the_loader_opens() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("users.toml"),
            "[[users]]\nusername = \"${SHADOWMASK_ADMIN_USERNAME:admin}\"\npassword = \"${SHADOWMASK_ADMIN_PASSWORD}\"\nlibraries = [\"${SHADOWMASK_FIRST_LIBRARY}\"]\ncount = 3\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("libraries.toml"), "root = \"${SHADOWMASK_MEDIA_ROOT}\"\n")
            .unwrap();

        let vars = referenced_vars(dir.path());

        assert_eq!(
            vars,
            BTreeSet::from([
                "SHADOWMASK_ADMIN_PASSWORD".to_owned(),
                "SHADOWMASK_ADMIN_USERNAME".to_owned(),
                "SHADOWMASK_FIRST_LIBRARY".to_owned(),
                "SHADOWMASK_MEDIA_ROOT".to_owned(),
            ])
        );
    }

    #[test]
    fn an_absent_directory_references_nothing() {
        assert!(referenced_vars(Path::new("does/not/exist")).is_empty());
    }

    #[test]
    fn unreadable_and_unparseable_files_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("users.toml")).unwrap();
        std::fs::write(dir.path().join("libraries.toml"), "this is = = not toml").unwrap();

        assert!(referenced_vars(dir.path()).is_empty());
    }

    #[test]
    fn a_file_the_loader_never_opens_references_nothing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("notes.toml"), "value = \"${SHADOWMASK_STRAY}\"\n").unwrap();
        std::fs::write(dir.path().join("users.toml"), "value = \"${SHADOWMASK_KEPT}\"\n").unwrap();

        assert_eq!(referenced_vars(dir.path()), BTreeSet::from(["SHADOWMASK_KEPT".to_owned()]));
    }

    #[test]
    fn a_variable_nested_inside_a_default_is_recorded() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("users.toml"),
            "value = \"${SHADOWMASK_OUTER:${SHADOWMASK_INNER}}\"\ntrailing = \"${SHADOWMASK_AFTER}\"\n",
        )
        .unwrap();

        assert_eq!(
            referenced_vars(dir.path()),
            BTreeSet::from([
                "SHADOWMASK_AFTER".to_owned(),
                "SHADOWMASK_INNER".to_owned(),
                "SHADOWMASK_OUTER".to_owned(),
            ])
        );
    }

    #[test]
    fn a_default_after_a_required_variable_in_one_string_is_still_swept() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("users.toml"),
            "value = \"${SHADOWMASK_A}${SHADOWMASK_B:${SHADOWMASK_C}}\"\n",
        )
        .unwrap();

        assert_eq!(
            referenced_vars(dir.path()),
            BTreeSet::from([
                "SHADOWMASK_A".to_owned(),
                "SHADOWMASK_B".to_owned(),
                "SHADOWMASK_C".to_owned(),
            ])
        );
    }

    #[test]
    fn parse_error_is_reported() {
        figment::Jail::expect_with(|jail| {
            jail.create_file("bad.toml", "this is = = not toml")?;
            let err = load_expanded(Path::new("bad.toml")).unwrap_err();
            assert!(matches!(err, BootstrapError::ParseFile { .. }));
            Ok(())
        });
    }
}
