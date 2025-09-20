use crate::app_state::AppState;
use crate::git::FileStatus;
use crate::ui::diff_view::{
    LINE_CONTENT_OFFSET, WordDiffLine, compute_word_diffs, get_scrolled_line,
};
use pancurses::{A_REVERSE, COLOR_PAIR, Window, chtype};
use unicode_width::UnicodeWidthStr;

pub fn render(window: &Window, state: &AppState) {
    window.clear();
    let (max_y, max_x) = window.get_max_yx();

    let num_files = state.files.len();
    let file_list_total_items = num_files + 3;
    let file_list_height = (max_y as usize / 3).max(3).min(file_list_total_items);

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
                staged_changes_text.push_str(" (Press R to re-add)");
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
            window.addstr(message);
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
                window.addstr(format!(" o {}", &state.previous_commit_message));
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
            let mut line_numbers: Vec<(Option<usize>, Option<usize>)> =
                vec![(None, None); all_lines.len()];
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
                    state,
                    line,
                    None,
                    line_index_in_file,
                    i as i32 + header_height as i32,
                    cursor_position,
                    old_line_num,
                    new_line_num,
                );
            }
        }
    } else if state.file_cursor > 0 && state.file_cursor <= num_files {
        let selected_file = &state.files[state.file_cursor - 1];
        let lines = &selected_file.lines;

        let mut line_numbers: Vec<(Option<usize>, Option<usize>)> = vec![(None, None); lines.len()];
        for hunk in &selected_file.hunks {
            for (hunk_line_index, (old, new)) in hunk.line_numbers.iter().enumerate() {
                let line_index = hunk.start_line + hunk_line_index;
                if line_index >= lines.len() {
                    continue;
                }
                line_numbers[line_index] = (*old, *new);
            }
        }

        let mut i = 0;
        let mut render_index = 0;
        while i < lines.len() {
            if render_index >= content_height {
                break;
            }

            let line = &lines[i];

            if line.starts_with('-') && !line.starts_with("--- ") {
                let mut minus_lines_indices = Vec::new();
                let mut current_pos = i;
                while current_pos < lines.len()
                    && lines[current_pos].starts_with('-')
                    && !lines[current_pos].starts_with("--- ")
                {
                    minus_lines_indices.push(current_pos);
                    current_pos += 1;
                }

                let mut plus_lines_indices = Vec::new();
                let mut next_pos = current_pos;
                while next_pos < lines.len()
                    && lines[next_pos].starts_with('+')
                    && !lines[next_pos].starts_with("+++ ")
                {
                    plus_lines_indices.push(next_pos);
                    next_pos += 1;
                }

                if !plus_lines_indices.is_empty() {
                    let old_text = minus_lines_indices
                        .iter()
                        .map(|&idx| &lines[idx][1..])
                        .collect::<Vec<_>>()
                        .join("\n");
                    let new_text = plus_lines_indices
                        .iter()
                        .map(|&idx| &lines[idx][1..])
                        .collect::<Vec<_>>()
                        .join("\n");

                    let (old_word_diffs, new_word_diffs) = compute_word_diffs(&old_text, &new_text);

                    for (k, &idx) in minus_lines_indices.iter().enumerate() {
                        if idx < state.scroll {
                            continue;
                        }
                        if render_index >= content_height {
                            break;
                        }
                        let (old_line_num, new_line_num) = line_numbers[idx];
                        render_line(
                            window,
                            state,
                            &lines[idx],
                            old_word_diffs.get(k),
                            idx,
                            render_index as i32 + header_height as i32,
                            cursor_position,
                            old_line_num,
                            new_line_num,
                        );
                        render_index += 1;
                    }

                    for (k, &idx) in plus_lines_indices.iter().enumerate() {
                        if idx < state.scroll {
                            continue;
                        }
                        if render_index >= content_height {
                            break;
                        }
                        let (old_line_num, new_line_num) = line_numbers[idx];
                        render_line(
                            window,
                            state,
                            &lines[idx],
                            new_word_diffs.get(k),
                            idx,
                            render_index as i32 + header_height as i32,
                            cursor_position,
                            old_line_num,
                            new_line_num,
                        );
                        render_index += 1;
                    }
                    i = next_pos;
                } else {
                    for &idx in &minus_lines_indices {
                        if idx < state.scroll {
                            continue;
                        }
                        if render_index >= content_height {
                            break;
                        }
                        let (old_line_num, new_line_num) = line_numbers[idx];
                        render_line(
                            window,
                            state,
                            &lines[idx],
                            None,
                            idx,
                            render_index as i32 + header_height as i32,
                            cursor_position,
                            old_line_num,
                            new_line_num,
                        );
                        render_index += 1;
                    }
                    i = current_pos;
                }
            } else {
                if i >= state.scroll {
                    let (old_line_num, new_line_num) = line_numbers[i];
                    render_line(
                        window,
                        state,
                        line,
                        None,
                        i,
                        render_index as i32 + header_height as i32,
                        cursor_position,
                        old_line_num,
                        new_line_num,
                    );
                    render_index += 1;
                }
                i += 1;
            }
        }
    }

    window.mv(carret_y, carret_x);
    window.refresh();
}

fn render_line(
    window: &Window,
    state: &AppState,
    line: &str,
    word_diff_line: Option<&WordDiffLine>,
    line_index_in_file: usize,
    line_render_index: i32,
    cursor_position: usize,
    old_line_num: Option<usize>,
    new_line_num: Option<usize>,
) {
    let is_cursor_line = line_index_in_file == cursor_position;

    let (default_pair, deletion_pair, addition_pair, hunk_header_pair, grey_pair) =
        if is_cursor_line {
            if state.is_diff_cursor_active {
                (5, 6, 7, 8, 10) // Active cursor pairs
            } else {
                (11, 12, 13, 14, 15) // Inactive cursor pairs
            }
        } else {
            (1, 2, 3, 4, 9) // Non-cursor pairs
        };

    let line_num_str = format!(
        "{:<4} {:<4}",
        if line.starts_with('+') {
            "".to_string()
        } else {
            old_line_num.map_or(String::new(), |n| n.to_string())
        },
        if line.starts_with('-') {
            "".to_string()
        } else {
            new_line_num.map_or(String::new(), |n| n.to_string())
        }
    );
    let line_content_offset = LINE_CONTENT_OFFSET as i32;

    window.mv(line_render_index, 0);
    window.clrtoeol();

    if is_cursor_line {
        window.attron(COLOR_PAIR(default_pair));
        for i in 0..window.get_max_x() {
            window.mvaddch(line_render_index, i, ' ');
        }
        window.attroff(COLOR_PAIR(default_pair));
    }

    let (base_pair, line_prefix) = if line.starts_with("--- ") || line.starts_with("+++ ") {
        (grey_pair, "")
    } else if line.starts_with('+') {
        (addition_pair, "+")
    } else if line.starts_with('-') {
        (deletion_pair, "-")
    } else if line.starts_with("@@ ") {
        (hunk_header_pair, "")
    } else if line.starts_with("diff --git ") {
        (default_pair, "")
    } else {
        (default_pair, " ")
    };

    let num_pair = if line.starts_with('+') || line.starts_with('-') {
        base_pair
    } else {
        grey_pair
    };

    window.attron(COLOR_PAIR(num_pair));
    window.mvaddstr(line_render_index, 0, &line_num_str);
    window.attroff(COLOR_PAIR(num_pair));

    window.mv(line_render_index, line_content_offset);

    let mut remaining_scroll = state.horizontal_scroll;

    let render_part = |win: &Window,
                       text: &str,
                       pair: chtype,
                       attr: pancurses::chtype,
                       remaining_scroll: &mut usize| {
        if *remaining_scroll == 0 {
            win.attron(COLOR_PAIR(pair));
            win.attron(attr);
            win.addstr(text);
            win.attroff(attr);
            win.attroff(COLOR_PAIR(pair));
        } else {
            let width = UnicodeWidthStr::width(text);
            if *remaining_scroll < width {
                let scrolled_text = get_scrolled_line(text, *remaining_scroll);
                win.attron(COLOR_PAIR(pair));
                win.attron(attr);
                win.addstr(scrolled_text);
                win.attroff(attr);
                win.attroff(COLOR_PAIR(pair));
                *remaining_scroll = 0;
            } else {
                *remaining_scroll -= width;
            }
        }
    };

    if line.starts_with("@@ ") {
        window.attroff(COLOR_PAIR(base_pair));
        window.attron(COLOR_PAIR(grey_pair));
        window.mvaddstr(line_render_index, 0, &line_num_str);
        window.attroff(COLOR_PAIR(grey_pair));

        if let Some(hunk_end_pos) = line.rfind("@@") {
            let hunk_header = &line[..hunk_end_pos + 2];
            let function_signature = &line[hunk_end_pos + 2..];

            render_part(
                window,
                hunk_header,
                hunk_header_pair,
                0,
                &mut remaining_scroll,
            );
            render_part(
                window,
                function_signature,
                addition_pair,
                0,
                &mut remaining_scroll,
            );
        } else {
            render_part(window, line, hunk_header_pair, 0, &mut remaining_scroll);
        }
    } else if let Some(word_diff) = word_diff_line {
        render_part(window, line_prefix, base_pair, 0, &mut remaining_scroll);
        for (text, is_changed) in &word_diff.0 {
            let attr = if *is_changed { A_REVERSE } else { 0 };
            render_part(window, text, base_pair, attr, &mut remaining_scroll);
        }
    } else {
        render_part(window, line, base_pair, 0, &mut remaining_scroll);
    }
}
