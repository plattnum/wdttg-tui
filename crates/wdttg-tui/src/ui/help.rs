use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};

use crate::theme::Theme;

const HELP_TEXT: &str = "\
 ─── Global ───────────────────────────
  q / Ctrl+C    Quit application
  1             Timeline screen
  2             Reports screen
  3             Manage screen
  ?             Toggle this help
  Esc           Close popup / cancel

 ─── Timeline ─────────────────────────
  j / ↓         Scroll down (15 min)
  k / ↑         Scroll up (15 min)
  h / ←         Previous day
  l / →         Next day
  H             Previous week
  L             Next week
  t             Jump to today/now
  PgDn/Ctrl+D   Page down
  PgUp/Ctrl+U   Page up
  Mouse wheel   Scroll timeline
  n             New entry at cursor
  e             Edit selected entry
  d             Delete selected entry

 ─── Reports ──────────────────────────
  ← / →         Change time period
  j / ↓         Navigate rows
  k / ↑         Navigate rows
  Enter         Expand/collapse client

 ─── Manage ───────────────────────────
  Tab / ← →     Switch pane
  j / ↓         Select next item
  k / ↑         Select prev item
  n / a         Add new item
  e             Edit selected item
  d / A         Toggle archive

 ─── Entry Form ───────────────────────
  Tab           Next field
  Shift+Tab     Previous field
  ← →           Cycle dropdown / cursor
  ↑ ↓           Cycle dropdown
  Enter         Newline (in text fields)
  Ctrl+S        Save entry
  Esc           Cancel
  Mouse click   Focus field";

pub fn render_help(frame: &mut Frame, area: Rect, theme: &Theme, scroll: usize) {
    let width = 48.min(area.width.saturating_sub(4));
    let height = (area.height - 4).min(30);
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup_area);

    let lines: Vec<&str> = HELP_TEXT.lines().collect();
    let total_lines = lines.len();
    let visible = (height - 2) as usize; // minus borders

    let help = Paragraph::new(HELP_TEXT)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent))
                .title(Span::styled(
                    " Help (? to close) ",
                    Style::default()
                        .fg(theme.title)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .style(Style::default().fg(theme.fg).bg(theme.bg))
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));

    frame.render_widget(help, popup_area);

    // Scrollbar
    if total_lines > visible {
        let mut sb_state = ScrollbarState::default()
            .content_length(total_lines)
            .viewport_content_length(visible)
            .position(scroll);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .style(Style::default().fg(theme.border)),
            popup_area,
            &mut sb_state,
        );
    }
}

pub fn render_welcome(frame: &mut Frame, area: Rect, theme: &Theme) {
    let width = 50.min(area.width.saturating_sub(4));
    let height = 12.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup_area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Welcome to wdttg!",
            Style::default()
                .fg(theme.title)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Where Did The Time Go?",
            Style::default().fg(theme.accent),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Config: ~/.config/wdttg/config.toml",
            Style::default().fg(theme.muted),
        )),
        Line::from(Span::styled(
            "  Data:   ~/.local/share/wdttg/data/",
            Style::default().fg(theme.muted),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Press ", Style::default().fg(theme.fg)),
            Span::styled(
                " Enter ",
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to get started or ", Style::default().fg(theme.fg)),
            Span::styled(
                " ? ",
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" for help", Style::default().fg(theme.fg)),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(
            " ∞ wdttg ",
            Style::default()
                .fg(theme.title)
                .add_modifier(Modifier::BOLD),
        ));

    let welcome = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(theme.fg).bg(theme.bg));

    frame.render_widget(welcome, popup_area);
}
