use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use wdttg_core::config::AppConfig;

use crate::theme::Theme;

use super::state::{EntryFormState, FormField, FormMode};

const DESC_ROWS: u16 = 4;
const NOTES_ROWS: u16 = 2;

pub fn render_entry_form(
    frame: &mut Frame,
    area: Rect,
    state: &mut EntryFormState,
    config: &AppConfig,
    theme: &Theme,
) {
    state.field_positions.clear();
    let width = 52.min(area.width.saturating_sub(4));
    // Form needs: 2 (start/end) + 1 (gap) + 3 (client/project/activity)
    // + 1 (gap) + 1 (desc label) + DESC_ROWS + 1 (notes label) + NOTES_ROWS
    // + 1 (gap) + 1 (actions) + 2 (warnings) + 2 (borders) = ~20 + textarea rows
    let height = (18 + DESC_ROWS + NOTES_ROWS).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup);

    let title = match state.mode {
        FormMode::Create => " New Entry ",
        FormMode::Edit => " Edit Entry ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(
            title,
            Style::default()
                .fg(theme.title)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let mut y_pos = inner.y;
    let label_width: u16 = 13;
    let field_width = inner.width.saturating_sub(label_width + 1);

    // Start
    state.field_positions.push((FormField::Start, y_pos));
    render_field(
        frame,
        inner.x,
        y_pos,
        label_width,
        field_width,
        "Start:",
        &state.start_input,
        state.focused_field == FormField::Start,
        state.cursor_pos,
        theme,
    );
    y_pos += 1;

    // End
    state.field_positions.push((FormField::End, y_pos));
    let end_suffix = format!("  ({})", state.duration_display());
    let end_display = format!("{}{}", state.end_input, end_suffix);
    render_field(
        frame,
        inner.x,
        y_pos,
        label_width,
        field_width,
        "End:",
        &end_display,
        state.focused_field == FormField::End,
        state.cursor_pos,
        theme,
    );
    y_pos += 2;

    // Client
    state.field_positions.push((FormField::Client, y_pos));
    let client_name = state.client_name(config);
    render_dropdown_field(
        frame,
        inner.x,
        y_pos,
        label_width,
        field_width,
        "Client:",
        client_name,
        state.focused_field == FormField::Client,
        theme,
    );
    y_pos += 1;

    // Project
    state.field_positions.push((FormField::Project, y_pos));
    let project_name = state.project_name(config);
    render_dropdown_field(
        frame,
        inner.x,
        y_pos,
        label_width,
        field_width,
        "Project:",
        project_name,
        state.focused_field == FormField::Project,
        theme,
    );
    y_pos += 1;

    // Activity
    state.field_positions.push((FormField::Activity, y_pos));
    let activity_name = state.activity_name(config);
    render_dropdown_field(
        frame,
        inner.x,
        y_pos,
        label_width,
        field_width,
        "Activity:",
        activity_name,
        state.focused_field == FormField::Activity,
        theme,
    );
    y_pos += 2;

    // Description label
    state.field_positions.push((FormField::Description, y_pos));
    let desc_label_style = if state.focused_field == FormField::Description {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.muted)
    };
    frame.render_widget(
        Paragraph::new(Span::styled("Description:", desc_label_style)),
        Rect::new(inner.x, y_pos, inner.width, 1),
    );
    y_pos += 1;

    // Description textarea
    let desc_focused = state.focused_field == FormField::Description;
    let desc_area = Rect::new(inner.x, y_pos, inner.width, DESC_ROWS);
    render_textarea(
        frame,
        desc_area,
        &mut state.description_textarea,
        desc_focused,
        theme,
    );
    y_pos += DESC_ROWS;

    // Notes label
    state.field_positions.push((FormField::Notes, y_pos));
    let notes_label_style = if state.focused_field == FormField::Notes {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.muted)
    };
    frame.render_widget(
        Paragraph::new(Span::styled("Notes:", notes_label_style)),
        Rect::new(inner.x, y_pos, inner.width, 1),
    );
    y_pos += 1;

    // Notes textarea
    let notes_focused = state.focused_field == FormField::Notes;
    let notes_area = Rect::new(inner.x, y_pos, inner.width, NOTES_ROWS);
    render_textarea(
        frame,
        notes_area,
        &mut state.notes_textarea,
        notes_focused,
        theme,
    );
    y_pos += NOTES_ROWS;

    // Overlap warning
    if let Some(ref warning) = state.overlap_warning {
        let w = Paragraph::new(Span::styled(
            format!("⚠ {warning}"),
            Style::default().fg(theme.error),
        ));
        frame.render_widget(w, Rect::new(inner.x, y_pos, inner.width, 1));
        y_pos += 1;
    }

    // Error message
    if let Some(ref err) = state.error_message {
        let e = Paragraph::new(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(theme.error),
        ));
        frame.render_widget(e, Rect::new(inner.x, y_pos, inner.width, 1));
        y_pos += 1;
    }

    // Action bar
    if y_pos < inner.y + inner.height {
        let save_style = if state.can_save() {
            Style::default()
                .fg(theme.bg)
                .bg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted).bg(theme.border)
        };

        let actions = Line::from(vec![
            Span::styled(" ^S ", save_style),
            Span::styled(" Save  ", Style::default().fg(theme.muted)),
            Span::styled("  ", Style::default()),
            Span::styled(
                " Esc ",
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel ", Style::default().fg(theme.muted)),
            Span::styled("  ", Style::default()),
            Span::styled(
                " Tab ",
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Next ", Style::default().fg(theme.muted)),
        ]);
        frame.render_widget(
            Paragraph::new(actions),
            Rect::new(inner.x, y_pos, inner.width, 1),
        );
    }
}

fn render_textarea(
    frame: &mut Frame,
    area: Rect,
    textarea: &mut tui_textarea::TextArea<'static>,
    focused: bool,
    theme: &Theme,
) {
    if focused {
        textarea.set_style(Style::default().fg(theme.fg).bg(theme.highlight_bg));
        textarea.set_cursor_line_style(Style::default().fg(theme.fg).bg(theme.highlight_bg));
        textarea.set_cursor_style(Style::default().fg(theme.bg).bg(theme.fg));
    } else {
        textarea.set_style(Style::default().fg(theme.muted).bg(theme.bg));
        textarea.set_cursor_line_style(Style::default());
        textarea.set_cursor_style(Style::default().fg(theme.muted));
    }

    frame.render_widget(&*textarea, area);
}

#[allow(clippy::too_many_arguments)]
fn render_field(
    frame: &mut Frame,
    x: u16,
    y: u16,
    label_width: u16,
    field_width: u16,
    label: &str,
    value: &str,
    focused: bool,
    cursor_pos: usize,
    theme: &Theme,
) {
    let label_style = Style::default().fg(theme.muted);
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{label:>width$}", width = label_width as usize),
            label_style,
        )),
        Rect::new(x, y, label_width, 1),
    );

    let field_x = x + label_width + 1;
    let (fg, bg) = if focused {
        (theme.fg, theme.highlight_bg)
    } else {
        (theme.fg, theme.bg)
    };

    // Truncate display to field width
    let display: String = if value.len() > field_width as usize {
        value[..field_width as usize].to_string()
    } else {
        format!("{:width$}", value, width = field_width as usize)
    };

    frame.render_widget(
        Paragraph::new(Span::styled(display, Style::default().fg(fg).bg(bg))),
        Rect::new(field_x, y, field_width, 1),
    );

    // Show cursor
    if focused {
        let cx = field_x + cursor_pos.min(field_width as usize) as u16;
        if cx < field_x + field_width {
            frame.set_cursor_position((cx, y));
        }
    }
}

fn render_dropdown_field(
    frame: &mut Frame,
    x: u16,
    y: u16,
    label_width: u16,
    field_width: u16,
    label: &str,
    value: &str,
    focused: bool,
    theme: &Theme,
) {
    let label_style = Style::default().fg(theme.muted);
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{label:>width$}", width = label_width as usize),
            label_style,
        )),
        Rect::new(x, y, label_width, 1),
    );

    let field_x = x + label_width + 1;
    let (fg, bg) = if focused {
        (theme.accent, theme.highlight_bg)
    } else {
        (theme.fg, theme.bg)
    };

    let indicator = if focused { "◂ ▸" } else { "  ▾" };
    let display = format!(
        " {value:width$} {indicator}",
        width = field_width.saturating_sub(6) as usize
    );

    frame.render_widget(
        Paragraph::new(Span::styled(display, Style::default().fg(fg).bg(bg))),
        Rect::new(field_x, y, field_width, 1),
    );
}
