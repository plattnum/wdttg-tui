use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use wdttg_core::config::AppConfig;
use wdttg_core::time_utils::format_duration;

use crate::theme::Theme;

use super::state::ReportsState;

pub fn render_reports(
    frame: &mut Frame,
    area: Rect,
    state: &mut ReportsState,
    config: &AppConfig,
    theme: &Theme,
) {
    if state.needs_refresh {
        state.refresh(config);
    }

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Reports ", theme.title_style()));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header: preset selector + date range + totals
            Constraint::Min(1),    // Report breakdown
        ])
        .split(inner);

    render_header(frame, chunks[0], state, theme);
    render_breakdown(frame, chunks[1], state, theme);
}

fn render_header(frame: &mut Frame, area: Rect, state: &ReportsState, theme: &Theme) {
    // Line 1: Preset selector
    let preset_line = Line::from(vec![
        Span::styled("◂ ", Style::default().fg(theme.accent)),
        Span::styled(
            state.preset_name(),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ▸", Style::default().fg(theme.accent)),
        Span::styled("    ", Style::default()),
        Span::styled(
            format!(
                "{} — {}",
                state.range.start.format("%Y-%m-%d"),
                state.range.end.format("%Y-%m-%d")
            ),
            Style::default().fg(theme.muted),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(preset_line),
        Rect::new(area.x, area.y, area.width, 1),
    );

    // Line 2: Grand totals
    let grand_total: i64 = state.reports.iter().map(|r| r.total_minutes).sum();
    let grand_billable: f64 = state.reports.iter().map(|r| r.billable_amount).sum();
    let client_count = state.reports.len();

    let totals_line = Line::from(vec![
        Span::styled(
            format!("{} ", format_duration(grand_total)),
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("total  ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("${grand_billable:.2} "),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("billable  ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{client_count} clients"),
            Style::default().fg(theme.muted),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(totals_line),
        Rect::new(area.x, area.y + 1, area.width, 1),
    );

    // Line 3: Navigation hints
    let hints = Line::from(vec![
        Span::styled("← → ", Style::default().fg(theme.accent)),
        Span::styled("period  ", Style::default().fg(theme.muted)),
        Span::styled("↑ ↓ ", Style::default().fg(theme.accent)),
        Span::styled("navigate  ", Style::default().fg(theme.muted)),
        Span::styled("Enter ", Style::default().fg(theme.accent)),
        Span::styled(
            "expand/collapse client or project  ",
            Style::default().fg(theme.muted),
        ),
        Span::styled("x ", Style::default().fg(theme.accent)),
        Span::styled("export CSV", Style::default().fg(theme.muted)),
    ]);
    frame.render_widget(
        Paragraph::new(hints),
        Rect::new(area.x, area.y + 2, area.width, 1),
    );
}

fn render_breakdown(frame: &mut Frame, area: Rect, state: &ReportsState, theme: &Theme) {
    let mut y = area.y;
    let mut visible_row = 0;
    let max_y = area.y + area.height;

    for (client_idx, report) in state.reports.iter().enumerate() {
        if y >= max_y {
            break;
        }

        let is_selected = visible_row == state.selected_row;
        let is_expanded = state.is_client_expanded(client_idx);

        let client_color = Theme::parse_hex(&report.color).unwrap_or(theme.accent);

        // Client row
        let expand_icon = if is_expanded { "▼" } else { "▶" };
        let bg = if is_selected {
            theme.highlight_bg
        } else {
            theme.bg
        };

        let bar_width = percentage_bar_width(report.percentage, area.width.saturating_sub(50));

        let client_line = Line::from(vec![
            Span::styled(format!("{expand_icon} "), Style::default().fg(theme.accent)),
            Span::styled("● ", Style::default().fg(client_color)),
            Span::styled(
                &report.name,
                Style::default()
                    .fg(theme.fg)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "  {}  ${:.2}  {:.0}%  ",
                    format_duration(report.total_minutes),
                    report.billable_amount,
                    report.percentage
                ),
                Style::default().fg(theme.muted).bg(bg),
            ),
            Span::styled(
                "█".repeat(bar_width as usize),
                Style::default().fg(client_color),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(client_line),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
        visible_row += 1;

        if !is_expanded {
            continue;
        }

        // Project rows
        for (project_idx, proj) in report.project_breakdown.iter().enumerate() {
            if y >= max_y {
                break;
            }

            let is_sel = visible_row == state.selected_row;
            let is_proj_expanded = state.is_project_expanded(client_idx, project_idx);
            let proj_bg = if is_sel { theme.highlight_bg } else { theme.bg };
            let proj_color = Theme::parse_hex(&proj.color).unwrap_or(client_color);
            let proj_bar = percentage_bar_width(proj.percentage, area.width.saturating_sub(55));
            let proj_expand_icon = if is_proj_expanded { "▼" } else { "▶" };

            let proj_line = Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    format!("{proj_expand_icon} "),
                    Style::default().fg(theme.accent),
                ),
                Span::styled("● ", Style::default().fg(proj_color)),
                Span::styled(&proj.name, Style::default().fg(theme.fg).bg(proj_bg)),
                Span::styled(
                    format!(
                        "  {}  ${:.2}  {:.0}%  ",
                        format_duration(proj.total_minutes),
                        proj.billable_amount,
                        proj.percentage
                    ),
                    Style::default().fg(theme.muted).bg(proj_bg),
                ),
                Span::styled(
                    "▓".repeat(proj_bar as usize),
                    Style::default().fg(proj_color),
                ),
            ]);
            frame.render_widget(
                Paragraph::new(proj_line),
                Rect::new(area.x, y, area.width, 1),
            );
            y += 1;
            visible_row += 1;

            if !is_proj_expanded {
                continue;
            }

            // Activity rows
            for act in &proj.activity_breakdown {
                if y >= max_y {
                    break;
                }

                let is_sel = visible_row == state.selected_row;
                let act_bg = if is_sel { theme.highlight_bg } else { theme.bg };
                let act_color = Theme::parse_hex(&act.color).unwrap_or(proj_color);
                let act_bar = percentage_bar_width(act.percentage, area.width.saturating_sub(60));

                let act_line = Line::from(vec![
                    Span::styled("        ", Style::default()),
                    Span::styled("● ", Style::default().fg(act_color)),
                    Span::styled(&act.name, Style::default().fg(theme.fg).bg(act_bg)),
                    Span::styled(
                        format!(
                            "  {}  {:.0}%  ",
                            format_duration(act.total_minutes),
                            act.percentage
                        ),
                        Style::default().fg(theme.muted).bg(act_bg),
                    ),
                    Span::styled("░".repeat(act_bar as usize), Style::default().fg(act_color)),
                ]);
                frame.render_widget(
                    Paragraph::new(act_line),
                    Rect::new(area.x, y, area.width, 1),
                );
                y += 1;
                visible_row += 1;
            }
        }
    }

    // Empty state
    if state.reports.is_empty() && area.height > 2 {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "  No entries for this period",
                Style::default().fg(theme.muted),
            )),
            Rect::new(area.x, area.y + 1, area.width, 1),
        );
    }
}

fn percentage_bar_width(percentage: f64, max_width: u16) -> u16 {
    ((percentage / 100.0) * max_width as f64).round() as u16
}
