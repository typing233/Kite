use crate::app::App;
use crate::theme::Theme;
use kite_core::entry::EntryKind;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

pub fn render_file_list(frame: &mut Frame, area: Rect, app: &App) {
    let theme = Theme::default();
    let visible = app.visible_entries();
    let viewport_height = area.height.saturating_sub(2) as usize;

    // Determine scroll offset
    let scroll_offset = if app.cursor_index >= app.scroll_offset + viewport_height {
        app.cursor_index - viewport_height + 1
    } else if app.cursor_index < app.scroll_offset {
        app.cursor_index
    } else {
        app.scroll_offset
    };

    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(viewport_height)
        .map(|(i, entry)| {
            let real_idx = app.filtered_indices[i];
            let is_cursor = i == app.cursor_index;
            let is_selected = app.selected.contains(&real_idx);

            let icon = entry.icon();
            let name = &entry.name;

            // Build display line
            let size_str = if entry.kind == EntryKind::Directory {
                "    <DIR>".to_string()
            } else {
                format_size(entry.size)
            };

            let entry_style = if is_selected && is_cursor {
                theme.selected_style().patch(theme.cursor_style())
            } else if is_cursor {
                theme.cursor_style()
            } else if is_selected {
                theme.selected_style()
            } else {
                entry_base_style(entry, &theme)
            };

            let mark = if is_selected { "*" } else { " " };

            let spans = vec![
                Span::styled(format!("{} ", mark), entry_style),
                Span::styled(format!("{} ", icon), entry_style),
                Span::styled(name.to_string(), entry_style.patch(entry_base_style(entry, &theme))),
                Span::styled(format!(" {}", size_str), Style::default().fg(Color::DarkGray)),
            ];

            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = format!(
        " {} ({}) ",
        app.current_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string()),
        app.filtered_indices.len()
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(title);

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn entry_base_style(entry: &kite_core::entry::FileEntry, theme: &Theme) -> Style {
    if entry.is_hidden {
        theme.hidden_style()
    } else {
        match entry.kind {
            EntryKind::Directory => theme.directory_style(),
            EntryKind::Symlink => theme.symlink_style(),
            EntryKind::File if entry.permissions.user.execute => theme.executable_style(),
            _ => Style::default().fg(theme.fg),
        }
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:>6.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:>6.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:>6.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{:>7}", bytes)
    }
}
