mod color;
pub mod commit_view;
mod diff_view;
mod keyboard;
pub mod main_screen;
mod render;
pub mod scroll;

pub mod update;
use crate::app_state::AppState;
use crate::external_command;
use color::setup_colors;
use pancurses::{Input, curs_set, endwin, initscr, noecho, start_color};
use render::render;
use std::io::Write;
use std::thread;
use std::time::Duration;
use update::update_state;

pub fn tui_loop(repo_path: std::path::PathBuf, files: Vec<crate::git::FileDiff>, debug: bool) {
    let mut window = initscr();
    window.keypad(true);
    noecho();
    curs_set(0);
    window.timeout(50);

    start_color();
    setup_colors();

    let mut state = AppState::new(repo_path, files);
    let mut needs_render = true;

    while state.running {
        if needs_render {
            render(&window, &state);
            needs_render = false;
        }

        if let Some(request) = state.editor_request.take() {
            endwin();
            let _ = external_command::open_editor(&request.file_path, request.line_number);

            state.refresh_diff(false);

            window = initscr();
            window.keypad(true);
            noecho();
            curs_set(0);
            window.timeout(50);
            start_color();
            setup_colors();
            needs_render = true;
            continue;
        }

        let input = window.getch();

        if input.is_none() {
            thread::sleep(Duration::from_millis(10));
            continue;
        }

        let (max_y, max_x) = window.get_max_yx();

        if debug {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("debug.log")
                .unwrap();
            writeln!(file, "Input: {input:?}").unwrap();
        }

        if let Some(Input::Character('\u{1b}')) = input {
            let next_char = window.getch();
            if debug {
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("debug.log")
                    .unwrap();
                writeln!(file, "Next char after ESC: {next_char:?}").unwrap();
            }
            if let Some(Input::Character('[')) = next_char {
                let third_char = window.getch();
                if debug {
                    let mut file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("debug.log")
                        .unwrap();
                    writeln!(file, "Third char after ESC [: {third_char:?}").unwrap();
                }
                if let Some(Input::Character('A')) = third_char {
                    state = update::update_state_with_alt(state, Some(Input::KeyUp), max_y, max_x);
                } else if let Some(Input::Character('B')) = third_char {
                    state =
                        update::update_state_with_alt(state, Some(Input::KeyDown), max_y, max_x);
                } else {
                    state = update_state(state, third_char, max_y, max_x);
                }
            } else if let Some(next_input) = next_char {
                state = update::update_state_with_alt(state, Some(next_input), max_y, max_x);
            } else {
                state = update_state(state, input, max_y, max_x);
            }
        } else {
            state = update_state(state, input, max_y, max_x);
        }
        needs_render = true;
    }

    endwin();
}
