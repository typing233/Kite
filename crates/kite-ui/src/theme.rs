use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub fg: Color,
    pub bg: Color,
    pub cursor_bg: Color,
    pub selection_bg: Color,
    pub directory_fg: Color,
    pub symlink_fg: Color,
    pub executable_fg: Color,
    pub hidden_fg: Color,
    pub border_fg: Color,
    pub status_fg: Color,
    pub status_bg: Color,
    pub error_fg: Color,
    pub warning_fg: Color,
    pub info_fg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            fg: Color::White,
            bg: Color::Reset,
            cursor_bg: Color::Rgb(68, 68, 120),
            selection_bg: Color::Rgb(51, 70, 124),
            directory_fg: Color::Rgb(122, 162, 247),
            symlink_fg: Color::Rgb(115, 218, 202),
            executable_fg: Color::Rgb(158, 206, 106),
            hidden_fg: Color::DarkGray,
            border_fg: Color::Rgb(86, 95, 137),
            status_fg: Color::White,
            status_bg: Color::Rgb(36, 40, 59),
            error_fg: Color::Rgb(247, 118, 142),
            warning_fg: Color::Rgb(224, 175, 104),
            info_fg: Color::Rgb(125, 207, 255),
        }
    }
}

impl Theme {
    pub fn cursor_style(&self) -> Style {
        Style::default().bg(self.cursor_bg)
    }

    pub fn selected_style(&self) -> Style {
        Style::default().bg(self.selection_bg).add_modifier(Modifier::BOLD)
    }

    pub fn directory_style(&self) -> Style {
        Style::default().fg(self.directory_fg).add_modifier(Modifier::BOLD)
    }

    pub fn symlink_style(&self) -> Style {
        Style::default().fg(self.symlink_fg)
    }

    pub fn executable_style(&self) -> Style {
        Style::default().fg(self.executable_fg)
    }

    pub fn hidden_style(&self) -> Style {
        Style::default().fg(self.hidden_fg)
    }

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border_fg)
    }

    pub fn status_style(&self) -> Style {
        Style::default().fg(self.status_fg).bg(self.status_bg)
    }
}
