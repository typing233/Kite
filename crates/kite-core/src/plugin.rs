use crate::event::AppCommand;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn on_load(&mut self, ctx: &mut PluginContext) -> Result<(), PluginError>;
    fn on_unload(&mut self) {}

    fn key_bindings(&self) -> Vec<crate::keymap::KeyBinding> {
        Vec::new()
    }

    fn commands(&self) -> Vec<String> {
        Vec::new()
    }

    fn execute_command(
        &mut self,
        cmd: &str,
        args: &[&str],
        ctx: &mut PluginContext,
    ) -> Result<(), PluginError>;

    fn on_directory_changed(&mut self, _path: &std::path::Path, _ctx: &mut PluginContext) {}

    fn on_before_operation(
        &mut self,
        _op: &FileOperation,
        _ctx: &mut PluginContext,
    ) -> OperationDecision {
        OperationDecision::Allow
    }
}

pub struct PluginContext {
    pub current_dir: PathBuf,
    pub selected_entries: Vec<crate::entry::FileEntry>,
    pub command_sender: UnboundedSender<AppCommand>,
}

#[derive(Debug, Clone)]
pub enum FileOperation {
    Copy {
        sources: Vec<PathBuf>,
        dest: PathBuf,
    },
    Move {
        sources: Vec<PathBuf>,
        dest: PathBuf,
    },
    Delete {
        targets: Vec<PathBuf>,
    },
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperationDecision {
    Allow,
    Deny(String),
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin initialization failed: {0}")]
    InitFailed(String),
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
}
