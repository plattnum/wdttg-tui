use chrono::{Datelike, Local, Timelike};
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use wdttg_core::config::AppConfig;
use wdttg_core::model::TimeEntry;
use wdttg_core::time_utils::format_duration;

use crate::theme::Theme;

use super::state::TimelineState;

const SLOTS_PER_DAY: i32 = 24 * 4;

pub fn render_timeline(
    frame: &mut Frame,
    area: Rect,
    state: &mut TimelineState,
    config: &AppConfig,
    theme: &Theme,
) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" Timeline ", theme.title_style()));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    render_day_header(frame, chunks[0], state, theme);

    let visible_slots = chunks[1].height as i32;
    state.viewport_slots = visible_slots;
    state.preload_visible(visible_slots);

    render_scrollable_timeline(frame, chunks[1], state, config, theme);
    render_info_bar(frame, chunks[2], state, config, theme);
}

fn render_day_header(frame: &mut Frame, area: Rect, state: &TimelineState, theme: &Theme) {
    let date = state.cursor_date;
    let today = Local::now().date_naive();
    let is_today = date == today;

    let date_str = format!(
        "{}, {} {}, {}",
        date.weekday(),
        date.format("%b"),
        date.day(),
        date.year()
    );

    let date_style = if is_today {
        Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)
    };

    let header = Line::from(vec![
        Span::styled("∞ ", Style::default().fg(theme.title)),
        Span::styled(date_str, date_style),
        Span::styled(
            if is_today { "  (today)" } else { "" },
            Style::default().fg(theme.muted),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(header),
        Rect::new(area.x, area.y, area.width, 1),
    );

    let hints = Line::from(vec![
        Span::styled("← → ", Style::default().fg(theme.accent)),
        Span::styled("day  ", Style::default().fg(theme.muted)),
        Span::styled("H L ", Style::default().fg(theme.accent)),
        Span::styled("week  ", Style::default().fg(theme.muted)),
        Span::styled("t ", Style::default().fg(theme.accent)),
        Span::styled("today  ", Style::default().fg(theme.muted)),
        Span::styled("n ", Style::default().fg(theme.accent)),
        Span::styled("new  ", Style::default().fg(theme.muted)),
        Span::styled("Space ", Style::default().fg(theme.accent)),
        Span::styled("mark  ", Style::default().fg(theme.muted)),
        Span::styled("j/k ", Style::default().fg(theme.accent)),
        Span::styled("scroll", Style::default().fg(theme.muted)),
    ]);
    frame.render_widget(
        Paragraph::new(hints),
        Rect::new(area.x, area.y + 1, area.width, 1),
    );
}

fn render_scrollable_timeline(
    frame: &mut Frame,
    area: Rect,
    state: &TimelineState,
    config: &AppConfig,
    theme: &Theme,
) {
    let gutter_width: u16 = 7;
    let content_x = area.x + gutter_width;
    let content_width = area.width.saturating_sub(gutter_width + 1);
    let visible_rows = area.height as i32;

    let cursor_slot = state.cursor_to_slot();
    let mark_range = state.mark_range_slots();

    for row in 0..visible_rows {
        let global_slot = state.scroll_offset + row;
        let (date, hour, minute) = state.slot_to_datetime(global_slot);
        let y = area.y + row as u16;

        // Day separator at midnight
        if hour == 0 && minute == 0 {
            let day_label = format!(
                "── {} {} {}, {} ",
                date.weekday(),
                date.format("%b"),
                date.day(),
                date.year()
            );
            let is_today = date == Local::now().date_naive();
            let sep_style = if is_today {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.accent_dim)
            };
            let padded = format!("{:─<width$}", day_label, width = area.width as usize);
            frame.render_widget(
                Paragraph::new(Span::styled(padded, sep_style)),
                Rect::new(area.x, y, area.width, 1),
            );
            continue;
        }

        // Hour label in gutter (only on :00)
        if minute == 0 {
            let label = format!("{hour:02}:00 ");
            let is_now = date == Local::now().date_naive() && hour == Local::now().hour();
            let gutter_style = if is_now {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.muted)
            };
            frame.render_widget(
                Paragraph::new(Span::styled(label, gutter_style)),
                Rect::new(area.x, y, gutter_width, 1),
            );
        } else if minute == 30 {
            frame.render_widget(
                Paragraph::new(Span::styled("  ·   ", Style::default().fg(theme.border))),
                Rect::new(area.x, y, gutter_width, 1),
            );
        }

        // Mark range highlight
        let in_mark_range =
            mark_range.is_some_and(|(start, end)| global_slot >= start && global_slot <= end);
        if in_mark_range {
            let is_cursor_slot = global_slot == cursor_slot;
            let mark_char = if is_cursor_slot { "▸" } else { "▎" };
            let fill = "░".repeat(content_width.saturating_sub(1) as usize);
            let mark_line = format!("{mark_char}{fill}");
            frame.render_widget(
                Paragraph::new(Span::styled(
                    mark_line,
                    Style::default()
                        .fg(theme.warning)
                        .bg(dim_color(theme.warning, 15)),
                )),
                Rect::new(content_x, y, content_width, 1),
            );
        }
    }

    // Render entry cards on top of the grid
    // Collect all dates visible in this scroll window
    let start_slot = state.scroll_offset;
    let end_slot = state.scroll_offset + visible_rows;
    let start_day = start_slot.div_euclid(SLOTS_PER_DAY) - 1;
    let end_day = end_slot.div_euclid(SLOTS_PER_DAY) + 1;

    for day_off in start_day..=end_day {
        let date = if day_off >= 0 {
            state
                .center_date
                .checked_add_days(chrono::Days::new(day_off as u64))
        } else {
            state
                .center_date
                .checked_sub_days(chrono::Days::new((-day_off) as u64))
        };
        let Some(date) = date else { continue };

        let entries = state.day_entries.get(&date).cloned().unwrap_or_default();
        let day_base_slot = (date - state.center_date).num_days() as i32 * SLOTS_PER_DAY;

        for (idx, entry) in entries.iter().enumerate() {
            let is_selected = date == state.cursor_date && state.selected_entry == Some(idx);
            render_entry_card(
                frame,
                area,
                content_x,
                content_width,
                entry,
                is_selected,
                config,
                day_base_slot,
                state.scroll_offset,
                visible_rows,
                theme,
            );
        }
    }

    // Cursor line — rendered after entry cards so it's always visible,
    // even when the cursor is inside an entry block.
    if mark_range.is_none() {
        let cursor_row = cursor_slot - state.scroll_offset;
        if cursor_row >= 0 && cursor_row < visible_rows {
            let y = area.y + cursor_row as u16;
            let cursor_line = format!("▸{}", "─".repeat(content_width.saturating_sub(1) as usize));
            frame.render_widget(
                Paragraph::new(Span::styled(cursor_line, Style::default().fg(theme.accent))),
                Rect::new(content_x, y, content_width, 1),
            );
        }
    }

    // Scrollbar (cosmetic, infinite so just show position indicator)
    let total_display = SLOTS_PER_DAY * 7; // show ~7 days worth for scrollbar
    let scroll_pos = (state.scroll_offset % total_display).unsigned_abs() as usize;
    let mut scrollbar_state = ScrollbarState::default()
        .content_length(total_display as usize)
        .viewport_content_length(visible_rows as usize)
        .position(scroll_pos);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(theme.border)),
        area,
        &mut scrollbar_state,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_entry_card(
    frame: &mut Frame,
    area: Rect,
    content_x: u16,
    content_width: u16,
    entry: &TimeEntry,
    is_selected: bool,
    config: &AppConfig,
    day_base_slot: i32,
    scroll_offset: i32,
    visible_rows: i32,
    theme: &Theme,
) {
    let start_hour = entry.start.time().hour();
    let start_min = entry.start.time().minute();
    let end_hour = entry.end.time().hour();
    let end_min = entry.end.time().minute();

    let entry_start_slot = day_base_slot + (start_hour as i32) * 4 + (start_min as i32 / 15);
    let entry_end_slot = day_base_slot + (end_hour as i32) * 4 + ((end_min as i32 + 14) / 15);

    // Check visibility
    if entry_end_slot <= scroll_offset || entry_start_slot >= scroll_offset + visible_rows {
        return;
    }

    let vis_start = (entry_start_slot - scroll_offset).max(0) as u16;
    let vis_end = (entry_end_slot - scroll_offset).min(visible_rows) as u16;
    let vis_height = vis_end.saturating_sub(vis_start).max(1);

    let card_y = area.y + vis_start;
    let card_area = Rect::new(content_x, card_y, content_width, vis_height);

    let client_color = find_client_color(config, &entry.client)
        .and_then(|c| Theme::parse_hex(&c))
        .unwrap_or(theme.accent);

    let bg = if is_selected {
        theme.highlight_bg
    } else {
        dim_color(client_color, 25)
    };

    let border_style = Style::default().fg(client_color);
    let fg = if is_selected { theme.accent } else { theme.fg };

    let time_str = format!(
        "{:02}:{:02} – {:02}:{:02}",
        start_hour, start_min, end_hour, end_min
    );
    let dur_str = format_duration(entry.duration_minutes());

    // Line 1: time + duration badge
    let line1 = vec![
        Span::styled("▎ ", border_style),
        Span::styled(&time_str, Style::default().fg(fg).bg(bg)),
        Span::styled(
            format!(" {dur_str} "),
            Style::default()
                .fg(theme.bg)
                .bg(client_color)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    // Build tag line
    let mut tag_spans = vec![Span::styled("▎ ", border_style)];
    let mut tags: Vec<(&str, Color)> = vec![(&entry.client, client_color)];
    if let Some(ref p) = entry.project {
        let c = find_tag_color(config, &entry.client, p).unwrap_or(theme.accent_dim);
        tags.push((p, c));
    }
    if let Some(ref a) = entry.activity {
        let c = find_tag_color(config, &entry.client, a).unwrap_or(theme.accent_dim);
        tags.push((a, c));
    }
    for (i, (tag, color)) in tags.iter().enumerate() {
        if i > 0 {
            tag_spans.push(Span::styled(" ", Style::default().bg(bg)));
        }
        tag_spans.push(Span::styled(
            format!(" {} ", tag.to_uppercase()),
            Style::default()
                .fg(Color::White)
                .bg(*color)
                .add_modifier(Modifier::BOLD),
        ));
    }
    let tag_line = Line::from(tag_spans);

    let mut lines = vec![Line::from(line1)];

    // Line 2: description (if room for description + tags)
    if vis_height > 2 && !entry.description.is_empty() {
        let w = content_width.saturating_sub(3) as usize;
        let desc = if entry.description.len() > w {
            format!("{}…", &entry.description[..w.saturating_sub(1)])
        } else {
            entry.description.clone()
        };
        lines.push(Line::from(vec![
            Span::styled("▎ ", border_style),
            Span::styled(desc, Style::default().fg(fg).bg(bg)),
        ]));
    }

    // Fill middle with bg
    let pad = " ".repeat(content_width.saturating_sub(2) as usize);
    let tags_reserved = 1; // reserve last line for tags
    while lines.len() < vis_height.saturating_sub(tags_reserved) as usize {
        lines.push(Line::from(vec![
            Span::styled("▎", border_style),
            Span::styled(&pad, Style::default().bg(bg)),
        ]));
    }

    // Last line: tags
    lines.push(tag_line);

    frame.render_widget(Paragraph::new(lines), card_area);
}

fn render_info_bar(
    frame: &mut Frame,
    area: Rect,
    state: &TimelineState,
    _config: &AppConfig,
    theme: &Theme,
) {
    let entries = state
        .day_entries
        .get(&state.cursor_date)
        .cloned()
        .unwrap_or_default();
    let total_minutes: i64 = entries.iter().map(|e| e.duration_minutes()).sum();
    let total_str = format_duration(total_minutes);
    let count = entries.len();

    let info = if let Some(mark_dt) = state.mark_start {
        Line::from(vec![
            Span::styled(
                format!(
                    " ● MARKING from {:02}:{:02} ",
                    mark_dt.time().hour(),
                    mark_dt.time().minute()
                ),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("→ {:02}:{:02} ", state.cursor_hour, state.cursor_minute),
                Style::default().fg(theme.fg),
            ),
            Span::styled(
                "│ Space confirm │ Esc cancel",
                Style::default().fg(theme.muted),
            ),
        ])
    } else if let Some(entry) = state.selected_time_entry() {
        let dur = format_duration(entry.duration_minutes());
        Line::from(vec![
            Span::styled(
                format!(" ● {} ", entry.description),
                Style::default().fg(theme.fg),
            ),
            Span::styled(
                format!("│ {} │ {} ", entry.client, dur),
                Style::default().fg(theme.muted),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!(" {} entries ", count),
                Style::default().fg(theme.accent_dim),
            ),
            Span::styled(
                format!("│ {total_str} total "),
                Style::default().fg(theme.muted),
            ),
            Span::styled("│ ", Style::default().fg(theme.border)),
            Span::styled(
                format!("cursor {:02}:{:02}", state.cursor_hour, state.cursor_minute),
                Style::default().fg(theme.muted),
            ),
        ])
    };
    frame.render_widget(Paragraph::new(info), area);
}

fn find_client_color(config: &AppConfig, client_id: &str) -> Option<String> {
    config
        .clients
        .iter()
        .find(|c| c.id == client_id)
        .map(|c| c.color.clone())
}

fn find_tag_color(config: &AppConfig, client_id: &str, tag: &str) -> Option<Color> {
    let client = config.clients.iter().find(|c| c.id == client_id)?;
    if let Some(p) = client.projects.iter().find(|p| p.id == tag) {
        return Theme::parse_hex(&p.color);
    }
    if let Some(a) = client.activities.iter().find(|a| a.id == tag) {
        return Theme::parse_hex(&a.color);
    }
    None
}

fn dim_color(color: Color, percent: u8) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as u16 * percent as u16 / 100) as u8,
            (g as u16 * percent as u16 / 100) as u8,
            (b as u16 * percent as u16 / 100) as u8,
        ),
        _ => Color::Rgb(30, 30, 40),
    }
}
