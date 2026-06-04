use crate::app::App;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use kite_core::event::AppCommand;
use kite_core::keymap::{command_to_app_command, KeyCombo, KeymapResult};

pub fn handle_event(app: &mut App, event: Event) -> Option<AppCommand> {
    match event {
        Event::Key(key_event) => handle_key(app, key_event),
        Event::Resize(_, _) => Some(AppCommand::Refresh),
        _ => None,
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> Option<AppCommand> {
    // If in input mode, handle character input directly
    if app.is_input_mode() {
        return handle_input_key(app, key);
    }

    // Convert to our key combo
    let combo = KeyCombo::from_crossterm(key.code, key.modifiers)?;
    let context = app.key_context();

    // Feed through keymap
    match app.keymap.feed_key(combo, &context) {
        KeymapResult::Command(cmd_str) => command_to_app_command(&cmd_str),
        KeymapResult::Pending => None,
        KeymapResult::Unhandled => None,
    }
}

fn handle_input_key(app: &mut App, key: KeyEvent) -> Option<AppCommand> {
    match key.code {
        KeyCode::Enter => Some(AppCommand::ConfirmInput),
        KeyCode::Esc => Some(AppCommand::CancelInput),
        KeyCode::Backspace => {
            app.handle_input_backspace();
            None
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    'c' => Some(AppCommand::CancelInput),
                    'n' => Some(AppCommand::SearchNext),
                    'p' => Some(AppCommand::SearchPrev),
                    _ => None,
                }
            } else {
                app.handle_input_char(c);
                None
            }
        }
        _ => None,
    }
}
