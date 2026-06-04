use crate::app::App;
use crate::theme::Theme;
use kite_core::entry::EntryKind;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

pub fn render_tree(frame: &mut Frame, area: Rect, app: &App) {
    let theme = Theme::default();
    let viewport_height = area.height.saturating_sub(2) as usize;

    // Show parent directory entries
    let items: Vec<ListItem> = app
        .parent_entries
        .iter()
        .filter(|e| app.show_hidden || !e.is_hidden)
        .take(viewport_height)
        .map(|entry| {
            let is_current = entry.kind == EntryKind::Directory
                && app
                    .current_dir
                    .file_name()
                    .map(|n| n.to_string_lossy() == entry.name)
                    .unwrap_or(false);

            let style = if is_current {
                theme.cursor_style().patch(theme.directory_style())
            } else if entry.kind == EntryKind::Directory {
                theme.directory_style()
            } else {
                Style::default().fg(theme.fg)
            };

            let icon = entry.icon();
            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", icon), style),
                Span::styled(&entry.name, style),
            ]))
        })
        .collect();

    let parent_name = app
        .current_dir
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "/".to_string());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(format!(" {} ", parent_name));

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
