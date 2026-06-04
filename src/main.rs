use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use kite_core::config::Config;
use kite_core::event::AppCommand;
use kite_plugin::PluginManager;
use kite_ui::input::handle_event;
use kite_ui::App;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, stdout};
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .init();

    // Load config
    let config = Config::load().unwrap_or_default();

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Install panic hook that restores terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    // Run the app
    let result = run_app(&mut terminal, config).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: Config,
) -> Result<()> {
    // Create command channel
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<AppCommand>();

    // Initialize plugin manager
    let mut plugin_manager = PluginManager::new(&config, cmd_tx.clone());
    plugin_manager.load_all().ok();

    // Merge plugin keybindings into app
    let plugin_bindings = plugin_manager.get_key_bindings();

    // Initialize app state
    let mut app = App::new(config).await?;
    app.keymap.merge_user_config(plugin_bindings);

    // Set up filesystem watcher channel
    let (fs_tx, mut fs_rx) = mpsc::unbounded_channel::<Vec<std::path::PathBuf>>();
    let _watcher = kite_fs::watch::FsWatcher::new(&app.current_dir, fs_tx).ok();

    // Event stream
    let mut event_stream = EventStream::new();
    let tick_rate = Duration::from_millis(app.config.ui.tick_rate_ms);
    let mut tick_interval = tokio::time::interval(tick_rate);

    loop {
        // Render
        terminal.draw(|frame| {
            kite_ui::render(frame, &app);
        })?;

        // Wait for events
        tokio::select! {
            Some(Ok(event)) = event_stream.next() => {
                if let Some(cmd) = handle_event(&mut app, event) {
                    let _ = cmd_tx.send(cmd);
                }
            }

            Some(cmd) = cmd_rx.recv() => {
                let quit = app.execute_command(cmd).await?;
                if quit {
                    break;
                }
            }

            Some(_paths) = fs_rx.recv() => {
                app.reload_directory().await.ok();
            }

            _ = tick_interval.tick() => {
                app.tick();
            }
        }
    }

    plugin_manager.unload_all();
    Ok(())
}
