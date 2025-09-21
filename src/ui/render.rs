use crate::app_state::{AppState, Screen};
use crate::git::FileStatus;
use crate::ui::diff_view::{render_diff_view, render_line};
use crate::ui::unstaged_view::render_unstaged_view;
use pancurses::{COLOR_PAIR, Window};
use unicode_width::UnicodeWidthStr;

pub fn render(window: &Window, state: &AppState) {
    match state.screen {
        Screen::Main => {
            render_main_view(window, state);
        }
        Screen::Unstaged => {
            render_unstaged_view(window, state);
        }
    }
}

fn render_main_view(window: &Window, state: &AppState) {
    window.erase();
    let (max_y, max_x) = window.get_max_yx();

    let (file_list_height, file_list_total_items) = state.main_header_height(max_y);
    let num_files = state.files.len();

    let mut carret_y = 0;
    let mut carret_x = 0;

    for i in 0..file_list_height {
        let item_index = state.file_list_scroll + i;
        if item_index >= file_list_total_items {
            break;
        }
        let line_y = i as i32;

        if item_index == 0 {
            // Render "Staged changes"
            let is_selected = state.file_cursor == 0;
            let pair = if is_selected { 5 } else { 1 };
            window.attron(COLOR_PAIR(pair));
            if is_selected {
                for x in 0..max_x {
                    window.mvaddch(line_y, x, ' ');
                }
            }
            window.mv(line_y, 0);
            let mut staged_changes_text = " Staged changes".to_string();
            if state.has_unstaged_changes {
                staged_changes_text.push_str(" (press ENTER to view)");
            }
            window.addstr(&staged_changes_text);
            window.attroff(COLOR_PAIR(pair));
        } else if item_index > 0 && item_index <= num_files {
            let file_index = item_index - 1;
            let file = &state.files[file_index];
            let is_selected = state.file_cursor == item_index;
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
        } else if item_index == num_files + 1 {
            // Render commit message line
            let is_selected = state.file_cursor == num_files + 1;
            let pair = if is_selected { 5 } else { 1 };
            window.attron(COLOR_PAIR(pair));
            if is_selected {
                for x in 0..max_x {
                    window.mvaddch(line_y, x, ' ');
                }
            }
            window.mv(line_y, 0);

            let (prefix, message) = if state.is_amend_mode {
                (" o ", &state.amend_message)
            } else {
                (" o ", &state.commit_message)
            };

            window.addstr(prefix);
            if message.is_empty() {
                let pair = if is_selected { 16 } else { 9 };
                window.attron(COLOR_PAIR(pair));
                window.addstr("Enter commit message...");
                window.attroff(COLOR_PAIR(pair));
            } else {
                window.addstr(message);
            }
            window.attroff(COLOR_PAIR(pair));

            let commit_line_y = line_y;
            let prefix_width = prefix.width();
            let message_before_cursor: String = message.chars().take(state.commit_cursor).collect();
            let cursor_display_pos = prefix_width + message_before_cursor.width();

            carret_y = commit_line_y;
            carret_x = cursor_display_pos as i32;
        } else if item_index == num_files + 2 {
            // Render previous commit info
            let is_selected = state.file_cursor == num_files + 2;
            let pair = if is_selected { 5 } else { 1 };
            window.attron(COLOR_PAIR(pair));
            if is_selected {
                for x in 0..max_x {
                    window.mvaddch(line_y, x, ' ');
                }
            }
            window.mv(line_y, 0);
            if state.is_amend_mode {
                window.addstr(" |");
            } else {
                let status = if state.previous_commit_is_on_remote {
                    "(remote)"
                } else {
                    "(local)"
                };
                window.addstr(format!(" o {} {}", status, &state.previous_commit_message));
            }
            window.attroff(COLOR_PAIR(pair));
        }
    }

    // Render separator
    let separator_y = file_list_height as i32;
    window.mv(separator_y, 0);
    window.attron(COLOR_PAIR(9));
    window.hline(pancurses::ACS_HLINE(), max_x);
    window.attroff(COLOR_PAIR(9));

    let header_height = file_list_height + 1;
    let content_height = (max_y as usize).saturating_sub(header_height);
    let cursor_position = state.get_cursor_line_index();

    if state.file_cursor == 0 {
        // "Staged changes" is selected, do nothing for now.
    } else if state.file_cursor == num_files + 2 {
        // Render previous commit diff
        let all_lines: Vec<String> = state
            .previous_commit_files
            .iter()
            .flat_map(|f| f.lines.clone())
            .collect();
        if !all_lines.is_empty() {
            let mut line_numbers: Vec<(usize, usize)> = vec![(0, 0); all_lines.len()];
            let mut line_offset = 0;
            for file in &state.previous_commit_files {
                for hunk in &file.hunks {
                    for (hunk_line_index, (old, new)) in hunk.line_numbers.iter().enumerate() {
                        let line_index = line_offset + hunk.start_line + hunk_line_index;
                        if line_index >= all_lines.len() {
                            continue;
                        }
                        line_numbers[line_index] = (*old, *new);
                    }
                }
                line_offset += file.lines.len();
            }

            for (i, line) in all_lines
                .iter()
                .skip(state.scroll)
                .take(content_height)
                .enumerate()
            {
                let line_index_in_file = i + state.scroll;
                let (old_line_num, new_line_num) = line_numbers[line_index_in_file];
                render_line(
                    window,
                    line,
                    None,
                    line_index_in_file,
                    i as i32 + header_height as i32,
                    cursor_position,
                    old_line_num,
                    new_line_num,
                    state.horizontal_scroll,
                    state.is_diff_cursor_active,
                );
            }
        }
    } else if state.file_cursor > 0 && state.file_cursor <= num_files {
        let selected_file = &state.files[state.file_cursor - 1];
        render_diff_view(
            window,
            selected_file,
            content_height,
            state.scroll,
            state.horizontal_scroll,
            header_height,
            cursor_position,
            state.is_diff_cursor_active,
        );
    }

    if state.file_cursor == num_files + 1 {
        #[cfg(not(test))]
        pancurses::curs_set(1);
        window.mv(carret_y, carret_x);
    } else {
        #[cfg(not(test))]
        pancurses::curs_set(0);
    }

    window.refresh();
}
