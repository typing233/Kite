use crate::app::App;
use crate::widgets::{file_list, preview, status_bar, tree};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // Main layout: content area + status bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(size);

    let content_area = main_chunks[0];
    let status_area = main_chunks[1];

    // Content panes
    let pane_constraints = if app.show_preview {
        let [l, c, r] = app.pane_ratios;
        let total = l + c + r;
        vec![
            Constraint::Ratio(l as u32, total as u32),
            Constraint::Ratio(c as u32, total as u32),
            Constraint::Ratio(r as u32, total as u32),
        ]
    } else {
        let [l, c, _] = app.pane_ratios;
        let total = l + c;
        vec![
            Constraint::Ratio(l as u32, total as u32),
            Constraint::Ratio(c as u32, total as u32),
        ]
    };

    let pane_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(pane_constraints)
        .split(content_area);

    // Left pane: parent directory
    tree::render_tree(frame, pane_chunks[0], app);

    // Center pane: current directory
    file_list::render_file_list(frame, pane_chunks[1], app);

    // Right pane: preview (if enabled)
    if app.show_preview && pane_chunks.len() > 2 {
        preview::render_preview(frame, pane_chunks[2], app);
    }

    // Status bar
    status_bar::render_status_bar(frame, status_area, app);
}
