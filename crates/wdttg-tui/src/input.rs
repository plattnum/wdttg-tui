use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::app::Screen;

/// Map a key event to an action, considering the current screen.
pub fn handle_key(key: KeyEvent, _screen: Screen, show_help: bool) -> Option<Action> {
    // Help popup overrides: Esc or ? closes it
    if show_help {
        return match key.code {
            KeyCode::Esc | KeyCode::Char('?') => Some(Action::ClosePopup),
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::Quit)
            }
            _ => None,
        };
    }

    // Global keys
    match key.code {
        KeyCode::Char('q') => return Some(Action::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Some(Action::Quit);
        }
        KeyCode::Char('1') => return Some(Action::SwitchToTimeline),
        KeyCode::Char('2') => return Some(Action::SwitchToReports),
        KeyCode::Char('3') => return Some(Action::SwitchToManage),
        KeyCode::Char('?') => return Some(Action::ToggleHelp),
        KeyCode::Esc => return Some(Action::ClosePopup),
        _ => {}
    }

    // Navigation and screen-specific actions
    match key.code {
        KeyCode::Char('H') => Some(Action::ScrollWeekLeft),
        KeyCode::Char('L') => Some(Action::ScrollWeekRight),
        KeyCode::Left | KeyCode::Char('h') => Some(Action::NavigateLeft),
        KeyCode::Right | KeyCode::Char('l') => Some(Action::NavigateRight),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::NavigateUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::NavigateDown),
        KeyCode::PageUp => Some(Action::PageUp),
        KeyCode::PageDown => Some(Action::PageDown),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::PageUp),
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::PageDown)
        }
        KeyCode::Enter => Some(Action::Select),
        KeyCode::Char('t') => Some(Action::JumpToToday),
        KeyCode::Char('A') => Some(Action::ToggleArchive),
        KeyCode::Char('n') | KeyCode::Char('a') => Some(Action::Create),
        KeyCode::Char('e') => Some(Action::Edit),
        KeyCode::Char('d') => Some(Action::Delete),
        KeyCode::Char(' ') => Some(Action::MarkTime),
        _ => None,
    }
}
