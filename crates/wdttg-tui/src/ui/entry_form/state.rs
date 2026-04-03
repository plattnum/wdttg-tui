use chrono::NaiveDateTime;

use wdttg_core::config::AppConfig;
use wdttg_core::model::{NewEntry, TimeEntry};
use wdttg_core::time_utils::parse_duration;
use wdttg_core::validation::find_overlaps;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormMode {
    Create,
    Edit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Start,
    End,
    Client,
    Project,
    Activity,
    Description,
    Notes,
}

impl FormField {
    pub fn next(self) -> Self {
        match self {
            Self::Start => Self::End,
            Self::End => Self::Client,
            Self::Client => Self::Project,
            Self::Project => Self::Activity,
            Self::Activity => Self::Description,
            Self::Description => Self::Notes,
            Self::Notes => Self::Start,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Start => Self::Notes,
            Self::End => Self::Start,
            Self::Client => Self::End,
            Self::Project => Self::Client,
            Self::Activity => Self::Project,
            Self::Description => Self::Activity,
            Self::Notes => Self::Description,
        }
    }
}

pub struct EntryFormState {
    pub mode: FormMode,
    pub original: Option<TimeEntry>,
    pub start_input: String,
    pub end_input: String,
    pub client_idx: usize,
    pub project_idx: usize,  // 0 = None
    pub activity_idx: usize, // 0 = None
    pub description: String,
    pub notes: String,
    pub focused_field: FormField,
    pub overlap_warning: Option<String>,
    pub error_message: Option<String>,
    pub cursor_pos: usize, // cursor within text field
    /// Y positions of fields, populated by render for mouse click targeting.
    pub field_positions: Vec<(FormField, u16)>,
}

impl EntryFormState {
    pub fn new_create(start: NaiveDateTime, end: NaiveDateTime, config: &AppConfig) -> Self {
        let client_idx = config.clients.iter().position(|c| !c.archived).unwrap_or(0);

        Self {
            mode: FormMode::Create,
            original: None,
            start_input: start.format("%Y-%m-%d %H:%M").to_string(),
            end_input: end.format("%Y-%m-%d %H:%M").to_string(),
            client_idx,
            project_idx: 0,
            activity_idx: 0,
            description: String::new(),
            notes: String::new(),
            focused_field: FormField::Client,
            overlap_warning: None,
            error_message: None,
            cursor_pos: 0,
            field_positions: vec![],
        }
    }

    pub fn new_edit(entry: &TimeEntry, config: &AppConfig) -> Self {
        let client_idx = config
            .clients
            .iter()
            .position(|c| c.id == entry.client)
            .unwrap_or(0);

        let project_idx = if let Some(ref pid) = entry.project {
            config
                .clients
                .get(client_idx)
                .and_then(|c| c.projects.iter().position(|p| &p.id == pid))
                .map(|i| i + 1) // +1 because 0 = None
                .unwrap_or(0)
        } else {
            0
        };

        let activity_idx = if let Some(ref aid) = entry.activity {
            config
                .clients
                .get(client_idx)
                .and_then(|c| c.activities.iter().position(|a| &a.id == aid))
                .map(|i| i + 1)
                .unwrap_or(0)
        } else {
            0
        };

        Self {
            mode: FormMode::Edit,
            original: Some(entry.clone()),
            start_input: entry.start.format("%Y-%m-%d %H:%M").to_string(),
            end_input: entry.end.format("%Y-%m-%d %H:%M").to_string(),
            client_idx,
            project_idx,
            activity_idx,
            description: entry.description.clone(),
            notes: entry.notes.clone().unwrap_or_default(),
            focused_field: FormField::Description,
            overlap_warning: None,
            error_message: None,
            cursor_pos: entry.description.len(),
            field_positions: vec![],
        }
    }

    /// Handle mouse click at (x, y) - focus the clicked field.
    pub fn click_at(&mut self, _x: u16, y: u16) {
        for &(field, field_y) in &self.field_positions {
            if y == field_y {
                self.focused_field = field;
                self.update_cursor_for_field();
                return;
            }
        }
    }

    pub fn next_field(&mut self) {
        self.focused_field = self.focused_field.next();
        self.update_cursor_for_field();
    }

    pub fn prev_field(&mut self) {
        self.focused_field = self.focused_field.prev();
        self.update_cursor_for_field();
    }

    fn update_cursor_for_field(&mut self) {
        self.cursor_pos = match self.focused_field {
            FormField::Start => self.start_input.len(),
            FormField::End => self.end_input.len(),
            FormField::Description => self.description.len(),
            FormField::Notes => self.notes.len(),
            _ => 0,
        };
    }

    pub fn cycle_client(&mut self, config: &AppConfig, forward: bool) {
        let active: Vec<usize> = config
            .clients
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.archived)
            .map(|(i, _)| i)
            .collect();
        if active.is_empty() {
            return;
        }
        let current_pos = active
            .iter()
            .position(|&i| i == self.client_idx)
            .unwrap_or(0);
        let new_pos = if forward {
            (current_pos + 1) % active.len()
        } else {
            (current_pos + active.len() - 1) % active.len()
        };
        self.client_idx = active[new_pos];
        // Reset project/activity when client changes
        self.project_idx = 0;
        self.activity_idx = 0;
    }

    pub fn cycle_project(&mut self, config: &AppConfig, forward: bool) {
        let count = config
            .clients
            .get(self.client_idx)
            .map(|c| c.projects.len())
            .unwrap_or(0)
            + 1; // +1 for None option
        if forward {
            self.project_idx = (self.project_idx + 1) % count;
        } else {
            self.project_idx = (self.project_idx + count - 1) % count;
        }
    }

    pub fn cycle_activity(&mut self, config: &AppConfig, forward: bool) {
        let count = config
            .clients
            .get(self.client_idx)
            .map(|c| c.activities.len())
            .unwrap_or(0)
            + 1;
        if forward {
            self.activity_idx = (self.activity_idx + 1) % count;
        } else {
            self.activity_idx = (self.activity_idx + count - 1) % count;
        }
    }

    pub fn type_char(&mut self, ch: char) {
        match self.focused_field {
            FormField::Start => {
                self.start_input.insert(self.cursor_pos, ch);
                self.cursor_pos += 1;
            }
            FormField::End => {
                self.end_input.insert(self.cursor_pos, ch);
                self.cursor_pos += 1;
            }
            FormField::Description => {
                self.description.insert(self.cursor_pos, ch);
                self.cursor_pos += 1;
            }
            FormField::Notes => {
                self.notes.insert(self.cursor_pos, ch);
                self.cursor_pos += 1;
            }
            _ => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.focused_field {
            FormField::Start if self.cursor_pos > 0 => {
                self.cursor_pos -= 1;
                self.start_input.remove(self.cursor_pos);
            }
            FormField::End if self.cursor_pos > 0 => {
                self.cursor_pos -= 1;
                self.end_input.remove(self.cursor_pos);
            }
            FormField::Description if self.cursor_pos > 0 => {
                self.cursor_pos -= 1;
                self.description.remove(self.cursor_pos);
            }
            FormField::Notes if self.cursor_pos > 0 => {
                self.cursor_pos -= 1;
                self.notes.remove(self.cursor_pos);
            }
            _ => {}
        }
    }

    pub fn cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn cursor_right(&mut self) {
        let max = match self.focused_field {
            FormField::Start => self.start_input.len(),
            FormField::End => self.end_input.len(),
            FormField::Description => self.description.len(),
            FormField::Notes => self.notes.len(),
            _ => 0,
        };
        if self.cursor_pos < max {
            self.cursor_pos += 1;
        }
    }

    /// Parse inputs and check overlaps. Returns None if invalid.
    pub fn check_overlaps(&mut self, existing: &[TimeEntry]) {
        self.overlap_warning = None;

        let (start, end) = match self.parse_times() {
            Some(t) => t,
            None => return,
        };

        let exclude = self.original.as_ref();
        let result = find_overlaps(start, end, existing, exclude);
        if result.has_overlaps {
            let c = &result.overlaps[0].conflicting_entry;
            self.overlap_warning = Some(format!(
                "Overlaps with: {} – {} {}",
                c.start.format("%H:%M"),
                c.end.format("%H:%M"),
                c.description
            ));
        }
    }

    pub fn parse_times(&self) -> Option<(NaiveDateTime, NaiveDateTime)> {
        let fmt = "%Y-%m-%d %H:%M";
        let start = NaiveDateTime::parse_from_str(&self.start_input, fmt).ok()?;

        // Try parsing end as datetime first
        if let Ok(end) = NaiveDateTime::parse_from_str(&self.end_input, fmt) {
            return Some((start, end));
        }

        // Try parsing end as duration
        if let Some(minutes) = parse_duration(&self.end_input) {
            let end = start + chrono::Duration::minutes(minutes);
            return Some((start, end));
        }

        None
    }

    pub fn build_entry(&self, config: &AppConfig) -> Option<NewEntry> {
        let (start, end) = self.parse_times()?;
        let client = config.clients.get(self.client_idx)?;

        let project = if self.project_idx > 0 {
            client
                .projects
                .get(self.project_idx - 1)
                .map(|p| p.id.clone())
        } else {
            None
        };

        let activity = if self.activity_idx > 0 {
            client
                .activities
                .get(self.activity_idx - 1)
                .map(|a| a.id.clone())
        } else {
            None
        };

        Some(NewEntry {
            start,
            end,
            description: self.description.clone(),
            client: client.id.clone(),
            project,
            activity,
            notes: if self.notes.is_empty() {
                None
            } else {
                Some(self.notes.clone())
            },
        })
    }

    pub fn can_save(&self) -> bool {
        self.overlap_warning.is_none()
            && self.parse_times().is_some()
            && !self.description.is_empty()
    }

    pub fn client_name<'a>(&self, config: &'a AppConfig) -> &'a str {
        config
            .clients
            .get(self.client_idx)
            .map(|c| c.name.as_str())
            .unwrap_or("(none)")
    }

    pub fn project_name<'a>(&self, config: &'a AppConfig) -> &'a str {
        if self.project_idx == 0 {
            return "(none)";
        }
        config
            .clients
            .get(self.client_idx)
            .and_then(|c| c.projects.get(self.project_idx - 1))
            .map(|p| p.name.as_str())
            .unwrap_or("(none)")
    }

    pub fn activity_name<'a>(&self, config: &'a AppConfig) -> &'a str {
        if self.activity_idx == 0 {
            return "(none)";
        }
        config
            .clients
            .get(self.client_idx)
            .and_then(|c| c.activities.get(self.activity_idx - 1))
            .map(|a| a.name.as_str())
            .unwrap_or("(none)")
    }

    pub fn duration_display(&self) -> String {
        match self.parse_times() {
            Some((start, end)) => {
                let mins = (end - start).num_minutes();
                wdttg_core::time_utils::format_duration(mins)
            }
            None => "—".into(),
        }
    }
}
