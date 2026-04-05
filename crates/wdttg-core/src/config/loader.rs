use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::model::{Activity, Client, Project};

use super::settings::{AppConfig, BillFrom, ClientDataFile, Preferences, PrefsFile};

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
    data_dir_from_prefs(&config.preferences)
}

/// Returns the data directory from preferences (avoids needing full AppConfig).
fn data_dir_from_prefs(prefs: &Preferences) -> Result<PathBuf> {
    match &prefs.data_dir {
        Some(dir) => Ok(dir.clone()),
        None => default_data_dir(),
    }
}

/// Returns the path to clients.toml in the data directory.
pub fn clients_path(config: &AppConfig) -> Result<PathBuf> {
    Ok(data_dir(config)?.join("clients.toml"))
}

/// Creates the config and data directories if they don't exist.
pub fn ensure_directories(config: &AppConfig) -> Result<()> {
    fs::create_dir_all(config_dir()?)?;
    fs::create_dir_all(data_dir(config)?)?;
    Ok(())
}

// --- Load functions ---

/// Load a TOML file and deserialize it.
fn load_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
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

/// Load the full AppConfig from split files (config.toml + clients.toml).
/// Handles migration from legacy single-file format.
pub fn load_config() -> Result<AppConfig> {
    let cfg_path = config_path()?;

    // Load preferences from config.toml
    let prefs_file: PrefsFile = load_toml(&cfg_path)?;
    let data_path = data_dir_from_prefs(&prefs_file.preferences)?.join("clients.toml");

    // Load clients from clients.toml in data directory
    let client_data: ClientDataFile = match load_toml(&data_path) {
        Ok(data) => data,
        Err(Error::NotFound) => ClientDataFile {
            bill_from: BillFrom::default(),
            clients: vec![],
        },
        Err(e) => return Err(e),
    };

    Ok(AppConfig {
        preferences: prefs_file.preferences,
        bill_from: client_data.bill_from,
        clients: client_data.clients,
    })
}

/// Load config from a specific path (legacy single-file format).
/// Used by tests and migration.
pub fn load_config_from(path: &Path) -> Result<AppConfig> {
    load_toml(path)
}

// --- Save functions ---

/// Atomic TOML write: serialize, write to .tmp, rename.
fn save_toml_atomic<T: serde::Serialize>(value: &T, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let toml_str = toml::to_string_pretty(value)
        .map_err(|e| Error::Config(format!("failed to serialize: {e}")))?;
    let tmp_path = path.with_extension("toml.tmp");
    fs::write(&tmp_path, &toml_str)?;
    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        Error::Io(e)
    })
}

/// Save preferences to config.toml.
pub fn save_config(config: &AppConfig) -> Result<()> {
    save_toml_atomic(
        &PrefsFile {
            preferences: config.preferences.clone(),
        },
        &config_path()?,
    )
}

/// Save config to a specific path (legacy single-file format). Used by tests.
pub fn save_config_to(config: &AppConfig, path: &Path) -> Result<()> {
    save_toml_atomic(config, path)
}

/// Save client data (bill_from + clients) to clients.toml in the data directory.
pub fn save_clients(config: &AppConfig) -> Result<()> {
    let path = clients_path(config)?;
    save_toml_atomic(
        &ClientDataFile {
            bill_from: config.bill_from.clone(),
            clients: config.clients.clone(),
        },
        &path,
    )
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
                        archived: false,
                    },
                    Activity {
                        id: "research".into(),
                        name: "Research".into(),
                        color: "#F59E0B".into(),
                        archived: false,
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
                        archived: false,
                    },
                    Activity {
                        id: "design".into(),
                        name: "Design".into(),
                        color: "#E040FB".into(),
                        archived: false,
                    },
                    Activity {
                        id: "meeting".into(),
                        name: "Meeting".into(),
                        color: "#42A5F5".into(),
                        archived: false,
                    },
                    Activity {
                        id: "review".into(),
                        name: "Code Review".into(),
                        color: "#FFA726".into(),
                        archived: false,
                    },
                ],
            },
        ],
    }
}

/// Load existing config, or create and save defaults on first run.
/// Writes config.toml (preferences) and clients.toml (client data) separately.
pub fn load_or_create_default() -> Result<AppConfig> {
    match load_config() {
        Ok(config) => Ok(config),
        Err(Error::NotFound) => {
            let config = create_default_config();
            ensure_directories(&config)?;
            save_config(&config)?;
            save_clients(&config)?;
            Ok(config)
        }
        Err(e) => Err(e),
    }
}

/// Load existing config from specific paths, or create and save defaults.
/// Used by tests with explicit temp directory paths.
pub fn load_or_create_default_at(config_root: &Path, config_file: &Path) -> Result<AppConfig> {
    match load_config_from(config_file) {
        Ok(config) => Ok(config),
        Err(Error::NotFound) => {
            let config = create_default_config();
            let data_dir = config_root.join("data");
            fs::create_dir_all(config_root)?;
            fs::create_dir_all(&data_dir)?;
            // Tests use legacy single-file format for simplicity
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
    fn split_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let config_file = dir.path().join("config.toml");
        let clients_file = dir.path().join("clients.toml");

        let config = create_default_config();

        // Save split files
        save_toml_atomic(
            &PrefsFile {
                preferences: config.preferences.clone(),
            },
            &config_file,
        )
        .unwrap();
        save_toml_atomic(
            &ClientDataFile {
                bill_from: config.bill_from.clone(),
                clients: config.clients.clone(),
            },
            &clients_file,
        )
        .unwrap();

        // Load back
        let prefs: PrefsFile = load_toml(&config_file).unwrap();
        let data: ClientDataFile = load_toml(&clients_file).unwrap();

        assert_eq!(prefs.preferences.time_format, "24h");
        assert_eq!(data.clients.len(), 2);
        assert_eq!(data.clients[0].id, "personal");
    }

    #[test]
    fn prefs_file_ignores_legacy_client_fields() {
        // Old-format config.toml with clients — PrefsFile should parse fine
        let toml_str = r##"
[preferences]
time_format = "12h"

[bill_from]
name = "Someone"

[[clients]]
id = "test"
name = "Test"
color = "#000"
rate = 0.0
currency = "USD"
"##;
        let prefs: PrefsFile = toml::from_str(toml_str).unwrap();
        assert_eq!(prefs.preferences.time_format, "12h");
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
    fn clients_path_in_data_dir() {
        let mut config = create_default_config();
        config.preferences.data_dir = Some(PathBuf::from("/my/data"));
        assert_eq!(
            clients_path(&config).unwrap(),
            PathBuf::from("/my/data/clients.toml")
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
