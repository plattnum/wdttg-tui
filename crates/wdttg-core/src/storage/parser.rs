use chrono::NaiveDateTime;

use crate::error::{Error, Result};
use crate::model::TimeEntry;

const DATETIME_FMT: &str = "%Y-%m-%d %H:%M";
const EXPECTED_COLUMNS: usize = 7;

/// Parse a monthly markdown file into a sorted Vec of TimeEntry.
/// Returns an empty vec for empty content.
pub fn parse_month_file(content: &str) -> Result<Vec<TimeEntry>> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Ok(vec![]);
    }

    // Find the header row (starts with | Start)
    let header_idx = lines
        .iter()
        .position(|l| {
            let trimmed = l.trim();
            trimmed.starts_with('|') && trimmed.to_lowercase().contains("start")
        })
        .ok_or_else(|| Error::Parse {
            line: 1,
            message: "no table header found".into(),
        })?;

    // Validate header columns
    let header_cols = split_row(lines[header_idx]);
    if header_cols.len() != EXPECTED_COLUMNS {
        return Err(Error::Parse {
            line: header_idx + 1,
            message: format!(
                "expected {EXPECTED_COLUMNS} columns, found {}",
                header_cols.len()
            ),
        });
    }

    // Skip separator row (header_idx + 1), parse data rows
    let data_start = header_idx + 2;
    let mut entries = Vec::new();

    for (i, line) in lines.iter().enumerate().skip(data_start) {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('|') {
            continue;
        }

        let cols = split_row(trimmed);
        if cols.len() != EXPECTED_COLUMNS {
            return Err(Error::Parse {
                line: i + 1,
                message: format!("expected {EXPECTED_COLUMNS} columns, found {}", cols.len()),
            });
        }

        let line_num = i + 1;
        let start = parse_datetime(cols[0], line_num, "Start")?;
        let end = parse_datetime(cols[1], line_num, "End")?;

        let client = cols[3].to_string();
        if client.is_empty() {
            return Err(Error::Parse {
                line: line_num,
                message: "Client is required".into(),
            });
        }

        entries.push(TimeEntry {
            start,
            end,
            description: cols[2].to_string(),
            client,
            project: non_empty(cols[4]),
            activity: non_empty(cols[5]),
            notes: non_empty(cols[6]),
        });
    }

    entries.sort_by_key(|e| e.start);
    Ok(entries)
}

fn parse_datetime(s: &str, line: usize, field: &str) -> Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, DATETIME_FMT).map_err(|_| Error::Parse {
        line,
        message: format!("invalid {field} datetime: \"{s}\""),
    })
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Split a GFM table row by '|', trimming each cell.
/// Handles leading/trailing pipes: "| a | b | c |" -> ["a", "b", "c"]
fn split_row(row: &str) -> Vec<&str> {
    let trimmed = row.trim();
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    inner.split('|').map(|s| s.trim()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    #[test]
    fn empty_content_returns_empty_vec() {
        assert!(parse_month_file("").unwrap().is_empty());
    }

    #[test]
    fn heading_only_no_table() {
        let result = parse_month_file("# 2026-03\n");
        assert!(result.is_err());
    }

    #[test]
    fn single_entry() {
        let content = "\
# 2026-03

| Start | End | Description | Client | Project | Activity | Notes |
|-------|-----|-------------|--------|---------|----------|-------|
| 2026-03-15 09:00 | 2026-03-15 10:30 | Sprint planning | acme | webapp | meeting | |
";
        let entries = parse_month_file(content).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].start, dt(2026, 3, 15, 9, 0));
        assert_eq!(entries[0].end, dt(2026, 3, 15, 10, 30));
        assert_eq!(entries[0].description, "Sprint planning");
        assert_eq!(entries[0].client, "acme");
        assert_eq!(entries[0].project.as_deref(), Some("webapp"));
        assert_eq!(entries[0].activity.as_deref(), Some("meeting"));
        assert!(entries[0].notes.is_none());
    }

    #[test]
    fn midnight_spanning_entry() {
        let content = "\
# 2026-03

| Start | End | Description | Client | Project | Activity | Notes |
|-------|-----|-------------|--------|---------|----------|-------|
| 2026-03-15 23:00 | 2026-03-16 02:00 | Hotfix | acme | webapp | fix | Incident #42 |
";
        let entries = parse_month_file(content).unwrap();
        assert_eq!(entries[0].start, dt(2026, 3, 15, 23, 0));
        assert_eq!(entries[0].end, dt(2026, 3, 16, 2, 0));
        assert_eq!(entries[0].notes.as_deref(), Some("Incident #42"));
    }

    #[test]
    fn all_optional_fields_empty() {
        let content = "\
# 2026-03

| Start | End | Description | Client | Project | Activity | Notes |
|-------|-----|-------------|--------|---------|----------|-------|
| 2026-03-15 09:00 | 2026-03-15 10:00 | Work | acme |  |  |  |
";
        let entries = parse_month_file(content).unwrap();
        assert!(entries[0].project.is_none());
        assert!(entries[0].activity.is_none());
        assert!(entries[0].notes.is_none());
    }

    #[test]
    fn entries_sorted_by_start() {
        let content = "\
# 2026-03

| Start | End | Description | Client | Project | Activity | Notes |
|-------|-----|-------------|--------|---------|----------|-------|
| 2026-03-15 14:00 | 2026-03-15 15:00 | Afternoon | acme |  |  |  |
| 2026-03-15 09:00 | 2026-03-15 10:00 | Morning | acme |  |  |  |
";
        let entries = parse_month_file(content).unwrap();
        assert_eq!(entries[0].description, "Morning");
        assert_eq!(entries[1].description, "Afternoon");
    }

    #[test]
    fn missing_client_errors() {
        let content = "\
# 2026-03

| Start | End | Description | Client | Project | Activity | Notes |
|-------|-----|-------------|--------|---------|----------|-------|
| 2026-03-15 09:00 | 2026-03-15 10:00 | Work |  |  |  |  |
";
        let result = parse_month_file(content);
        assert!(matches!(result, Err(Error::Parse { .. })));
    }

    #[test]
    fn invalid_datetime_errors() {
        let content = "\
# 2026-03

| Start | End | Description | Client | Project | Activity | Notes |
|-------|-----|-------------|--------|---------|----------|-------|
| not-a-date | 2026-03-15 10:00 | Work | acme |  |  |  |
";
        let result = parse_month_file(content);
        assert!(matches!(result, Err(Error::Parse { .. })));
    }

    #[test]
    fn wrong_column_count_errors() {
        let content = "\
# 2026-03

| Start | End | Description | Client |
|-------|-----|-------------|--------|
| 2026-03-15 09:00 | 2026-03-15 10:00 | Work | acme |
";
        let result = parse_month_file(content);
        assert!(matches!(result, Err(Error::Parse { .. })));
    }

    #[test]
    fn br_tags_preserved_in_description() {
        let content = "\
# 2026-03

| Start | End | Description | Client | Project | Activity | Notes |
|-------|-----|-------------|--------|---------|----------|-------|
| 2026-03-15 09:00 | 2026-03-15 10:00 | Line one<br>Line two | acme |  |  |  |
";
        let entries = parse_month_file(content).unwrap();
        assert_eq!(entries[0].description, "Line one<br>Line two");
    }
}
