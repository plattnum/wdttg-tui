use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub accent_dim: Color,
    pub error: Color,
    pub success: Color,
    pub warning: Color,
    pub muted: Color,
    pub highlight_bg: Color,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
    pub tab_active: Color,
    pub tab_inactive: Color,
    pub border: Color,
    pub title: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Rgb(22, 22, 30),
            fg: Color::Rgb(205, 214, 244),
            accent: Color::Rgb(137, 180, 250),
            accent_dim: Color::Rgb(88, 91, 112),
            error: Color::Rgb(243, 139, 168),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            muted: Color::Rgb(108, 112, 134),
            highlight_bg: Color::Rgb(49, 50, 68),
            status_bar_bg: Color::Rgb(30, 30, 46),
            status_bar_fg: Color::Rgb(166, 173, 200),
            tab_active: Color::Rgb(137, 180, 250),
            tab_inactive: Color::Rgb(108, 112, 134),
            border: Color::Rgb(69, 71, 90),
            title: Color::Rgb(203, 166, 247),
        }
    }
}

impl Theme {
    pub fn style(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }

    pub fn title_style(&self) -> Style {
        Style::default().fg(self.title).add_modifier(Modifier::BOLD)
    }

    pub fn status_bar_style(&self) -> Style {
        Style::default()
            .fg(self.status_bar_fg)
            .bg(self.status_bar_bg)
    }

    /// Parse a hex color string like "#FF6B6B" into a ratatui Color.
    pub fn parse_hex(hex: &str) -> Option<Color> {
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color::Rgb(r, g, b))
    }
}
