use std::path::Path;

use super::BootstrapError;

pub fn load_expanded(path: &Path) -> Result<toml::Value, BootstrapError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(toml::Value::Table(toml::value::Table::new()));
        }
        Err(source) => {
            return Err(BootstrapError::ReadFile {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    let mut value: toml::Value =
        toml::from_str(&text).map_err(|source| BootstrapError::ParseFile {
            path: path.to_path_buf(),
            source,
        })?;
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
    fn expands_env_reference() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("SHADOWMASK_TEST_SECRET", "hunter2");
            jail.create_file(
                "users.toml",
                "[[users]]\npassword = \"${SHADOWMASK_TEST_SECRET}\"\n",
            )?;
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
    fn parse_error_is_reported() {
        figment::Jail::expect_with(|jail| {
            jail.create_file("bad.toml", "this is = = not toml")?;
            let err = load_expanded(Path::new("bad.toml")).unwrap_err();
            assert!(matches!(err, BootstrapError::ParseFile { .. }));
            Ok(())
        });
    }
}
