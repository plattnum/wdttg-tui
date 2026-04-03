use super::TimeEntry;

/// Filter criteria for time entries. All fields are optional.
/// When multiple fields are set, they are AND-combined.
#[derive(Debug, Clone, Default)]
pub struct EntryFilter {
    pub client: Option<String>,
    pub project: Option<String>,
    pub activity: Option<String>,
    pub description_contains: Option<String>,
}

impl EntryFilter {
    /// Returns true if the entry matches all set filter fields.
    pub fn matches(&self, entry: &TimeEntry) -> bool {
        if let Some(ref c) = self.client
            && entry.client != *c
        {
            return false;
        }
        if let Some(ref p) = self.project
            && entry.project.as_deref() != Some(p.as_str())
        {
            return false;
        }
        if let Some(ref a) = self.activity
            && entry.activity.as_deref() != Some(a.as_str())
        {
            return false;
        }
        if let Some(ref d) = self.description_contains
            && !entry.description.to_lowercase().contains(&d.to_lowercase())
        {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    fn sample_entry() -> TimeEntry {
        TimeEntry {
            start: dt(2026, 3, 15, 9, 0),
            end: dt(2026, 3, 15, 10, 30),
            description: "Sprint planning meeting".into(),
            client: "acme".into(),
            project: Some("webapp".into()),
            activity: Some("meeting".into()),
            notes: None,
        }
    }

    #[test]
    fn empty_filter_matches_all() {
        let filter = EntryFilter::default();
        assert!(filter.matches(&sample_entry()));
    }

    #[test]
    fn filter_by_client_match() {
        let filter = EntryFilter {
            client: Some("acme".into()),
            ..Default::default()
        };
        assert!(filter.matches(&sample_entry()));
    }

    #[test]
    fn filter_by_client_no_match() {
        let filter = EntryFilter {
            client: Some("other".into()),
            ..Default::default()
        };
        assert!(!filter.matches(&sample_entry()));
    }

    #[test]
    fn filter_by_project_match() {
        let filter = EntryFilter {
            project: Some("webapp".into()),
            ..Default::default()
        };
        assert!(filter.matches(&sample_entry()));
    }

    #[test]
    fn filter_by_project_no_match() {
        let filter = EntryFilter {
            project: Some("api".into()),
            ..Default::default()
        };
        assert!(!filter.matches(&sample_entry()));
    }

    #[test]
    fn filter_by_project_entry_has_none() {
        let filter = EntryFilter {
            project: Some("webapp".into()),
            ..Default::default()
        };
        let mut entry = sample_entry();
        entry.project = None;
        assert!(!filter.matches(&entry));
    }

    #[test]
    fn filter_by_activity_match() {
        let filter = EntryFilter {
            activity: Some("meeting".into()),
            ..Default::default()
        };
        assert!(filter.matches(&sample_entry()));
    }

    #[test]
    fn filter_by_activity_no_match() {
        let filter = EntryFilter {
            activity: Some("dev".into()),
            ..Default::default()
        };
        assert!(!filter.matches(&sample_entry()));
    }

    #[test]
    fn filter_by_description_case_insensitive() {
        let filter = EntryFilter {
            description_contains: Some("SPRINT".into()),
            ..Default::default()
        };
        assert!(filter.matches(&sample_entry()));
    }

    #[test]
    fn filter_by_description_partial_match() {
        let filter = EntryFilter {
            description_contains: Some("planning".into()),
            ..Default::default()
        };
        assert!(filter.matches(&sample_entry()));
    }

    #[test]
    fn filter_by_description_no_match() {
        let filter = EntryFilter {
            description_contains: Some("standup".into()),
            ..Default::default()
        };
        assert!(!filter.matches(&sample_entry()));
    }

    #[test]
    fn filter_multiple_fields_all_match() {
        let filter = EntryFilter {
            client: Some("acme".into()),
            project: Some("webapp".into()),
            activity: Some("meeting".into()),
            description_contains: Some("sprint".into()),
        };
        assert!(filter.matches(&sample_entry()));
    }

    #[test]
    fn filter_multiple_fields_one_fails() {
        let filter = EntryFilter {
            client: Some("acme".into()),
            project: Some("webapp".into()),
            activity: Some("dev".into()), // doesn't match
            description_contains: Some("sprint".into()),
        };
        assert!(!filter.matches(&sample_entry()));
    }

    #[test]
    fn filter_client_and_project() {
        let filter = EntryFilter {
            client: Some("acme".into()),
            project: Some("webapp".into()),
            ..Default::default()
        };
        assert!(filter.matches(&sample_entry()));
    }
}
