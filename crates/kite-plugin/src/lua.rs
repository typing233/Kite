use kite_core::event::AppCommand;
use kite_core::plugin::PluginError;
use mlua::{Function, Lua, Table};
use std::path::Path;
use tokio::sync::mpsc::UnboundedSender;

pub struct LuaRuntime {
    lua: Lua,
}

impl LuaRuntime {
    pub fn new(_cmd_sender: UnboundedSender<AppCommand>) -> Result<Self, PluginError> {
        let lua = Lua::new();

        // Register the `kite` API table
        let kite_table = lua.create_table().map_err(lua_err)?;

        // kite.notify(msg) -- show status message (placeholder for now)
        kite_table
            .set(
                "notify",
                lua.create_function(|_, msg: String| {
                    log::info!("[plugin] {}", msg);
                    Ok(())
                })
                .map_err(lua_err)?,
            )
            .map_err(lua_err)?;

        // kite.register_command(name, callback)
        kite_table
            .set(
                "register_command",
                lua.create_function(|lua, (name, func): (String, Function)| {
                    let registry: Table = lua.globals().get("_kite_commands")?;
                    registry.set(name, func)?;
                    Ok(())
                })
                .map_err(lua_err)?,
            )
            .map_err(lua_err)?;

        // kite.bind_key(key_spec, command_name)
        kite_table
            .set(
                "bind_key",
                lua.create_function(|lua, (key, cmd): (String, String)| {
                    let registry: Table = lua.globals().get("_kite_keybindings")?;
                    registry.set(key, cmd)?;
                    Ok(())
                })
                .map_err(lua_err)?,
            )
            .map_err(lua_err)?;

        // kite.current_dir() -- returns current directory
        kite_table
            .set(
                "current_dir",
                lua.create_function(|_, ()| {
                    let cwd = std::env::current_dir()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    Ok(cwd)
                })
                .map_err(lua_err)?,
            )
            .map_err(lua_err)?;

        lua.globals().set("kite", kite_table).map_err(lua_err)?;
        lua.globals()
            .set("_kite_commands", lua.create_table().map_err(lua_err)?)
            .map_err(lua_err)?;
        lua.globals()
            .set("_kite_keybindings", lua.create_table().map_err(lua_err)?)
            .map_err(lua_err)?;

        Ok(Self { lua })
    }

    pub fn load_script(&self, path: &Path) -> Result<(), PluginError> {
        let code = std::fs::read_to_string(path)
            .map_err(|e| PluginError::InitFailed(format!("Failed to read script: {}", e)))?;

        self.lua
            .load(&code)
            .set_name(path.to_string_lossy())
            .exec()
            .map_err(|e| PluginError::ExecutionError(format!("Lua error: {}", e)))?;

        Ok(())
    }

    pub fn execute_command(&self, name: &str, args: &[&str]) -> Result<(), PluginError> {
        let registry: Table = self
            .lua
            .globals()
            .get("_kite_commands")
            .map_err(|e| PluginError::ExecutionError(e.to_string()))?;

        let func: Function = registry
            .get(name)
            .map_err(|_| PluginError::CommandNotFound(name.to_string()))?;

        let lua_args = self
            .lua
            .create_table()
            .map_err(|e| PluginError::ExecutionError(e.to_string()))?;

        for (i, arg) in args.iter().enumerate() {
            lua_args
                .set(i + 1, *arg)
                .map_err(|e| PluginError::ExecutionError(e.to_string()))?;
        }

        func.call::<()>(lua_args)
            .map_err(|e| PluginError::ExecutionError(format!("Lua error in {}: {}", name, e)))?;

        Ok(())
    }

    pub fn get_registered_commands(&self) -> Vec<String> {
        let mut commands = Vec::new();
        if let Ok(registry) = self.lua.globals().get::<Table>("_kite_commands") {
            for pair in registry.pairs::<String, Function>() {
                if let Ok((name, _)) = pair {
                    commands.push(name);
                }
            }
        }
        commands
    }

    pub fn get_registered_keybindings(&self) -> Vec<(String, String)> {
        let mut bindings = Vec::new();
        if let Ok(registry) = self.lua.globals().get::<Table>("_kite_keybindings") {
            for pair in registry.pairs::<String, String>() {
                if let Ok((key, cmd)) = pair {
                    bindings.push((key, cmd));
                }
            }
        }
        bindings
    }
}

fn lua_err(e: mlua::Error) -> PluginError {
    PluginError::InitFailed(format!("Lua initialization error: {}", e))
}
