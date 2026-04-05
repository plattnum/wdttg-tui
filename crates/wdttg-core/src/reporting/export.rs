use crate::model::{ClientReport, TimeEntry};
use crate::time_utils::format_duration;

/// Export time entries as CSV. One row per entry.
/// Columns: Start, End, Duration, Client, Project, Activity, Description, Notes
pub fn entries_to_csv(entries: &[TimeEntry]) -> String {
    let mut out = String::from("Start,End,Duration,Client,Project,Activity,Description,Notes\n");
    for e in entries {
        let start = e.start.format("%Y-%m-%d %H:%M");
        let end = e.end.format("%Y-%m-%d %H:%M");
        let duration = format_duration(e.duration_minutes());
        let project = e.project.as_deref().unwrap_or("");
        let activity = e.activity.as_deref().unwrap_or("");
        let notes = e.notes.as_deref().unwrap_or("");

        out.push_str(&format!(
            "{start},{end},{duration},{},{},{},{},{}\n",
            csv_escape(&e.client),
            csv_escape(project),
            csv_escape(activity),
            csv_escape(&e.description),
            csv_escape(notes),
        ));
    }
    out
}

/// Export aggregated report as JSON.
pub fn report_to_json(reports: &[ClientReport], total_minutes: i64) -> String {
    let total_billable: f64 = reports.iter().map(|r| r.billable_amount).sum();

    let clients: Vec<serde_json::Value> = reports
        .iter()
        .map(|cr| {
            let projects: Vec<serde_json::Value> = cr
                .project_breakdown
                .iter()
                .map(|pr| {
                    let activities: Vec<serde_json::Value> = pr
                        .activity_breakdown
                        .iter()
                        .map(|ar| {
                            serde_json::json!({
                                "activity_id": ar.activity_id,
                                "name": ar.name,
                                "total_minutes": ar.total_minutes,
                                "total_formatted": format_duration(ar.total_minutes),
                                "percentage": (ar.percentage * 10.0).round() / 10.0,
                            })
                        })
                        .collect();
                    serde_json::json!({
                        "project_id": pr.project_id,
                        "name": pr.name,
                        "total_minutes": pr.total_minutes,
                        "total_formatted": format_duration(pr.total_minutes),
                        "billable_amount": pr.billable_amount,
                        "percentage": (pr.percentage * 10.0).round() / 10.0,
                        "activities": activities,
                    })
                })
                .collect();
            serde_json::json!({
                "client_id": cr.client_id,
                "name": cr.name,
                "rate": cr.rate,
                "currency": cr.currency,
                "total_minutes": cr.total_minutes,
                "total_formatted": format_duration(cr.total_minutes),
                "billable_amount": cr.billable_amount,
                "percentage": (cr.percentage * 10.0).round() / 10.0,
                "projects": projects,
            })
        })
        .collect();

    serde_json::json!({
        "report": clients,
        "total_minutes": total_minutes,
        "total_formatted": format_duration(total_minutes),
        "total_billable": total_billable,
    })
    .to_string()
}

/// Escape a value for CSV (RFC 4180). Converts `<br>` to real newlines first,
/// then quotes the field if it contains commas, quotes, newlines, or carriage returns.
fn csv_escape(s: &str) -> String {
    let s = s.replace("<br>", "\n");
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    #[test]
    fn csv_basic() {
        let entries = vec![TimeEntry {
            start: dt(2026, 4, 5, 9, 0),
            end: dt(2026, 4, 5, 10, 30),
            description: "Sprint planning".into(),
            client: "acme".into(),
            project: Some("webapp".into()),
            activity: Some("meeting".into()),
            notes: None,
        }];
        let csv = entries_to_csv(&entries);
        assert!(csv.starts_with("Start,End,"));
        assert!(csv.contains(
            "2026-04-05 09:00,2026-04-05 10:30,1h 30m,acme,webapp,meeting,Sprint planning,"
        ));
    }

    #[test]
    fn csv_escapes_commas() {
        let entries = vec![TimeEntry {
            start: dt(2026, 4, 5, 9, 0),
            end: dt(2026, 4, 5, 10, 0),
            description: "Meeting with Bob, Alice".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        }];
        let csv = entries_to_csv(&entries);
        assert!(csv.contains("\"Meeting with Bob, Alice\""));
    }

    #[test]
    fn csv_empty_optional_fields() {
        let entries = vec![TimeEntry {
            start: dt(2026, 4, 5, 9, 0),
            end: dt(2026, 4, 5, 10, 0),
            description: "Work".into(),
            client: "acme".into(),
            project: None,
            activity: None,
            notes: None,
        }];
        let csv = entries_to_csv(&entries);
        assert!(csv.contains("acme,,,Work,\n"));
    }

    #[test]
    fn json_basic() {
        let reports = vec![ClientReport {
            client_id: "acme".into(),
            name: "Acme Corp".into(),
            color: "#FF0000".into(),
            rate: 150.0,
            currency: "USD".into(),
            total_minutes: 90,
            billable_amount: 225.0,
            percentage: 100.0,
            project_breakdown: vec![],
        }];
        let json = report_to_json(&reports, 90);
        assert!(json.contains("\"total_minutes\":90"));
        assert!(json.contains("\"total_billable\":225.0"));
        assert!(json.contains("Acme Corp"));
    }
}
