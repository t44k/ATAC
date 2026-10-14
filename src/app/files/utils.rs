use std::path::PathBuf;
use directories::UserDirs;

pub fn expand_tilde(path_buf: PathBuf) -> PathBuf {
    if !path_buf.starts_with("~/") {
        return path_buf;
    }

    match UserDirs::new() {
        Some(user_dirs) => {
            let mut home_dir = user_dirs.home_dir().to_path_buf();
            home_dir.push(path_buf.strip_prefix("~/").unwrap());
            home_dir
        },
        None => panic!("No home directory found when trying to expand \"~\"")
    }
}

/// Find a configuration file: the environment variable first, then the app directory, then the user config directory
pub fn find_config_file(env_variable: &str, file_names: &[&str]) -> Option<PathBuf> {
    if let Ok(path) = std::env::var(env_variable) {
        return Some(expand_tilde(PathBuf::from(path)));
    }

    let directories = [crate::cli::args::ARGS.directory.as_ref(), crate::cli::args::ARGS.config_directory.as_ref()];

    for directory in directories.into_iter().flatten() {
        for file_name in file_names {
            let path = directory.join(file_name);

            if path.is_file() {
                return Some(path);
            }
        }
    }

    None
}

/// Deserialize a TOML file that may only contain some of the keys, the missing ones are taken from `defaults`
pub fn parse_partial_toml<T: serde::Serialize + serde::de::DeserializeOwned>(defaults: &T, content: &str) -> anyhow::Result<T> {
    let mut merged = toml::Value::try_from(defaults)?;
    let overrides: toml::Value = toml::from_str(content)?;

    merge_toml(&mut merged, overrides);

    Ok(merged.try_into()?)
}

fn merge_toml(base: &mut toml::Value, overrides: toml::Value) {
    match (base, overrides) {
        (toml::Value::Table(base), toml::Value::Table(overrides)) => {
            for (key, value) in overrides {
                // Files may use the serde aliases (e.g. "UI" for "ui")
                let base_key = base.keys().find(|base_key| is_same_key(base_key, &key)).cloned();

                match base_key.and_then(|base_key| base.get_mut(&base_key)) {
                    Some(base_value) => merge_toml(base_value, value),
                    None => {
                        base.insert(key, value);
                    }
                }
            }
        },
        (base, overrides) => *base = overrides
    }
}

fn is_same_key(base_key: &str, key: &str) -> bool {
    let normalize = |key: &str| key.to_lowercase().replace([' ', '_', '-'], "");

    let base_key = normalize(base_key);
    let key = normalize(key);

    // Plural aliases, e.g. "Connection Statuses" for "connection_status"
    base_key == key || (key.starts_with(&base_key) && key.len() - base_key.len() <= 2)
}
