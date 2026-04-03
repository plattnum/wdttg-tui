use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::model::Client;

use super::settings::{AppConfig, BillFrom, Preferences};

/// Returns the default config directory: ~/.wdttg/
pub fn config_dir() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| Error::Config("cannot determine home directory".into()))?;
    Ok(home.join(".wdttg"))
}

/// Returns the default config file path: ~/.wdttg/config.toml
pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// Returns the data directory. Uses config.preferences.data_dir if set,
/// otherwise <config_root>/data/
pub fn data_dir(config: &AppConfig, config_root: &Path) -> PathBuf {
    match &config.preferences.data_dir {
        Some(dir) => dir.clone(),
        None => config_root.join("data"),
    }
}

/// Creates the config and data directories if they don't exist.
pub fn ensure_directories(config: &AppConfig, config_root: &Path) -> Result<()> {
    fs::create_dir_all(config_root)?;
    fs::create_dir_all(data_dir(config, config_root))?;
    Ok(())
}

/// Load config from a specific path.
pub fn load_config_from(path: &Path) -> Result<AppConfig> {
    let contents = fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::NotFound
        } else {
            Error::Io(e)
        }
    })?;
    toml::from_str(&contents)
        .map_err(|e| Error::Config(format!("failed to parse {}: {e}", path.display())))
}

/// Load config from the default path (~/.wdttg/config.toml).
pub fn load_config() -> Result<AppConfig> {
    load_config_from(&config_path()?)
}

/// Save config to a specific path with atomic write (write .tmp, then rename).
pub fn save_config_to(config: &AppConfig, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let toml_str = toml::to_string_pretty(config)
        .map_err(|e| Error::Config(format!("failed to serialize config: {e}")))?;
    let tmp_path = path.with_extension("toml.tmp");
    fs::write(&tmp_path, &toml_str)?;
    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        Error::Io(e)
    })
}

/// Save config to the default path (~/.wdttg/config.toml).
pub fn save_config(config: &AppConfig) -> Result<()> {
    save_config_to(config, &config_path()?)
}

/// Creates a default configuration.
pub fn create_default_config() -> AppConfig {
    AppConfig {
        preferences: Preferences::default(),
        bill_from: BillFrom::default(),
        clients: vec![Client {
            id: "personal".into(),
            name: "Personal".into(),
            color: "#4F46E5".into(),
            rate: 0.0,
            currency: "USD".into(),
            archived: false,
            address: None,
            email: None,
            tax_id: None,
            payment_terms: None,
            notes: None,
            projects: vec![],
            activities: vec![],
        }],
    }
}

/// Load existing config, or create and save a default on first run.
/// Uses the default paths (~/.wdttg/).
pub fn load_or_create_default() -> Result<AppConfig> {
    let root = config_dir()?;
    let path = root.join("config.toml");
    load_or_create_default_at(&root, &path)
}

/// Load existing config from a specific path, or create and save a default.
pub fn load_or_create_default_at(config_root: &Path, config_file: &Path) -> Result<AppConfig> {
    match load_config_from(config_file) {
        Ok(config) => Ok(config),
        Err(Error::NotFound) => {
            let config = create_default_config();
            ensure_directories(&config, config_root)?;
            save_config_to(&config, config_file)?;
            Ok(config)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_personal_client() {
        let config = create_default_config();
        assert_eq!(config.clients.len(), 1);
        assert_eq!(config.clients[0].id, "personal");
        assert_eq!(config.clients[0].rate, 0.0);
        assert_eq!(config.clients[0].currency, "USD");
    }

    #[test]
    fn default_config_serializes_to_valid_toml() {
        let config = create_default_config();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let _: AppConfig = toml::from_str(&toml_str).unwrap();
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let config = create_default_config();
        save_config_to(&config, &path).unwrap();

        let loaded = load_config_from(&path).unwrap();
        assert_eq!(loaded.clients.len(), 1);
        assert_eq!(loaded.clients[0].id, "personal");
        assert_eq!(loaded.preferences.time_format, "24h");
    }

    #[test]
    fn load_missing_returns_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.toml");
        let result = load_config_from(&path);
        assert!(matches!(result, Err(Error::NotFound)));
    }

    #[test]
    fn load_or_create_default_creates_on_first_run() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("wdttg");
        let config_file = root.join("config.toml");

        let config = load_or_create_default_at(&root, &config_file).unwrap();
        assert_eq!(config.clients[0].id, "personal");
        assert!(config_file.exists());
        assert!(root.join("data").exists());
    }

    #[test]
    fn load_or_create_default_loads_existing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("wdttg");
        fs::create_dir_all(&root).unwrap();
        let config_file = root.join("config.toml");
        fs::write(
            &config_file,
            r##"
[preferences]
time_format = "12h"
"##,
        )
        .unwrap();

        let config = load_or_create_default_at(&root, &config_file).unwrap();
        assert_eq!(config.preferences.time_format, "12h");
    }

    #[test]
    fn data_dir_uses_custom_when_set() {
        let root = PathBuf::from("/any");
        let mut config = create_default_config();
        config.preferences.data_dir = Some(PathBuf::from("/custom/data"));
        assert_eq!(data_dir(&config, &root), PathBuf::from("/custom/data"));
    }

    #[test]
    fn data_dir_defaults_to_root_data() {
        let root = PathBuf::from("/home/user/.wdttg");
        let config = create_default_config();
        assert_eq!(
            data_dir(&config, &root),
            PathBuf::from("/home/user/.wdttg/data")
        );
    }

    #[test]
    fn atomic_write_no_tmp_left_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let config = create_default_config();
        save_config_to(&config, &path).unwrap();

        let tmp = dir.path().join("config.toml.tmp");
        assert!(!tmp.exists());
    }

    #[test]
    fn malformed_toml_returns_config_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "this is [not valid { toml").unwrap();

        let result = load_config_from(&path);
        assert!(matches!(result, Err(Error::Config(_))));
    }
}
