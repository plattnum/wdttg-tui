use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use wdttg_core::config::AppConfig;

use crate::theme::Theme;

use super::state::{EntryFormState, FormField, FormMode};

pub fn render_entry_form(
    frame: &mut Frame,
    area: Rect,
    state: &mut EntryFormState,
    config: &AppConfig,
    theme: &Theme,
) {
    state.field_positions.clear();
    let width = 52.min(area.width.saturating_sub(4));
    let height = 22.min(area.height.saturating_sub(4));
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

    // Description
    state.field_positions.push((FormField::Description, y_pos));
    render_field(
        frame,
        inner.x,
        y_pos,
        label_width,
        field_width,
        "Description:",
        &state.description,
        state.focused_field == FormField::Description,
        state.cursor_pos,
        theme,
    );
    y_pos += 2;

    // Notes
    state.field_positions.push((FormField::Notes, y_pos));
    render_field(
        frame,
        inner.x,
        y_pos,
        label_width,
        field_width,
        "Notes:",
        &state.notes,
        state.focused_field == FormField::Notes,
        state.cursor_pos,
        theme,
    );
    y_pos += 2;

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
            Span::styled(" Enter ", save_style),
            Span::styled(" Save  ", Style::default().fg(theme.muted)),
            Span::styled("    ", Style::default()),
            Span::styled(
                " Esc ",
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel ", Style::default().fg(theme.muted)),
            Span::styled("    ", Style::default()),
            Span::styled(
                " Tab ",
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Next field ", Style::default().fg(theme.muted)),
        ]);
        frame.render_widget(
            Paragraph::new(actions),
            Rect::new(inner.x, y_pos, inner.width, 1),
        );
    }
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
