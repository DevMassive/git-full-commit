use crate::app_state::AppState;
use crate::ui::main_screen;
use pancurses::Window;

pub fn render(window: &Window, state: &AppState) {
    window.erase();
    main_screen::render(window, state);
    window.refresh();
}
