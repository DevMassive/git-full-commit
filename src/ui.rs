use pancurses::{A_REVERSE, COLOR_BLACK, COLOR_PAIR, Input, Window, curs_set, endwin, init_color, init_pair, initscr, noecho, start_color};
use unicode_width::UnicodeWidthStr;
use std::process::Command as OsCommand;
use similar::TextDiff;

// Represents a line of text with word-level diff information.
// Each element in the vector is a tuple of (text, is_changed).
#[derive(Debug, Clone, PartialEq)]
pub struct WordDiffLine(pub Vec<(String, bool)>);

use crate::app_state::AppState;
use crate::commit_storage;
use crate::git::{FileStatus, get_previous_commit_message};
use crate::command::{
    ApplyPatchCommand, CheckoutFileCommand, IgnoreFileCommand, RemoveFileCommand,
    UnstageFileCommand,
};

fn compute_word_diffs(old: &str, new: &str) -> (Vec<WordDiffLine>, Vec<WordDiffLine>) {
    if old.trim().is_empty() || new.trim().is_empty() {
        let old_lines = old
            .lines()
            .map(|l| WordDiffLine(vec![(l.to_string(), false)]))
            .collect();
        let new_lines = new
            .lines()
            .map(|l| WordDiffLine(vec![(l.to_string(), false)]))
            .collect();
        return (old_lines, new_lines);
    }

    let diff = TextDiff::from_unicode_words(old, new);

    if diff.ratio() < 0.7 {
        let old_lines = old
            .lines()
            .map(|l| WordDiffLine(vec![(l.to_string(), false)]))
            .collect();
        let new_lines = new
            .lines()
            .map(|l| WordDiffLine(vec![(l.to_string(), false)]))
            .collect();
        return (old_lines, new_lines);
    }

    let mut old_line_parts = Vec::new();
    let mut new_line_parts = Vec::new();

    for change in diff.iter_all_changes() {
        let text = change.value().to_string();
        match change.tag() {
            similar::ChangeTag::Delete => old_line_parts.push((text, true)),
            similar::ChangeTag::Insert => new_line_parts.push((text, true)),
            similar::ChangeTag::Equal => {
                old_line_parts.push((text.clone(), false));
                new_line_parts.push((text, false));
            }
        }
    }

    let mut old_lines = Vec::new();
    let mut current_line = WordDiffLine(Vec::new());
    for (text, changed) in old_line_parts {
        let mut parts = text.split_inclusive('\n').peekable();
        while let Some(part) = parts.next() {
            let content = part.strip_suffix('\n').unwrap_or(part);
            if !content.is_empty() {
                current_line.0.push((content.to_string(), changed));
            }
            if part.ends_with('\n') {
                old_lines.push(current_line);
                current_line = WordDiffLine(Vec::new());
            }
        }
    }
    if !current_line.0.is_empty() {
        old_lines.push(current_line);
    }

    let mut new_lines = Vec::new();
    current_line = WordDiffLine(Vec::new());
    for (text, changed) in new_line_parts {
        let mut parts = text.split_inclusive('\n').peekable();
        while let Some(part) = parts.next() {
            let content = part.strip_suffix('\n').unwrap_or(part);
            if !content.is_empty() {
                current_line.0.push((content.to_string(), changed));
            }
            if part.ends_with('\n') {
                new_lines.push(current_line);
                current_line = WordDiffLine(Vec::new());
            }
        }
    }
    if !current_line.0.is_empty() {
        new_lines.push(current_line);
    }

    (old_lines, new_lines)
}

pub fn tui_loop(repo_path: std::path::PathBuf, files: Vec<crate::git::FileDiff>) {
    let window = initscr();
    window.keypad(true);
    noecho();
    curs_set(0);

    start_color();
    // Base colors
    let color_white = 20;
    let color_red = 21;
    let color_green = 22;
    let color_cyan = 23;
    let color_selected_bg = 24;
    let color_grey = 25;

    init_color(color_white, 968, 968, 941); // #F7F7F0
    init_color(color_red, 1000, 0, 439); // #FF0070
    init_color(color_green, 525, 812, 0); // #86CF00
    init_color(color_cyan, 0, 769, 961); // #00C4F5
    init_color(color_selected_bg, 133, 133, 133); // #222222
    init_color(color_grey, 266, 266, 266); // #444444

    // Color pairs
    init_pair(1, color_white, COLOR_BLACK); // Default: White on Black
    init_pair(2, color_red, COLOR_BLACK); // Deletion: Red on Black
    init_pair(3, color_green, COLOR_BLACK); // Addition: Green on Black
    init_pair(4, color_cyan, COLOR_BLACK); // Hunk Header: Cyan on Black
    init_pair(9, color_grey, COLOR_BLACK); // Grey on Black

    // Selected line pairs
    init_pair(5, color_white, color_selected_bg); // Default: White on #222222
    init_pair(6, color_red, color_selected_bg); // Deletion: Red on #222222
    init_pair(7, color_green, color_selected_bg); // Addition: Green on #222222
    init_pair(8, color_cyan, color_selected_bg); // Hunk Header: Cyan on #222222
    init_pair(10, color_grey, color_selected_bg); // Grey on #222222

    let mut state = AppState::new(repo_path, files);

    while state.running {
        render(&window, &state);
        let input = window.getch();
        let (max_y, _) = window.get_max_yx();
        state = update_state(state, input, max_y);
    }

    endwin();
}

fn render(window: &Window, state: &AppState) {
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
    window.addstr(&format!(
        "{} {}",
        &state.previous_commit_hash, &state.previous_commit_message
    ));
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
        ("Amend: ", &state.amend_message)
    } else {
        ("Commit: ", &state.commit_message)
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
        let all_lines: Vec<String> = state.previous_commit_files.iter().flat_map(|f| f.lines.clone()).collect();
        if !all_lines.is_empty() {
            let mut line_numbers: Vec<(Option<usize>, Option<usize>)> = vec![(None, None); all_lines.len()];
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
                            line_numbers[line_index] = (Some(old_line_counter), Some(new_line_counter));
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

                    let (old_word_diffs, new_word_diffs) =
                        compute_word_diffs(&old_text, &new_text);

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
                ("Amend: ", &state.amend_message)
            } else {
                ("Commit: ", &state.commit_message)
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
    _state: &AppState,
    line: &str,
    word_diff_line: Option<&WordDiffLine>,
    line_index_in_file: usize,
    line_render_index: i32,
    cursor_position: usize,
    old_line_num: Option<usize>,
    new_line_num: Option<usize>,
) {
    let is_cursor_line = line_index_in_file == cursor_position;

    let default_pair = if is_cursor_line { 5 } else { 1 };
    let deletion_pair = if is_cursor_line { 6 } else { 2 };
    let addition_pair = if is_cursor_line { 7 } else { 3 };
    let hunk_header_pair = if is_cursor_line { 8 } else { 4 };
    let grey_pair = if is_cursor_line { 10 } else { 9 };

    let line_num_str = format!(
        "{:>4} {:>4}",
        old_line_num.map_or(String::new(), |n| n.to_string()),
        new_line_num.map_or(String::new(), |n| n.to_string())
    );
    let line_content_offset = 10;

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

    let num_pair = if line.starts_with("@@ ") || line.starts_with("--- ") || line.starts_with("+++ ") {
        grey_pair
    } else {
        base_pair
    };

    window.attron(COLOR_PAIR(num_pair));
    window.mvaddstr(line_render_index, 0, &line_num_str);
    window.attroff(COLOR_PAIR(num_pair));

    window.attron(COLOR_PAIR(base_pair));
    window.mv(line_render_index, line_content_offset);

    if line.starts_with("@@ ") {
        window.attroff(COLOR_PAIR(base_pair));
        window.attron(COLOR_PAIR(grey_pair));
        window.mvaddstr(line_render_index, 0, &line_num_str);
        window.attroff(COLOR_PAIR(grey_pair));

        if let Some(hunk_end_pos) = line.rfind("@@") {
            let hunk_header = &line[..hunk_end_pos + 2];
            let function_signature = &line[hunk_end_pos + 2..];

            window.attron(COLOR_PAIR(hunk_header_pair));
            window.mvaddstr(line_render_index, line_content_offset, hunk_header);
            window.attroff(COLOR_PAIR(hunk_header_pair));

            window.attron(COLOR_PAIR(addition_pair));
            window.addstr(function_signature);
            window.attroff(COLOR_PAIR(addition_pair));
        } else {
            window.attron(COLOR_PAIR(hunk_header_pair));
            window.mvaddstr(line_render_index, line_content_offset, line);
            window.attroff(COLOR_PAIR(hunk_header_pair));
        }
    } else if let Some(word_diff) = word_diff_line {
        window.addstr(line_prefix);
        for (text, is_changed) in &word_diff.0 {
            if *is_changed {
                window.attron(A_REVERSE);
            }
            window.addstr(text);
            if *is_changed {
                window.attroff(A_REVERSE);
            }
        }
    } else {
        window.mvaddstr(line_render_index, line_content_offset, line);
    }
    window.attroff(COLOR_PAIR(base_pair));
}

pub fn update_state(mut state: AppState, input: Option<Input>, max_y: i32) -> AppState {
    if state.is_commit_mode {
        match input {
            Some(Input::KeyUp) => {
                state.is_commit_mode = false;
                #[cfg(not(test))]
                curs_set(0);
                state.file_cursor = state.files.len();
                state.line_cursor = 0;
                state.scroll = 0;
                return state;
            }
            Some(Input::Character('\t')) => {
                state.is_amend_mode = !state.is_amend_mode;
                if state.is_amend_mode {
                    // Switched to amend mode
                    if state.amend_message.is_empty() {
                        state.amend_message =
                            get_previous_commit_message(&state.repo_path).unwrap_or_default();
                    }
                    state.commit_cursor = state.amend_message.chars().count();
                } else {
                    // Switched back to commit mode
                    state.commit_cursor = state.commit_message.chars().count();
                }
                return state;
            }
            Some(Input::Character('\n')) => {
                if state.is_amend_mode {
                    if state.amend_message.is_empty() {
                        return state;
                    }
                    OsCommand::new("git")
                        .arg("commit")
                        .arg("--amend")
                        .arg("-m")
                        .arg(&state.amend_message)
                        .current_dir(&state.repo_path)
                        .output()
                        .expect("Failed to amend commit.");
                    let _ = commit_storage::delete_commit_message(&state.repo_path);
                    state.command_history.clear();
                } else {
                    if state.commit_message.is_empty() {
                        return state;
                    }
                    OsCommand::new("git")
                        .arg("commit")
                        .arg("-m")
                        .arg(&state.commit_message)
                        .current_dir(&state.repo_path)
                        .output()
                        .expect("Failed to commit.");
                    let _ = commit_storage::delete_commit_message(&state.repo_path);
                    state.commit_message.clear();
                    state.command_history.clear();
                }

                state.amend_message =
                    get_previous_commit_message(&state.repo_path).unwrap_or_default();

                OsCommand::new("git")
                    .arg("add")
                    .arg("-A")
                    .current_dir(&state.repo_path)
                    .output()
                    .expect("Failed to git add -A.");

                let staged_diff_output = OsCommand::new("git")
                    .arg("diff")
                    .arg("--staged")
                    .current_dir(&state.repo_path)
                    .output()
                    .expect("Failed to git diff --staged.");

                if staged_diff_output.stdout.is_empty() {
                    state.running = false;
                } else {
                    state.refresh_diff();
                    state.is_commit_mode = false;
                    #[cfg(not(test))]
                    curs_set(0);
                }

                return state;
            }
            Some(Input::KeyBackspace) => {
                if state.commit_cursor > 0 {
                    let message = if state.is_amend_mode {
                        &mut state.amend_message
                    } else {
                        &mut state.commit_message
                    };
                    let char_index_to_remove = state.commit_cursor - 1;
                    if let Some((byte_index, _)) = message.char_indices().nth(char_index_to_remove)
                    {
                        message.remove(byte_index);
                        state.commit_cursor -= 1;
                        if !state.is_amend_mode {
                            let _ = commit_storage::save_commit_message(
                                &state.repo_path,
                                &state.commit_message,
                            );
                        }
                    }
                }
                return state;
            }
            Some(Input::KeyDC) => {
                let message = if state.is_amend_mode {
                    &mut state.amend_message
                } else {
                    &mut state.commit_message
                };
                if state.commit_cursor < message.chars().count() {
                    if let Some((byte_index, _)) = message.char_indices().nth(state.commit_cursor) {
                        message.remove(byte_index);
                        if !state.is_amend_mode {
                            let _ = commit_storage::save_commit_message(
                                &state.repo_path,
                                &state.commit_message,
                            );
                        }
                    }
                }
                return state;
            }
            Some(Input::KeyLeft) => {
                state.commit_cursor = state.commit_cursor.saturating_sub(1);
                return state;
            }
            Some(Input::KeyRight) => {
                let message_len = if state.is_amend_mode {
                    state.amend_message.chars().count()
                } else {
                    state.commit_message.chars().count()
                };
                state.commit_cursor = state.commit_cursor.saturating_add(1).min(message_len);
                return state;
            }
            Some(Input::Character(c)) => {
                if c == '\u{1b}' {
                    // ESC key
                    state.is_commit_mode = false;
                    state.is_amend_mode = false; // Also reset amend mode
                    #[cfg(not(test))]
                    curs_set(0);
                } else if c == '\u{1}' {
                    // Ctrl-A: beginning of line
                    state.commit_cursor = 0;
                } else if c == '\u{5}' {
                    // Ctrl-E: end of line
                    let message = if state.is_amend_mode {
                        &state.amend_message
                    } else {
                        &state.commit_message
                    };
                    state.commit_cursor = message.chars().count();
                } else if c == '\u{b}' {
                    // Ctrl-K: kill to end of line
                    let message = if state.is_amend_mode {
                        &mut state.amend_message
                    } else {
                        &mut state.commit_message
                    };
                    if state.commit_cursor < message.chars().count() {
                        let byte_offset = message
                            .char_indices()
                            .nth(state.commit_cursor)
                            .map_or(message.len(), |(idx, _)| idx);
                        message.truncate(byte_offset);
                        if !state.is_amend_mode {
                            let _ = commit_storage::save_commit_message(
                                &state.repo_path,
                                &state.commit_message,
                            );
                        }
                    }
                } else if c == '\u{7f}' || c == '\u{08}' {
                    // Backspace
                    if state.commit_cursor > 0 {
                        let message = if state.is_amend_mode {
                            &mut state.amend_message
                        } else {
                            &mut state.commit_message
                        };
                        let char_index_to_remove = state.commit_cursor - 1;
                        if let Some((byte_index, _)) =
                            message.char_indices().nth(char_index_to_remove)
                        {
                            message.remove(byte_index);
                            state.commit_cursor -= 1;
                            if !state.is_amend_mode {
                                let _ = commit_storage::save_commit_message(
                                    &state.repo_path,
                                    &state.commit_message,
                                );
                            }
                        }
                    }
                } else if !c.is_control() {
                    let message = if state.is_amend_mode {
                        &mut state.amend_message
                    } else {
                        &mut state.commit_message
                    };
                    let byte_offset = message
                        .char_indices()
                        .nth(state.commit_cursor)
                        .map_or(message.len(), |(idx, _)| idx);
                    message.insert(byte_offset, c);
                    state.commit_cursor += 1;
                    if !state.is_amend_mode {
                        let _ = commit_storage::save_commit_message(
                            &state.repo_path,
                            &state.commit_message,
                        );
                    }
                }
                return state;
            }
            _ => return state,
        }
    }

    match input {
        Some(Input::Character('\u{3}')) => {
            // Ctrl+C
            let _ = commit_storage::save_commit_message(&state.repo_path, &state.commit_message);
            state.running = false;
        }
        Some(Input::Character('i')) => {
            if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                if let Some(file) = state.files.get(state.file_cursor - 1).cloned() {
                    let command = Box::new(IgnoreFileCommand {
                        repo_path: state.repo_path.clone(),
                        file_name: file.file_name.clone(),
                    });
                    state.command_history.execute(command);
                    state.refresh_diff();
                }
            }
        }
        Some(Input::Character('!')) => {
            if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                if let Some(file) = state.files.get(state.file_cursor - 1).cloned() {
                    // Get the patch before doing anything
                    let output = OsCommand::new("git")
                        .arg("diff")
                        .arg("--staged")
                        .arg("--")
                        .arg(&file.file_name)
                        .current_dir(&state.repo_path)
                        .output()
                        .expect("Failed to get diff for file.");
                    let patch = String::from_utf8_lossy(&output.stdout).to_string();

                    if file.status == FileStatus::Added {
                        let command = Box::new(RemoveFileCommand {
                            repo_path: state.repo_path.clone(),
                            file_name: file.file_name.clone(),
                            patch,
                        });
                        state.command_history.execute(command);
                    } else {
                        let command = Box::new(CheckoutFileCommand {
                            repo_path: state.repo_path.clone(),
                            file_name: file.file_name.clone(),
                            patch,
                        });
                        state.command_history.execute(command);
                    }
                    state.refresh_diff();
                }
            }
        }
        Some(Input::Character('\n')) => {
            if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                if let Some(file) = state.files.get(state.file_cursor - 1).cloned() {
                    let line_index = state.line_cursor;
                    if let Some(hunk) = file.hunks.iter().find(|hunk| {
                        let hunk_start = hunk.start_line;
                        let hunk_end = hunk_start + hunk.lines.len();
                        line_index >= hunk_start && line_index < hunk_end
                    }) {
                        let mut patch = String::new();
                        patch.push_str(&format!(
                            "diff --git a/{} b/{}\n",
                            file.file_name, file.file_name
                        ));
                        patch.push_str(&format!("--- a/{}\n", file.file_name));
                        patch.push_str(&format!("+++ b/{}\n", file.file_name));
                        patch.push_str(&hunk.lines.join("\n"));
                        patch.push('\n');

                        let command = Box::new(ApplyPatchCommand {
                            repo_path: state.repo_path.clone(),
                            patch,
                        });
                        state.command_history.execute(command);
                        state.refresh_diff();
                    } else {
                        let command = Box::new(UnstageFileCommand {
                            repo_path: state.repo_path.clone(),
                            file_name: file.file_name.clone(),
                        });
                        state.command_history.execute(command);
                        state.refresh_diff();
                    }
                }
            }
        }
        Some(Input::Character('1')) => {
            if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                if let Some(file) = state.files.get(state.file_cursor - 1) {
                    let line_index = state.line_cursor;
                    if let Some(line_to_unstage) = file.lines.get(line_index) {
                        if !line_to_unstage.starts_with('+') && !line_to_unstage.starts_with('-') {
                            return state;
                        }

                        if let Some(hunk) = file.hunks.iter().find(|hunk| {
                            let hunk_start = hunk.start_line;
                            let hunk_end = hunk_start + hunk.lines.len();
                            line_index >= hunk_start && line_index < hunk_end
                        }) {
                            let hunk_header = &hunk.lines[0];
                            let mut parts = hunk_header.split(' ');
                            let old_range = parts.nth(1).unwrap();
                            let new_range = parts.next().unwrap();

                            let mut old_range_parts = old_range.split(',');
                            let old_start: u32 = old_range_parts
                                .next()
                                .unwrap()
                                .trim_start_matches('-')
                                .parse()
                                .unwrap();

                            let mut new_range_parts = new_range.split(',');
                            let new_start: u32 = new_range_parts
                                .next()
                                .unwrap()
                                .trim_start_matches('+')
                                .parse()
                                .unwrap();

                            let mut current_old_line = old_start;
                            let mut current_new_line = new_start;
                            let mut patch_old_line = 0;
                            let mut patch_new_line = 0;

                            for (i, line) in hunk.lines.iter().skip(1).enumerate() {
                                let current_line_index_in_file = hunk.start_line + 1 + i;

                                if current_line_index_in_file == line_index {
                                    patch_old_line = current_old_line;
                                    patch_new_line = current_new_line;
                                    break;
                                }

                                if line.starts_with('-') {
                                    current_old_line += 1;
                                } else if line.starts_with('+') {
                                    current_new_line += 1;
                                } else {
                                    current_old_line += 1;
                                    current_new_line += 1;
                                }
                            }

                            let new_hunk_header = if line_to_unstage.starts_with('-') {
                                format!("@@ -{},1 +{},0 @@", patch_old_line, patch_new_line)
                            } else {
                                format!("@@ -{},0 +{},1 @@", patch_old_line, patch_new_line)
                            };

                            let mut patch = String::new();
                            patch.push_str(&format!(
                                "diff --git a/{} b/{}\n",
                                file.file_name, file.file_name
                            ));
                            patch.push_str(&format!("--- a/{}\n", file.file_name));
                            patch.push_str(&format!("+++ b/{}\n", file.file_name));
                            patch.push_str(&new_hunk_header);
                            patch.push('\n');
                            patch.push_str(line_to_unstage);
                            patch.push('\n');

                            let command = Box::new(ApplyPatchCommand {
                                repo_path: state.repo_path.clone(),
                                patch,
                            });
                            let old_line_cursor = state.line_cursor;
                            state.command_history.execute(command);
                            state.refresh_diff();
                            if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                                if let Some(file) = state.files.get(state.file_cursor - 1) {
                                    state.line_cursor =
                                        old_line_cursor.min(file.lines.len().saturating_sub(1));
                                    let header_height = state.files.len() + 3;
                                    let content_height =
                                        (max_y as usize).saturating_sub(header_height);
                                    if state.line_cursor >= state.scroll + content_height {
                                        state.scroll = state.line_cursor - content_height + 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Some(Input::Character('u')) => {
            state.command_history.undo();
            state.refresh_diff();
        }
        Some(Input::Character('r')) => {
            state.command_history.redo();
            state.refresh_diff();
        }
        Some(Input::Character('R')) => {
            OsCommand::new("git")
                .arg("add")
                .arg("-A")
                .current_dir(&state.repo_path)
                .output()
                .expect("Failed to git add -A.");
            state.refresh_diff();
        }
        Some(Input::Character(' ')) => {
            // Page down
            let header_height = state.files.len() + 3;
            let content_height = (max_y as usize).saturating_sub(header_height);
            let lines_count = if state.file_cursor == 0 {
                state.previous_commit_files.iter().map(|f| f.lines.len()).sum()
            } else if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                state.files.get(state.file_cursor - 1).map_or(0, |f| f.lines.len())
            } else {
                0
            };

            if lines_count > 0 {
                let scroll_amount = content_height;
                let old_scroll = state.scroll;
                let max_scroll = lines_count.saturating_sub(content_height).max(0);
                let new_scroll = state.scroll.saturating_add(scroll_amount).min(max_scroll);
                state.scroll = new_scroll;
                let scrolled_by = new_scroll - old_scroll;
                state.line_cursor = state
                    .line_cursor
                    .saturating_add(scrolled_by)
                    .min(lines_count.saturating_sub(1));
            }
        }
        Some(Input::Character('b')) => {
            // Page up
            let header_height = state.files.len() + 3;
            let content_height = (max_y as usize).saturating_sub(header_height);
            let lines_count = if state.file_cursor == 0 {
                state.previous_commit_files.iter().map(|f| f.lines.len()).sum()
            } else if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                state.files.get(state.file_cursor - 1).map_or(0, |f| f.lines.len())
            } else {
                0
            };

            if lines_count > 0 {
                let scroll_amount = content_height;
                let old_scroll = state.scroll;
                state.scroll = state.scroll.saturating_sub(scroll_amount);
                let scrolled_by = old_scroll - state.scroll;
                state.line_cursor = state.line_cursor.saturating_sub(scrolled_by);
            }
        }
        Some(Input::Character('\u{4}')) => {
            // Half page down
            let header_height = state.files.len() + 3;
            let content_height = (max_y as usize).saturating_sub(header_height);
            let lines_count = if state.file_cursor == 0 {
                state.previous_commit_files.iter().map(|f| f.lines.len()).sum()
            } else if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                state.files.get(state.file_cursor - 1).map_or(0, |f| f.lines.len())
            } else {
                0
            };

            if lines_count > 0 {
                let scroll_amount = (content_height / 2).max(1);
                let old_scroll = state.scroll;
                let max_scroll = lines_count.saturating_sub(content_height).max(0);
                let new_scroll = state.scroll.saturating_add(scroll_amount).min(max_scroll);
                state.scroll = new_scroll;
                let scrolled_by = new_scroll - old_scroll;
                state.line_cursor = state
                    .line_cursor
                    .saturating_add(scrolled_by)
                    .min(lines_count.saturating_sub(1));
            }
        }
        Some(Input::Character('\u{15}')) => {
            // Half page up
            let header_height = state.files.len() + 3;
            let content_height = (max_y as usize).saturating_sub(header_height);
            let lines_count = if state.file_cursor == 0 {
                state.previous_commit_files.iter().map(|f| f.lines.len()).sum()
            } else if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                state.files.get(state.file_cursor - 1).map_or(0, |f| f.lines.len())
            } else {
                0
            };

            if lines_count > 0 {
                let scroll_amount = (content_height / 2).max(1);
                let old_scroll = state.scroll;
                state.scroll = state.scroll.saturating_sub(scroll_amount);
                let scrolled_by = old_scroll - state.scroll;
                state.line_cursor = state.line_cursor.saturating_sub(scrolled_by);
            }
        }
        Some(Input::KeyUp) => {
            state.file_cursor = state.file_cursor.saturating_sub(1);
            state.scroll = 0;
            state.line_cursor = 0;
        }
        Some(Input::KeyDown) => {
            if state.file_cursor < state.files.len() + 1 {
                state.file_cursor += 1;
                state.scroll = 0;
                state.line_cursor = 0;
            }

            if state.file_cursor == state.files.len() + 1 {
                state.is_commit_mode = true;
                #[cfg(not(test))]
                curs_set(1);
            }
        }
        Some(Input::Character('k')) => {
            state.line_cursor = state.line_cursor.saturating_sub(1);
            let cursor_line = state.get_cursor_line_index();
            if cursor_line < state.scroll {
                state.scroll = cursor_line;
            }
        }
        Some(Input::Character('j')) => {
            let lines_count = if state.file_cursor == 0 {
                state.previous_commit_files.iter().map(|f| f.lines.len()).sum()
            } else if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
                state.files.get(state.file_cursor - 1).map_or(0, |f| f.lines.len())
            } else {
                0
            };

            if lines_count > 0 && state.line_cursor < lines_count.saturating_sub(1) {
                state.line_cursor += 1;
            }

            let header_height = state.files.len() + 3;
            let content_height = (max_y as usize).saturating_sub(header_height);
            let cursor_line = state.get_cursor_line_index();

            if cursor_line >= state.scroll + content_height {
                state.scroll = cursor_line - content_height + 1;
            }
        }
        _ => {}
    }

    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::AppState;
    use crate::git::{FileDiff, FileStatus, Hunk};
    use pancurses::Input;
    use std::path::PathBuf;

    #[test]
    fn test_compute_word_diffs() {
        let old = "The quick brown fox\njumps over the lazy dog";
        let new = "The slow brown cat\njumps over the lazy dog";

        let (old_diff, new_diff) = compute_word_diffs(old, new);

        assert_eq!(old_diff.len(), 2);
        assert_eq!(new_diff.len(), 2);

        let expected_old_line1 = WordDiffLine(vec![
            ("The".to_string(), false),
            (" ".to_string(), false),
            ("quick".to_string(), true),
            (" ".to_string(), false),
            ("brown".to_string(), false),
            (" ".to_string(), false),
            ("fox".to_string(), true),
        ]);
        assert_eq!(old_diff[0], expected_old_line1);

        let expected_new_line1 = WordDiffLine(vec![
            ("The".to_string(), false),
            (" ".to_string(), false),
            ("slow".to_string(), true),
            (" ".to_string(), false),
            ("brown".to_string(), false),
            (" ".to_string(), false),
            ("cat".to_string(), true),
        ]);
        assert_eq!(new_diff[0], expected_new_line1);

        let expected_line2 = WordDiffLine(vec![
            ("jumps".to_string(), false),
            (" ".to_string(), false),
            ("over".to_string(), false),
            (" ".to_string(), false),
            ("the".to_string(), false),
            (" ".to_string(), false),
            ("lazy".to_string(), false),
            (" ".to_string(), false),
            ("dog".to_string(), false),
        ]);
        assert_eq!(old_diff[1], expected_line2);
        assert_eq!(new_diff[1], expected_line2);
    }

    #[test]
    fn test_compute_word_diffs_empty() {
        let old = "";
        let new = "a";
        let (old_diff, new_diff) = compute_word_diffs(old, new);
        assert_eq!(old_diff.len(), 0);
        assert_eq!(new_diff.len(), 1);
        assert_eq!(new_diff[0], WordDiffLine(vec![("a".to_string(), false)]));

        let old = "a";
        let new = "";
        let (old_diff, new_diff) = compute_word_diffs(old, new);
        assert_eq!(old_diff.len(), 1);
        assert_eq!(new_diff.len(), 0);
        assert_eq!(old_diff[0], WordDiffLine(vec![("a".to_string(), false)]));
    }

    #[test]
    fn test_compute_word_diffs_low_similarity() {
        let old = "completely different";
        let new = "something else entirely";

        let (old_diff, new_diff) = compute_word_diffs(old, new);

        // Expect no word-level highlighting due to low similarity
        let expected_old = WordDiffLine(vec![("completely different".to_string(), false)]);
        let expected_new = WordDiffLine(vec![("something else entirely".to_string(), false)]);

        assert_eq!(old_diff.len(), 1);
        assert_eq!(old_diff[0], expected_old);
        assert_eq!(new_diff.len(), 1);
        assert_eq!(new_diff[0], expected_new);
    }

    fn create_test_state(
        lines_count: usize,
        file_cursor: usize,
        line_cursor: usize,
        scroll: usize,
    ) -> AppState {
        let mut files = Vec::new();
        if lines_count > 0 {
            let lines = (0..lines_count).map(|i| format!("line {}", i)).collect();
            files.push(FileDiff {
                file_name: "test_file.rs".to_string(),
                status: FileStatus::Modified,
                lines,
                hunks: vec![Hunk {
                    old_start: 1,
                    new_start: 1,
                    lines: Vec::new(),
                    start_line: 0,
                }],
            });
        }

        let mut state = AppState::new(PathBuf::from("/tmp"), files);
        state.file_cursor = file_cursor;
        state.line_cursor = line_cursor;
        state.scroll = scroll;
        // Mock previous commit files to avoid git command execution in tests
        state.previous_commit_files = vec![];
        state
    }

    // --- Page Down Tests ---

    #[test]
    fn test_page_down_maintains_relative_cursor() {
        let initial_state = create_test_state(100, 1, 5, 0);
        let max_y = 30;
        let content_height = (max_y as usize).saturating_sub(1 + 3); // 26

        let final_state = update_state(initial_state, Some(Input::Character(' ')), max_y);

        assert_eq!(final_state.scroll, content_height, "Scroll should move down by one page");
        assert_eq!(final_state.line_cursor, 5 + content_height, "Cursor should also move down by one page");
    }

    #[test]
    fn test_page_down_at_end_stops_at_max_scroll() {
        let lines_count = 100;
        let max_y = 30;
        let content_height = (max_y as usize).saturating_sub(1 + 3); // 26
        let max_scroll = lines_count - content_height; // 74
        let initial_state = create_test_state(lines_count, 1, 80, max_scroll);

        let final_state = update_state(initial_state, Some(Input::Character(' ')), max_y);

        assert_eq!(final_state.scroll, max_scroll, "Scroll should not change as it's at the end");
        assert_eq!(final_state.line_cursor, 80, "Cursor should not move as scroll did not change");
    }

    #[test]
    fn test_page_down_clamps_at_end() {
        let lines_count = 40;
        let max_y = 30;
        let content_height = (max_y as usize).saturating_sub(1 + 3); // 26
        let initial_state = create_test_state(lines_count, 1, 20, 0);
        let max_scroll = lines_count - content_height; // 14

        let final_state = update_state(initial_state, Some(Input::Character(' ')), max_y);

        assert_eq!(final_state.scroll, max_scroll, "Scroll should clamp to the max scroll position");
        assert_eq!(final_state.line_cursor, 20 + max_scroll, "Cursor should move by the amount scrolled");
    }

    // --- Page Up Tests ---

    #[test]
    fn test_page_up_maintains_relative_cursor() {
        let max_y = 30;
        let content_height = (max_y as usize).saturating_sub(1 + 3); // 26
        let initial_state = create_test_state(100, 1, 60, 50);

        let final_state = update_state(initial_state, Some(Input::Character('b')), max_y);

        assert_eq!(final_state.scroll, 50 - content_height, "Scroll should move up by one page");
        assert_eq!(final_state.line_cursor, 60 - content_height, "Cursor should also move up by one page");
    }

    #[test]
    fn test_page_up_stops_at_top() {
        let max_y = 30;
        let _content_height = (max_y as usize).saturating_sub(1 + 3); // 26
        let initial_state = create_test_state(100, 1, 20, 15);

        let final_state = update_state(initial_state, Some(Input::Character('b')), max_y);

        assert_eq!(final_state.scroll, 0, "Scroll should clamp at the top");
        assert_eq!(final_state.line_cursor, 20 - 15, "Cursor should move by the amount scrolled");
    }

    #[test]
    fn test_page_up_at_top_does_nothing() {
        let max_y = 30;
        let _content_height = (max_y as usize).saturating_sub(1 + 3); // 26
        let initial_state = create_test_state(100, 1, 10, 0);

        let final_state = update_state(initial_state, Some(Input::Character('b')), max_y);

        assert_eq!(final_state.scroll, 0, "Scroll should not change");
        assert_eq!(final_state.line_cursor, 10, "Cursor should not change");
    }

    #[test]
    fn test_ignore_file() {
        // Setup a temporary git repository
        let temp_dir = std::env::temp_dir().join("test_repo_for_ignore_v2");
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).unwrap();
        }
        std::fs::create_dir(&temp_dir).unwrap();
        OsCommand::new("git")
            .arg("init")
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to init git repo");
        OsCommand::new("git")
            .arg("config")
            .arg("user.name")
            .arg("Test")
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to set git user.name");
        OsCommand::new("git")
            .arg("config")
            .arg("user.email")
            .arg("test@example.com")
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to set git user.email");
        std::fs::write(temp_dir.join("a.txt"), "initial content").unwrap();
        OsCommand::new("git")
            .arg("add")
            .arg("a.txt")
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to git add");
        OsCommand::new("git")
            .arg("commit")
            .arg("-m")
            .arg("initial commit")
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to git commit");

        // Create a file to be ignored
        let file_to_ignore = "some_file.txt";
        std::fs::write(temp_dir.join(file_to_ignore), "Hello").unwrap();

        // Stage the file
        OsCommand::new("git")
            .arg("add")
            .arg(file_to_ignore)
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to git add");

        // Create initial state
        let files = crate::git::get_diff(temp_dir.clone());
        let mut state = AppState::new(temp_dir.clone(), files);
        state.file_cursor = 1; // Select the file

        // Simulate pressing 'i'
        let mut updated_state = update_state(state, Some(Input::Character('i')), 80);

        // Check if .gitignore is correct
        let gitignore_path = temp_dir.join(".gitignore");
        assert!(gitignore_path.exists(), ".gitignore should be created");
        let gitignore_content = std::fs::read_to_string(gitignore_path).unwrap();
        assert!(
            gitignore_content.contains(file_to_ignore),
            ".gitignore should contain the ignored file"
        );

        // After ignoring, the file should be gone from the diff,
        // and the .gitignore file should be the only change.
        assert_eq!(
            updated_state.files.len(),
            1,
            "File list should only contain .gitignore"
        );
        assert_eq!(
            updated_state.files[0].file_name, ".gitignore",
            "The remaining file should be .gitignore"
        );

        // Simulate undo
        updated_state.command_history.undo();
        updated_state.refresh_diff();

        // After undo, the original file should be back and .gitignore should be gone
        assert_eq!(
            updated_state.files.len(),
            1,
            "File list should contain the original file again"
        );
        assert_eq!(
            updated_state.files[0].file_name, file_to_ignore,
            "The file should be the one we ignored"
        );

        // Simulate undo
        updated_state.command_history.undo();
        updated_state.refresh_diff();

        // After undo, the original file should be back and .gitignore should be gone
        assert_eq!(
            updated_state.files.len(),
            1,
            "File list should contain the original file again"
        );
        assert_eq!(
            updated_state.files[0].file_name, file_to_ignore,
            "The file should be the one we ignored"
        );

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_half_page_down() {
        let lines_count = 100;
        let max_y = 30; // content_height = 26
        let content_height = (max_y as usize).saturating_sub(1 + 3);
        let scroll_amount = (content_height / 2).max(1);
        let initial_state = create_test_state(lines_count, 1, 10, 5);

        let final_state = update_state(initial_state, Some(Input::Character('\u{4}')), max_y);

        let expected_scroll = 5 + scroll_amount;
        assert_eq!(final_state.scroll, expected_scroll);
        assert_eq!(final_state.line_cursor, 10 + scroll_amount);
    }

    #[test]
    fn test_half_page_down_and_scroll() {
        let lines_count = 100;
        let max_y = 30; // content_height = 26
        let content_height = (max_y as usize).saturating_sub(1 + 3);
        let scroll_amount = (content_height / 2).max(1);
        let initial_state = create_test_state(lines_count, 1, 25, 0);

        let final_state = update_state(initial_state, Some(Input::Character('\u{4}')), max_y);

        assert_eq!(final_state.line_cursor, 25 + scroll_amount);
        assert_eq!(final_state.scroll, 13);
    }

    #[test]
    fn test_half_page_up() {
        let lines_count = 100;
        let max_y = 30; // content_height = 26
        let content_height = (max_y as usize).saturating_sub(1 + 3);
        let scroll_amount = (content_height / 2).max(1);
        let initial_state = create_test_state(lines_count, 1, 20, 15);

        let final_state = update_state(initial_state, Some(Input::Character('\u{15}')), max_y);

        assert_eq!(final_state.line_cursor, 20 - scroll_amount);
        assert_eq!(final_state.scroll, 2);
    }

    #[test]
    fn test_half_page_up_and_scroll() {
        let lines_count = 100;
        let max_y = 30; // content_height = 26
        let scroll_amount = ((max_y as usize).saturating_sub(1 + 3) / 2).max(1);
        let initial_state = create_test_state(lines_count, 1, 10, 10);

        let final_state = update_state(initial_state, Some(Input::Character('\u{15}')), max_y);

        assert_eq!(final_state.line_cursor, 0); // 10 - 13 saturates at 0
        assert_eq!(final_state.scroll, 0); // 10 - 13 saturates at 0
    }
}
