use crate::model::TimeEntry;

const DATETIME_FMT: &str = "%Y-%m-%d %H:%M";

/// Serialize entries into a GFM markdown table for a monthly file.
/// Entries are sorted by start time before writing.
pub fn serialize_entries(month_key: &str, entries: &[TimeEntry]) -> String {
    let mut sorted: Vec<&TimeEntry> = entries.iter().collect();
    sorted.sort_by_key(|e| e.start);

    let mut out = String::new();
    out.push_str(&format!("# {month_key}\n\n"));
    out.push_str("| Start | End | Description | Client | Project | Activity | Notes |\n");
    out.push_str("|-------|-----|-------------|--------|---------|----------|-------|\n");

    for entry in sorted {
        let start = entry.start.format(DATETIME_FMT);
        let end = entry.end.format(DATETIME_FMT);
        let desc = escape_pipe(&entry.description);
        let client = escape_pipe(&entry.client);
        let project = entry
            .project
            .as_deref()
            .map(escape_pipe)
            .unwrap_or_default();
        let activity = entry
            .activity
            .as_deref()
            .map(escape_pipe)
            .unwrap_or_default();
        let notes = entry.notes.as_deref().map(escape_pipe).unwrap_or_default();

        out.push_str(&format!(
            "| {start} | {end} | {desc} | {client} | {project} | {activity} | {notes} |\n"
        ));
    }

    out
}

/// Escape pipe characters in cell content so they don't break the table.
fn escape_pipe(s: &str) -> String {
    s.replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::parser::parse_month_file;
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
            description: "Sprint planning".into(),
            client: "acme".into(),
            project: Some("webapp".into()),
            activity: Some("meeting".into()),
            notes: None,
        }
    }

    #[test]
    fn serialize_single_entry() {
        let entries = vec![sample_entry()];
        let output = serialize_entries("2026-03", &entries);
        assert!(output.starts_with("# 2026-03\n"));
        assert!(output.contains("| 2026-03-15 09:00 | 2026-03-15 10:30 |"));
        assert!(output.contains("| Sprint planning |"));
        assert!(output.contains("| acme |"));
    }

    #[test]
    fn serialize_empty_optional_fields() {
        let entry = TimeEntry {
            start: dt(2026, 3, 15, 9, 0),
            end: dt(2026, 3, 15, 10, 0),
            description: "Work".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        };
        let output = serialize_entries("2026-03", &[entry]);
        // Empty optional fields should be blank between pipes
        assert!(output.contains("|  |  |  |"));
    }

    #[test]
    fn serialize_entries_sorted() {
        let entries = vec![
            TimeEntry {
                start: dt(2026, 3, 15, 14, 0),
                end: dt(2026, 3, 15, 15, 0),
                description: "Afternoon".into(),
                client: "acme".into(),
                project: None,
                activity: None,
                notes: None,
            },
            TimeEntry {
                start: dt(2026, 3, 15, 9, 0),
                end: dt(2026, 3, 15, 10, 0),
                description: "Morning".into(),
                client: "acme".into(),
                project: None,
                activity: None,
                notes: None,
            },
        ];
        let output = serialize_entries("2026-03", &entries);
        let morning_pos = output.find("Morning").unwrap();
        let afternoon_pos = output.find("Afternoon").unwrap();
        assert!(morning_pos < afternoon_pos);
    }

    #[test]
    fn roundtrip_parse_serialize_parse() {
        let entries = vec![
            TimeEntry {
                start: dt(2026, 3, 15, 9, 0),
                end: dt(2026, 3, 15, 10, 30),
                description: "Sprint planning".into(),
                client: "acme".into(),
                project: Some("webapp".into()),
                activity: Some("meeting".into()),
                notes: None,
            },
            TimeEntry {
                start: dt(2026, 3, 15, 10, 30),
                end: dt(2026, 3, 15, 12, 0),
                description: "Implement auth<br>flow".into(),
                client: "acme".into(),
                project: Some("webapp".into()),
                activity: Some("dev".into()),
                notes: Some("See auth-notes.md".into()),
            },
            TimeEntry {
                start: dt(2026, 3, 15, 23, 0),
                end: dt(2026, 3, 16, 2, 0),
                description: "Hotfix".into(),
                client: "acme".into(),
                project: Some("webapp".into()),
                activity: Some("fix".into()),
                notes: Some("Incident #42".into()),
            },
        ];

        let serialized = serialize_entries("2026-03", &entries);
        let parsed = parse_month_file(&serialized).unwrap();

        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].description, "Sprint planning");
        assert_eq!(parsed[1].description, "Implement auth<br>flow");
        assert_eq!(parsed[2].notes.as_deref(), Some("Incident #42"));
        assert_eq!(parsed[2].start, dt(2026, 3, 15, 23, 0));
        assert_eq!(parsed[2].end, dt(2026, 3, 16, 2, 0));
    }

    #[test]
    fn pipe_in_content_escaped() {
        let entry = TimeEntry {
            start: dt(2026, 3, 15, 9, 0),
            end: dt(2026, 3, 15, 10, 0),
            description: "A | B".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        };
        let output = serialize_entries("2026-03", &[entry]);
        assert!(output.contains(r"A \| B"));
    }
}
