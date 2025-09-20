use crate::app_state::{AppState, Screen};
use crate::git::FileStatus;
use pancurses::{Input, Window, COLOR_PAIR};

pub fn render_unstaged_view(window: &Window, state: &AppState) {
    window.clear();
    let (max_y, max_x) = window.get_max_yx();

    let mut line_y = 0;

    // Title for unstaged changes
    window.attron(COLOR_PAIR(1));
    window.mvaddstr(line_y, 0, " Unstaged changes");
    window.attroff(COLOR_PAIR(1));
    line_y += 1;

    // List of unstaged files
    for (i, file) in state.unstaged_files.iter().enumerate() {
        if line_y >= max_y {
            break;
        }
        let is_selected = state.unstaged_cursor == i + 1;
        let pair = if is_selected { 5 } else { 1 };
        let status_pair = if is_selected { 6 } else { 2 };

        window.attron(COLOR_PAIR(pair));
        if is_selected {
            for x in 0..max_x {
                window.mvaddch(line_y, x, ' ');
            }
        }
        window.mv(line_y, 0);
        window.attroff(COLOR_PAIR(pair));

        let status_char = match file.status {
            FileStatus::Added => 'A',
            FileStatus::Modified => 'M',
            FileStatus::Renamed => 'R',
            FileStatus::Deleted => 'D',
        };
        window.attron(COLOR_PAIR(pair));
        window.addstr("   ");
        window.attroff(COLOR_PAIR(pair));
        window.attron(COLOR_PAIR(status_pair));
        window.addstr(format!("{status_char}"));
        window.attroff(COLOR_PAIR(status_pair));
        window.attron(COLOR_PAIR(pair));
        window.addstr(format!(" {}", file.file_name));
        window.attroff(COLOR_PAIR(pair));
        line_y += 1;
    }

    // Title for untracked files
    if !state.untracked_files.is_empty() {
        if line_y < max_y {
            line_y += 1; // Add a space
        }
        if line_y < max_y {
            window.attron(COLOR_PAIR(1));
            window.mvaddstr(line_y, 0, " Untracked files");
            window.attroff(COLOR_PAIR(1));
            line_y += 1;
        }

        // List of untracked files
        for (i, file) in state.untracked_files.iter().enumerate() {
            if line_y >= max_y {
                break;
            }
            let cursor_index = state.unstaged_files.len() + i + 2;
            let is_selected = state.unstaged_cursor == cursor_index;
            let pair = if is_selected { 5 } else { 1 };

            window.attron(COLOR_PAIR(pair));
            if is_selected {
                for x in 0..max_x {
                    window.mvaddch(line_y, x, ' ');
                }
            }
            window.mv(line_y, 0);
            window.attroff(COLOR_PAIR(pair));

            window.attron(COLOR_PAIR(pair));
            window.addstr(format!("    ? {}", file));
            window.attroff(COLOR_PAIR(pair));
            line_y += 1;
        }
    }

    window.refresh();
}

pub fn handle_unstaged_view_input(state: &mut AppState, input: Input) {
    let unstaged_items_count = state.unstaged_files.len() + state.untracked_files.len() + 2;

    match input {
        Input::Character('q') | Input::Character('Q') => {
            state.screen = Screen::Main;
        }
        Input::KeyUp => {
            state.unstaged_cursor = state.unstaged_cursor.saturating_sub(1);
        }
        Input::KeyDown => {
            state.unstaged_cursor = state.unstaged_cursor.saturating_add(1).min(unstaged_items_count - 1);
        }
        _ => {}
    }
}
