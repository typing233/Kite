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
    commands: HashMap<String, PluginSource>,
}

struct LoadedNativePlugin {
    plugin: NativePlugin,
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

        Self {
            native_plugins: Vec::new(),
            lua_runtime,
            plugin_dir,
            commands: HashMap::new(),
        }
    }

    pub fn load_all(&mut self) -> Result<(), PluginError> {
        if !self.plugin_dir.exists() {
            return Ok(());
        }

        // Load native plugins (.so / .dylib)
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
                        match unsafe { NativePlugin::load(&path) } {
                            Ok(plugin) => {
                                log::info!(
                                    "Loaded native plugin: {}",
                                    plugin.instance.manifest().name
                                );
                                let idx = self.native_plugins.len();
                                for cmd in plugin.instance.commands() {
                                    self.commands.insert(cmd, PluginSource::Native(idx));
                                }
                                self.native_plugins.push(LoadedNativePlugin {
                                    plugin,
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

        // Load Lua plugins
        let lua_dir = self.plugin_dir.join("lua");
        if lua_dir.exists() {
            if let Some(ref runtime) = self.lua_runtime {
                if let Ok(entries) = std::fs::read_dir(&lua_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("lua") {
                            match runtime.load_script(&path) {
                                Ok(()) => {
                                    log::info!("Loaded Lua plugin: {:?}", path.file_name());
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

    pub fn execute_command(
        &mut self,
        _plugin_name: &str,
        command: &str,
        args: &[&str],
        ctx: &mut PluginContext,
    ) -> Result<(), PluginError> {
        // Check if it's a known command
        if let Some(source) = self.commands.get(command).cloned() {
            match source {
                PluginSource::Native(idx) => {
                    if let Some(loaded) = self.native_plugins.get_mut(idx) {
                        return loaded.plugin.instance.execute_command(command, args, ctx);
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
