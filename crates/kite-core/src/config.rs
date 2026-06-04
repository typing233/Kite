use crate::event::SortField;
use crate::keymap::KeyBinding;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub sort: SortConfig,
    #[serde(default)]
    pub preview: PreviewConfig,
    #[serde(default)]
    pub plugins: PluginConfig,
    #[serde(default)]
    pub keys: KeysConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_true")]
    pub show_hidden: bool,
    #[serde(default = "default_true")]
    pub confirm_delete: bool,
    #[serde(default = "default_true")]
    pub follow_symlinks: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            confirm_delete: true,
            follow_symlinks: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_tick_rate")]
    pub tick_rate_ms: u64,
    #[serde(default = "default_true")]
    pub show_icons: bool,
    #[serde(default = "default_true")]
    pub show_permissions: bool,
    #[serde(default = "default_true")]
    pub show_size: bool,
    #[serde(default = "default_true")]
    pub show_modified: bool,
    #[serde(default = "default_pane_ratios")]
    pub pane_ratios: [u16; 3],
    #[serde(default)]
    pub theme: ThemeConfig,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            tick_rate_ms: 100,
            show_icons: true,
            show_permissions: true,
            show_size: true,
            show_modified: true,
            pane_ratios: [1, 3, 2],
            theme: ThemeConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_theme_preset")]
    pub preset: String,
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            preset: "default".to_string(),
            custom: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortConfig {
    #[serde(default = "default_sort_field")]
    pub field: String,
    #[serde(default = "default_true")]
    pub ascending: bool,
    #[serde(default = "default_true")]
    pub dirs_first: bool,
}

impl Default for SortConfig {
    fn default() -> Self {
        Self {
            field: "name".to_string(),
            ascending: true,
            dirs_first: true,
        }
    }
}

impl SortConfig {
    pub fn sort_field(&self) -> SortField {
        match self.field.as_str() {
            "size" => SortField::Size,
            "modified" => SortField::Modified,
            "extension" => SortField::Extension,
            _ => SortField::Name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: String,
    #[serde(default = "default_syntax_theme")]
    pub syntax_theme: String,
    #[serde(default = "default_image_protocol")]
    pub image_protocol: String,
    #[serde(default = "default_tab_width")]
    pub tab_width: usize,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_file_size: "10MB".to_string(),
            syntax_theme: "base16-ocean.dark".to_string(),
            image_protocol: "kitty".to_string(),
            tab_width: 4,
        }
    }
}

impl PreviewConfig {
    pub fn max_file_size_bytes(&self) -> u64 {
        let s = self.max_file_size.trim();
        if let Some(num) = s.strip_suffix("MB") {
            num.trim().parse::<u64>().unwrap_or(10) * 1024 * 1024
        } else if let Some(num) = s.strip_suffix("KB") {
            num.trim().parse::<u64>().unwrap_or(10240) * 1024
        } else {
            s.parse::<u64>().unwrap_or(10 * 1024 * 1024)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    #[serde(default = "default_plugin_dir")]
    pub directory: String,
    #[serde(default)]
    pub enabled: Vec<String>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            directory: "~/.config/kite/plugins".to_string(),
            enabled: Vec::new(),
        }
    }
}

impl PluginConfig {
    pub fn plugin_dir(&self) -> PathBuf {
        let expanded = shellexpand(&self.directory);
        PathBuf::from(expanded)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeysConfig {
    #[serde(default)]
    pub normal: HashMap<String, String>,
    #[serde(default)]
    pub visual: HashMap<String, String>,
    #[serde(default)]
    pub search: HashMap<String, String>,
    #[serde(default)]
    pub command: HashMap<String, String>,
}

impl Config {
    pub fn load() -> Result<Self, crate::error::KiteError> {
        let config_path = config_path();
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| crate::error::KiteError::Config(e.to_string()))?;
            toml::from_str(&content)
                .map_err(|e| crate::error::KiteError::Config(e.to_string()))
        } else {
            Ok(Self::default())
        }
    }

    pub fn user_key_bindings(&self) -> Vec<KeyBinding> {
        let mut bindings = Vec::new();

        for (key_spec, command) in &self.keys.normal {
            if let Some(key) = crate::keymap::parse_key_spec(key_spec) {
                bindings.push(KeyBinding {
                    key,
                    command: command.clone(),
                    context: crate::keymap::KeyContext::Normal,
                    description: String::new(),
                });
            }
        }
        for (key_spec, command) in &self.keys.visual {
            if let Some(key) = crate::keymap::parse_key_spec(key_spec) {
                bindings.push(KeyBinding {
                    key,
                    command: command.clone(),
                    context: crate::keymap::KeyContext::Visual,
                    description: String::new(),
                });
            }
        }
        for (key_spec, command) in &self.keys.search {
            if let Some(key) = crate::keymap::parse_key_spec(key_spec) {
                bindings.push(KeyBinding {
                    key,
                    command: command.clone(),
                    context: crate::keymap::KeyContext::Search,
                    description: String::new(),
                });
            }
        }
        for (key_spec, command) in &self.keys.command {
            if let Some(key) = crate::keymap::parse_key_spec(key_spec) {
                bindings.push(KeyBinding {
                    key,
                    command: command.clone(),
                    context: crate::keymap::KeyContext::Command,
                    description: String::new(),
                });
            }
        }

        bindings
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            ui: UiConfig::default(),
            sort: SortConfig::default(),
            preview: PreviewConfig::default(),
            plugins: PluginConfig::default(),
            keys: KeysConfig::default(),
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("kite")
        .join("config.toml")
}

fn shellexpand(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().to_string();
        }
    }
    path.to_string()
}

fn default_true() -> bool {
    true
}
fn default_tick_rate() -> u64 {
    100
}
fn default_pane_ratios() -> [u16; 3] {
    [1, 3, 2]
}
fn default_theme_preset() -> String {
    "default".to_string()
}
fn default_sort_field() -> String {
    "name".to_string()
}
fn default_max_file_size() -> String {
    "10MB".to_string()
}
fn default_syntax_theme() -> String {
    "base16-ocean.dark".to_string()
}
fn default_image_protocol() -> String {
    "kitty".to_string()
}
fn default_tab_width() -> usize {
    4
}
fn default_plugin_dir() -> String {
    "~/.config/kite/plugins".to_string()
}
