mod color;
pub mod commit_view;
mod diff_view;
mod render;
pub mod scroll;
pub mod unstaged_view;
pub mod update;
use crate::app_state::AppState;
use crate::external_command;
use color::setup_colors;
use pancurses::{curs_set, endwin, initscr, noecho, start_color};
use render::render;
use update::update_state;

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

            let old_file_cursor = state.file_cursor;
            let old_line_cursor = state.line_cursor;
            let old_scroll = state.scroll;
            let old_file_list_scroll = state.file_list_scroll;

            state.refresh_diff();

            state.file_cursor = old_file_cursor.min(state.files.len() + 1);
            if let Some(file) = state.current_file() {
                let max_line = file.lines.len().saturating_sub(1);
                state.line_cursor = old_line_cursor.min(max_line);
                state.scroll = old_scroll.min(max_line);
            } else {
                state.line_cursor = 0;
                state.scroll = 0;
            }
            state.file_list_scroll = old_file_list_scroll;

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
