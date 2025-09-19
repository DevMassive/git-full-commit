use crate::app_state::AppState;
use crate::git::FileStatus;
use crate::ui::diff_view::{
    compute_word_diffs,
    get_scrolled_line,
    WordDiffLine,
    LINE_CONTENT_OFFSET,
};
use pancurses::{chtype, A_REVERSE, COLOR_PAIR, Window};
use unicode_width::UnicodeWidthStr;

pub fn render(window: &Window, state: &AppState) {
    window.clear();
    let (max_y, max_x) = window.get_max_yx();

    let num_files = state.files.len();

    // Render previous commit info
    let prev_commit_line_y = 0;
    let is_selected_prev_commit = state.file_cursor == 0;
    let pair = if is_selected_prev_commit { 5 } else { 1 };
    window.attron(COLOR_PAIR(pair));
    window.mv(prev_commit_line_y, 0);
    if is_selected_prev_commit {
        for x in 0..max_x {
            window.mvaddch(prev_commit_line_y, x, ' ');
        }
        window.mv(prev_commit_line_y, 0);
    } else {
        window.clrtoeol();
    }
    window.addstr(&format!(" o {}", &state.previous_commit_message));
    window.attroff(COLOR_PAIR(pair));

    // Render sticky header
    if !state.files.is_empty() {
        for (i, file) in state.files.iter().enumerate() {
            let file_line_y = i as i32 + 1;
            let is_selected_file = state.file_cursor == i + 1;
            let pair = if is_selected_file { 5 } else { 1 };
            let status_pair = if is_selected_file { 6 } else { 2 };
            window.attron(COLOR_PAIR(pair));
            window.mv(file_line_y, 0);
            if is_selected_file {
                for x in 0..max_x {
                    window.mvaddch(file_line_y, x, ' ');
                }
                window.mv(file_line_y, 0);
            } else {
                window.clrtoeol();
            }
            window.attroff(COLOR_PAIR(pair));
            let status_char = match file.status {
                FileStatus::Added => 'A',
                FileStatus::Modified => 'M',
                FileStatus::Renamed => 'R',
                FileStatus::Deleted => 'D',
            };
            window.attron(COLOR_PAIR(pair));
            window.addstr(" | ");
            window.attroff(COLOR_PAIR(pair));
            window.attron(COLOR_PAIR(status_pair));
            window.addstr(&format!("{}", status_char));
            window.attroff(COLOR_PAIR(status_pair));
            window.attron(COLOR_PAIR(pair));
            window.addstr(&format!(" {}", file.file_name));
            window.attroff(COLOR_PAIR(pair));
        }
    }

    // Render commit message line
    let commit_line_y = (num_files + 1) as i32;
    let is_selected = state.file_cursor == num_files + 1;
    let pair = if is_selected { 5 } else { 1 };
    window.attron(COLOR_PAIR(pair));
    window.mv(commit_line_y, 0);
    if is_selected {
        for x in 0..max_x {
            window.mvaddch(commit_line_y, x, ' ');
        }
        window.mv(commit_line_y, 0);
    } else {
        window.clrtoeol();
    }

    let (prefix, message) = if state.is_amend_mode {
        (" A ", &state.amend_message)
    } else {
        (" o ", &state.commit_message)
    };

    window.addstr(prefix);
    window.addstr(message);
    window.attroff(COLOR_PAIR(pair));

    // Render separator
    window.mv((num_files + 2) as i32, 0);
    window.attron(COLOR_PAIR(9));
    window.hline(pancurses::ACS_HLINE(), max_x);
    window.attroff(COLOR_PAIR(9));

    let header_height = num_files + 3;
    let content_height = (max_y as usize).saturating_sub(header_height);
    let cursor_position = state.get_cursor_line_index();

    if state.file_cursor == 0 {
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
                    let mut old_line_counter = hunk.old_start;
                    let mut new_line_counter = hunk.new_start;

                    for (hunk_line_index, hunk_line) in hunk.lines.iter().enumerate() {
                        let line_index = line_offset + hunk.start_line + hunk_line_index;
                        if line_index >= all_lines.len() {
                            continue;
                        }

                        if hunk_line.starts_with('+') {
                            line_numbers[line_index] = (None, Some(new_line_counter));
                            new_line_counter += 1;
                        } else if hunk_line.starts_with('-') {
                            line_numbers[line_index] = (Some(old_line_counter), None);
                            old_line_counter += 1;
                        } else if !hunk_line.starts_with("@@") {
                            line_numbers[line_index] =
                                (Some(old_line_counter), Some(new_line_counter));
                            old_line_counter += 1;
                            new_line_counter += 1;
                        }
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
            let mut old_line_counter = hunk.old_start;
            let mut new_line_counter = hunk.new_start;

            for (hunk_line_index, hunk_line) in hunk.lines.iter().enumerate() {
                let line_index = hunk.start_line + hunk_line_index;
                if line_index >= lines.len() {
                    continue;
                }

                if hunk_line.starts_with('+') {
                    line_numbers[line_index] = (None, Some(new_line_counter));
                    new_line_counter += 1;
                } else if hunk_line.starts_with('-') {
                    line_numbers[line_index] = (Some(old_line_counter), None);
                    old_line_counter += 1;
                } else if !hunk_line.starts_with("@@") {
                    line_numbers[line_index] = (Some(old_line_counter), Some(new_line_counter));
                    old_line_counter += 1;
                    new_line_counter += 1;
                }
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
    } else if state.file_cursor == num_files + 1 {
        if state.is_commit_mode {
            let (prefix, message) = if state.is_amend_mode {
                (" A ", &state.amend_message)
            } else {
                (" o ", &state.commit_message)
            };
            let prefix_width = prefix.width();
            let message_before_cursor: String = message.chars().take(state.commit_cursor).collect();
            let cursor_display_pos = prefix_width + message_before_cursor.width();
            window.mv(commit_line_y, cursor_display_pos as i32);
        }
    }
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

    let default_pair: chtype = if is_cursor_line { 5 } else { 1 };
    let deletion_pair: chtype = if is_cursor_line { 6 } else { 2 };
    let addition_pair: chtype = if is_cursor_line { 7 } else { 3 };
    let hunk_header_pair: chtype = if is_cursor_line { 8 } else { 4 };
    let grey_pair: chtype = if is_cursor_line { 10 } else { 9 };

    let line_num_str = format!(
        "{:<4} {:<4}",
        old_line_num.map_or(String::new(), |n| n.to_string()),
        new_line_num.map_or(String::new(), |n| n.to_string())
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
