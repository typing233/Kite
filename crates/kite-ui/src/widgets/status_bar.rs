use crate::app::{App, AppMode, ConfirmAction, InputContext, StatusLevel};
use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let theme = Theme::default();

    let content = match &app.mode {
        AppMode::Normal | AppMode::Visual => build_normal_status(app, &theme),
        AppMode::Search => build_input_status("Search: ", &app.input_buffer, &theme),
        AppMode::Filter => build_input_status("Filter: ", &app.input_buffer, &theme),
        AppMode::Command => build_input_status(":", &app.input_buffer, &theme),
        AppMode::Input(ctx) => {
            let prefix = match ctx {
                InputContext::Rename { .. } => "Rename: ",
                InputContext::CreateFile => "New file: ",
                InputContext::CreateDir => "New dir: ",
                _ => "> ",
            };
            build_input_status(prefix, &app.input_buffer, &theme)
        }
        AppMode::Confirm(action) => {
            let msg = match action {
                ConfirmAction::Delete(paths) => {
                    format!("Delete {} item(s)? [Enter/Esc]", paths.len())
                }
            };
            Line::from(Span::styled(msg, Style::default().fg(theme.warning_fg)))
        }
    };

    // Check for status message override
    let line = if let Some(ref msg) = app.status_message {
        let color = match msg.level {
            StatusLevel::Info => theme.info_fg,
            StatusLevel::Warning => theme.warning_fg,
            StatusLevel::Error => theme.error_fg,
        };
        Line::from(Span::styled(&msg.text, Style::default().fg(color)))
    } else {
        content
    };

    let paragraph = Paragraph::new(line).style(theme.status_style());
    frame.render_widget(paragraph, area);
}

fn build_normal_status<'a>(app: &App, theme: &Theme) -> Line<'a> {
    let mode_str = match app.mode {
        AppMode::Visual => " VISUAL ",
        _ => " NORMAL ",
    };
    let mode_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Rgb(122, 162, 247))
        .add_modifier(Modifier::BOLD);

    let path = app.current_dir.to_string_lossy().to_string();
    let sort_info = format!(
        " [{}{}] ",
        app.sort_field.label(),
        if app.sort_ascending { "↑" } else { "↓" }
    );

    let selection_info = if !app.selected.is_empty() {
        format!(" {} selected ", app.selected.len())
    } else {
        String::new()
    };

    let position = if !app.filtered_indices.is_empty() {
        format!(" {}/{} ", app.cursor_index + 1, app.filtered_indices.len())
    } else {
        " empty ".to_string()
    };

    Line::from(vec![
        Span::styled(mode_str, mode_style),
        Span::styled(format!(" {} ", path), Style::default().fg(theme.fg)),
        Span::styled(sort_info, Style::default().fg(Color::DarkGray)),
        Span::styled(selection_info, Style::default().fg(Color::Yellow)),
        Span::styled(position, Style::default().fg(Color::DarkGray)),
    ])
}

fn build_input_status<'a>(prefix: &str, buffer: &str, theme: &Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            prefix.to_string(),
            Style::default().fg(theme.info_fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            buffer.to_string(),
            Style::default().fg(theme.fg),
        ),
        Span::styled("█", Style::default().fg(theme.fg)),
    ])
}
