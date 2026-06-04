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

    // For image files: the actual Kitty rendering happens in main.rs after frame draw.
    // Here we just render the block border. If Kitty is not supported, show a placeholder.
    if entry.is_image_file() {
        let kitty_supported = detect_kitty_support_cached();
        if kitty_supported {
            // Just render the empty block — main.rs writes the image into this area
            let paragraph = Paragraph::new("").block(block);
            frame.render_widget(paragraph, area);
        } else {
            let size_str = format_file_size(entry.size);
            let info = format!(
                "Image: {}\nSize: {}\n\n(Terminal does not support Kitty graphics protocol.\n Use Kitty, WezTerm, or another compatible terminal\n for image preview.)",
                entry.name, size_str
            );
            let paragraph = Paragraph::new(info)
                .block(block)
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(paragraph, area);
        }
        return;
    }

    // Non-image files: use preview manager
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
        Ok(PreviewContent::KittyImage { .. }) => {
            // Shouldn't reach here for non-image files, but handle gracefully
            let paragraph = Paragraph::new("").block(block);
            frame.render_widget(paragraph, area);
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

fn detect_kitty_support_cached() -> bool {
    if let Ok(term) = std::env::var("TERM_PROGRAM") {
        let t = term.to_lowercase();
        if t.contains("kitty") || t.contains("wezterm") {
            return true;
        }
    }
    if let Ok(term) = std::env::var("TERM") {
        if term.contains("kitty") {
            return true;
        }
    }
    std::env::var("KITTY_WINDOW_ID").is_ok()
}

fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
