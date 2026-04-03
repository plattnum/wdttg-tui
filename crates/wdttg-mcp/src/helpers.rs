use chrono::{Local, NaiveDate, NaiveDateTime};
use serde_json::{Value, json};

use wdttg_core::model::{DateRange, TimeEntry, TimeRangePreset};
use wdttg_core::time_utils::format_duration;
use wdttg_core::validation::{OverlapInfo, OverlapType};

/// Parse "YYYY-MM-DD HH:mm" into NaiveDateTime.
pub fn parse_datetime(s: &str) -> Result<NaiveDateTime, String> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M")
        .map_err(|_| format!("invalid datetime format: '{s}', expected YYYY-MM-DD HH:mm"))
}

/// Parse "YYYY-MM-DD" into NaiveDate.
pub fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| format!("invalid date format: '{s}', expected YYYY-MM-DD"))
}

/// Resolve a date range from params: either preset overrides start/end, or both are required.
pub fn resolve_date_range(
    start_date: &Option<String>,
    end_date: &Option<String>,
    preset: &Option<String>,
    week_start: &str,
) -> Result<DateRange, String> {
    let today = Local::now().date_naive();

    if let Some(preset_str) = preset {
        let p = parse_preset(preset_str)?;
        Ok(DateRange::from_preset(p, today, week_start))
    } else {
        let start = start_date
            .as_ref()
            .ok_or("start_date is required when preset is not provided")?;
        let end = end_date
            .as_ref()
            .ok_or("end_date is required when preset is not provided")?;
        Ok(DateRange::new(parse_date(start)?, parse_date(end)?))
    }
}

fn parse_preset(s: &str) -> Result<TimeRangePreset, String> {
    match s {
        "today" => Ok(TimeRangePreset::Today),
        "yesterday" => Ok(TimeRangePreset::Yesterday),
        "this_week" => Ok(TimeRangePreset::ThisWeek),
        "last_week" => Ok(TimeRangePreset::LastWeek),
        "this_month" => Ok(TimeRangePreset::ThisMonth),
        "last_month" => Ok(TimeRangePreset::LastMonth),
        _ => Err(format!(
            "unknown preset: '{s}'. Valid: today, yesterday, this_week, last_week, this_month, last_month"
        )),
    }
}

/// Serialize a TimeEntry to JSON with computed fields.
pub fn entry_to_json(entry: &TimeEntry) -> Value {
    let minutes = entry.duration_minutes();
    json!({
        "entry_id": entry.entry_id(),
        "start": entry.start.format("%Y-%m-%d %H:%M").to_string(),
        "end": entry.end.format("%Y-%m-%d %H:%M").to_string(),
        "description": entry.description,
        "client": entry.client,
        "project": entry.project,
        "activity": entry.activity,
        "notes": entry.notes,
        "duration_minutes": minutes,
        "duration_formatted": format_duration(minutes),
    })
}

/// Format a wdttg-core error as a structured JSON error string.
pub fn error_json(error: &wdttg_core::Error) -> String {
    let (error_type, message) = match error {
        wdttg_core::Error::Validation(msg) => ("validation", msg.clone()),
        wdttg_core::Error::Overlap(msg) => ("overlap", msg.clone()),
        wdttg_core::Error::NotFound => (
            "not_found",
            "No entry matches the given ID or timestamps".into(),
        ),
        wdttg_core::Error::Parse { line, message } => ("parse", format!("line {line}: {message}")),
        wdttg_core::Error::Config(msg) => ("config", msg.clone()),
        wdttg_core::Error::Io(e) => ("io", e.to_string()),
    };
    json!({ "error": error_type, "message": message }).to_string()
}

/// Format a validation error string as JSON.
pub fn validation_error(msg: &str) -> String {
    json!({ "error": "validation", "message": msg }).to_string()
}

/// Serialize overlap info to JSON.
pub fn overlap_to_json(info: &OverlapInfo) -> Value {
    let type_str = match info.overlap_type {
        OverlapType::StartOverlap => "StartOverlap",
        OverlapType::EndOverlap => "EndOverlap",
        OverlapType::Encompassed => "Encompassed",
    };
    json!({
        "type": type_str,
        "conflicting_entry": entry_to_json(&info.conflicting_entry),
    })
}
