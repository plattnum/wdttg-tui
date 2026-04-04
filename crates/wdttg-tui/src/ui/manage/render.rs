use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use wdttg_core::config::AppConfig;

use crate::theme::Theme;

use super::state::{EditForm, FormField, ManagePane, ManageState};

pub fn render_manage(
    frame: &mut Frame,
    area: Rect,
    state: &ManageState,
    config: &AppConfig,
    theme: &Theme,
) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Manage ", theme.title_style()));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // Three-pane layout: Clients | Projects | Activities
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(inner);

    render_client_pane(frame, panes[0], state, config, theme);
    render_project_pane(frame, panes[1], state, config, theme);
    render_activity_pane(frame, panes[2], state, config, theme);

    // Render edit form overlay if active
    if let Some(form) = &state.edit_form {
        render_edit_form(frame, area, form, theme);
    }
}

fn render_client_pane(
    frame: &mut Frame,
    area: Rect,
    state: &ManageState,
    config: &AppConfig,
    theme: &Theme,
) {
    let is_active = state.active_pane == ManagePane::Clients;
    let border_color = if is_active {
        theme.accent
    } else {
        theme.border
    };
    let title_style = if is_active {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.muted)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(" Clients ", title_style));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = config
        .clients
        .iter()
        .enumerate()
        .map(|(i, client)| {
            let is_selected = i == state.client_idx;
            let color_dot = Theme::parse_hex(&client.color).unwrap_or(theme.accent);

            let style = if client.archived {
                Style::default().fg(theme.muted)
            } else if is_selected && is_active {
                Style::default()
                    .fg(theme.fg)
                    .bg(theme.highlight_bg)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(theme.fg).bg(theme.highlight_bg)
            } else {
                Style::default().fg(theme.fg)
            };

            let archived_marker = if client.archived { " ⊘" } else { "" };
            let line = Line::from(vec![
                Span::styled("● ", Style::default().fg(color_dot)),
                Span::styled(format!("{}{}", client.name, archived_marker), style),
                Span::styled(
                    format!(" ({}) ${:.0}/{}", client.id, client.rate, client.currency),
                    Style::default().fg(theme.muted),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);

    // Hints at bottom
    if is_active && inner.height > 1 {
        let hint_y = inner.y + inner.height - 1;
        let hints = Span::styled(
            " a:add e:edit d/A:archive ",
            Style::default().fg(theme.muted),
        );
        frame.render_widget(
            Paragraph::new(hints),
            Rect::new(inner.x, hint_y, inner.width, 1),
        );
    }
}

fn render_project_pane(
    frame: &mut Frame,
    area: Rect,
    state: &ManageState,
    config: &AppConfig,
    theme: &Theme,
) {
    let is_active = state.active_pane == ManagePane::Projects;
    let border_color = if is_active {
        theme.accent
    } else {
        theme.border
    };
    let title_style = if is_active {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.muted)
    };

    let client = config.clients.get(state.client_idx);
    let client_name = client.map(|c| c.name.as_str()).unwrap_or("—");
    let title = format!(" Projects ({client_name}) ");

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title, title_style));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let projects = client.map(|c| &c.projects[..]).unwrap_or(&[]);

    if projects.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "  No projects",
                Style::default().fg(theme.muted),
            )),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
    } else {
        let items: Vec<ListItem> = projects
            .iter()
            .enumerate()
            .map(|(i, proj)| {
                let is_selected = i == state.project_idx;
                let color_dot = Theme::parse_hex(&proj.color).unwrap_or(theme.accent);

                let style = if proj.archived {
                    Style::default().fg(theme.muted)
                } else if is_selected && is_active {
                    Style::default()
                        .fg(theme.fg)
                        .bg(theme.highlight_bg)
                        .add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default().fg(theme.fg).bg(theme.highlight_bg)
                } else {
                    Style::default().fg(theme.fg)
                };

                let rate_str = proj
                    .rate_override
                    .map(|r| format!(" ${r:.0}"))
                    .unwrap_or_default();
                let archived_marker = if proj.archived { " ⊘" } else { "" };

                let line = Line::from(vec![
                    Span::styled("● ", Style::default().fg(color_dot)),
                    Span::styled(format!("{}{}", proj.name, archived_marker), style),
                    Span::styled(rate_str, Style::default().fg(theme.muted)),
                ]);
                ListItem::new(line)
            })
            .collect();

        frame.render_widget(List::new(items), inner);
    }

    if is_active && inner.height > 1 {
        let hint_y = inner.y + inner.height - 1;
        frame.render_widget(
            Paragraph::new(Span::styled(
                " a:add e:edit d/A:archive ",
                Style::default().fg(theme.muted),
            )),
            Rect::new(inner.x, hint_y, inner.width, 1),
        );
    }
}

fn render_activity_pane(
    frame: &mut Frame,
    area: Rect,
    state: &ManageState,
    config: &AppConfig,
    theme: &Theme,
) {
    let is_active = state.active_pane == ManagePane::Activities;
    let border_color = if is_active {
        theme.accent
    } else {
        theme.border
    };
    let title_style = if is_active {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.muted)
    };

    let client = config.clients.get(state.client_idx);
    let client_name = client.map(|c| c.name.as_str()).unwrap_or("—");
    let title = format!(" Activities ({client_name}) ");

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title, title_style));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let activities = client.map(|c| &c.activities[..]).unwrap_or(&[]);

    if activities.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "  No activities",
                Style::default().fg(theme.muted),
            )),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
    } else {
        let items: Vec<ListItem> = activities
            .iter()
            .enumerate()
            .map(|(i, act)| {
                let is_selected = i == state.activity_idx;
                let color_dot = Theme::parse_hex(&act.color).unwrap_or(theme.accent);

                let style = if act.archived {
                    Style::default().fg(theme.muted)
                } else if is_selected && is_active {
                    Style::default()
                        .fg(theme.fg)
                        .bg(theme.highlight_bg)
                        .add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default().fg(theme.fg).bg(theme.highlight_bg)
                } else {
                    Style::default().fg(theme.fg)
                };

                let archived_marker = if act.archived { " ⊘" } else { "" };
                let line = Line::from(vec![
                    Span::styled("● ", Style::default().fg(color_dot)),
                    Span::styled(format!("{}{}", act.name, archived_marker), style),
                ]);
                ListItem::new(line)
            })
            .collect();

        frame.render_widget(List::new(items), inner);
    }

    if is_active && inner.height > 1 {
        let hint_y = inner.y + inner.height - 1;
        frame.render_widget(
            Paragraph::new(Span::styled(
                " a:add e:edit d/A:archive ",
                Style::default().fg(theme.muted),
            )),
            Rect::new(inner.x, hint_y, inner.width, 1),
        );
    }
}

fn render_edit_form(frame: &mut Frame, area: Rect, form: &EditForm, theme: &Theme) {
    let width = 44.min(area.width.saturating_sub(4));
    let height = match form.target {
        super::state::EditTarget::Client => 12,
        _ => 10,
    }
    .min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup);

    let title = if form.is_new { " Add " } else { " Edit " };
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
    let lw: u16 = 10;
    let fw = inner.width.saturating_sub(lw + 1);

    // ID (editable for new, read-only display for edit)
    if form.is_new {
        render_form_field(
            frame,
            inner.x,
            y_pos,
            lw,
            fw,
            "ID:",
            &form.id,
            form.focused == FormField::Id,
            form.cursor_pos,
            theme,
        );
    } else {
        // Show ID as read-only label
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("{:>width$}", "ID:", width = lw as usize),
                Style::default().fg(theme.muted),
            )),
            Rect::new(inner.x, y_pos, lw, 1),
        );
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!(" {}", form.id),
                Style::default().fg(theme.accent_dim),
            )),
            Rect::new(inner.x + lw + 1, y_pos, fw, 1),
        );
    }
    y_pos += 1;

    render_form_field(
        frame,
        inner.x,
        y_pos,
        lw,
        fw,
        "Name:",
        &form.name,
        form.focused == FormField::Name,
        form.cursor_pos,
        theme,
    );
    y_pos += 1;

    render_form_field(
        frame,
        inner.x,
        y_pos,
        lw,
        fw,
        "Color:",
        &form.color,
        form.focused == FormField::Color,
        form.cursor_pos,
        theme,
    );
    // Show color preview
    if let Some(color) = Theme::parse_hex(&form.color) {
        let preview_x = inner.x + lw + 1 + form.color.len() as u16 + 1;
        if preview_x + 4 < inner.x + inner.width {
            frame.render_widget(
                Paragraph::new(Span::styled("████", Style::default().fg(color))),
                Rect::new(preview_x, y_pos, 4, 1),
            );
        }
    }
    y_pos += 1;

    if matches!(
        form.target,
        super::state::EditTarget::Client | super::state::EditTarget::Project
    ) {
        let label = if matches!(form.target, super::state::EditTarget::Client) {
            "Rate:"
        } else {
            "Override:"
        };
        render_form_field(
            frame,
            inner.x,
            y_pos,
            lw,
            fw,
            label,
            &form.rate,
            form.focused == FormField::Rate,
            form.cursor_pos,
            theme,
        );
        y_pos += 1;
    }

    if matches!(form.target, super::state::EditTarget::Client) {
        render_form_field(
            frame,
            inner.x,
            y_pos,
            lw,
            fw,
            "Currency:",
            &form.currency,
            form.focused == FormField::Currency,
            form.cursor_pos,
            theme,
        );
        y_pos += 1;
    }

    y_pos += 1;

    if let Some(ref err) = form.error {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("✗ {err}"),
                Style::default().fg(theme.error),
            )),
            Rect::new(inner.x, y_pos, inner.width, 1),
        );
        y_pos += 1;
    }

    if y_pos < inner.y + inner.height {
        let actions = Line::from(vec![
            Span::styled(
                " Enter ",
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Save  ", Style::default().fg(theme.muted)),
            Span::styled("  ", Style::default()),
            Span::styled(
                " Esc ",
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Cancel  ", Style::default().fg(theme.muted)),
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

#[allow(clippy::too_many_arguments)]
fn render_form_field(
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
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{label:>width$}", width = label_width as usize),
            Style::default().fg(theme.muted),
        )),
        Rect::new(x, y, label_width, 1),
    );

    let field_x = x + label_width + 1;
    let (fg, bg) = if focused {
        (theme.fg, theme.highlight_bg)
    } else {
        (theme.fg, theme.bg)
    };

    let display: String = if value.len() > field_width as usize {
        value[..field_width as usize].to_string()
    } else {
        format!("{:width$}", value, width = field_width as usize)
    };

    frame.render_widget(
        Paragraph::new(Span::styled(display, Style::default().fg(fg).bg(bg))),
        Rect::new(field_x, y, field_width, 1),
    );

    if focused {
        let cx = field_x + cursor_pos.min(field_width as usize) as u16;
        if cx < field_x + field_width {
            frame.set_cursor_position((cx, y));
        }
    }
}
