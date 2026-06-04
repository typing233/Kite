use crate::loader::NativePlugin;
use crate::lua::LuaRuntime;
use kite_core::config::Config;
use kite_core::event::AppCommand;
use kite_core::keymap::{parse_key_spec, KeyBinding, KeyContext};
use kite_core::plugin::{PluginContext, PluginError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;

pub struct PluginManager {
    native_plugins: Vec<LoadedNativePlugin>,
    lua_runtime: Option<LuaRuntime>,
    plugin_dir: PathBuf,
    enabled_list: Vec<String>,
    commands: HashMap<String, PluginSource>,
}

struct LoadedNativePlugin {
    plugin: NativePlugin,
    #[allow(dead_code)]
    name: String,
    enabled: bool,
}

#[derive(Debug, Clone)]
enum PluginSource {
    Native(usize),
    Lua,
}

impl PluginManager {
    pub fn new(config: &Config, cmd_sender: UnboundedSender<AppCommand>) -> Self {
        let plugin_dir = config.plugins.plugin_dir();
        let lua_runtime = LuaRuntime::new(cmd_sender).ok();
        let enabled_list = config.plugins.enabled.clone();

        Self {
            native_plugins: Vec::new(),
            lua_runtime,
            plugin_dir,
            enabled_list,
            commands: HashMap::new(),
        }
    }

    pub fn load_all(&mut self) -> Result<(), PluginError> {
        if !self.plugin_dir.exists() {
            return Ok(());
        }

        // Load native plugins (.so / .dylib) — only if in enabled list
        let native_ext = if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };

        if let Ok(entries) = std::fs::read_dir(&self.plugin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == native_ext {
                        // Derive plugin name from filename (e.g. "git-status.so" -> "git-status")
                        let plugin_file_name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .strip_prefix("lib")
                            .unwrap_or(
                                path.file_stem().and_then(|s| s.to_str()).unwrap_or(""),
                            )
                            .to_string();

                        // Only load if in the enabled list
                        if !self.is_enabled(&plugin_file_name) {
                            log::debug!("Skipping disabled plugin: {}", plugin_file_name);
                            continue;
                        }

                        match unsafe { NativePlugin::load(&path) } {
                            Ok(plugin) => {
                                let name = plugin.instance.manifest().name.clone();
                                log::info!("Loaded native plugin: {}", name);
                                let idx = self.native_plugins.len();
                                for cmd in plugin.instance.commands() {
                                    self.commands.insert(cmd, PluginSource::Native(idx));
                                }
                                self.native_plugins.push(LoadedNativePlugin {
                                    plugin,
                                    name,
                                    enabled: true,
                                });
                            }
                            Err(e) => {
                                log::warn!("Failed to load plugin {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        // Load Lua plugins — only if in enabled list
        let lua_dir = self.plugin_dir.join("lua");
        if lua_dir.exists() {
            if let Some(ref runtime) = self.lua_runtime {
                if let Ok(entries) = std::fs::read_dir(&lua_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("lua") {
                            // Derive plugin name from filename (e.g. "fzf-integration.lua" -> "fzf-integration")
                            let script_name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_string();

                            if !self.is_enabled(&script_name) {
                                log::debug!("Skipping disabled Lua plugin: {}", script_name);
                                continue;
                            }

                            match runtime.load_script(&path) {
                                Ok(()) => {
                                    log::info!("Loaded Lua plugin: {}", script_name);
                                }
                                Err(e) => {
                                    log::warn!("Failed to load Lua plugin {:?}: {}", path, e);
                                }
                            }
                        }
                    }
                }

                // Register Lua commands
                for cmd in runtime.get_registered_commands() {
                    self.commands.insert(cmd, PluginSource::Lua);
                }
            }
        }

        Ok(())
    }

    fn is_enabled(&self, name: &str) -> bool {
        self.enabled_list.iter().any(|e| e == name)
    }

    pub fn has_command(&self, command: &str) -> bool {
        self.commands.contains_key(command)
    }

    pub fn execute_command(
        &mut self,
        command: &str,
        args: &[&str],
        ctx: &mut PluginContext,
    ) -> Result<(), PluginError> {
        if let Some(source) = self.commands.get(command).cloned() {
            match source {
                PluginSource::Native(idx) => {
                    if let Some(loaded) = self.native_plugins.get_mut(idx) {
                        if loaded.enabled {
                            return loaded.plugin.instance.execute_command(command, args, ctx);
                        }
                    }
                }
                PluginSource::Lua => {
                    if let Some(ref runtime) = self.lua_runtime {
                        return runtime.execute_command(command, args);
                    }
                }
            }
        }

        Err(PluginError::CommandNotFound(command.to_string()))
    }

    pub fn get_key_bindings(&self) -> Vec<KeyBinding> {
        let mut bindings = Vec::new();

        // Collect native plugin bindings
        for loaded in &self.native_plugins {
            if loaded.enabled {
                bindings.extend(loaded.plugin.instance.key_bindings());
            }
        }

        // Collect Lua keybindings
        if let Some(ref runtime) = self.lua_runtime {
            for (key_spec, command) in runtime.get_registered_keybindings() {
                if let Some(key) = parse_key_spec(&key_spec) {
                    bindings.push(KeyBinding {
                        key,
                        command,
                        context: KeyContext::Normal,
                        description: String::new(),
                    });
                }
            }
        }

        bindings
    }

    pub fn notify_directory_changed(&mut self, path: &Path, ctx: &mut PluginContext) {
        for loaded in &mut self.native_plugins {
            if loaded.enabled {
                loaded.plugin.instance.on_directory_changed(path, ctx);
            }
        }
    }

    pub fn unload_all(&mut self) {
        for loaded in &mut self.native_plugins {
            loaded.plugin.instance.on_unload();
        }
        self.native_plugins.clear();
        self.commands.clear();
    }
}
