use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    event::{DisableMouseCapture, EnableMouseCapture, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use kite_core::config::Config;
use kite_core::event::AppCommand;
use kite_core::plugin::PluginContext;
use kite_plugin::PluginManager;
use kite_preview::image::{clear_kitty_image, encode_kitty_image};
use kite_preview::traits::{ImageFormat, PreviewContent};
use kite_preview::PreviewManager;
use kite_ui::input::handle_event;
use kite_ui::App;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, stdout, Write};
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let config = Config::load().unwrap_or_default();

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    let result = run_app(&mut terminal, config).await;

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
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<AppCommand>();

    let mut plugin_manager = PluginManager::new(&config, cmd_tx.clone());
    plugin_manager.load_all().ok();

    let plugin_bindings = plugin_manager.get_key_bindings();

    let mut app = App::new(config).await?;
    app.keymap.merge_user_config(plugin_bindings);

    let (fs_tx, mut fs_rx) = mpsc::unbounded_channel::<Vec<std::path::PathBuf>>();
    let _watcher = kite_fs::watch::FsWatcher::new(&app.current_dir, fs_tx).ok();

    let mut event_stream = EventStream::new();
    let tick_rate = Duration::from_millis(app.config.ui.tick_rate_ms);
    let mut tick_interval = tokio::time::interval(tick_rate);

    let preview_manager = PreviewManager::new();
    let kitty_supported = detect_kitty_support();
    let mut last_image_shown = false;

    loop {
        // Compute the preview area before drawing
        let terminal_size = terminal.get_frame().area();
        let preview_area = get_preview_area(terminal_size, &app);

        // Render frame
        terminal.draw(|frame| {
            kite_ui::render(frame, &app);
        })?;

        // After frame render: display Kitty image if applicable
        if app.show_preview && kitty_supported {
            if let Some(entry) = app.current_entry().cloned() {
                if entry.is_image_file() {
                    if let Some(area) = preview_area {
                        let inner_w = area.width.saturating_sub(2);
                        let inner_h = area.height.saturating_sub(2);
                        if let Ok(PreviewContent::KittyImage { data, width, height, format }) =
                            preview_manager.preview(&entry.path, inner_w, inner_h)
                        {
                            render_kitty_image(
                                terminal.backend_mut(),
                                &data,
                                width,
                                height,
                                format,
                                area.x + 1,
                                area.y + 1,
                            );
                            last_image_shown = true;
                        }
                    }
                } else if last_image_shown {
                    clear_kitty_on_terminal(terminal.backend_mut());
                    last_image_shown = false;
                }
            } else if last_image_shown {
                clear_kitty_on_terminal(terminal.backend_mut());
                last_image_shown = false;
            }
        } else if last_image_shown {
            clear_kitty_on_terminal(terminal.backend_mut());
            last_image_shown = false;
        }

        // Wait for events
        tokio::select! {
            Some(Ok(event)) = event_stream.next() => {
                if let Some(cmd) = handle_event(&mut app, event) {
                    let _ = cmd_tx.send(cmd);
                }
            }

            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    AppCommand::PluginCommand { plugin: _, ref command, ref args } => {
                        // Only dispatch if this is an actually registered plugin command
                        if plugin_manager.has_command(command) {
                            let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                            let mut ctx = PluginContext {
                                current_dir: app.current_dir.clone(),
                                selected_entries: app.get_selected_entries(),
                                command_sender: cmd_tx.clone(),
                            };
                            if let Err(e) = plugin_manager.execute_command(command, &args_refs, &mut ctx) {
                                app.set_status(
                                    format!("Plugin error: {}", e),
                                    kite_ui::app::StatusLevel::Error,
                                );
                            }
                        }
                        // Unknown commands that aren't registered plugins are silently ignored
                    }
                    other => {
                        let quit = app.execute_command(other).await?;
                        if quit {
                            break;
                        }
                    }
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

    // Cleanup kitty image if showing
    if last_image_shown {
        clear_kitty_on_terminal(terminal.backend_mut());
    }
    plugin_manager.unload_all();
    Ok(())
}

fn detect_kitty_support() -> bool {
    if let Ok(term) = std::env::var("TERM_PROGRAM") {
        let t = term.to_lowercase();
        if t.contains("kitty") || t.contains("wezterm") {
            return true;
        }
    }
    if let Ok(term) = std::env::var("TERM") {
        if term.contains("kitty") {
            return true;
        }
    }
    std::env::var("KITTY_WINDOW_ID").is_ok()
}

/// Compute the preview pane area from the terminal layout.
fn get_preview_area(
    size: ratatui::layout::Rect,
    app: &App,
) -> Option<ratatui::layout::Rect> {
    if !app.show_preview {
        return None;
    }
    use ratatui::layout::{Constraint, Direction, Layout};

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(size);

    let [l, c, r] = app.pane_ratios;
    let total = l + c + r;
    let pane_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(l as u32, total as u32),
            Constraint::Ratio(c as u32, total as u32),
            Constraint::Ratio(r as u32, total as u32),
        ])
        .split(main_chunks[0]);

    Some(pane_chunks[2])
}

fn render_kitty_image(
    backend: &mut CrosstermBackend<io::Stdout>,
    data: &[u8],
    width: u32,
    height: u32,
    format: ImageFormat,
    col: u16,
    row: u16,
) {
    let escape_seq = encode_kitty_image(data, width, height, format);
    // Move cursor to the preview pane interior position
    let _ = execute!(backend, MoveTo(col, row));
    let _ = backend.write_all(escape_seq.as_bytes());
    let _ = backend.flush();
}

fn clear_kitty_on_terminal(backend: &mut CrosstermBackend<io::Stdout>) {
    let _ = backend.write_all(clear_kitty_image().as_bytes());
    let _ = backend.flush();
}
