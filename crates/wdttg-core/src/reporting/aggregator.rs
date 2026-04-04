use std::collections::HashMap;

use crate::config::AppConfig;
use crate::model::{ActivityReport, ClientReport, DateRange, ProjectReport, TimeEntry};

/// Generate a hierarchical report from entries within a date range.
/// Groups by Client -> Project -> Activity with totals, billable amounts, and percentages.
pub fn generate_report(
    range: &DateRange,
    entries: &[TimeEntry],
    config: &AppConfig,
) -> Vec<ClientReport> {
    // Filter entries to those whose start date falls within the range
    let filtered: Vec<&TimeEntry> = entries
        .iter()
        .filter(|e| {
            let d = e.start.date();
            d >= range.start && d <= range.end
        })
        .collect();

    // Group: client_id -> project_id -> activity_id -> total_minutes
    let mut client_map: HashMap<String, HashMap<String, HashMap<String, i64>>> = HashMap::new();

    for entry in &filtered {
        let client_id = &entry.client;
        let project_id = entry
            .project
            .as_deref()
            .unwrap_or("(no project)")
            .to_string();
        let activity_id = entry
            .activity
            .as_deref()
            .unwrap_or("(no activity)")
            .to_string();

        let minutes = entry.duration_minutes();

        *client_map
            .entry(client_id.clone())
            .or_default()
            .entry(project_id)
            .or_default()
            .entry(activity_id)
            .or_default() += minutes;
    }

    // Calculate grand total for percentages
    let grand_total: i64 = filtered.iter().map(|e| e.duration_minutes()).sum();

    // Build reports
    let mut reports: Vec<ClientReport> = client_map
        .into_iter()
        .map(|(client_id, projects)| {
            let client_config = config.clients.iter().find(|c| c.id == client_id);
            let client_name = client_config
                .map(|c| c.name.clone())
                .unwrap_or_else(|| client_id.clone());
            let client_color = client_config
                .map(|c| c.color.clone())
                .unwrap_or_else(|| "#888888".into());
            let client_rate = client_config.map(|c| c.rate).unwrap_or(0.0);
            let currency = client_config
                .map(|c| c.currency.clone())
                .unwrap_or_else(|| "USD".into());

            let client_total: i64 = projects.values().flat_map(|a| a.values()).sum();

            let mut project_reports: Vec<ProjectReport> = projects
                .into_iter()
                .map(|(proj_id, activities)| {
                    let proj_config =
                        client_config.and_then(|c| c.projects.iter().find(|p| p.id == proj_id));
                    let proj_name = proj_config
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| proj_id.clone());
                    let proj_color = proj_config
                        .map(|p| p.color.clone())
                        .unwrap_or_else(|| client_color.clone());
                    let rate = proj_config
                        .and_then(|p| p.rate_override)
                        .unwrap_or(client_rate);

                    let proj_total: i64 = activities.values().sum();
                    let billable = (proj_total as f64 / 60.0) * rate;
                    let percentage = if client_total > 0 {
                        (proj_total as f64 / client_total as f64) * 100.0
                    } else {
                        0.0
                    };

                    let mut activity_reports: Vec<ActivityReport> = activities
                        .into_iter()
                        .map(|(act_id, minutes)| {
                            let act_config = client_config
                                .and_then(|c| c.activities.iter().find(|a| a.id == act_id));
                            let act_name = act_config
                                .map(|a| a.name.clone())
                                .unwrap_or_else(|| act_id.clone());
                            let act_color = act_config
                                .map(|a| a.color.clone())
                                .unwrap_or_else(|| proj_color.clone());
                            let act_pct = if proj_total > 0 {
                                (minutes as f64 / proj_total as f64) * 100.0
                            } else {
                                0.0
                            };

                            ActivityReport {
                                activity_id: act_id,
                                name: act_name,
                                color: act_color,
                                total_minutes: minutes,
                                percentage: act_pct,
                            }
                        })
                        .collect();

                    activity_reports.sort_by(|a, b| b.total_minutes.cmp(&a.total_minutes));

                    ProjectReport {
                        project_id: proj_id,
                        name: proj_name,
                        color: proj_color,
                        total_minutes: proj_total,
                        billable_amount: billable,
                        percentage,
                        activity_breakdown: activity_reports,
                    }
                })
                .collect();

            project_reports.sort_by(|a, b| b.total_minutes.cmp(&a.total_minutes));

            let client_billable: f64 = project_reports.iter().map(|p| p.billable_amount).sum();
            let client_pct = if grand_total > 0 {
                (client_total as f64 / grand_total as f64) * 100.0
            } else {
                0.0
            };

            ClientReport {
                client_id,
                name: client_name,
                color: client_color,
                rate: client_rate,
                currency,
                total_minutes: client_total,
                billable_amount: client_billable,
                percentage: client_pct,
                project_breakdown: project_reports,
            }
        })
        .collect();

    reports.sort_by(|a, b| b.total_minutes.cmp(&a.total_minutes));
    reports
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
            preferences: Preferences::default(),
            bill_from: Default::default(),
            clients: vec![
                Client {
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
                    projects: vec![
                        Project {
                            id: "webapp".into(),
                            name: "Web App".into(),
                            color: "#4ECDC4".into(),
                            rate_override: Some(175.0),
                            archived: false,
                        },
                        Project {
                            id: "mobile".into(),
                            name: "Mobile".into(),
                            color: "#45B7D1".into(),
                            rate_override: None,
                            archived: false,
                        },
                    ],
                    activities: vec![
                        Activity {
                            id: "dev".into(),
                            name: "Development".into(),
                            color: "#2ECC71".into(),
                            archived: false,
                        },
                        Activity {
                            id: "meeting".into(),
                            name: "Meeting".into(),
                            color: "#E74C3C".into(),
                            archived: false,
                        },
                    ],
                },
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
                    projects: vec![],
                    activities: vec![],
                },
            ],
        }
    }

    fn test_entries() -> Vec<TimeEntry> {
        vec![
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
                description: "Auth flow".into(),
                client: "acme".into(),
                project: Some("webapp".into()),
                activity: Some("dev".into()),
                notes: None,
            },
            TimeEntry {
                start: dt(2026, 3, 15, 13, 0),
                end: dt(2026, 3, 15, 14, 0),
                description: "Mobile bug".into(),
                client: "acme".into(),
                project: Some("mobile".into()),
                activity: Some("dev".into()),
                notes: None,
            },
            TimeEntry {
                start: dt(2026, 3, 15, 14, 0),
                end: dt(2026, 3, 15, 15, 0),
                description: "Personal stuff".into(),
                client: "personal".into(),
                project: None,
                activity: None,
                notes: None,
            },
        ]
    }

    #[test]
    fn basic_aggregation() {
        let config = test_config();
        let entries = test_entries();
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
        );

        let reports = generate_report(&range, &entries, &config);

        // Two clients: acme and personal
        assert_eq!(reports.len(), 2);
        // Acme has more time, should be first
        assert_eq!(reports[0].client_id, "acme");
        // sprint planning=90 + auth flow=90 + mobile bug=60 = 240
        assert_eq!(reports[0].total_minutes, 240);
        assert_eq!(reports[1].client_id, "personal");
        assert_eq!(reports[1].total_minutes, 60);
    }

    #[test]
    fn project_breakdown() {
        let config = test_config();
        let entries = test_entries();
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
        );

        let reports = generate_report(&range, &entries, &config);
        let acme = &reports[0];

        // webapp: 180min, mobile: 60min
        assert_eq!(acme.project_breakdown.len(), 2);
        assert_eq!(acme.project_breakdown[0].project_id, "webapp");
        assert_eq!(acme.project_breakdown[0].total_minutes, 180);
        assert_eq!(acme.project_breakdown[1].project_id, "mobile");
        assert_eq!(acme.project_breakdown[1].total_minutes, 60);
    }

    #[test]
    fn rate_override_used() {
        let config = test_config();
        let entries = test_entries();
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
        );

        let reports = generate_report(&range, &entries, &config);
        let acme = &reports[0];

        // webapp has rate_override=175, 180min = 3h * 175 = 525
        let webapp = &acme.project_breakdown[0];
        assert_eq!(webapp.billable_amount, 525.0);

        // mobile uses client rate=150, 60min = 1h * 150 = 150
        let mobile = &acme.project_breakdown[1];
        assert_eq!(mobile.billable_amount, 150.0);
    }

    #[test]
    fn percentages_correct() {
        let config = test_config();
        let entries = test_entries();
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
        );

        let reports = generate_report(&range, &entries, &config);
        // Acme: 240/300 = 80%
        assert!((reports[0].percentage - 80.0).abs() < 0.1);
        // Personal: 60/300 = 20%
        assert!((reports[1].percentage - 20.0).abs() < 0.1);
    }

    #[test]
    fn no_project_no_activity() {
        let config = test_config();
        let entries = test_entries();
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
        );

        let reports = generate_report(&range, &entries, &config);
        let personal = &reports[1];
        assert_eq!(personal.project_breakdown[0].project_id, "(no project)");
        assert_eq!(
            personal.project_breakdown[0].activity_breakdown[0].activity_id,
            "(no activity)"
        );
    }

    #[test]
    fn entries_outside_range_excluded() {
        let config = test_config();
        let entries = test_entries();
        // Range only covers March 16 -- all entries are on the 15th
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 16).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 16).unwrap(),
        );

        let reports = generate_report(&range, &entries, &config);
        assert!(reports.is_empty());
    }

    #[test]
    fn empty_entries() {
        let config = test_config();
        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
        );

        let reports = generate_report(&range, &[], &config);
        assert!(reports.is_empty());
    }
}
