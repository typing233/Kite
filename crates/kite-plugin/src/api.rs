use kite_core::event::AppCommand;

/// API surface exposed to plugins for interacting with the application.
pub struct PluginApi {
    pub command_sender: tokio::sync::mpsc::UnboundedSender<AppCommand>,
}

impl PluginApi {
    pub fn new(command_sender: tokio::sync::mpsc::UnboundedSender<AppCommand>) -> Self {
        Self { command_sender }
    }

    pub fn send_command(&self, cmd: AppCommand) -> bool {
        self.command_sender.send(cmd).is_ok()
    }

    pub fn refresh(&self) -> bool {
        self.send_command(AppCommand::Refresh)
    }
}
