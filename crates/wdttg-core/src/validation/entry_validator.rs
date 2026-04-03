use crate::config::AppConfig;
use crate::error::{Error, Result};
use crate::model::{Activity, Client, NewEntry, Project};

/// Look up a client by ID in the config.
pub fn find_client<'a>(config: &'a AppConfig, client_id: &str) -> Option<&'a Client> {
    config.clients.iter().find(|c| c.id == client_id)
}

/// Look up a project by ID within a client.
pub fn find_project<'a>(client: &'a Client, project_id: &str) -> Option<&'a Project> {
    client.projects.iter().find(|p| p.id == project_id)
}

/// Look up an activity by ID within a client.
pub fn find_activity<'a>(client: &'a Client, activity_id: &str) -> Option<&'a Activity> {
    client.activities.iter().find(|a| a.id == activity_id)
}

/// Validate a new entry against temporal constraints and config rules.
pub fn validate_new_entry(entry: &NewEntry, config: &AppConfig) -> Result<()> {
    // end must be after start
    if entry.end <= entry.start {
        return Err(Error::Validation("end must be after start".into()));
    }

    // Duration <= 24 hours (1440 minutes)
    let duration = (entry.end - entry.start).num_minutes();
    if duration > 1440 {
        return Err(Error::Validation(format!(
            "duration {duration} minutes exceeds 24-hour limit"
        )));
    }

    // Client must exist and not be archived
    let client = find_client(config, &entry.client).ok_or_else(|| {
        Error::Validation(format!("client \"{}\" not found in config", entry.client))
    })?;
    if client.archived {
        return Err(Error::Validation(format!(
            "client \"{}\" is archived",
            entry.client
        )));
    }

    // Project (if specified) must exist under this client
    if let Some(ref project_id) = entry.project
        && find_project(client, project_id).is_none()
    {
        return Err(Error::Validation(format!(
            "project \"{project_id}\" not found under client \"{}\"",
            entry.client
        )));
    }

    // Activity (if specified) must exist under this client
    if let Some(ref activity_id) = entry.activity
        && find_activity(client, activity_id).is_none()
    {
        return Err(Error::Validation(format!(
            "activity \"{activity_id}\" not found under client \"{}\"",
            entry.client
        )));
    }

    // Description length check
    let max_len = config.preferences.description_max_length;
    if max_len > 0 && entry.description.len() > max_len as usize {
        return Err(Error::Validation(format!(
            "description length {} exceeds max {}",
            entry.description.len(),
            max_len
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Preferences;
    use crate::model::{Activity, Client, Project};
    use chrono::NaiveDate;

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    fn test_config() -> AppConfig {
        AppConfig {
            preferences: Preferences {
                description_max_length: 200,
                ..Preferences::default()
            },
            bill_from: Default::default(),
            clients: vec![
                Client {
                    id: "acme".into(),
                    name: "Acme Corp".into(),
                    color: "#FF0000".into(),
                    rate: 150.0,
                    currency: "USD".into(),
                    archived: false,
                    address: None,
                    email: None,
                    tax_id: None,
                    payment_terms: None,
                    notes: None,
                    projects: vec![Project {
                        id: "webapp".into(),
                        name: "Web App".into(),
                        color: "#00FF00".into(),
                        rate_override: None,
                        archived: false,
                    }],
                    activities: vec![Activity {
                        id: "dev".into(),
                        name: "Development".into(),
                        color: "#0000FF".into(),
                    }],
                },
                Client {
                    id: "old".into(),
                    name: "Old Client".into(),
                    color: "#888888".into(),
                    rate: 100.0,
                    currency: "USD".into(),
                    archived: true,
                    address: None,
                    email: None,
                    tax_id: None,
                    payment_terms: None,
                    notes: None,
                    projects: vec![],
                    activities: vec![],
                },
            ],
        }
    }

    fn valid_entry() -> NewEntry {
        NewEntry {
            start: dt(2026, 3, 15, 9, 0),
            end: dt(2026, 3, 15, 10, 0),
            description: "Work".into(),
            client: "acme".into(),
            project: Some("webapp".into()),
            activity: Some("dev".into()),
            notes: None,
        }
    }

    #[test]
    fn valid_entry_passes() {
        let config = test_config();
        assert!(validate_new_entry(&valid_entry(), &config).is_ok());
    }

    #[test]
    fn end_before_start_fails() {
        let config = test_config();
        let mut entry = valid_entry();
        entry.end = dt(2026, 3, 15, 8, 0);
        let err = validate_new_entry(&entry, &config).unwrap_err();
        assert!(err.to_string().contains("end must be after start"));
    }

    #[test]
    fn end_equals_start_fails() {
        let config = test_config();
        let mut entry = valid_entry();
        entry.end = entry.start;
        assert!(validate_new_entry(&entry, &config).is_err());
    }

    #[test]
    fn duration_over_24h_fails() {
        let config = test_config();
        let mut entry = valid_entry();
        entry.end = dt(2026, 3, 16, 10, 0); // 25 hours
        let err = validate_new_entry(&entry, &config).unwrap_err();
        assert!(err.to_string().contains("24-hour limit"));
    }

    #[test]
    fn midnight_spanning_under_24h_passes() {
        let config = test_config();
        let mut entry = valid_entry();
        entry.start = dt(2026, 3, 15, 23, 0);
        entry.end = dt(2026, 3, 16, 2, 0);
        assert!(validate_new_entry(&entry, &config).is_ok());
    }

    #[test]
    fn nonexistent_client_fails() {
        let config = test_config();
        let mut entry = valid_entry();
        entry.client = "nope".into();
        let err = validate_new_entry(&entry, &config).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn archived_client_fails() {
        let config = test_config();
        let mut entry = valid_entry();
        entry.client = "old".into();
        entry.project = None;
        entry.activity = None;
        let err = validate_new_entry(&entry, &config).unwrap_err();
        assert!(err.to_string().contains("archived"));
    }

    #[test]
    fn invalid_project_fails() {
        let config = test_config();
        let mut entry = valid_entry();
        entry.project = Some("nonexistent".into());
        let err = validate_new_entry(&entry, &config).unwrap_err();
        assert!(err.to_string().contains("project"));
    }

    #[test]
    fn invalid_activity_fails() {
        let config = test_config();
        let mut entry = valid_entry();
        entry.activity = Some("nonexistent".into());
        let err = validate_new_entry(&entry, &config).unwrap_err();
        assert!(err.to_string().contains("activity"));
    }

    #[test]
    fn none_project_activity_passes() {
        let config = test_config();
        let mut entry = valid_entry();
        entry.project = None;
        entry.activity = None;
        assert!(validate_new_entry(&entry, &config).is_ok());
    }

    #[test]
    fn description_too_long_fails() {
        let config = test_config();
        let mut entry = valid_entry();
        entry.description = "x".repeat(201);
        let err = validate_new_entry(&entry, &config).unwrap_err();
        assert!(err.to_string().contains("description length"));
    }

    #[test]
    fn description_unlimited_when_zero() {
        let mut config = test_config();
        config.preferences.description_max_length = 0;
        let mut entry = valid_entry();
        entry.description = "x".repeat(10000);
        assert!(validate_new_entry(&entry, &config).is_ok());
    }

    #[test]
    fn find_client_works() {
        let config = test_config();
        assert!(find_client(&config, "acme").is_some());
        assert!(find_client(&config, "nope").is_none());
    }

    #[test]
    fn find_project_works() {
        let config = test_config();
        let client = find_client(&config, "acme").unwrap();
        assert!(find_project(client, "webapp").is_some());
        assert!(find_project(client, "nope").is_none());
    }

    #[test]
    fn find_activity_works() {
        let config = test_config();
        let client = find_client(&config, "acme").unwrap();
        assert!(find_activity(client, "dev").is_some());
        assert!(find_activity(client, "nope").is_none());
    }
}
