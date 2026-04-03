use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{Datelike, Days, Local, NaiveDate, NaiveDateTime, Timelike};

use wdttg_core::config::{self, AppConfig};
use wdttg_core::model::TimeEntry;
use wdttg_core::storage::cache::MonthCache;
use wdttg_core::storage::file_manager::FileManager;

/// Slots per day (24 hours * 4 slots per hour at 15-min granularity)
const SLOTS_PER_DAY: i32 = 24 * 4;

pub struct TimelineState {
    pub cursor_date: NaiveDate,
    pub cursor_hour: u32,
    pub cursor_minute: u32,
    pub selected_entry: Option<usize>,
    pub day_entries: HashMap<NaiveDate, Vec<TimeEntry>>,
    /// Scroll position in 15-min slots, relative to center_date midnight.
    pub scroll_offset: i32,
    /// The date at the center of our loaded data window.
    pub center_date: NaiveDate,
    /// Last known viewport height in slots (set by render).
    pub viewport_slots: i32,
    /// When set, the user is marking a time range. First press stores start,
    /// second press uses cursor as end and opens the entry form.
    pub mark_start: Option<NaiveDateTime>,
    file_manager: FileManager,
    cache: MonthCache,
}

impl TimelineState {
    pub fn new(config: &AppConfig) -> Self {
        let config_root = config::config_dir().unwrap_or_else(|_| ".wdttg".into());
        let data_dir = config::data_dir(config, &config_root);
        let today = Local::now().date_naive();
        let now_hour = Local::now().time().hour();

        // Start scroll so current time is visible
        let initial_scroll = (now_hour as i32 - 2).max(0) * 4;

        Self {
            cursor_date: today,
            cursor_hour: now_hour,
            cursor_minute: 0,
            selected_entry: None,
            day_entries: HashMap::new(),
            scroll_offset: initial_scroll,
            mark_start: None,
            center_date: today,
            viewport_slots: 40, // default, updated by render
            file_manager: FileManager::new(data_dir),
            cache: MonthCache::default(),
        }
    }

    pub fn snap_minutes(&self, config: &AppConfig) -> u32 {
        config.preferences.snap_minutes.max(1)
    }

    /// Load entries for a date (cached).
    pub fn entries_for_date(&mut self, date: NaiveDate) -> &[TimeEntry] {
        if !self.day_entries.contains_key(&date) {
            let month_key = format!("{:04}-{:02}", date.year(), date.month());
            let month_entries =
                wdttg_core::storage::load_month(&month_key, &self.file_manager, &mut self.cache)
                    .unwrap_or_default();
            let day_entries: Vec<TimeEntry> = month_entries
                .into_iter()
                .filter(|e| e.start.date() == date)
                .collect();
            self.day_entries.insert(date, day_entries);
        }
        &self.day_entries[&date]
    }

    /// Preload entries for days around the scroll position.
    pub fn preload_visible(&mut self, visible_slots: i32) {
        let start_slot = self.scroll_offset;
        let end_slot = self.scroll_offset + visible_slots;

        // Convert slots to date range (with buffer)
        let start_day_offset = (start_slot.div_euclid(SLOTS_PER_DAY)) - 1;
        let end_day_offset = (end_slot.div_euclid(SLOTS_PER_DAY)) + 1;

        for day_off in start_day_offset..=end_day_offset {
            let date = if day_off >= 0 {
                self.center_date.checked_add_days(Days::new(day_off as u64))
            } else {
                self.center_date
                    .checked_sub_days(Days::new((-day_off) as u64))
            };
            if let Some(d) = date {
                self.entries_for_date(d);
            }
        }
    }

    /// Convert a global slot offset to (date, hour, minute).
    pub fn slot_to_datetime(&self, slot: i32) -> (NaiveDate, u32, u32) {
        let day_offset = slot.div_euclid(SLOTS_PER_DAY);
        let slot_in_day = slot.rem_euclid(SLOTS_PER_DAY) as u32;
        let hour = slot_in_day / 4;
        let minute = (slot_in_day % 4) * 15;

        let date = if day_offset >= 0 {
            self.center_date
                .checked_add_days(Days::new(day_offset as u64))
                .unwrap_or(self.center_date)
        } else {
            self.center_date
                .checked_sub_days(Days::new((-day_offset) as u64))
                .unwrap_or(self.center_date)
        };

        (date, hour, minute)
    }

    /// Convert cursor (date, hour, minute) to a global slot offset.
    pub fn cursor_to_slot(&self) -> i32 {
        let days_from_center = (self.cursor_date - self.center_date).num_days() as i32;
        days_from_center * SLOTS_PER_DAY
            + (self.cursor_hour as i32) * 4
            + (self.cursor_minute as i32 / 15)
    }

    // -- Navigation (infinite) --

    pub fn navigate_up(&mut self, snap: u32) {
        let total_minutes = self.cursor_hour as i64 * 60 + self.cursor_minute as i64 - snap as i64;
        if total_minutes < 0 {
            // Wrap to previous day
            self.cursor_date = self.cursor_date.pred_opt().unwrap_or(self.cursor_date);
            let wrapped = (24 * 60 + total_minutes) as u32;
            self.cursor_hour = wrapped / 60;
            self.cursor_minute = wrapped % 60;
        } else {
            self.cursor_hour = total_minutes as u32 / 60;
            self.cursor_minute = total_minutes as u32 % 60;
        }
        self.ensure_cursor_visible();
        self.update_selection();
    }

    pub fn navigate_down(&mut self, snap: u32) {
        let total_minutes = self.cursor_hour * 60 + self.cursor_minute + snap;
        if total_minutes >= 24 * 60 {
            // Wrap to next day
            self.cursor_date = self.cursor_date.succ_opt().unwrap_or(self.cursor_date);
            let wrapped = total_minutes - 24 * 60;
            self.cursor_hour = wrapped / 60;
            self.cursor_minute = wrapped % 60;
        } else {
            self.cursor_hour = total_minutes / 60;
            self.cursor_minute = total_minutes % 60;
        }
        self.ensure_cursor_visible();
        self.update_selection();
    }

    pub fn navigate_left(&mut self) {
        self.cursor_date = self.cursor_date.pred_opt().unwrap_or(self.cursor_date);
        self.ensure_cursor_visible();
        self.update_selection();
    }

    pub fn navigate_right(&mut self) {
        self.cursor_date = self.cursor_date.succ_opt().unwrap_or(self.cursor_date);
        self.ensure_cursor_visible();
        self.update_selection();
    }

    pub fn page_up(&mut self) {
        // Jump cursor by viewport height (slots * 15 minutes)
        let jump_minutes = (self.viewport_slots * 15) as u32;
        self.navigate_up(jump_minutes);
    }

    pub fn page_down(&mut self) {
        let jump_minutes = (self.viewport_slots * 15) as u32;
        self.navigate_down(jump_minutes);
    }

    pub fn mouse_scroll_up(&mut self, lines: u32) {
        self.scroll_offset -= lines as i32;
        self.recenter_if_needed();
    }

    pub fn mouse_scroll_down(&mut self, lines: u32) {
        self.scroll_offset += lines as i32;
        self.recenter_if_needed();
    }

    /// Re-center when scroll has drifted far from center_date.
    /// This keeps the slot math from overflowing and ensures data loads.
    fn recenter_if_needed(&mut self) {
        let drift_days = self.scroll_offset / SLOTS_PER_DAY;
        if drift_days.abs() > 3 {
            // Move center_date to absorb the drift
            if drift_days > 0 {
                if let Some(new_center) = self
                    .center_date
                    .checked_add_days(Days::new(drift_days as u64))
                {
                    self.center_date = new_center;
                    self.scroll_offset -= drift_days * SLOTS_PER_DAY;
                }
            } else {
                if let Some(new_center) = self
                    .center_date
                    .checked_sub_days(Days::new((-drift_days) as u64))
                {
                    self.center_date = new_center;
                    self.scroll_offset -= drift_days * SLOTS_PER_DAY;
                }
            }
        }
    }

    pub fn jump_to_today(&mut self) {
        let today = Local::now().date_naive();
        let now_hour = Local::now().time().hour();
        self.cursor_date = today;
        self.cursor_hour = now_hour;
        self.cursor_minute = 0;
        self.center_date = today;
        self.scroll_offset = (now_hour as i32 - 2).max(0) * 4;
        self.update_selection();
    }

    pub fn scroll_week_left(&mut self) {
        self.cursor_date = self
            .cursor_date
            .checked_sub_days(Days::new(7))
            .unwrap_or(self.cursor_date);
        self.ensure_cursor_visible();
        self.update_selection();
    }

    pub fn scroll_week_right(&mut self) {
        self.cursor_date = self
            .cursor_date
            .checked_add_days(Days::new(7))
            .unwrap_or(self.cursor_date);
        self.ensure_cursor_visible();
        self.update_selection();
    }

    /// Ensure cursor is within the visible scroll window.
    /// Only call this when the cursor moves, NOT on every render frame.
    fn ensure_cursor_visible(&mut self) {
        let cursor_slot = self.cursor_to_slot();
        let vp = self.viewport_slots;

        // If cursor is above viewport, scroll up to show it
        if cursor_slot < self.scroll_offset + 2 {
            self.scroll_offset = cursor_slot - 2;
        }
        // If cursor is below viewport, scroll down to show it
        if vp > 0 && cursor_slot >= self.scroll_offset + vp - 2 {
            self.scroll_offset = cursor_slot - vp + 4;
        }

        self.recenter_if_needed();
    }

    fn update_selection(&mut self) {
        let cursor_dt = self.cursor_datetime();
        let entries = self.day_entries.get(&self.cursor_date);
        self.selected_entry = entries.and_then(|entries| {
            entries
                .iter()
                .position(|e| cursor_dt >= e.start && cursor_dt < e.end)
        });
    }

    pub fn cursor_datetime(&self) -> NaiveDateTime {
        self.cursor_date
            .and_hms_opt(self.cursor_hour, self.cursor_minute, 0)
            .unwrap()
    }

    pub fn selected_time_entry(&self) -> Option<&TimeEntry> {
        let entries = self.day_entries.get(&self.cursor_date)?;
        let idx = self.selected_entry?;
        entries.get(idx)
    }

    /// Returns the marked range as (start_slot, end_slot) in global slot space,
    /// ordered so start <= end. Returns None if no mark is active.
    pub fn mark_range_slots(&self) -> Option<(i32, i32)> {
        let mark_dt = self.mark_start?;
        let mark_date = mark_dt.date();
        let mark_days = (mark_date - self.center_date).num_days() as i32;
        let mark_slot =
            mark_days * SLOTS_PER_DAY + mark_dt.hour() as i32 * 4 + mark_dt.minute() as i32 / 15;
        let cursor_slot = self.cursor_to_slot();
        Some((mark_slot.min(cursor_slot), mark_slot.max(cursor_slot)))
    }

    pub fn data_dir(&self) -> PathBuf {
        self.file_manager.data_dir().to_path_buf()
    }

    pub fn invalidate_month(&mut self, date: NaiveDate) {
        let month_key = format!("{:04}-{:02}", date.year(), date.month());
        self.cache.invalidate(&month_key);
        self.day_entries
            .retain(|d, _| !(d.year() == date.year() && d.month() == date.month()));
    }

    pub fn do_create(
        &mut self,
        new: wdttg_core::model::NewEntry,
        config: &wdttg_core::config::AppConfig,
    ) -> wdttg_core::Result<wdttg_core::model::TimeEntry> {
        wdttg_core::storage::create_entry(new, config, &self.file_manager, &mut self.cache)
    }

    pub fn do_update(
        &mut self,
        orig_start: NaiveDateTime,
        orig_end: NaiveDateTime,
        new: wdttg_core::model::NewEntry,
        config: &wdttg_core::config::AppConfig,
    ) -> wdttg_core::Result<wdttg_core::model::TimeEntry> {
        wdttg_core::storage::update_entry(
            orig_start,
            orig_end,
            new,
            config,
            &self.file_manager,
            &mut self.cache,
        )
    }

    pub fn do_delete(
        &mut self,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> wdttg_core::Result<()> {
        wdttg_core::storage::delete_entry(start, end, &self.file_manager, &mut self.cache)
    }
}
