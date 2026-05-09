use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::Client;

/// Combined in-memory application state. Loaded from two files:
/// - Preferences from `~/.config/wdttg/config.toml`
/// - Clients/bill_from from `<data_dir>/clients.toml`
///
/// Also supports the legacy single-file format for migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub preferences: Preferences,
    #[serde(default)]
    pub bill_from: BillFrom,
    #[serde(default)]
    pub clients: Vec<Client>,
}

/// Preferences-only file format for config.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefsFile {
    #[serde(default)]
    pub preferences: Preferences,
}

/// Client data file format for clients.toml (lives in data directory).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientDataFile {
    #[serde(default)]
    pub bill_from: BillFrom,
    #[serde(default)]
    pub clients: Vec<Client>,
}

/// Visual style for tag chips on timeline entries.
///
/// - `Flat`: each chip rendered with a 1-col space separator. Works on any terminal.
/// - `Powerline`: chips separated by U+E0B0 right-triangle chevrons that blend
///   between adjacent chip colors. Requires a Powerline / Nerd Font in the terminal.
/// - `Triangle`: chips separated by U+25B6 black right-pointing triangle. Works on
///   any Unicode terminal but doesn't blend as smoothly as the Powerline glyph.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TagStyle {
    #[default]
    Flat,
    Powerline,
    Triangle,
}

impl TagStyle {
    /// Next style in cycle order. Used by the TUI's tag-style toggle key.
    pub fn next(self) -> Self {
        match self {
            Self::Flat => Self::Powerline,
            Self::Powerline => Self::Triangle,
            Self::Triangle => Self::Flat,
        }
    }

    /// Lowercase display name, matches the `serde(rename_all = "lowercase")`
    /// representation used in config.toml.
    pub fn name(self) -> &'static str {
        match self {
            Self::Flat => "flat",
            Self::Powerline => "powerline",
            Self::Triangle => "triangle",
        }
    }
}

/// User preferences for display and behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default = "default_time_format")]
    pub time_format: String,
    #[serde(default = "default_week_start")]
    pub week_start: String,
    #[serde(default = "default_day_start_hour")]
    pub day_start_hour: u32,
    #[serde(default = "default_day_end_hour")]
    pub day_end_hour: u32,
    #[serde(default = "default_snap_minutes")]
    pub snap_minutes: u32,
    #[serde(default = "default_description_max_length")]
    pub description_max_length: u32,
    #[serde(default)]
    pub data_dir: Option<PathBuf>,
    #[serde(default)]
    pub tag_style: TagStyle,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            time_format: default_time_format(),
            week_start: default_week_start(),
            day_start_hour: default_day_start_hour(),
            day_end_hour: default_day_end_hour(),
            snap_minutes: default_snap_minutes(),
            description_max_length: default_description_max_length(),
            data_dir: None,
            tag_style: TagStyle::default(),
        }
    }
}

fn default_time_format() -> String {
    "24h".into()
}

fn default_week_start() -> String {
    "monday".into()
}

fn default_day_start_hour() -> u32 {
    6
}

fn default_day_end_hour() -> u32 {
    22
}

fn default_snap_minutes() -> u32 {
    15
}

fn default_description_max_length() -> u32 {
    200
}

/// Billing identity for invoices.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BillFrom {
    #[serde(default)]
    pub name: String,
    pub address: Option<String>,
    pub email: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preferences() {
        let prefs = Preferences::default();
        assert_eq!(prefs.time_format, "24h");
        assert_eq!(prefs.week_start, "monday");
        assert_eq!(prefs.day_start_hour, 6);
        assert_eq!(prefs.day_end_hour, 22);
        assert_eq!(prefs.snap_minutes, 15);
        assert_eq!(prefs.description_max_length, 200);
        assert!(prefs.data_dir.is_none());
        assert_eq!(prefs.tag_style, TagStyle::Flat);
    }

    #[test]
    fn tag_style_parses_lowercase_variants() {
        let toml_str = r#"
[preferences]
tag_style = "powerline"
"#;
        let prefs: PrefsFile = toml::from_str(toml_str).unwrap();
        assert_eq!(prefs.preferences.tag_style, TagStyle::Powerline);

        let toml_str = r#"
[preferences]
tag_style = "triangle"
"#;
        let prefs: PrefsFile = toml::from_str(toml_str).unwrap();
        assert_eq!(prefs.preferences.tag_style, TagStyle::Triangle);

        let toml_str = r#"
[preferences]
tag_style = "flat"
"#;
        let prefs: PrefsFile = toml::from_str(toml_str).unwrap();
        assert_eq!(prefs.preferences.tag_style, TagStyle::Flat);
    }

    #[test]
    fn tag_style_defaults_to_flat_when_missing() {
        let prefs: PrefsFile = toml::from_str("").unwrap();
        assert_eq!(prefs.preferences.tag_style, TagStyle::Flat);
    }

    #[test]
    fn tag_style_cycles_flat_powerline_triangle() {
        assert_eq!(TagStyle::Flat.next(), TagStyle::Powerline);
        assert_eq!(TagStyle::Powerline.next(), TagStyle::Triangle);
        assert_eq!(TagStyle::Triangle.next(), TagStyle::Flat);
    }

    #[test]
    fn tag_style_name_matches_serde_repr() {
        assert_eq!(TagStyle::Flat.name(), "flat");
        assert_eq!(TagStyle::Powerline.name(), "powerline");
        assert_eq!(TagStyle::Triangle.name(), "triangle");
    }

    #[test]
    fn config_deserializes_with_missing_optional_fields() {
        let toml_str = "";
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.preferences.time_format, "24h");
        assert!(config.clients.is_empty());
        assert_eq!(config.bill_from.name, "");
    }

    #[test]
    fn config_roundtrip_toml() {
        let config = AppConfig {
            preferences: Preferences::default(),
            bill_from: BillFrom {
                name: "Jane Freelancer".into(),
                address: Some("123 Main St".into()),
                email: Some("jane@example.com".into()),
            },
            clients: vec![Client {
                id: "acme".into(),
                name: "Acme Corp".into(),
                color: "#FF6B6B".into(),
                rate: 150.0,
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
        };
        let toml_str = toml::to_string(&config).unwrap();
        let loaded: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(loaded.bill_from.name, "Jane Freelancer");
        assert_eq!(loaded.clients.len(), 1);
        assert_eq!(loaded.clients[0].id, "acme");
    }

    #[test]
    fn full_prd_config_parses() {
        let toml_str = r##"
[preferences]
time_format = "24h"
week_start = "monday"
day_start_hour = 6
day_end_hour = 22
snap_minutes = 15
description_max_length = 200

[bill_from]
name = "Jane Freelancer"
address = "123 Main St\nNew York, NY 10001"
email = "jane@example.com"

[[clients]]
id = "acme"
name = "Acme Corp"
color = "#FF6B6B"
rate = 150.0
currency = "USD"
archived = false
address = "456 Corporate Blvd\nSan Francisco, CA 94105"
email = "billing@acme.com"
payment_terms = "Net 30"

[[clients.projects]]
id = "webapp"
name = "Web Application"
color = "#4ECDC4"
rate_override = 175.0

[[clients.activities]]
id = "dev"
name = "Development"
color = "#2ECC71"

[[clients]]
id = "personal"
name = "Personal"
color = "#4F46E5"
rate = 0.0
currency = "USD"
archived = false
"##;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.clients.len(), 2);
        assert_eq!(config.clients[0].projects.len(), 1);
        assert_eq!(config.clients[0].activities.len(), 1);
        assert_eq!(config.clients[0].projects[0].rate_override, Some(175.0));
        assert_eq!(config.bill_from.name, "Jane Freelancer");
    }
}
