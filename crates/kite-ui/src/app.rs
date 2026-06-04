use kite_core::config::Config;
use kite_core::entry::{EntryKind, FileEntry};
use kite_core::event::{AppCommand, SortField};
use kite_core::keymap::{
    default_keymap, KeyContext, Keymap,
};
use kite_fs::ops;
use kite_fs::read::read_directory;
use kite_fs::sort::sort_entries;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Visual,
    Command,
    Search,
    Filter,
    Confirm(ConfirmAction),
    Input(InputContext),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    Delete(Vec<PathBuf>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputContext {
    Rename { original: String },
    CreateFile,
    CreateDir,
    Search,
    Filter,
    Command,
}

pub struct App {
    // Navigation
    pub current_dir: PathBuf,
    pub parent_entries: Vec<FileEntry>,
    pub entries: Vec<FileEntry>,
    pub filtered_indices: Vec<usize>,
    pub cursor_index: usize,
    pub scroll_offset: usize,

    // Selection
    pub selected: HashSet<usize>,
    pub visual_anchor: Option<usize>,
    pub clipboard: Option<Clipboard>,

    // Display
    pub show_hidden: bool,
    pub sort_field: SortField,
    pub sort_ascending: bool,
    pub filter: Option<String>,
    pub search_query: Option<String>,

    // Mode
    pub mode: AppMode,
    pub input_buffer: String,

    // Layout
    pub show_preview: bool,
    pub pane_ratios: [u16; 3],

    // Status
    pub status_message: Option<StatusMessage>,

    // Directory history
    pub history: Vec<PathBuf>,
    pub history_index: usize,

    // Keymap
    pub keymap: Keymap,

    // Config
    pub config: Config,

    // Should quit flag
    pub should_quit: bool,
}

#[derive(Debug, Clone)]
pub struct Clipboard {
    pub operation: ClipboardOp,
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
pub enum ClipboardOp {
    Copy,
    Cut,
}

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub level: StatusLevel,
    pub expires: Instant,
}

#[derive(Debug, Clone, Copy)]
pub enum StatusLevel {
    Info,
    Warning,
    Error,
}

impl App {
    pub async fn new(config: Config) -> std::io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let mut keymap = default_keymap();

        // Merge user keybindings
        let user_bindings = config.user_key_bindings();
        keymap.merge_user_config(user_bindings);

        let show_hidden = config.general.show_hidden;
        let sort_field = config.sort.sort_field();
        let sort_ascending = config.sort.ascending;
        let pane_ratios = config.ui.pane_ratios;
        let show_preview = config.preview.enabled;

        let mut app = Self {
            current_dir: current_dir.clone(),
            parent_entries: Vec::new(),
            entries: Vec::new(),
            filtered_indices: Vec::new(),
            cursor_index: 0,
            scroll_offset: 0,
            selected: HashSet::new(),
            visual_anchor: None,
            clipboard: None,
            show_hidden,
            sort_field,
            sort_ascending,
            filter: None,
            search_query: None,
            mode: AppMode::Normal,
            input_buffer: String::new(),
            show_preview,
            pane_ratios,
            status_message: None,
            history: vec![current_dir],
            history_index: 0,
            keymap,
            config,
            should_quit: false,
        };

        app.reload_directory().await?;
        Ok(app)
    }

    pub async fn reload_directory(&mut self) -> std::io::Result<()> {
        let mut entries = read_directory(&self.current_dir).await?;
        sort_entries(
            &mut entries,
            self.sort_field,
            self.sort_ascending,
            self.config.sort.dirs_first,
        );
        self.entries = entries;
        self.rebuild_filtered_indices();

        // Load parent
        if let Some(parent) = self.current_dir.parent() {
            if let Ok(mut parent_entries) = read_directory(parent).await {
                sort_entries(
                    &mut parent_entries,
                    SortField::Name,
                    true,
                    true,
                );
                self.parent_entries = parent_entries;
            }
        } else {
            self.parent_entries.clear();
        }

        Ok(())
    }

    fn rebuild_filtered_indices(&mut self) {
        self.filtered_indices = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                if !self.show_hidden && e.is_hidden {
                    return false;
                }
                if let Some(ref filter) = self.filter {
                    if !e.name.to_lowercase().contains(&filter.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .map(|(i, _)| i)
            .collect();

        // Clamp cursor
        if !self.filtered_indices.is_empty() {
            self.cursor_index = self.cursor_index.min(self.filtered_indices.len() - 1);
        } else {
            self.cursor_index = 0;
        }
    }

    pub fn visible_entries(&self) -> Vec<&FileEntry> {
        self.filtered_indices
            .iter()
            .map(|&i| &self.entries[i])
            .collect()
    }

    pub fn current_entry(&self) -> Option<&FileEntry> {
        self.filtered_indices
            .get(self.cursor_index)
            .map(|&i| &self.entries[i])
    }

    pub fn current_entry_real_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.cursor_index).copied()
    }

    pub fn key_context(&self) -> KeyContext {
        match self.mode {
            AppMode::Normal => KeyContext::Normal,
            AppMode::Visual => KeyContext::Visual,
            AppMode::Search | AppMode::Filter => KeyContext::Search,
            AppMode::Command => KeyContext::Command,
            _ => KeyContext::Normal,
        }
    }

    pub async fn execute_command(&mut self, cmd: AppCommand) -> std::io::Result<bool> {
        match cmd {
            AppCommand::CursorUp => self.cursor_up(),
            AppCommand::CursorDown => self.cursor_down(),
            AppCommand::CursorTop => self.cursor_top(),
            AppCommand::CursorBottom => self.cursor_bottom(),
            AppCommand::PageUp => self.page_up(),
            AppCommand::PageDown => self.page_down(),
            AppCommand::EnterDirectory => self.enter_directory().await?,
            AppCommand::ParentDirectory => self.parent_directory().await?,
            AppCommand::GoBack => self.go_back().await?,
            AppCommand::GoForward => self.go_forward().await?,
            AppCommand::ToggleSelect => self.toggle_select(),
            AppCommand::SelectAll => self.select_all(),
            AppCommand::ClearSelection => self.clear_selection(),
            AppCommand::VisualMode => self.enter_visual_mode(),
            AppCommand::NormalMode => self.enter_normal_mode(),
            AppCommand::Copy => self.copy_selected(),
            AppCommand::Cut => self.cut_selected(),
            AppCommand::Paste => self.paste().await?,
            AppCommand::Delete => self.delete_selected().await?,
            AppCommand::Rename => self.start_rename(),
            AppCommand::CreateFile => self.start_create_file(),
            AppCommand::CreateDir => self.start_create_dir(),
            AppCommand::SortNext => {
                self.sort_field = self.sort_field.next();
                self.resort();
            }
            AppCommand::SortBy(field) => {
                self.sort_field = field;
                self.resort();
            }
            AppCommand::ToggleSortOrder => {
                self.sort_ascending = !self.sort_ascending;
                self.resort();
            }
            AppCommand::TogglePreview => self.show_preview = !self.show_preview,
            AppCommand::ToggleHidden => {
                self.show_hidden = !self.show_hidden;
                self.rebuild_filtered_indices();
            }
            AppCommand::Refresh => self.reload_directory().await?,
            AppCommand::SearchMode => {
                self.mode = AppMode::Search;
                self.input_buffer.clear();
            }
            AppCommand::FilterMode => {
                self.mode = AppMode::Filter;
                self.input_buffer.clear();
            }
            AppCommand::SearchNext => self.search_next(),
            AppCommand::SearchPrev => self.search_prev(),
            AppCommand::ConfirmInput => self.confirm_input().await?,
            AppCommand::CancelInput => self.cancel_input(),
            AppCommand::Quit => {
                self.should_quit = true;
            }
            AppCommand::ForceQuit => {
                self.should_quit = true;
            }
            AppCommand::CommandMode => {
                self.mode = AppMode::Command;
                self.input_buffer.clear();
            }
            AppCommand::PluginCommand { .. } => {}
        }
        Ok(self.should_quit)
    }

    fn cursor_up(&mut self) {
        if self.cursor_index > 0 {
            self.cursor_index -= 1;
            if self.mode == AppMode::Visual {
                self.update_visual_selection();
            }
        }
    }

    fn cursor_down(&mut self) {
        if self.cursor_index + 1 < self.filtered_indices.len() {
            self.cursor_index += 1;
            if self.mode == AppMode::Visual {
                self.update_visual_selection();
            }
        }
    }

    fn cursor_top(&mut self) {
        self.cursor_index = 0;
        self.scroll_offset = 0;
    }

    fn cursor_bottom(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.cursor_index = self.filtered_indices.len() - 1;
        }
    }

    fn page_up(&mut self) {
        self.cursor_index = self.cursor_index.saturating_sub(20);
    }

    fn page_down(&mut self) {
        let max = self.filtered_indices.len().saturating_sub(1);
        self.cursor_index = (self.cursor_index + 20).min(max);
    }

    async fn enter_directory(&mut self) -> std::io::Result<()> {
        if let Some(entry) = self.current_entry().cloned() {
            if entry.kind == EntryKind::Directory {
                self.current_dir = entry.path;
                self.cursor_index = 0;
                self.scroll_offset = 0;
                self.selected.clear();
                self.history.truncate(self.history_index + 1);
                self.history.push(self.current_dir.clone());
                self.history_index = self.history.len() - 1;
                self.reload_directory().await?;
            }
        }
        Ok(())
    }

    async fn parent_directory(&mut self) -> std::io::Result<()> {
        if let Some(parent) = self.current_dir.parent().map(|p| p.to_path_buf()) {
            let old_name = self
                .current_dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string());
            self.current_dir = parent;
            self.cursor_index = 0;
            self.scroll_offset = 0;
            self.selected.clear();
            self.history.truncate(self.history_index + 1);
            self.history.push(self.current_dir.clone());
            self.history_index = self.history.len() - 1;
            self.reload_directory().await?;

            // Try to position cursor on the directory we came from
            if let Some(name) = old_name {
                let visible = self.visible_entries();
                if let Some(pos) = visible.iter().position(|e| e.name == name) {
                    self.cursor_index = pos;
                }
            }
        }
        Ok(())
    }

    async fn go_back(&mut self) -> std::io::Result<()> {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.current_dir = self.history[self.history_index].clone();
            self.cursor_index = 0;
            self.scroll_offset = 0;
            self.selected.clear();
            self.reload_directory().await?;
        }
        Ok(())
    }

    async fn go_forward(&mut self) -> std::io::Result<()> {
        if self.history_index + 1 < self.history.len() {
            self.history_index += 1;
            self.current_dir = self.history[self.history_index].clone();
            self.cursor_index = 0;
            self.scroll_offset = 0;
            self.selected.clear();
            self.reload_directory().await?;
        }
        Ok(())
    }

    fn toggle_select(&mut self) {
        if let Some(real_idx) = self.current_entry_real_index() {
            if self.selected.contains(&real_idx) {
                self.selected.remove(&real_idx);
            } else {
                self.selected.insert(real_idx);
            }
            self.cursor_down();
        }
    }

    fn select_all(&mut self) {
        self.selected = self.filtered_indices.iter().copied().collect();
    }

    fn clear_selection(&mut self) {
        self.selected.clear();
    }

    fn enter_visual_mode(&mut self) {
        self.mode = AppMode::Visual;
        self.visual_anchor = Some(self.cursor_index);
        self.selected.clear();
        if let Some(real_idx) = self.current_entry_real_index() {
            self.selected.insert(real_idx);
        }
    }

    fn enter_normal_mode(&mut self) {
        self.mode = AppMode::Normal;
        self.visual_anchor = None;
    }

    fn update_visual_selection(&mut self) {
        if let Some(anchor) = self.visual_anchor {
            self.selected.clear();
            let start = anchor.min(self.cursor_index);
            let end = anchor.max(self.cursor_index);
            for i in start..=end {
                if let Some(&real_idx) = self.filtered_indices.get(i) {
                    self.selected.insert(real_idx);
                }
            }
        }
    }

    fn copy_selected(&mut self) {
        let paths = self.get_selected_paths();
        if !paths.is_empty() {
            self.clipboard = Some(Clipboard {
                operation: ClipboardOp::Copy,
                paths,
            });
            self.set_status(
                format!("{} item(s) copied", self.selected.len()),
                StatusLevel::Info,
            );
            self.enter_normal_mode();
        }
    }

    fn cut_selected(&mut self) {
        let paths = self.get_selected_paths();
        if !paths.is_empty() {
            self.clipboard = Some(Clipboard {
                operation: ClipboardOp::Cut,
                paths,
            });
            self.set_status(
                format!("{} item(s) cut", self.selected.len()),
                StatusLevel::Info,
            );
            self.enter_normal_mode();
        }
    }

    async fn paste(&mut self) -> std::io::Result<()> {
        if let Some(clipboard) = self.clipboard.clone() {
            let dest = self.current_dir.clone();
            match clipboard.operation {
                ClipboardOp::Copy => {
                    let count = ops::copy_entries(&clipboard.paths, &dest, None).await?;
                    self.set_status(format!("{} item(s) pasted", count), StatusLevel::Info);
                }
                ClipboardOp::Cut => {
                    let count = ops::move_entries(&clipboard.paths, &dest, None).await?;
                    self.set_status(format!("{} item(s) moved", count), StatusLevel::Info);
                    self.clipboard = None;
                }
            }
            self.reload_directory().await?;
        }
        Ok(())
    }

    async fn delete_selected(&mut self) -> std::io::Result<()> {
        let paths = self.get_selected_paths();
        if paths.is_empty() {
            if let Some(entry) = self.current_entry() {
                let paths = vec![entry.path.clone()];
                if self.config.general.confirm_delete {
                    self.mode = AppMode::Confirm(ConfirmAction::Delete(paths));
                    return Ok(());
                }
                ops::delete_entries(&paths, None).await?;
                self.set_status("1 item deleted".to_string(), StatusLevel::Info);
                self.reload_directory().await?;
            }
        } else {
            if self.config.general.confirm_delete {
                self.mode = AppMode::Confirm(ConfirmAction::Delete(paths));
                return Ok(());
            }
            let count = ops::delete_entries(&paths, None).await?;
            self.set_status(format!("{} item(s) deleted", count), StatusLevel::Info);
            self.selected.clear();
            self.reload_directory().await?;
        }
        self.enter_normal_mode();
        Ok(())
    }

    fn start_rename(&mut self) {
        if let Some(entry) = self.current_entry() {
            let name = entry.name.clone();
            self.input_buffer = name.clone();
            self.mode = AppMode::Input(InputContext::Rename {
                original: name,
            });
        }
    }

    fn start_create_file(&mut self) {
        self.input_buffer.clear();
        self.mode = AppMode::Input(InputContext::CreateFile);
    }

    fn start_create_dir(&mut self) {
        self.input_buffer.clear();
        self.mode = AppMode::Input(InputContext::CreateDir);
    }

    async fn confirm_input(&mut self) -> std::io::Result<()> {
        match self.mode.clone() {
            AppMode::Input(InputContext::Rename { original }) => {
                let new_name = self.input_buffer.clone();
                if !new_name.is_empty() && new_name != original {
                    let from = self.current_dir.join(&original);
                    let to = self.current_dir.join(&new_name);
                    ops::rename_entry(&from, &to).await?;
                    self.set_status(format!("Renamed to {}", new_name), StatusLevel::Info);
                    self.reload_directory().await?;
                }
                self.mode = AppMode::Normal;
            }
            AppMode::Input(InputContext::CreateFile) => {
                let name = self.input_buffer.clone();
                if !name.is_empty() {
                    let path = self.current_dir.join(&name);
                    ops::create_file(&path).await?;
                    self.set_status(format!("Created {}", name), StatusLevel::Info);
                    self.reload_directory().await?;
                }
                self.mode = AppMode::Normal;
            }
            AppMode::Input(InputContext::CreateDir) => {
                let name = self.input_buffer.clone();
                if !name.is_empty() {
                    let path = self.current_dir.join(&name);
                    ops::create_directory(&path).await?;
                    self.set_status(format!("Created directory {}", name), StatusLevel::Info);
                    self.reload_directory().await?;
                }
                self.mode = AppMode::Normal;
            }
            AppMode::Search => {
                self.search_query = Some(self.input_buffer.clone());
                self.search_next();
                self.mode = AppMode::Normal;
            }
            AppMode::Filter => {
                self.filter = if self.input_buffer.is_empty() {
                    None
                } else {
                    Some(self.input_buffer.clone())
                };
                self.rebuild_filtered_indices();
                self.mode = AppMode::Normal;
            }
            AppMode::Confirm(ConfirmAction::Delete(paths)) => {
                let count = ops::delete_entries(&paths, None).await?;
                self.set_status(format!("{} item(s) deleted", count), StatusLevel::Info);
                self.selected.clear();
                self.reload_directory().await?;
                self.mode = AppMode::Normal;
            }
            _ => {
                self.mode = AppMode::Normal;
            }
        }
        self.input_buffer.clear();
        Ok(())
    }

    fn cancel_input(&mut self) {
        self.input_buffer.clear();
        self.mode = AppMode::Normal;
    }

    fn search_next(&mut self) {
        if let Some(ref query) = self.search_query {
            let query_lower = query.to_lowercase();
            let visible = self.visible_entries();
            let start = self.cursor_index + 1;
            for i in 0..visible.len() {
                let idx = (start + i) % visible.len();
                if visible[idx].name.to_lowercase().contains(&query_lower) {
                    self.cursor_index = idx;
                    return;
                }
            }
        }
    }

    fn search_prev(&mut self) {
        if let Some(ref query) = self.search_query {
            let query_lower = query.to_lowercase();
            let visible = self.visible_entries();
            let len = visible.len();
            if len == 0 {
                return;
            }
            for i in 1..=len {
                let idx = (self.cursor_index + len - i) % len;
                if visible[idx].name.to_lowercase().contains(&query_lower) {
                    self.cursor_index = idx;
                    return;
                }
            }
        }
    }

    fn resort(&mut self) {
        sort_entries(
            &mut self.entries,
            self.sort_field,
            self.sort_ascending,
            self.config.sort.dirs_first,
        );
        self.rebuild_filtered_indices();
    }

    fn get_selected_paths(&self) -> Vec<PathBuf> {
        self.selected
            .iter()
            .filter_map(|&i| self.entries.get(i).map(|e| e.path.clone()))
            .collect()
    }

    pub fn set_status(&mut self, text: String, level: StatusLevel) {
        self.status_message = Some(StatusMessage {
            text,
            level,
            expires: Instant::now() + Duration::from_secs(3),
        });
    }

    pub fn tick(&mut self) {
        if let Some(ref msg) = self.status_message {
            if Instant::now() > msg.expires {
                self.status_message = None;
            }
        }
    }

    pub fn handle_input_char(&mut self, ch: char) {
        self.input_buffer.push(ch);
    }

    pub fn handle_input_backspace(&mut self) {
        self.input_buffer.pop();
    }

    pub fn is_input_mode(&self) -> bool {
        matches!(
            self.mode,
            AppMode::Search | AppMode::Filter | AppMode::Command | AppMode::Input(_)
        )
    }

    pub fn ensure_cursor_visible(&mut self, viewport_height: usize) {
        if self.cursor_index < self.scroll_offset {
            self.scroll_offset = self.cursor_index;
        } else if self.cursor_index >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.cursor_index - viewport_height + 1;
        }
    }
}
