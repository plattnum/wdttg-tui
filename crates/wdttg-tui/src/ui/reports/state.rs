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
    pub scroll_offset: usize,
    pub selected_row: usize,
    pub needs_refresh: bool,
    file_manager: FileManager,
    cache: MonthCache,
}

impl ReportsState {
    pub fn new(config: &AppConfig) -> Self {
        let config_root = config::config_dir().unwrap_or_else(|_| ".wdttg".into());
        let data_dir = config::data_dir(config, &config_root);
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
        self.expanded_clients = vec![true; self.reports.len()];
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
        // Find which client the selected_row maps to and toggle it
        let mut row = 0;
        for (i, report) in self.reports.iter().enumerate() {
            if row == self.selected_row {
                if i < self.expanded_clients.len() {
                    self.expanded_clients[i] = !self.expanded_clients[i];
                }
                return;
            }
            row += 1;
            if i < self.expanded_clients.len() && self.expanded_clients[i] {
                for proj in &report.project_breakdown {
                    row += 1;
                    row += proj.activity_breakdown.len();
                }
            }
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

    fn total_visible_rows(&self) -> usize {
        let mut rows = 0;
        for (i, report) in self.reports.iter().enumerate() {
            rows += 1; // client row
            if i < self.expanded_clients.len() && self.expanded_clients[i] {
                for proj in &report.project_breakdown {
                    rows += 1; // project row
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
