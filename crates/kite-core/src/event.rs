use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Event {
    Terminal(crossterm::event::Event),
    Tick,
    FsChange(PathBuf),
    TaskComplete(TaskResult),
}

#[derive(Debug, Clone)]
pub enum AppCommand {
    // Navigation
    CursorUp,
    CursorDown,
    CursorTop,
    CursorBottom,
    PageUp,
    PageDown,
    EnterDirectory,
    ParentDirectory,
    GoBack,
    GoForward,

    // Selection
    ToggleSelect,
    SelectAll,
    ClearSelection,
    VisualMode,
    NormalMode,

    // File operations
    Copy,
    Cut,
    Paste,
    Delete,
    Rename,
    CreateFile,
    CreateDir,

    // Sorting
    SortBy(SortField),
    SortNext,
    ToggleSortOrder,

    // UI
    TogglePreview,
    ToggleHidden,
    Refresh,

    // Search/Filter
    SearchMode,
    FilterMode,
    SearchNext,
    SearchPrev,
    ConfirmInput,
    CancelInput,

    // Application
    Quit,
    ForceQuit,
    CommandMode,

    // Plugin
    PluginCommand {
        plugin: String,
        command: String,
        args: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Name,
    Size,
    Modified,
    Extension,
}

impl SortField {
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Size,
            Self::Size => Self::Modified,
            Self::Modified => Self::Extension,
            Self::Extension => Self::Name,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Size => "size",
            Self::Modified => "modified",
            Self::Extension => "extension",
        }
    }
}

#[derive(Debug, Clone)]
pub enum TaskResult {
    CopyComplete { count: usize },
    MoveComplete { count: usize },
    DeleteComplete { count: usize },
    Error(String),
}
