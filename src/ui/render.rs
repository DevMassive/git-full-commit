use crate::app_state::{AppState, Screen};
use crate::ui::main_screen;
use crate::ui::unstaged_screen;
use pancurses::Window;

pub fn render(window: &Window, state: &AppState) {
    match state.screen {
        Screen::Main => {
            main_screen::render(window, state);
        }
        Screen::Unstaged => {
            unstaged_screen::render(window, state);
        }
    }
}
