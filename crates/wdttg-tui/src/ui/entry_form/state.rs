use chrono::NaiveDateTime;
use crossterm::event::KeyEvent;
use tui_textarea::TextArea;

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

    pub fn is_textarea(self) -> bool {
        matches!(self, Self::Description | Self::Notes)
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
    pub description_textarea: TextArea<'static>,
    pub notes_textarea: TextArea<'static>,
    pub focused_field: FormField,
    pub overlap_warning: Option<String>,
    pub error_message: Option<String>,
    pub cursor_pos: usize, // cursor within single-line text fields (Start, End)
    /// Y positions of fields, populated by render for mouse click targeting.
    pub field_positions: Vec<(FormField, u16)>,
}

/// Convert a stored string (with `<br>` tags) to textarea lines.
fn to_textarea_lines(s: &str) -> Vec<String> {
    if s.is_empty() {
        return vec![String::new()];
    }
    s.split("<br>").map(|l| l.to_string()).collect()
}

/// Convert textarea lines back to a storage string (with `<br>` tags).
fn from_textarea_lines(textarea: &TextArea<'_>) -> String {
    textarea.lines().join("<br>")
}

fn make_textarea(lines: Vec<String>) -> TextArea<'static> {
    TextArea::new(lines)
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
            description_textarea: make_textarea(vec![String::new()]),
            notes_textarea: make_textarea(vec![String::new()]),
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

        let desc_lines = to_textarea_lines(&entry.description);
        let notes_lines = to_textarea_lines(entry.notes.as_deref().unwrap_or(""));

        Self {
            mode: FormMode::Edit,
            original: Some(entry.clone()),
            start_input: entry.start.format("%Y-%m-%d %H:%M").to_string(),
            end_input: entry.end.format("%Y-%m-%d %H:%M").to_string(),
            client_idx,
            project_idx,
            activity_idx,
            description_textarea: make_textarea(desc_lines),
            notes_textarea: make_textarea(notes_lines),
            focused_field: FormField::Description,
            overlap_warning: None,
            error_message: None,
            cursor_pos: entry.description.len().min(16), // reasonable cursor start
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
            _ => 0,
        };
    }

    /// Pass a key event to the active textarea (Description or Notes).
    /// Returns true if the event was handled by the textarea.
    pub fn handle_textarea_input(&mut self, key: KeyEvent) -> bool {
        match self.focused_field {
            FormField::Description => {
                self.description_textarea.input(key);
                true
            }
            FormField::Notes => {
                self.notes_textarea.input(key);
                true
            }
            _ => false,
        }
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
            // Description and Notes are handled by textarea
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
            // Description and Notes are handled by textarea
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
            _ => 0,
        };
        if self.cursor_pos < max {
            self.cursor_pos += 1;
        }
    }

    /// Get the description text (with <br> for newlines).
    pub fn description(&self) -> String {
        from_textarea_lines(&self.description_textarea)
    }

    /// Get the notes text (with <br> for newlines).
    pub fn notes_value(&self) -> String {
        from_textarea_lines(&self.notes_textarea)
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

        let description = self.description();
        let notes = self.notes_value();

        Some(NewEntry {
            start,
            end,
            description,
            client: client.id.clone(),
            project,
            activity,
            notes: if notes.is_empty() { None } else { Some(notes) },
        })
    }

    pub fn can_save(&self) -> bool {
        let desc = self.description();
        self.overlap_warning.is_none() && self.parse_times().is_some() && !desc.is_empty()
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
