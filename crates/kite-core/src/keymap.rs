use crate::event::AppCommand;
use crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCombo {
    pub code: SerializableKeyCode,
    #[serde(default)]
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SerializableKeyCode {
    Char(char),
    F(u8),
    Enter,
    Esc,
    Tab,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Space,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Modifiers {
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub shift: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyContext {
    Normal,
    Visual,
    Command,
    Search,
    Global,
}

impl Default for KeyContext {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    pub key: KeyCombo,
    pub command: String,
    #[serde(default)]
    pub context: KeyContext,
    #[serde(default)]
    pub description: String,
}

pub struct Keymap {
    bindings: Vec<KeyBinding>,
    sequence_buffer: Vec<KeyCombo>,
    sequence_bindings: HashMap<Vec<KeyCombo>, String>,
    last_key_time: Option<std::time::Instant>,
    sequence_timeout: std::time::Duration,
}

impl Keymap {
    pub fn new(bindings: Vec<KeyBinding>) -> Self {
        let sequence_bindings = HashMap::new();

        Self {
            bindings,
            sequence_buffer: Vec::new(),
            sequence_bindings,
            last_key_time: None,
            sequence_timeout: std::time::Duration::from_millis(500),
        }
    }

    pub fn resolve(&self, key: &KeyCombo, context: &KeyContext) -> Option<&str> {
        self.bindings
            .iter()
            .find(|b| &b.key == key && (&b.context == context || b.context == KeyContext::Global))
            .map(|b| b.command.as_str())
    }

    pub fn merge_user_config(&mut self, user_bindings: Vec<KeyBinding>) {
        for ub in user_bindings {
            self.bindings
                .retain(|b| !(b.key == ub.key && b.context == ub.context));
            self.bindings.push(ub);
        }
    }

    pub fn feed_key(&mut self, key: KeyCombo, context: &KeyContext) -> KeymapResult {
        let now = std::time::Instant::now();

        // If there's a pending sequence and timeout has elapsed, discard it
        if !self.sequence_buffer.is_empty() {
            if let Some(last_time) = self.last_key_time {
                if now.duration_since(last_time) > self.sequence_timeout {
                    self.sequence_buffer.clear();
                }
            }
        }

        self.last_key_time = Some(now);
        self.sequence_buffer.push(key.clone());

        // Check if current buffer matches any sequence exactly
        if let Some(cmd) = self.sequence_bindings.get(&self.sequence_buffer) {
            let cmd = cmd.clone();
            self.sequence_buffer.clear();
            return KeymapResult::Command(cmd);
        }

        // Check if current buffer is a prefix of any sequence
        let is_prefix = self.sequence_bindings.keys().any(|seq| {
            seq.len() > self.sequence_buffer.len()
                && seq.starts_with(&self.sequence_buffer)
        });

        if is_prefix {
            return KeymapResult::Pending;
        }

        // No sequence match — try single key resolve for the latest key
        self.sequence_buffer.clear();
        if let Some(cmd) = self.resolve(&key, context) {
            KeymapResult::Command(cmd.to_string())
        } else {
            KeymapResult::Unhandled
        }
    }

    pub fn clear_sequence(&mut self) {
        self.sequence_buffer.clear();
    }

    pub fn register_sequence(&mut self, keys: Vec<KeyCombo>, command: String) {
        self.sequence_bindings.insert(keys, command);
    }
}

#[derive(Debug, Clone)]
pub enum KeymapResult {
    Command(String),
    Pending,
    Unhandled,
}

impl KeyCombo {
    pub fn from_crossterm(key_code: KeyCode, modifiers: KeyModifiers) -> Option<Self> {
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);
        let alt = modifiers.contains(KeyModifiers::ALT);
        let shift = modifiers.contains(KeyModifiers::SHIFT);

        let code = match key_code {
            KeyCode::Char(' ') => SerializableKeyCode::Space,
            KeyCode::Char(c) => SerializableKeyCode::Char(c),
            KeyCode::Enter => SerializableKeyCode::Enter,
            KeyCode::Esc => SerializableKeyCode::Esc,
            KeyCode::Tab => SerializableKeyCode::Tab,
            KeyCode::Backspace => SerializableKeyCode::Backspace,
            KeyCode::Delete => SerializableKeyCode::Delete,
            KeyCode::Up => SerializableKeyCode::Up,
            KeyCode::Down => SerializableKeyCode::Down,
            KeyCode::Left => SerializableKeyCode::Left,
            KeyCode::Right => SerializableKeyCode::Right,
            KeyCode::Home => SerializableKeyCode::Home,
            KeyCode::End => SerializableKeyCode::End,
            KeyCode::PageUp => SerializableKeyCode::PageUp,
            KeyCode::PageDown => SerializableKeyCode::PageDown,
            KeyCode::F(n) => SerializableKeyCode::F(n),
            _ => return None,
        };

        // For uppercase letters, crossterm reports SHIFT modifier, but our bindings
        // register them as Char('A') with shift=true via kb_shift(). For plain chars
        // (lowercase + symbols), ignore the shift modifier since the char itself
        // already reflects it.
        let effective_shift = match &code {
            SerializableKeyCode::Char(c) if c.is_ascii_uppercase() => true,
            SerializableKeyCode::Char(_) => false,
            _ => shift,
        };

        Some(Self {
            code,
            modifiers: Modifiers {
                ctrl,
                alt,
                shift: effective_shift,
            },
        })
    }
}

pub fn parse_key_spec(spec: &str) -> Option<KeyCombo> {
    let parts: Vec<&str> = spec.split('+').collect();
    let mut mods = Modifiers::default();
    let key_part;

    if parts.len() == 1 {
        key_part = parts[0];
    } else {
        for &part in &parts[..parts.len() - 1] {
            match part.to_lowercase().as_str() {
                "ctrl" | "c" => mods.ctrl = true,
                "alt" | "a" => mods.alt = true,
                "shift" | "s" => mods.shift = true,
                _ => {}
            }
        }
        key_part = parts[parts.len() - 1];
    }

    let code = match key_part.to_lowercase().as_str() {
        "enter" | "return" | "cr" => SerializableKeyCode::Enter,
        "esc" | "escape" => SerializableKeyCode::Esc,
        "tab" => SerializableKeyCode::Tab,
        "backspace" | "bs" => SerializableKeyCode::Backspace,
        "delete" | "del" => SerializableKeyCode::Delete,
        "up" => SerializableKeyCode::Up,
        "down" => SerializableKeyCode::Down,
        "left" => SerializableKeyCode::Left,
        "right" => SerializableKeyCode::Right,
        "home" => SerializableKeyCode::Home,
        "end" => SerializableKeyCode::End,
        "pageup" | "pgup" => SerializableKeyCode::PageUp,
        "pagedown" | "pgdn" => SerializableKeyCode::PageDown,
        "space" => SerializableKeyCode::Space,
        s if s.starts_with('f') && s.len() > 1 => {
            let n: u8 = s[1..].parse().ok()?;
            SerializableKeyCode::F(n)
        }
        s if s.len() == 1 => SerializableKeyCode::Char(s.chars().next().unwrap()),
        _ => return None,
    };

    Some(KeyCombo {
        code,
        modifiers: mods,
    })
}

pub fn command_to_app_command(cmd: &str) -> Option<AppCommand> {
    match cmd {
        "cursor_up" => Some(AppCommand::CursorUp),
        "cursor_down" => Some(AppCommand::CursorDown),
        "cursor_top" => Some(AppCommand::CursorTop),
        "cursor_bottom" => Some(AppCommand::CursorBottom),
        "page_up" => Some(AppCommand::PageUp),
        "page_down" => Some(AppCommand::PageDown),
        "enter_directory" => Some(AppCommand::EnterDirectory),
        "parent_directory" => Some(AppCommand::ParentDirectory),
        "go_back" => Some(AppCommand::GoBack),
        "go_forward" => Some(AppCommand::GoForward),
        "toggle_select" => Some(AppCommand::ToggleSelect),
        "select_all" => Some(AppCommand::SelectAll),
        "clear_selection" => Some(AppCommand::ClearSelection),
        "visual_mode" => Some(AppCommand::VisualMode),
        "normal_mode" => Some(AppCommand::NormalMode),
        "copy" => Some(AppCommand::Copy),
        "cut" => Some(AppCommand::Cut),
        "paste" => Some(AppCommand::Paste),
        "delete" => Some(AppCommand::Delete),
        "rename" => Some(AppCommand::Rename),
        "create_file" => Some(AppCommand::CreateFile),
        "create_dir" => Some(AppCommand::CreateDir),
        "sort_next" => Some(AppCommand::SortNext),
        "toggle_sort_order" => Some(AppCommand::ToggleSortOrder),
        "toggle_preview" => Some(AppCommand::TogglePreview),
        "toggle_hidden" => Some(AppCommand::ToggleHidden),
        "refresh" => Some(AppCommand::Refresh),
        "search" => Some(AppCommand::SearchMode),
        "filter" => Some(AppCommand::FilterMode),
        "search_next" => Some(AppCommand::SearchNext),
        "search_prev" => Some(AppCommand::SearchPrev),
        "confirm" => Some(AppCommand::ConfirmInput),
        "cancel" => Some(AppCommand::CancelInput),
        "quit" => Some(AppCommand::Quit),
        "force_quit" => Some(AppCommand::ForceQuit),
        "command_mode" => Some(AppCommand::CommandMode),
        // Unknown commands are routed to the plugin system
        other => Some(AppCommand::PluginCommand {
            plugin: String::new(),
            command: other.to_string(),
            args: Vec::new(),
        }),
    }
}

pub fn default_keymap() -> Keymap {
    let bindings = vec![
        // Normal mode - navigation
        kb('j', "cursor_down", KeyContext::Normal),
        kb('k', "cursor_up", KeyContext::Normal),
        kb('h', "parent_directory", KeyContext::Normal),
        kb('l', "enter_directory", KeyContext::Normal),
        kb_code(SerializableKeyCode::Up, "cursor_up", KeyContext::Normal),
        kb_code(SerializableKeyCode::Down, "cursor_down", KeyContext::Normal),
        kb_code(SerializableKeyCode::Left, "parent_directory", KeyContext::Normal),
        kb_code(SerializableKeyCode::Right, "enter_directory", KeyContext::Normal),
        kb_code(SerializableKeyCode::Enter, "enter_directory", KeyContext::Normal),
        kb_code(SerializableKeyCode::PageUp, "page_up", KeyContext::Normal),
        kb_code(SerializableKeyCode::PageDown, "page_down", KeyContext::Normal),
        kb_ctrl('d', "page_down", KeyContext::Normal),
        kb_ctrl('u', "page_up", KeyContext::Normal),
        kb_code(SerializableKeyCode::Home, "cursor_top", KeyContext::Normal),
        kb_code(SerializableKeyCode::End, "cursor_bottom", KeyContext::Normal),
        kb_shift('G', "cursor_bottom", KeyContext::Normal),
        // Normal mode - selection
        kb_code(SerializableKeyCode::Space, "toggle_select", KeyContext::Normal),
        kb('v', "visual_mode", KeyContext::Normal),
        // Normal mode - operations
        kb('p', "paste", KeyContext::Normal),
        kb('r', "rename", KeyContext::Normal),
        kb('a', "create_file", KeyContext::Normal),
        kb_shift('A', "create_dir", KeyContext::Normal),
        kb('d', "delete", KeyContext::Normal),
        kb('y', "copy", KeyContext::Normal),
        kb('x', "cut", KeyContext::Normal),
        // Normal mode - display
        kb('.', "toggle_hidden", KeyContext::Normal),
        kb('s', "sort_next", KeyContext::Normal),
        kb_shift('S', "toggle_sort_order", KeyContext::Normal),
        kb_ctrl('r', "refresh", KeyContext::Normal),
        kb_ctrl('p', "toggle_preview", KeyContext::Normal),
        // Normal mode - search/filter
        kb('/', "search", KeyContext::Normal),
        kb('f', "filter", KeyContext::Normal),
        kb(':', "command_mode", KeyContext::Normal),
        // Normal mode - quit
        kb('q', "quit", KeyContext::Normal),
        kb_ctrl('c', "force_quit", KeyContext::Global),
        // Visual mode
        kb('j', "cursor_down", KeyContext::Visual),
        kb('k', "cursor_up", KeyContext::Visual),
        kb_code(SerializableKeyCode::Esc, "normal_mode", KeyContext::Visual),
        kb('y', "copy", KeyContext::Visual),
        kb('d', "delete", KeyContext::Visual),
        kb('x', "cut", KeyContext::Visual),
        // Search mode
        kb_code(SerializableKeyCode::Enter, "confirm", KeyContext::Search),
        kb_code(SerializableKeyCode::Esc, "cancel", KeyContext::Search),
        kb_ctrl('n', "search_next", KeyContext::Search),
        kb_ctrl('p', "search_prev", KeyContext::Search),
    ];

    let mut keymap = Keymap::new(bindings);

    // Register sequence bindings
    keymap.register_sequence(
        vec![
            KeyCombo { code: SerializableKeyCode::Char('g'), modifiers: Modifiers::default() },
            KeyCombo { code: SerializableKeyCode::Char('g'), modifiers: Modifiers::default() },
        ],
        "cursor_top".to_string(),
    );

    keymap
}

fn kb(ch: char, command: &str, context: KeyContext) -> KeyBinding {
    KeyBinding {
        key: KeyCombo {
            code: SerializableKeyCode::Char(ch),
            modifiers: Modifiers::default(),
        },
        command: command.to_string(),
        context,
        description: String::new(),
    }
}

fn kb_shift(ch: char, command: &str, context: KeyContext) -> KeyBinding {
    KeyBinding {
        key: KeyCombo {
            code: SerializableKeyCode::Char(ch),
            modifiers: Modifiers { shift: true, ..Default::default() },
        },
        command: command.to_string(),
        context,
        description: String::new(),
    }
}

fn kb_ctrl(ch: char, command: &str, context: KeyContext) -> KeyBinding {
    KeyBinding {
        key: KeyCombo {
            code: SerializableKeyCode::Char(ch),
            modifiers: Modifiers { ctrl: true, ..Default::default() },
        },
        command: command.to_string(),
        context,
        description: String::new(),
    }
}

fn kb_code(code: SerializableKeyCode, command: &str, context: KeyContext) -> KeyBinding {
    KeyBinding {
        key: KeyCombo {
            code,
            modifiers: Modifiers::default(),
        },
        command: command.to_string(),
        context,
        description: String::new(),
    }
}
