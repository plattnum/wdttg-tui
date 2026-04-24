use chrono::Local;

use wdttg_core::config::{self, AppConfig};
use wdttg_core::model::{ClientReport, DateRange, TimeEntry, TimeRangePreset};
use wdttg_core::reporting::generate_report;
use wdttg_core::storage::cache::MonthCache;
use wdttg_core::storage::file_manager::FileManager;

pub struct ReportsState {
    pub preset: TimeRangePreset,
    pub range: DateRange,
    pub reports: Vec<ClientReport>,
    pub expanded_clients: Vec<bool>,
    pub expanded_projects: Vec<Vec<bool>>,
    pub scroll_offset: usize,
    pub selected_row: usize,
    pub needs_refresh: bool,
    file_manager: FileManager,
    cache: MonthCache,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibleRow {
    Client(usize),
    Project(usize, usize),
    Activity(usize, usize, usize),
}

impl ReportsState {
    pub fn new(config: &AppConfig) -> Self {
        let data_dir =
            config::data_dir(config).unwrap_or_else(|_| ".local/share/wdttg/data".into());
        let today = Local::now().date_naive();
        let range = DateRange::from_preset(
            TimeRangePreset::ThisMonth,
            today,
            &config.preferences.week_start,
        );

        Self {
            preset: TimeRangePreset::ThisMonth,
            range,
            reports: vec![],
            expanded_clients: vec![],
            expanded_projects: vec![],
            scroll_offset: 0,
            selected_row: 0,
            needs_refresh: true,
            file_manager: FileManager::new(data_dir),
            cache: MonthCache::default(),
        }
    }

    pub fn refresh(&mut self, config: &AppConfig) {
        // Load all entries for the range
        let months = self.range.months_spanned();
        let mut all_entries: Vec<TimeEntry> = Vec::new();

        for month_key in &months {
            let entries =
                wdttg_core::storage::load_month(month_key, &self.file_manager, &mut self.cache)
                    .unwrap_or_default();
            all_entries.extend(entries);
        }

        self.reports = generate_report(&self.range, &all_entries, config);
        self.reset_expansion_state();
        self.needs_refresh = false;
    }

    pub fn invalidate_month(&mut self, month_key: &str) {
        self.cache.invalidate(month_key);
    }

    pub fn set_preset(&mut self, preset: TimeRangePreset, config: &AppConfig) {
        let today = Local::now().date_naive();
        self.preset = preset;
        self.range = DateRange::from_preset(preset, today, &config.preferences.week_start);
        self.needs_refresh = true;
        self.selected_row = 0;
        self.scroll_offset = 0;
    }

    pub fn cycle_preset_forward(&mut self, config: &AppConfig) {
        let next = match self.preset {
            TimeRangePreset::Today => TimeRangePreset::Yesterday,
            TimeRangePreset::Yesterday => TimeRangePreset::ThisWeek,
            TimeRangePreset::ThisWeek => TimeRangePreset::LastWeek,
            TimeRangePreset::LastWeek => TimeRangePreset::ThisMonth,
            TimeRangePreset::ThisMonth => TimeRangePreset::LastMonth,
            TimeRangePreset::LastMonth => TimeRangePreset::Today,
            TimeRangePreset::Custom => TimeRangePreset::Today,
        };
        self.set_preset(next, config);
    }

    pub fn cycle_preset_backward(&mut self, config: &AppConfig) {
        let prev = match self.preset {
            TimeRangePreset::Today => TimeRangePreset::LastMonth,
            TimeRangePreset::Yesterday => TimeRangePreset::Today,
            TimeRangePreset::ThisWeek => TimeRangePreset::Yesterday,
            TimeRangePreset::LastWeek => TimeRangePreset::ThisWeek,
            TimeRangePreset::ThisMonth => TimeRangePreset::LastWeek,
            TimeRangePreset::LastMonth => TimeRangePreset::ThisMonth,
            TimeRangePreset::Custom => TimeRangePreset::Today,
        };
        self.set_preset(prev, config);
    }

    pub fn toggle_expand(&mut self) {
        match self.visible_row_at(self.selected_row) {
            Some(VisibleRow::Client(client_idx)) => {
                if let Some(expanded) = self.expanded_clients.get_mut(client_idx) {
                    *expanded = !*expanded;
                }
            }
            Some(VisibleRow::Project(client_idx, project_idx)) => {
                if let Some(expanded) = self
                    .expanded_projects
                    .get_mut(client_idx)
                    .and_then(|projects| projects.get_mut(project_idx))
                {
                    *expanded = !*expanded;
                }
            }
            Some(VisibleRow::Activity(_, _, _)) | None => {}
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_row > 0 {
            self.selected_row -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let max = self.total_visible_rows().saturating_sub(1);
        if self.selected_row < max {
            self.selected_row += 1;
        }
    }

    pub(crate) fn is_client_expanded(&self, client_idx: usize) -> bool {
        self.expanded_clients
            .get(client_idx)
            .copied()
            .unwrap_or(false)
    }

    pub(crate) fn is_project_expanded(&self, client_idx: usize, project_idx: usize) -> bool {
        self.expanded_projects
            .get(client_idx)
            .and_then(|projects| projects.get(project_idx))
            .copied()
            .unwrap_or(false)
    }

    fn reset_expansion_state(&mut self) {
        self.expanded_clients = vec![true; self.reports.len()];
        self.expanded_projects = self
            .reports
            .iter()
            .map(|report| vec![true; report.project_breakdown.len()])
            .collect();
    }

    fn visible_row_at(&self, target_row: usize) -> Option<VisibleRow> {
        let mut row = 0;

        for (client_idx, report) in self.reports.iter().enumerate() {
            if row == target_row {
                return Some(VisibleRow::Client(client_idx));
            }
            row += 1;

            if !self.is_client_expanded(client_idx) {
                continue;
            }

            for (project_idx, project) in report.project_breakdown.iter().enumerate() {
                if row == target_row {
                    return Some(VisibleRow::Project(client_idx, project_idx));
                }
                row += 1;

                if !self.is_project_expanded(client_idx, project_idx) {
                    continue;
                }

                for (activity_idx, _) in project.activity_breakdown.iter().enumerate() {
                    if row == target_row {
                        return Some(VisibleRow::Activity(client_idx, project_idx, activity_idx));
                    }
                    row += 1;
                }
            }
        }

        None
    }

    fn total_visible_rows(&self) -> usize {
        let mut rows = 0;
        for (client_idx, report) in self.reports.iter().enumerate() {
            rows += 1; // client row
            if !self.is_client_expanded(client_idx) {
                continue;
            }

            for (project_idx, proj) in report.project_breakdown.iter().enumerate() {
                rows += 1; // project row
                if self.is_project_expanded(client_idx, project_idx) {
                    rows += proj.activity_breakdown.len(); // activity rows
                }
            }
        }
        rows
    }

    pub fn preset_name(&self) -> &str {
        match self.preset {
            TimeRangePreset::Today => "Today",
            TimeRangePreset::Yesterday => "Yesterday",
            TimeRangePreset::ThisWeek => "This Week",
            TimeRangePreset::LastWeek => "Last Week",
            TimeRangePreset::ThisMonth => "This Month",
            TimeRangePreset::LastMonth => "Last Month",
            TimeRangePreset::Custom => "Custom",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use chrono::NaiveDate;
    use wdttg_core::config::{BillFrom, Preferences};
    use wdttg_core::model::{ActivityReport, ProjectReport};

    fn date_range() -> DateRange {
        DateRange {
            start: NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2026, 4, 30).unwrap(),
        }
    }

    fn app_config() -> AppConfig {
        AppConfig {
            preferences: Preferences::default(),
            bill_from: BillFrom::default(),
            clients: vec![],
        }
    }

    fn activity(name: &str) -> ActivityReport {
        ActivityReport {
            activity_id: name.into(),
            name: name.into(),
            color: "#ffffff".into(),
            total_minutes: 60,
            percentage: 50.0,
        }
    }

    fn project(name: &str, activities: Vec<ActivityReport>) -> ProjectReport {
        ProjectReport {
            project_id: name.into(),
            name: name.into(),
            color: "#aaaaaa".into(),
            total_minutes: (activities.len() as i64) * 60,
            billable_amount: activities.len() as f64 * 100.0,
            percentage: 50.0,
            activity_breakdown: activities,
        }
    }

    fn client(name: &str, projects: Vec<ProjectReport>) -> ClientReport {
        ClientReport {
            client_id: name.into(),
            name: name.into(),
            color: "#000000".into(),
            rate: 100.0,
            currency: "USD".into(),
            total_minutes: projects.iter().map(|p| p.total_minutes).sum(),
            billable_amount: projects.iter().map(|p| p.billable_amount).sum(),
            percentage: 100.0,
            project_breakdown: projects,
        }
    }

    fn reports_state(reports: Vec<ClientReport>) -> ReportsState {
        let mut state = ReportsState {
            preset: TimeRangePreset::ThisMonth,
            range: date_range(),
            reports,
            expanded_clients: vec![],
            expanded_projects: vec![],
            scroll_offset: 0,
            selected_row: 0,
            needs_refresh: false,
            file_manager: FileManager::new(PathBuf::from(".")),
            cache: MonthCache::default(),
        };
        state.reset_expansion_state();
        state
    }

    #[test]
    fn visible_row_mapping_includes_project_and_activity_levels() {
        let state = reports_state(vec![client(
            "acme",
            vec![
                project("webapp", vec![activity("build"), activity("review")]),
                project("mobile", vec![activity("qa")]),
            ],
        )]);

        assert_eq!(state.visible_row_at(0), Some(VisibleRow::Client(0)));
        assert_eq!(state.visible_row_at(1), Some(VisibleRow::Project(0, 0)));
        assert_eq!(state.visible_row_at(2), Some(VisibleRow::Activity(0, 0, 0)));
        assert_eq!(state.visible_row_at(3), Some(VisibleRow::Activity(0, 0, 1)));
        assert_eq!(state.visible_row_at(4), Some(VisibleRow::Project(0, 1)));
        assert_eq!(state.visible_row_at(5), Some(VisibleRow::Activity(0, 1, 0)));
        assert_eq!(state.visible_row_at(6), None);
    }

    #[test]
    fn toggling_project_hides_only_its_activities() {
        let mut state = reports_state(vec![client(
            "acme",
            vec![
                project("webapp", vec![activity("build"), activity("review")]),
                project("mobile", vec![activity("qa")]),
            ],
        )]);

        assert_eq!(state.total_visible_rows(), 6);

        state.selected_row = 1;
        state.toggle_expand();

        assert!(!state.is_project_expanded(0, 0));
        assert_eq!(state.total_visible_rows(), 4);
        assert_eq!(state.visible_row_at(2), Some(VisibleRow::Project(0, 1)));
        assert_eq!(state.visible_row_at(3), Some(VisibleRow::Activity(0, 1, 0)));
    }

    #[test]
    fn toggling_client_hides_all_nested_rows() {
        let mut state = reports_state(vec![client(
            "acme",
            vec![project("webapp", vec![activity("build")])],
        )]);

        assert_eq!(state.total_visible_rows(), 3);

        state.selected_row = 0;
        state.toggle_expand();

        assert!(!state.is_client_expanded(0));
        assert_eq!(state.total_visible_rows(), 1);
        assert_eq!(state.visible_row_at(1), None);
    }

    #[test]
    fn toggling_activity_row_does_nothing() {
        let mut state = reports_state(vec![client(
            "acme",
            vec![project("webapp", vec![activity("build")])],
        )]);

        state.selected_row = 2;
        state.toggle_expand();

        assert!(state.is_client_expanded(0));
        assert!(state.is_project_expanded(0, 0));
        assert_eq!(state.total_visible_rows(), 3);
    }

    #[test]
    fn reports_state_new_starts_dirty() {
        let state = ReportsState::new(&app_config());
        assert!(state.needs_refresh);
        assert!(state.expanded_clients.is_empty());
        assert!(state.expanded_projects.is_empty());
    }
}
