use crate::app::App;
use crate::theme::Theme;
use kite_preview::traits::PreviewContent;
use kite_preview::PreviewManager;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub fn render_preview(frame: &mut Frame, area: Rect, app: &App) {
    let theme = Theme::default();
    let inner_width = area.width.saturating_sub(2);
    let inner_height = area.height.saturating_sub(2);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(" Preview ");

    let entry = app.current_entry();
    if entry.is_none() {
        let paragraph = Paragraph::new("No file selected")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(paragraph, area);
        return;
    }

    let entry = entry.unwrap();
    let preview_manager = PreviewManager::new();

    match preview_manager.preview(&entry.path, inner_width, inner_height) {
        Ok(PreviewContent::StyledText(styled_lines)) => {
            let lines: Vec<Line> = styled_lines
                .iter()
                .map(|sl| {
                    let spans: Vec<Span> = sl
                        .spans
                        .iter()
                        .map(|s| Span::styled(s.text.clone(), s.style))
                        .collect();
                    Line::from(spans)
                })
                .collect();
            let paragraph = Paragraph::new(lines).block(block);
            frame.render_widget(paragraph, area);
        }
        Ok(PreviewContent::KittyImage { data: _, width, height, format: _ }) => {
            // For Kitty images, we render a placeholder in the widget
            // and write the escape sequence directly after rendering
            let info = format!(
                "Image: {}x{}\n(Kitty graphics protocol)",
                width, height
            );
            let paragraph = Paragraph::new(info)
                .block(block)
                .style(Style::default().fg(Color::Cyan));
            frame.render_widget(paragraph, area);

            // The actual image rendering happens via direct terminal writes
            // in the main loop after frame rendering
        }
        Ok(PreviewContent::Placeholder(msg)) => {
            let paragraph = Paragraph::new(msg)
                .block(block)
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(paragraph, area);
        }
        Ok(PreviewContent::AnsiText(text)) => {
            let paragraph = Paragraph::new(text)
                .block(block)
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
        }
        Err(e) => {
            let paragraph = Paragraph::new(format!("Preview error: {}", e))
                .block(block)
                .style(Style::default().fg(theme.error_fg));
            frame.render_widget(paragraph, area);
        }
    }
}
