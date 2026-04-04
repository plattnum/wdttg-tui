use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::model::{Activity, Client, Project};

use super::settings::{AppConfig, BillFrom, Preferences};

/// Returns the XDG config directory: $XDG_CONFIG_HOME/wdttg/ or ~/.config/wdttg/
pub fn config_dir() -> Result<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return Ok(PathBuf::from(xdg).join("wdttg"));
    }
    let home =
        dirs::home_dir().ok_or_else(|| Error::Config("cannot determine home directory".into()))?;
    Ok(home.join(".config").join("wdttg"))
}

/// Returns the config file path: <config_dir>/config.toml
pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// Returns the XDG data directory: $XDG_DATA_HOME/wdttg/data/ or ~/.local/share/wdttg/data/
fn default_data_dir() -> Result<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME")
        && !xdg.is_empty()
    {
        return Ok(PathBuf::from(xdg).join("wdttg").join("data"));
    }
    let home =
        dirs::home_dir().ok_or_else(|| Error::Config("cannot determine home directory".into()))?;
    Ok(home.join(".local").join("share").join("wdttg").join("data"))
}

/// Returns the data directory. Uses config.preferences.data_dir if set,
/// otherwise the XDG data directory.
pub fn data_dir(config: &AppConfig) -> Result<PathBuf> {
    match &config.preferences.data_dir {
        Some(dir) => Ok(dir.clone()),
        None => default_data_dir(),
    }
}

/// Creates the config and data directories if they don't exist.
pub fn ensure_directories(config: &AppConfig) -> Result<()> {
    fs::create_dir_all(config_dir()?)?;
    fs::create_dir_all(data_dir(config)?)?;
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

/// Load config from the default XDG path.
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

/// Save config to the default XDG path.
pub fn save_config(config: &AppConfig) -> Result<()> {
    save_config_to(config, &config_path()?)
}

/// Creates a default configuration with sample clients, projects, and activities.
pub fn create_default_config() -> AppConfig {
    AppConfig {
        preferences: Preferences::default(),
        bill_from: BillFrom::default(),
        clients: vec![
            Client {
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
                projects: vec![
                    Project {
                        id: "side-project".into(),
                        name: "Side Project".into(),
                        color: "#7C3AED".into(),
                        rate_override: None,
                        archived: false,
                    },
                    Project {
                        id: "learning".into(),
                        name: "Learning".into(),
                        color: "#06B6D4".into(),
                        rate_override: None,
                        archived: false,
                    },
                ],
                activities: vec![
                    Activity {
                        id: "development".into(),
                        name: "Development".into(),
                        color: "#2ECC71".into(),
                    },
                    Activity {
                        id: "research".into(),
                        name: "Research".into(),
                        color: "#F59E0B".into(),
                    },
                ],
            },
            Client {
                id: "sample-client".into(),
                name: "Sample Client".into(),
                color: "#FF6B6B".into(),
                rate: 100.0,
                currency: "USD".into(),
                archived: false,
                address: None,
                email: None,
                tax_id: None,
                payment_terms: None,
                notes: None,
                projects: vec![
                    Project {
                        id: "website".into(),
                        name: "Website".into(),
                        color: "#4ECDC4".into(),
                        rate_override: None,
                        archived: false,
                    },
                    Project {
                        id: "mobile-app".into(),
                        name: "Mobile App".into(),
                        color: "#FF8A65".into(),
                        rate_override: None,
                        archived: false,
                    },
                ],
                activities: vec![
                    Activity {
                        id: "development".into(),
                        name: "Development".into(),
                        color: "#2ECC71".into(),
                    },
                    Activity {
                        id: "design".into(),
                        name: "Design".into(),
                        color: "#E040FB".into(),
                    },
                    Activity {
                        id: "meeting".into(),
                        name: "Meeting".into(),
                        color: "#42A5F5".into(),
                    },
                    Activity {
                        id: "review".into(),
                        name: "Code Review".into(),
                        color: "#FFA726".into(),
                    },
                ],
            },
        ],
    }
}

/// Load existing config, or create and save a default on first run.
/// Uses XDG-compliant paths.
pub fn load_or_create_default() -> Result<AppConfig> {
    let path = config_path()?;
    match load_config_from(&path) {
        Ok(config) => Ok(config),
        Err(Error::NotFound) => {
            let config = create_default_config();
            ensure_directories(&config)?;
            save_config_to(&config, &path)?;
            Ok(config)
        }
        Err(e) => Err(e),
    }
}

/// Load existing config from a specific path, or create and save a default.
/// Used by tests with explicit temp directory paths.
pub fn load_or_create_default_at(config_root: &Path, config_file: &Path) -> Result<AppConfig> {
    match load_config_from(config_file) {
        Ok(config) => Ok(config),
        Err(Error::NotFound) => {
            let config = create_default_config();
            fs::create_dir_all(config_root)?;
            fs::create_dir_all(config_root.join("data"))?;
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
    fn default_config_has_two_clients() {
        let config = create_default_config();
        assert_eq!(config.clients.len(), 2);

        let personal = &config.clients[0];
        assert_eq!(personal.id, "personal");
        assert_eq!(personal.rate, 0.0);
        assert_eq!(personal.projects.len(), 2);
        assert_eq!(personal.activities.len(), 2);

        let sample = &config.clients[1];
        assert_eq!(sample.id, "sample-client");
        assert_eq!(sample.rate, 100.0);
        assert_eq!(sample.projects.len(), 2);
        assert_eq!(sample.activities.len(), 4);
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
        assert_eq!(loaded.clients.len(), 2);
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
        let mut config = create_default_config();
        config.preferences.data_dir = Some(PathBuf::from("/custom/data"));
        assert_eq!(data_dir(&config).unwrap(), PathBuf::from("/custom/data"));
    }

    #[test]
    fn data_dir_defaults_to_xdg() {
        let dir = tempfile::tempdir().unwrap();
        let xdg_data = dir.path().join("xdg_data");

        // SAFETY: test-only, not running concurrent threads
        unsafe { std::env::set_var("XDG_DATA_HOME", &xdg_data) };
        let config = create_default_config();
        let result = data_dir(&config).unwrap();
        unsafe { std::env::remove_var("XDG_DATA_HOME") };

        assert_eq!(result, xdg_data.join("wdttg").join("data"));
    }

    #[test]
    fn config_dir_respects_xdg() {
        let dir = tempfile::tempdir().unwrap();
        let xdg_config = dir.path().join("xdg_config");

        // SAFETY: test-only, not running concurrent threads
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &xdg_config) };
        let result = config_dir().unwrap();
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };

        assert_eq!(result, xdg_config.join("wdttg"));
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
