mod color;
pub mod commit_view;
mod diff_view;
mod render;
pub mod scroll;
pub mod main_screen;
pub mod unstaged_screen;
pub mod update;
use crate::app_state::AppState;
use crate::external_command;
use color::setup_colors;
use pancurses::{curs_set, endwin, initscr, noecho, start_color};
use render::render;
use update::update_state;

#[cfg(test)]
mod update_test;

pub fn tui_loop(repo_path: std::path::PathBuf, files: Vec<crate::git::FileDiff>) {
    let mut window = initscr();
    window.keypad(true);
    noecho();
    curs_set(0);

    start_color();
    setup_colors();

    let mut state = AppState::new(repo_path, files);

    while state.running {
        if let Some(request) = state.editor_request.take() {
            endwin();
            let _ = external_command::open_editor(&request.file_path, request.line_number);

            state.refresh_diff();

            window = initscr();
            window.keypad(true);
            noecho();
            curs_set(0);
            start_color();
            setup_colors();
            continue;
        }

        render(&window, &state);
        let input = window.getch();
        let (max_y, max_x) = window.get_max_yx();
        state = update_state(state, input, max_y, max_x);
    }

    endwin();
}
