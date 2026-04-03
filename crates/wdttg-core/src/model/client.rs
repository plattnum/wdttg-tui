use serde::{Deserialize, Serialize};

/// A client with nested projects and activities. Clients own projects and activities (not global).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Client {
    pub id: String,
    pub name: String,
    pub color: String,
    pub rate: f64,
    pub currency: String,
    #[serde(default)]
    pub archived: bool,
    pub address: Option<String>,
    pub email: Option<String>,
    pub tax_id: Option<String>,
    pub payment_terms: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub activities: Vec<Activity>,
}

/// A project scoped to a client. rate_override takes precedence over the client rate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: String,
    pub rate_override: Option<f64>,
    #[serde(default)]
    pub archived: bool,
}

/// An activity type scoped to a client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_roundtrip_toml() {
        let toml_str = r##"
id = "acme"
name = "Acme Corp"
color = "#FF6B6B"
rate = 150.0
currency = "USD"
archived = false
address = "456 Corporate Blvd\nSan Francisco, CA 94105"
email = "billing@acme.com"
payment_terms = "Net 30"

[[projects]]
id = "webapp"
name = "Web Application"
color = "#4ECDC4"
rate_override = 175.0

[[projects]]
id = "mobile"
name = "Mobile App"
color = "#45B7D1"

[[activities]]
id = "dev"
name = "Development"
color = "#2ECC71"
"##;
        let client: Client = toml::from_str(toml_str).unwrap();
        assert_eq!(client.id, "acme");
        assert_eq!(client.rate, 150.0);
        assert_eq!(client.projects.len(), 2);
        assert_eq!(client.projects[0].rate_override, Some(175.0));
        assert_eq!(client.projects[1].rate_override, None);
        assert_eq!(client.activities.len(), 1);
        assert!(!client.archived);

        // Round-trip
        let serialized = toml::to_string(&client).unwrap();
        let deserialized: Client = toml::from_str(&serialized).unwrap();
        assert_eq!(client, deserialized);
    }

    #[test]
    fn defaults_for_missing_optional_fields() {
        let toml_str = r##"
id = "personal"
name = "Personal"
color = "#4F46E5"
rate = 0.0
currency = "USD"
"##;
        let client: Client = toml::from_str(toml_str).unwrap();
        assert!(!client.archived);
        assert!(client.projects.is_empty());
        assert!(client.activities.is_empty());
        assert!(client.address.is_none());
        assert!(client.email.is_none());
        assert!(client.tax_id.is_none());
        assert!(client.payment_terms.is_none());
        assert!(client.notes.is_none());
    }
}
