use std::io;
use std::time::Duration;

use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use wdttg_core::config::AppConfig;

use crate::action::Action;
use crate::event::{AppEvent, EventHandler};
use crate::input::handle_key;
use crate::theme::Theme;
use crate::ui;
use crate::ui::entry_form::{EntryFormState, FormMode};
use crate::ui::manage::ManageState;
use crate::ui::reports::ReportsState;
use crate::ui::timeline::TimelineState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Timeline,
    Reports,
    Manage,
}

impl Screen {
    pub fn title(&self) -> &str {
        match self {
            Screen::Timeline => "Timeline",
            Screen::Reports => "Reports",
            Screen::Manage => "Manage",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Screen::Timeline => 0,
            Screen::Reports => 1,
            Screen::Manage => 2,
        }
    }
}

pub struct App {
    pub config: AppConfig,
    pub screen: Screen,
    pub show_help: bool,
    pub help_scroll: usize,
    pub show_welcome: bool,
    pub should_quit: bool,
    pub status_message: Option<(String, std::time::Instant)>,
    pub theme: Theme,
    pub timeline: TimelineState,
    pub reports: ReportsState,
    pub manage: ManageState,
    pub entry_form: Option<EntryFormState>,
}

impl App {
    pub fn new(mut config: AppConfig, first_run: bool) -> Self {
        Self::sort_config(&mut config);
        let timeline = TimelineState::new(&config);
        let reports = ReportsState::new(&config);
        Self {
            config,
            screen: Screen::Timeline,
            show_help: false,
            help_scroll: 0,
            show_welcome: first_run,
            should_quit: false,
            theme: Theme::default(),
            timeline,
            reports,
            manage: ManageState::new(),
            entry_form: None,
            status_message: None,
        }
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> color_eyre::Result<()> {
        let data_dir = self.timeline.data_dir();
        let events = EventHandler::new(Duration::from_millis(250), data_dir);

        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;

            match events.next()? {
                AppEvent::Key(key) => {
                    if self.show_welcome {
                        match key.code {
                            KeyCode::Enter | KeyCode::Esc => self.show_welcome = false,
                            KeyCode::Char('?') => {
                                self.show_welcome = false;
                                self.show_help = true;
                            }
                            _ => {}
                        }
                    } else if self.entry_form.is_some() {
                        self.handle_form_key(key);
                    } else if self.manage.edit_form.is_some() {
                        self.handle_manage_form_key(key);
                    } else if let Some(action) = handle_key(key, self.screen, self.show_help) {
                        self.handle_action(action);
                    }
                }
                AppEvent::Mouse(mouse) => {
                    if self.entry_form.is_some() {
                        self.handle_form_mouse(mouse);
                    } else {
                        self.handle_mouse(mouse);
                    }
                }
                AppEvent::FileChanged(month_key) => {
                    self.handle_file_changed(&month_key);
                }
                AppEvent::Resize | AppEvent::Tick => {}
            }
        }

        Ok(())
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::SwitchToTimeline => self.screen = Screen::Timeline,
            Action::SwitchToReports => {
                self.screen = Screen::Reports;
                self.reports.needs_refresh = true;
            }
            Action::SwitchToManage => self.screen = Screen::Manage,
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
                self.help_scroll = 0;
            }
            Action::ClosePopup => {
                if self.timeline.mark_start.is_some() {
                    self.timeline.mark_start = None;
                } else {
                    self.show_help = false;
                }
            }
            Action::NavigateDown if self.show_help => {
                self.help_scroll += 1;
            }
            Action::NavigateUp if self.show_help => {
                self.help_scroll = self.help_scroll.saturating_sub(1);
            }
            _ if self.screen == Screen::Timeline => {
                self.handle_timeline_action(action);
            }
            _ if self.screen == Screen::Reports => {
                self.handle_reports_action(action);
            }
            _ if self.screen == Screen::Manage => {
                self.handle_manage_action(action);
            }
            _ => {}
        }
    }

    fn handle_file_changed(&mut self, month_key: &str) {
        // Parse month_key "YYYY-MM" into a NaiveDate (first of month) for invalidation
        if let Some(date) = parse_month_key(month_key) {
            self.timeline.invalidate_month(date);
            self.reports.invalidate_month(month_key);
            self.reports.needs_refresh = true;
        }
    }

    fn handle_timeline_action(&mut self, action: Action) {
        let snap = self.timeline.snap_minutes(&self.config);
        match action {
            Action::NavigateLeft => self.timeline.navigate_left(),
            Action::NavigateRight => self.timeline.navigate_right(),
            Action::NavigateUp => self.timeline.navigate_up(snap),
            Action::NavigateDown => self.timeline.navigate_down(snap),
            Action::JumpToToday => self.timeline.jump_to_today(),
            Action::ScrollWeekLeft => self.timeline.scroll_week_left(),
            Action::ScrollWeekRight => self.timeline.scroll_week_right(),
            Action::PageUp => self.timeline.page_up(),
            Action::PageDown => self.timeline.page_down(),
            Action::Create => self.open_create_form(),
            Action::Edit => self.open_edit_form(),
            Action::Delete => self.delete_selected_entry(),
            Action::MarkTime => self.handle_mark_time(),
            _ => {}
        }
    }

    fn handle_mark_time(&mut self) {
        let cursor_dt = self.timeline.cursor_datetime();

        if let Some(mark_dt) = self.timeline.mark_start.take() {
            // Second press: open form with marked range
            let (start, end) = if mark_dt <= cursor_dt {
                (mark_dt, cursor_dt)
            } else {
                (cursor_dt, mark_dt)
            };

            // Don't open a zero-length entry
            if start == end {
                return;
            }

            let mut form = EntryFormState::new_create(start, end, &self.config);

            let entries = self
                .timeline
                .day_entries
                .get(&start.date())
                .cloned()
                .unwrap_or_default();
            form.check_overlaps(&entries);

            self.entry_form = Some(form);
        } else {
            // First press: set the mark
            self.timeline.mark_start = Some(cursor_dt);
        }
    }

    fn handle_reports_action(&mut self, action: Action) {
        match action {
            Action::NavigateLeft => self.reports.cycle_preset_backward(&self.config),
            Action::NavigateRight => self.reports.cycle_preset_forward(&self.config),
            Action::NavigateUp => self.reports.move_up(),
            Action::NavigateDown => self.reports.move_down(),
            Action::Select => self.reports.toggle_expand(),
            _ => {}
        }
    }

    fn handle_manage_action(&mut self, action: Action) {
        if self.manage.edit_form.is_some() {
            return; // form handles its own input
        }
        match action {
            Action::NavigateUp => self.manage.move_up(),
            Action::NavigateDown => self.manage.move_down(&self.config),
            Action::NavigateRight | Action::Select => self.manage.switch_pane(),
            Action::NavigateLeft => {
                self.manage.active_pane = self.manage.active_pane.prev();
            }
            Action::Create => self.manage.open_add(),
            Action::Edit => self.manage.open_edit(&self.config),
            Action::Delete => {
                self.manage.delete_selected(&mut self.config);
                self.save_config();
            }
            Action::ToggleArchive => {
                self.manage.toggle_archive(&mut self.config);
                self.save_config();
            }
            _ => {}
        }
    }

    fn handle_manage_form_key(&mut self, key: KeyEvent) {
        let Some(form) = &mut self.manage.edit_form else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                self.manage.edit_form = None;
                return;
            }
            KeyCode::Tab => form.next_field(),
            KeyCode::Enter => {
                let client_idx = self.manage.client_idx;
                if form.validate(&self.config, Some(client_idx)) {
                    form.apply_to_config(&mut self.config, client_idx);
                    self.manage.edit_form = None;
                    self.save_config();
                }
                return;
            }
            KeyCode::Backspace => form.backspace(),
            KeyCode::Left => form.cursor_left(),
            KeyCode::Right => form.cursor_right(),
            KeyCode::Char(ch) => form.type_char(ch),
            _ => {}
        }
    }

    fn save_config(&mut self) {
        Self::sort_config(&mut self.config);
        let _ = wdttg_core::config::save_config(&self.config);
    }

    fn sort_config(config: &mut AppConfig) {
        config
            .clients
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        for client in &mut config.clients {
            client
                .projects
                .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            client
                .activities
                .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }
    }

    fn open_create_form(&mut self) {
        let cursor_dt = self.timeline.cursor_datetime();
        let snap = self.config.preferences.snap_minutes.max(15);
        let end = cursor_dt + chrono::Duration::minutes(snap as i64);
        let mut form = EntryFormState::new_create(cursor_dt, end, &self.config);

        // Check overlaps with existing entries
        let entries = self
            .timeline
            .day_entries
            .get(&self.timeline.cursor_date)
            .cloned()
            .unwrap_or_default();
        form.check_overlaps(&entries);

        self.entry_form = Some(form);
    }

    fn open_edit_form(&mut self) {
        if let Some(entry) = self.timeline.selected_time_entry().cloned() {
            let form = EntryFormState::new_edit(&entry, &self.config);
            self.entry_form = Some(form);
        }
    }

    fn delete_selected_entry(&mut self) {
        let entry = match self.timeline.selected_time_entry().cloned() {
            Some(e) => e,
            None => return,
        };
        let date = entry.start.date();
        match self.timeline.do_delete(entry.start, entry.end) {
            Ok(()) => {
                self.timeline.invalidate_month(date);
                self.set_status("Entry deleted");
            }
            Err(e) => self.set_status(format!("Delete failed: {e}")),
        }
    }

    fn handle_form_key(&mut self, key: KeyEvent) {
        use crate::ui::entry_form::state::FormField;

        let Some(form) = &mut self.entry_form else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                self.entry_form = None;
                return;
            }
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    form.prev_field();
                } else {
                    form.next_field();
                }
            }
            KeyCode::BackTab => form.prev_field(),
            KeyCode::Enter => {
                self.save_form();
                return;
            }
            KeyCode::Backspace => form.backspace(),
            KeyCode::Left => match form.focused_field {
                FormField::Client => form.cycle_client(&self.config, false),
                FormField::Project => form.cycle_project(&self.config, false),
                FormField::Activity => form.cycle_activity(&self.config, false),
                _ => form.cursor_left(),
            },
            KeyCode::Right => match form.focused_field {
                FormField::Client => form.cycle_client(&self.config, true),
                FormField::Project => form.cycle_project(&self.config, true),
                FormField::Activity => form.cycle_activity(&self.config, true),
                _ => form.cursor_right(),
            },
            KeyCode::Up => match form.focused_field {
                FormField::Client => form.cycle_client(&self.config, false),
                FormField::Project => form.cycle_project(&self.config, false),
                FormField::Activity => form.cycle_activity(&self.config, false),
                _ => {}
            },
            KeyCode::Down => match form.focused_field {
                FormField::Client => form.cycle_client(&self.config, true),
                FormField::Project => form.cycle_project(&self.config, true),
                FormField::Activity => form.cycle_activity(&self.config, true),
                _ => {}
            },
            KeyCode::Char(ch) => form.type_char(ch),
            _ => {}
        }

        // Re-check overlaps after any input change
        if let Some(form) = &mut self.entry_form {
            let entries = self
                .timeline
                .day_entries
                .get(&self.timeline.cursor_date)
                .cloned()
                .unwrap_or_default();
            form.check_overlaps(&entries);
        }
    }

    fn save_form(&mut self) {
        // Extract what we need from the form before mutating self
        let (new_entry, mode, original_start_end) = {
            let Some(form) = &self.entry_form else {
                return;
            };
            if !form.can_save() {
                return;
            }
            let Some(entry) = form.build_entry(&self.config) else {
                return;
            };
            let orig = form.original.as_ref().map(|o| (o.start, o.end));
            (entry, form.mode, orig)
        };

        let save_date = new_entry.start.date();

        let result = match mode {
            FormMode::Create => self.timeline.do_create(new_entry, &self.config),
            FormMode::Edit => {
                let (os, oe) = original_start_end.unwrap();
                self.timeline.do_update(os, oe, new_entry, &self.config)
            }
        };

        match result {
            Ok(_entry) => {
                self.timeline.invalidate_month(save_date);
                self.entry_form = None;
                let action = if mode == FormMode::Create {
                    "created"
                } else {
                    "updated"
                };
                self.set_status(format!("Entry {action}"));
            }
            Err(e) => {
                if let Some(form) = &mut self.entry_form {
                    form.error_message = Some(e.to_string());
                }
            }
        }
    }

    fn handle_form_mouse(&mut self, mouse: MouseEvent) {
        if let MouseEventKind::Down(_) = mouse.kind {
            if let Some(form) = &mut self.entry_form {
                form.click_at(mouse.column, mouse.row);
            }
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            // Tab bar clicks (first 3 rows)
            MouseEventKind::Down(_) if mouse.row < 3 => {
                self.handle_tab_click(mouse.column);
            }
            MouseEventKind::ScrollUp if self.screen == Screen::Timeline => {
                self.timeline.mouse_scroll_up(3);
            }
            MouseEventKind::ScrollDown if self.screen == Screen::Timeline => {
                self.timeline.mouse_scroll_down(3);
            }
            _ => {}
        }
    }

    fn handle_tab_click(&mut self, x: u16) {
        // Tab layout: " wdttg " border, then tabs separated by " │ "
        // Approximate positions based on tab text widths:
        // "① Timeline" ~12 chars, "② Reports" ~10 chars, "③ Manage" ~9 chars
        // With padding and dividers, roughly:
        //   Timeline: columns ~2-15
        //   Reports:  columns ~18-30
        //   Manage:   columns ~33-44
        // These are approximate -- any click in the tab bar area selects the closest tab.
        if x < 16 {
            self.screen = Screen::Timeline;
        } else if x < 32 {
            self.screen = Screen::Reports;
        } else {
            self.screen = Screen::Manage;
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let theme = &self.theme;

        frame.render_widget(Block::default().style(theme.style()), area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        self.render_tabs(frame, chunks[0]);
        self.render_content(frame, chunks[1]);
        self.render_status_bar(frame, chunks[2]);

        if self.show_welcome {
            ui::help::render_welcome(frame, area, &self.theme);
        } else if self.show_help {
            ui::help::render_help(frame, area, &self.theme, self.help_scroll);
        }

        if let Some(form) = &mut self.entry_form {
            let config = &self.config;
            let theme = &self.theme;
            ui::entry_form::render_entry_form(frame, area, form, config, theme);
        }
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let theme = &self.theme;
        let tab_titles: Vec<Line> = ["① Timeline", "② Reports", "③ Manage"]
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let color = if i == self.screen.index() {
                    theme.tab_active
                } else {
                    theme.tab_inactive
                };
                Line::from(Span::styled(*t, Style::default().fg(color)))
            })
            .collect();

        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(theme.border))
                    .title(Span::styled(
                        " wdttg ",
                        Style::default()
                            .fg(theme.title)
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .select(self.screen.index())
            .style(Style::default().fg(theme.tab_inactive))
            .highlight_style(
                Style::default()
                    .fg(theme.tab_active)
                    .add_modifier(Modifier::BOLD),
            )
            .divider(Span::styled(" │ ", Style::default().fg(theme.border)));

        frame.render_widget(tabs, area);
    }

    fn render_content(&mut self, frame: &mut Frame, area: Rect) {
        match self.screen {
            Screen::Timeline => {
                let config = &self.config;
                let theme = &self.theme;
                ui::timeline::render_timeline(frame, area, &mut self.timeline, config, theme);
            }
            Screen::Reports => {
                let config = &self.config;
                let theme = &self.theme;
                ui::reports::render_reports(frame, area, &mut self.reports, config, theme);
            }
            Screen::Manage => {
                let config = &self.config;
                let theme = &self.theme;
                ui::manage::render_manage(frame, area, &self.manage, config, theme);
            }
        }
    }

    fn render_status_bar(&mut self, frame: &mut Frame, area: Rect) {
        let theme = &self.theme;

        let screen_name = Span::styled(
            format!(" {} ", self.screen.title()),
            Style::default()
                .fg(theme.bg)
                .bg(theme.accent)
                .add_modifier(Modifier::BOLD),
        );

        // Check for status message (auto-dismiss after 4 seconds)
        let msg_span = if let Some((ref msg, when)) = self.status_message {
            if when.elapsed().as_secs() < 4 {
                Span::styled(
                    format!(" {msg} "),
                    Style::default()
                        .fg(theme.bg)
                        .bg(theme.success)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                self.status_message = None;
                Span::styled("", Style::default())
            }
        } else {
            Span::styled("", Style::default())
        };

        let hints = Span::styled(
            " q:quit  1-3:screens  ?:help  n:new  e:edit  d:del ",
            Style::default().fg(theme.muted),
        );

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M");
        let time = Span::styled(format!(" {now} "), Style::default().fg(theme.accent_dim));

        let bar = Line::from(vec![screen_name, msg_span, hints, time]);
        let status = Paragraph::new(bar).style(theme.status_bar_style());
        frame.render_widget(status, area);
    }
}

/// Parse a "YYYY-MM" month key into a NaiveDate (first of month).
fn parse_month_key(key: &str) -> Option<NaiveDate> {
    if key.len() != 7 || key.as_bytes()[4] != b'-' {
        return None;
    }
    let year: i32 = key[..4].parse().ok()?;
    let month: u32 = key[5..7].parse().ok()?;
    NaiveDate::from_ymd_opt(year, month, 1)
}
