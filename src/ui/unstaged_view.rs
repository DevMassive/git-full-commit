use crate::app_state::{AppState, Screen};
use crate::git::FileStatus;
use crate::ui::diff_view::render_diff_view;
use crate::ui::scroll;
use pancurses::{COLOR_PAIR, Input, Window};pub fn render_unstaged_view(window: &Window, state: &AppState) {
    window.clear();
    let (max_y, max_x) = window.get_max_yx();

    let unstaged_file_count = state.unstaged_files.len();
    let untracked_file_count = state.untracked_files.len();
    let file_list_total_items = unstaged_file_count + untracked_file_count + 2;
    let file_list_height = (max_y as usize / 3).max(3).min(file_list_total_items);

    // Render file list
    for i in 0..file_list_height {
        let item_index = state.unstaged_scroll + i;
        if item_index >= file_list_total_items {
            break;
        }
        let line_y = i as i32;

        if item_index == 0 {
            window.attron(COLOR_PAIR(1));
            window.mvaddstr(line_y, 0, " Unstaged changes");
            window.attroff(COLOR_PAIR(1));
        } else if item_index > 0 && item_index <= unstaged_file_count {
            let file_index = item_index - 1;
            let file = &state.unstaged_files[file_index];
            let is_selected = state.unstaged_cursor == item_index;
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
        } else if item_index == unstaged_file_count + 1 {
            window.attron(COLOR_PAIR(1));
            window.mvaddstr(line_y, 0, " Untracked files");
            window.attroff(COLOR_PAIR(1));
        } else {
            let file_index = item_index - unstaged_file_count - 2;
            let file = &state.untracked_files[file_index];
            let is_selected = state.unstaged_cursor == item_index;
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
            window.addstr(format!("    ? {file}"));
            window.attroff(COLOR_PAIR(pair));
        }
    }

    // Render separator
    let separator_y = file_list_height as i32;
    window.mv(separator_y, 0);
    window.attron(COLOR_PAIR(9));
    window.hline(pancurses::ACS_HLINE(), max_x);
    window.attroff(COLOR_PAIR(9));

    // Render diff view
    if state.unstaged_cursor > 0 && state.unstaged_cursor <= unstaged_file_count {
        let file_index = state.unstaged_cursor - 1;
        if let Some(file) = state.unstaged_files.get(file_index) {
            let content_height = (max_y as usize).saturating_sub(file_list_height + 1);
            render_diff_view(
                window,
                file,
                content_height,
                state.unstaged_diff_scroll,
                state.unstaged_horizontal_scroll,
                file_list_height + 1,
                state.line_cursor, // Using main line_cursor for now
                true,              // Diff cursor is always active here
            );
        }
    }

    window.refresh();
}

pub fn handle_unstaged_view_input(state: &mut AppState, input: Input) {
    let unstaged_file_count = state.unstaged_files.len();
    let untracked_file_count = state.untracked_files.len();
    let unstaged_items_count = unstaged_file_count + untracked_file_count + 2;

    let (max_y, _) = pancurses::initscr().get_max_yx();
    let file_list_height = (max_y as usize / 3).max(3).min(unstaged_items_count);

    match input {
        Input::Character('q') | Input::Character('Q') => {
            state.screen = Screen::Main;
            state.line_cursor = 0;
            state.scroll = 0;
        }
        Input::KeyUp => {
            state.unstaged_cursor = state.unstaged_cursor.saturating_sub(1);
            state.unstaged_diff_scroll = 0;
            state.line_cursor = 0;
            if state.unstaged_cursor < state.unstaged_scroll {
                state.unstaged_scroll = state.unstaged_cursor;
            }
        }
        Input::KeyDown => {
            state.unstaged_cursor = state
                .unstaged_cursor
                .saturating_add(1)
                .min(unstaged_items_count - 1);
            state.unstaged_diff_scroll = 0;
            state.line_cursor = 0;
            if state.unstaged_cursor >= state.unstaged_scroll + file_list_height {
                state.unstaged_scroll = state.unstaged_cursor - file_list_height + 1;
            }
        }
        Input::Character('k') => {
            state.line_cursor = state.line_cursor.saturating_sub(1);
            if state.line_cursor < state.unstaged_diff_scroll {
                state.unstaged_diff_scroll = state.line_cursor;
            }
        }
        Input::Character('j') => {
            if state.unstaged_cursor > 0 && state.unstaged_cursor <= unstaged_file_count {
                let file_index = state.unstaged_cursor - 1;
                if let Some(file) = state.unstaged_files.get(file_index) {
                    if state.line_cursor < file.lines.len().saturating_sub(1) {
                        state.line_cursor += 1;
                        let content_height = (max_y as usize).saturating_sub(file_list_height + 1);
                        if state.line_cursor >= state.unstaged_diff_scroll + content_height {
                            state.unstaged_diff_scroll = state.line_cursor - content_height + 1;
                        }
                    }
                }
            }
        }
        Input::KeyLeft => {
            state.unstaged_horizontal_scroll = state.unstaged_horizontal_scroll.saturating_sub(10);
        }
        Input::KeyRight => {
            state.unstaged_horizontal_scroll = state.unstaged_horizontal_scroll.saturating_add(10);
        }
        _ => scroll::handle_scroll(state, input, max_y),
    }
}
