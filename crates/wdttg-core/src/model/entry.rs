use std::hash::{DefaultHasher, Hash, Hasher};

use chrono::NaiveDateTime;

/// A time tracking entry. Identity is (start, end) since overlaps are forbidden.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeEntry {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub description: String,
    pub client: String,
    pub project: Option<String>,
    pub activity: Option<String>,
    pub notes: Option<String>,
}

impl TimeEntry {
    /// Duration in minutes.
    pub fn duration_minutes(&self) -> i64 {
        (self.end - self.start).num_minutes()
    }

    /// Month key (YYYY-MM) for file routing. Based on start time.
    pub fn month_key(&self) -> String {
        self.start.format("%Y-%m").to_string()
    }

    /// Deterministic short hash ID from (start, end).
    /// Format: `e_XXXXXXXX` (8 hex chars). Computed on read, not stored.
    pub fn entry_id(&self) -> String {
        compute_entry_id(&self.start, &self.end)
    }
}

/// Compute a deterministic entry ID from start and end times.
pub fn compute_entry_id(start: &NaiveDateTime, end: &NaiveDateTime) -> String {
    let mut hasher = DefaultHasher::new();
    start.hash(&mut hasher);
    end.hash(&mut hasher);
    format!("e_{:08x}", hasher.finish() & 0xFFFF_FFFF)
}

/// Input struct for creating or updating an entry.
#[derive(Debug, Clone)]
pub struct NewEntry {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub description: String,
    pub client: String,
    pub project: Option<String>,
    pub activity: Option<String>,
    pub notes: Option<String>,
}

impl From<NewEntry> for TimeEntry {
    fn from(new: NewEntry) -> Self {
        Self {
            start: new.start,
            end: new.end,
            description: new.description,
            client: new.client,
            project: new.project,
            activity: new.activity,
            notes: new.notes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_entry(start: (u32, u32, u32, u32, u32), end: (u32, u32, u32, u32, u32)) -> TimeEntry {
        TimeEntry {
            start: NaiveDate::from_ymd_opt(start.0 as i32, start.1, start.2)
                .unwrap()
                .and_hms_opt(start.3, start.4, 0)
                .unwrap(),
            end: NaiveDate::from_ymd_opt(end.0 as i32, end.1, end.2)
                .unwrap()
                .and_hms_opt(end.3, end.4, 0)
                .unwrap(),
            description: "test".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        }
    }

    #[test]
    fn entry_id_format() {
        let e = make_entry((2026, 3, 15, 9, 0), (2026, 3, 15, 10, 0));
        let id = e.entry_id();
        assert!(id.starts_with("e_"));
        assert_eq!(id.len(), 10); // "e_" + 8 hex chars
    }

    #[test]
    fn entry_id_deterministic() {
        let e1 = make_entry((2026, 3, 15, 9, 0), (2026, 3, 15, 10, 0));
        let e2 = make_entry((2026, 3, 15, 9, 0), (2026, 3, 15, 10, 0));
        assert_eq!(e1.entry_id(), e2.entry_id());
    }

    #[test]
    fn entry_id_different_for_different_times() {
        let e1 = make_entry((2026, 3, 15, 9, 0), (2026, 3, 15, 10, 0));
        let e2 = make_entry((2026, 3, 15, 9, 0), (2026, 3, 15, 11, 0));
        assert_ne!(e1.entry_id(), e2.entry_id());
    }

    #[test]
    fn entry_id_different_start_same_end() {
        let e1 = make_entry((2026, 3, 15, 9, 0), (2026, 3, 15, 10, 0));
        let e2 = make_entry((2026, 3, 15, 8, 0), (2026, 3, 15, 10, 0));
        assert_ne!(e1.entry_id(), e2.entry_id());
    }

    #[test]
    fn duration_one_hour() {
        let e = make_entry((2026, 3, 15, 9, 0), (2026, 3, 15, 10, 0));
        assert_eq!(e.duration_minutes(), 60);
    }

    #[test]
    fn duration_90_minutes() {
        let e = make_entry((2026, 3, 15, 9, 0), (2026, 3, 15, 10, 30));
        assert_eq!(e.duration_minutes(), 90);
    }

    #[test]
    fn duration_midnight_spanning() {
        let e = make_entry((2026, 3, 15, 23, 0), (2026, 3, 16, 2, 0));
        assert_eq!(e.duration_minutes(), 180);
    }

    #[test]
    fn month_key_format() {
        let e = make_entry((2026, 3, 15, 9, 0), (2026, 3, 15, 10, 0));
        assert_eq!(e.month_key(), "2026-03");
    }

    #[test]
    fn month_key_uses_start_not_end() {
        let e = make_entry((2026, 3, 31, 23, 0), (2026, 4, 1, 1, 0));
        assert_eq!(e.month_key(), "2026-03");
    }

    #[test]
    fn new_entry_converts_to_time_entry() {
        let new = NewEntry {
            start: NaiveDate::from_ymd_opt(2026, 3, 15)
                .unwrap()
                .and_hms_opt(9, 0, 0)
                .unwrap(),
            end: NaiveDate::from_ymd_opt(2026, 3, 15)
                .unwrap()
                .and_hms_opt(10, 0, 0)
                .unwrap(),
            description: "planning".into(),
            client: "acme".into(),
            project: Some("webapp".into()),
            activity: Some("meeting".into()),
            notes: Some("sprint planning".into()),
        };
        let entry: TimeEntry = new.into();
        assert_eq!(entry.client, "acme");
        assert_eq!(entry.project.as_deref(), Some("webapp"));
        assert_eq!(entry.activity.as_deref(), Some("meeting"));
        assert_eq!(entry.notes.as_deref(), Some("sprint planning"));
    }
}
