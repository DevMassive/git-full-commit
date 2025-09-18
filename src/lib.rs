use anyhow::{Result, bail};
use lazy_static::lazy_static;
use pancurses::{
    A_DIM, A_REVERSE, COLOR_BLACK, COLOR_CYAN, COLOR_MAGENTA, COLOR_PAIR, Input, Window, curs_set,
    endwin, init_color, init_pair, initscr, noecho, start_color,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command as OsCommand;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;

mod commit_storage;

lazy_static! {
    pub static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    pub static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

pub trait Command {
    fn execute(&mut self);
    fn undo(&mut self);
}

struct UnstageFileCommand {
    repo_path: PathBuf,
    file_name: String,
}

impl Command for UnstageFileCommand {
    fn execute(&mut self) {
        OsCommand::new("git")
            .arg("reset")
            .arg("HEAD")
            .arg("--")
            .arg(&self.file_name)
            .current_dir(&self.repo_path)
            .output()
            .expect("Failed to unstage file.");
    }

    fn undo(&mut self) {
        OsCommand::new("git")
            .arg("add")
            .arg(&self.file_name)
            .current_dir(&self.repo_path)
            .output()
            .expect("Failed to stage file.");
    }
}

struct UnstageHunkCommand {
    repo_path: PathBuf,
    file_name: String,
    hunk_lines: Vec<String>,
}

impl Command for UnstageHunkCommand {
    fn execute(&mut self) {
        self.apply_patch(true);
    }

    fn undo(&mut self) {
        self.apply_patch(false);
    }
}

impl UnstageHunkCommand {
    fn apply_patch(&self, reverse: bool) {
        use std::io::Write;
        use std::process::{Command as OsCommand, Stdio};

        let mut patch = String::new();
        patch.push_str(&format!(
            "diff --git a/{} b/{}\n",
            self.file_name, self.file_name
        ));
        patch.push_str(&format!("--- a/{}\n", self.file_name));
        patch.push_str(&format!("+++ b/{}\n", self.file_name));
        patch.push_str(&self.hunk_lines.join("\n"));
        patch.push('\n');

        let mut args = vec!["apply"];
        if reverse {
            args.push("--cached");
            args.push("--reverse");
        } else {
            args.push("--cached");
        }
        args.push("--unidiff-zero");
        args.push("-");

        let mut child = OsCommand::new("git")
            .args(&args)
            .current_dir(&self.repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn git apply process.");

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(patch.as_bytes())
                .expect("Failed to write to stdin.");
        }

        let status = child.wait().expect("Failed to wait for git apply process.");
        if !status.success() {
            // For debugging, but should not panic in production
            eprintln!(
                "git apply failed for patch (reverse={}):\n{}\n",
                reverse, patch
            );
        }
    }
}

struct CheckoutFileCommand {
    repo_path: PathBuf,
    file_name: String,
    patch: String,
}

impl Command for CheckoutFileCommand {
    fn execute(&mut self) {
        OsCommand::new("git")
            .arg("checkout")
            .arg("HEAD")
            .arg("--")
            .arg(&self.file_name)
            .current_dir(&self.repo_path)
            .output()
            .expect("Failed to checkout file.");
    }

    fn undo(&mut self) {
        self.apply_patch(false);
    }
}

impl CheckoutFileCommand {
    fn apply_patch(&self, reverse: bool) {
        use std::io::Write;
        use std::process::{Command as OsCommand, Stdio};

        let mut args = vec!["apply"];
        if reverse {
            // This command is not meant to be reversed in the traditional sense.
            // The 'undo' operation applies the stored patch to restore the state.
        } else {
            args.push("--cached");
        }
        args.push("--unidiff-zero");
        args.push("-");

        let mut child = OsCommand::new("git")
            .args(&args)
            .current_dir(&self.repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn git apply process.");

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(self.patch.as_bytes())
                .expect("Failed to write to stdin.");
        }

        let status = child.wait().expect("Failed to wait for git apply process.");
        if !status.success() {
            eprintln!(
                "git apply failed for patch (reverse={}):\n{}\n",
                reverse, self.patch
            );
        }
    }
}

struct RemoveFileCommand {
    repo_path: PathBuf,
    file_name: String,
    patch: String,
}

impl Command for RemoveFileCommand {
    fn execute(&mut self) {
        OsCommand::new("git")
            .arg("rm")
            .arg("-f")
            .arg(&self.file_name)
            .current_dir(&self.repo_path)
            .output()
            .expect("Failed to remove file.");
    }

    fn undo(&mut self) {
        use std::io::Write;
        use std::process::{Command as OsCommand, Stdio};

        // First, apply the patch to restore the file content
        let mut child = OsCommand::new("git")
            .arg("apply")
            .arg("--unidiff-zero")
            .arg("-")
            .current_dir(&self.repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn git apply process.");

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(self.patch.as_bytes())
                .expect("Failed to write to stdin.");
        }
        child.wait().expect("Failed to wait for git apply process.");

        // Then, add the restored file to the index
        OsCommand::new("git")
            .arg("add")
            .arg(&self.file_name)
            .current_dir(&self.repo_path)
            .output()
            .expect("Failed to stage file.");
    }
}

pub struct CommandHistory {
    pub undo_stack: Vec<Box<dyn Command>>,
    pub redo_stack: Vec<Box<dyn Command>>,
}

impl CommandHistory {
    fn new() -> Self {
        CommandHistory {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    fn execute(&mut self, mut command: Box<dyn Command>) {
        command.execute();
        self.undo_stack.push(command);
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some(mut command) = self.undo_stack.pop() {
            command.undo();
            self.redo_stack.push(command);
        }
    }

    fn redo(&mut self) {
        if let Some(mut command) = self.redo_stack.pop() {
            command.execute();
            self.undo_stack.push(command);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub start_line: usize,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub file_name: String,
    pub hunks: Vec<Hunk>,
    pub lines: Vec<String>,
    pub is_new_file: bool,
}

pub struct AppState {
    pub repo_path: PathBuf,
    pub scroll: usize,
    pub running: bool,
    pub file_cursor: usize,
    pub line_cursor: usize,
    pub files: Vec<FileDiff>,
    pub command_history: CommandHistory,
    pub commit_message: String,
    pub is_commit_mode: bool,
    pub commit_cursor: usize,
    pub amend_message: String,
    pub is_amend_mode: bool,
}

impl AppState {
    pub fn new(repo_path: PathBuf, files: Vec<FileDiff>) -> Self {
        let commit_message =
            commit_storage::load_commit_message(&repo_path).unwrap_or_else(|_| String::new());
        Self {
            repo_path,
            scroll: 0,
            running: true,
            file_cursor: 0,
            line_cursor: 0,
            files,
            command_history: CommandHistory::new(),
            commit_message,
            is_commit_mode: false,
            commit_cursor: 0,
            amend_message: String::new(),
            is_amend_mode: false,
        }
    }

    pub fn get_cursor_line_index(&self) -> usize {
        if self.files.is_empty() || self.file_cursor >= self.files.len() {
            return 0;
        }
        self.line_cursor
    }

    pub fn refresh_diff(&mut self) {
        let files = get_diff(self.repo_path.clone());
        self.files = files;

        if self.files.is_empty() {
            self.file_cursor = 0;
            self.line_cursor = 0;
            self.scroll = 0;
            return;
        }

        self.file_cursor = self.file_cursor.min(self.files.len().saturating_sub(1));
        self.line_cursor = 0;
        self.scroll = self.get_cursor_line_index();
    }
}

fn get_previous_commit_message(repo_path: &Path) -> Result<String> {
    let output = OsCommand::new("git")
        .arg("log")
        .arg("-1")
        .arg("--pretty=%s")
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        return Ok(String::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn update_state(mut state: AppState, input: Option<Input>, window: &Window) -> AppState {
    let (max_y, _) = window.get_max_yx();

    if state.is_commit_mode {
        match input {
            Some(Input::KeyUp) => {
                state.is_commit_mode = false;
                curs_set(0);
                state.file_cursor = state.files.len().saturating_sub(1);
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
                    state.amend_message.clear();
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
                    curs_set(0);
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
                } else {
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
        Some(Input::Character('q')) => {
            let _ = commit_storage::save_commit_message(&state.repo_path, &state.commit_message);
            state.running = false;
        }
        Some(Input::Character('!')) => {
            if let Some(file) = state.files.get(state.file_cursor) {
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

                if file.is_new_file {
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
        Some(Input::Character('\n')) => {
            if let Some(file) = state.files.get(state.file_cursor) {
                let line_index = state.line_cursor;
                if let Some(hunk) = file.hunks.iter().find(|hunk| {
                    let hunk_start = hunk.start_line;
                    let hunk_end = hunk_start + hunk.lines.len();
                    line_index >= hunk_start && line_index < hunk_end
                }) {
                    let command = Box::new(UnstageHunkCommand {
                        repo_path: state.repo_path.clone(),
                        file_name: file.file_name.clone(),
                        hunk_lines: hunk.lines.clone(),
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
            if let Some(file) = state.files.get(state.file_cursor) {
                let header_height = if state.files.is_empty() {
                    0
                } else {
                    state.files.len() + 2
                };
                let content_height = (max_y as usize).saturating_sub(header_height);
                let new_scroll = state.scroll.saturating_add(content_height);
                let max_scroll = file.lines.len().saturating_sub(content_height);
                state.scroll = new_scroll.min(max_scroll);
                state.line_cursor = state.scroll;
            }
        }
        Some(Input::Character('b')) => {
            // Page up
            let header_height = if state.files.is_empty() {
                0
            } else {
                state.files.len() + 2
            };
            let content_height = (max_y as usize).saturating_sub(header_height);
            state.scroll = state.scroll.saturating_sub(content_height);
            state.line_cursor = state.scroll;
        }
        Some(Input::KeyUp) => {
            state.file_cursor = state.file_cursor.saturating_sub(1);
            state.scroll = 0;
            state.line_cursor = 0;
        }
        Some(Input::KeyDown) => {
            if state.file_cursor < state.files.len() {
                state.file_cursor += 1;
                state.scroll = 0;
                state.line_cursor = 0;
            }

            if state.file_cursor == state.files.len() {
                state.is_commit_mode = true;
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
            if let Some(file) = state.files.get(state.file_cursor) {
                if state.line_cursor < file.lines.len().saturating_sub(1) {
                    state.line_cursor += 1;
                }
            }
            let header_height = if state.files.is_empty() { 0 } else { state.files.len() + 2 };
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

fn render(
    window: &Window,
    state: &AppState,
    color_map: &mut HashMap<syntect::highlighting::Color, i16>,
    pair_map: &mut HashMap<(i16, i16), i16>,
    next_color_num: &mut i16,
    next_pair_num: &mut i16,
) {
    window.clear();
    let (max_y, max_x) = window.get_max_yx();

    let num_files = state.files.len();

    // Render sticky header
    if !state.files.is_empty() {
        window.attron(COLOR_PAIR(5));
        for (i, file) in state.files.iter().enumerate() {
            window.mv(i as i32, 0);
            window.clrtoeol();
            if i == state.file_cursor {
                window.attron(A_REVERSE);
            }
            window.addstr(&file.file_name);
            if i == state.file_cursor {
                window.attroff(A_REVERSE);
            }
        }
        window.attroff(COLOR_PAIR(5));
    }

    // Render commit message line
    let commit_line_y = num_files as i32;
    window.mv(commit_line_y, 0);
    window.clrtoeol();

    let (prefix, message) = if state.is_amend_mode {
        ("Amend: ", &state.amend_message)
    } else {
        ("Commit: ", &state.commit_message)
    };

    if state.file_cursor == num_files {
        window.attron(A_REVERSE);
        window.addstr(prefix);
        window.attroff(A_REVERSE);
    } else {
        window.addstr(prefix);
    }
    window.addstr(message);

    // Render separator
    window.mv((num_files + 1) as i32, 0);
    window.hline(pancurses::ACS_HLINE(), max_x);

    if state.file_cursor >= num_files {
        if state.is_commit_mode {
            let prefix_len = if state.is_amend_mode {
                "Amend: ".len()
            } else {
                "Commit: ".len()
            };
            window.mv(commit_line_y, (prefix_len + state.commit_cursor) as i32);
        }
        window.refresh();
        return;
    }

    let header_height = num_files + 2;
    let content_height = (max_y as usize).saturating_sub(header_height);

    let selected_file = &state.files[state.file_cursor];
    let lines = &selected_file.lines;

    let cursor_position = state.get_cursor_line_index();

    let syntax = SYNTAX_SET
        .find_syntax_by_extension(
            Path::new(&selected_file.file_name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("txt"),
        )
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let theme = &THEME_SET.themes["Solarized (dark)"];
    let mut h = HighlightLines::new(syntax, theme);

    // Warm up highlighter state
    for line in lines.iter().take(state.scroll) {
        if line.starts_with('+') || line.starts_with('-') {
            let _ = h.highlight_line(&line[1..], &SYNTAX_SET).unwrap();
        } else if line.starts_with("diff --git")
            || line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("@@ ")
            || line.starts_with("new file mode ")
        {
            // Do nothing, just as render_line does
        } else {
            // This is a context line
            let _ = h.highlight_line(line, &SYNTAX_SET).unwrap();
        }
    }

    for (i, line) in lines
        .iter()
        .skip(state.scroll)
        .take(content_height)
        .enumerate()
    {
        let line_index_in_file = i + state.scroll;
        render_line(
            window,
            state,
            line,
            line_index_in_file,
            i as i32 + header_height as i32,
            cursor_position,
            &mut h,
            color_map,
            pair_map,
            next_color_num,
            next_pair_num,
        );
    }

    window.refresh();
}

fn render_line(
    window: &Window,
    state: &AppState,
    line: &str,
    line_index_in_file: usize,
    line_render_index: i32,
    cursor_position: usize,
    h: &mut HighlightLines,
    color_map: &mut HashMap<syntect::highlighting::Color, i16>,
    pair_map: &mut HashMap<(i16, i16), i16>,
    next_color_num: &mut i16,
    next_pair_num: &mut i16,
) {
    let is_cursor_line = line_index_in_file == cursor_position;

    let is_selected = if let Some(file) = state.files.get(state.file_cursor) {
        if let Some(hunk) = file.hunks.iter().find(|hunk| {
            let hunk_start = hunk.start_line;
            let hunk_end = hunk_start + hunk.lines.len();
            state.line_cursor >= hunk_start && state.line_cursor < hunk_end
        }) {
            let hunk_start = hunk.start_line;
            let hunk_end = hunk_start + hunk.lines.len();
            line_index_in_file >= hunk_start && line_index_in_file < hunk_end
        } else {
            true
        }
    } else {
        true
    };

    if is_cursor_line {
        window.attron(A_REVERSE);
    }

    if line.starts_with("--- ") {
        window.attron(A_DIM);
        window.mvaddstr(line_render_index, 0, line);
        window.attroff(A_DIM);
    } else if line.starts_with("+++ ") {
        window.attron(A_DIM);
        window.mvaddstr(line_render_index, 0, line);
        window.attroff(A_DIM);
    } else if line.starts_with('+') {
        let (sign_pair_num, bg_color) = if is_selected { (1, 18) } else { (3, 18) };

        let bg_pair = *pair_map.entry((-1, bg_color)).or_insert_with(|| {
            let pair_num = *next_pair_num;
            *next_pair_num += 1;
            init_pair(pair_num, COLOR_BLACK, bg_color);
            pair_num
        });
        window.attron(COLOR_PAIR(bg_pair as u32));
        window.mv(line_render_index, 0);
        window.clrtoeol();
        window.attroff(COLOR_PAIR(bg_pair as u32));

        if !is_selected {
            window.attron(A_DIM);
        }

        window.attron(COLOR_PAIR(sign_pair_num as u32));
        window.mvaddstr(line_render_index, 0, "+");
        window.attroff(COLOR_PAIR(sign_pair_num as u32));

        window.mv(line_render_index, 1);
        highlight_line(
            window,
            &line[1..],
            h,
            color_map,
            pair_map,
            next_color_num,
            next_pair_num,
            bg_color,
        );

        if !is_selected {
            window.attroff(A_DIM);
        }
    } else if line.starts_with('-') {
        let (sign_pair_num, bg_color) = if is_selected { (2, 19) } else { (4, 19) };

        let bg_pair = *pair_map.entry((-1, bg_color)).or_insert_with(|| {
            let pair_num = *next_pair_num;
            *next_pair_num += 1;
            init_pair(pair_num, COLOR_BLACK, bg_color);
            pair_num
        });
        window.attron(COLOR_PAIR(bg_pair as u32));
        window.mv(line_render_index, 0);
        window.clrtoeol();
        window.attroff(COLOR_PAIR(bg_pair as u32));

        if !is_selected {
            window.attron(A_DIM);
        }

        window.attron(COLOR_PAIR(sign_pair_num as u32));
        window.mvaddstr(line_render_index, 0, "-");
        window.attroff(COLOR_PAIR(sign_pair_num as u32));

        window.mv(line_render_index, 1);
        highlight_line(
            window,
            &line[1..],
            h,
            color_map,
            pair_map,
            next_color_num,
            next_pair_num,
            bg_color,
        );

        if !is_selected {
            window.attroff(A_DIM);
        }
    } else if line.starts_with("@@ ") {
        window.attron(COLOR_PAIR(6));
        window.mvaddstr(line_render_index, 0, line);
        window.attroff(COLOR_PAIR(6));
    } else if line.starts_with("diff --git ") {
        window.attron(COLOR_PAIR(5));
        window.mvaddstr(line_render_index, 0, line);
        window.attroff(COLOR_PAIR(5));
    } else if line.starts_with("index ") {
        window.attron(COLOR_PAIR(5));
        window.mvaddstr(line_render_index, 0, line);
        window.attroff(COLOR_PAIR(5));
    } else {
        window.mv(line_render_index, 0);
        if !is_selected {
            window.attron(A_DIM);
        }
        highlight_line(
            window,
            line,
            h,
            color_map,
            pair_map,
            next_color_num,
            next_pair_num,
            COLOR_BLACK,
        );
        if !is_selected {
            window.attroff(A_DIM);
        }
    }

    if is_cursor_line {
        window.attroff(A_REVERSE);
    }
}

fn highlight_line(
    window: &Window,
    line: &str,
    h: &mut HighlightLines,
    color_map: &mut HashMap<syntect::highlighting::Color, i16>,
    pair_map: &mut HashMap<(i16, i16), i16>,
    next_color_num: &mut i16,
    next_pair_num: &mut i16,
    bg_color_num: i16,
) {
    let ranges: Vec<(Style, &str)> = h.highlight_line(line, &SYNTAX_SET).unwrap();
    for (style, text) in ranges {
        let fg_syntect_color = style.foreground;
        let fg_color_num = *color_map.entry(fg_syntect_color).or_insert_with(|| {
            let color_num = *next_color_num;
            *next_color_num += 1;
            init_color(
                color_num,
                (fg_syntect_color.r as f32 / 255.0 * 1000.0) as i16,
                (fg_syntect_color.g as f32 / 255.0 * 1000.0) as i16,
                (fg_syntect_color.b as f32 / 255.0 * 1000.0) as i16,
            );
            color_num
        });

        let pair_key = (fg_color_num, bg_color_num);
        let pair_num = *pair_map.entry(pair_key).or_insert_with(|| {
            let pair_num = *next_pair_num;
            *next_pair_num += 1;
            init_pair(pair_num, fg_color_num, bg_color_num);
            pair_num
        });
        window.attron(COLOR_PAIR(pair_num as u32));
        window.addstr(text);
        window.attroff(COLOR_PAIR(pair_num as u32));
    }
}

pub fn get_diff(repo_path: PathBuf) -> Vec<FileDiff> {
    let output = OsCommand::new("git")
        .arg("diff")
        .arg("--staged")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to execute git diff");

    let diff_str = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();
    let mut current_file: Option<FileDiff> = None;
    let mut current_hunk: Option<Hunk> = None;
    let mut current_file_lines: Vec<String> = Vec::new();
    let mut current_file_line_index = 0;

    for line in diff_str.lines() {
        if line.starts_with("diff --git") {
            if let Some(mut file) = current_file.take() {
                if let Some(hunk) = current_hunk.take() {
                    file.hunks.push(hunk);
                }
                file.lines = current_file_lines;
                files.push(file);
                current_file_lines = Vec::new();
                current_file_line_index = 0;
            }
            let file_name_part = line.split(' ').nth(2).unwrap_or("");
            let file_name = if file_name_part.starts_with("a/") {
                &file_name_part[2..]
            } else {
                file_name_part
            };
            current_file = Some(FileDiff {
                file_name: file_name.to_string(),
                hunks: Vec::new(),
                lines: Vec::new(), // Will be filled in later
                is_new_file: false,
            });
        } else if line.starts_with("new file mode") {
            if let Some(file) = current_file.as_mut() {
                file.is_new_file = true;
            }
        } else if line.starts_with("@@ ") {
            if let Some(hunk) = current_hunk.take() {
                if let Some(file) = current_file.as_mut() {
                    file.hunks.push(hunk);
                }
            }
            current_hunk = Some(Hunk {
                start_line: current_file_line_index,
                lines: vec![line.to_string()],
            });
        } else if let Some(hunk) = current_hunk.as_mut() {
            hunk.lines.push(line.to_string());
        }

        if current_file.is_some() {
            current_file_lines.push(line.to_string());
            current_file_line_index += 1;
        }
    }

    if let Some(mut file) = current_file.take() {
        if let Some(hunk) = current_hunk.take() {
            file.hunks.push(hunk);
        }
        file.lines = current_file_lines;
        files.push(file);
    }

    files
}

pub fn tui_loop(repo_path: PathBuf, files: Vec<FileDiff>) {
    let window = initscr();
    window.keypad(true);
    noecho();
    curs_set(0);

    start_color();
    init_color(14, 0, 1000, 0);
    init_color(15, 1000, 0, 0);
    init_color(16, 0, 500, 0);
    init_color(17, 500, 0, 0);
    init_color(18, 0, 200, 0);
    init_color(19, 200, 0, 0);

    init_pair(1, 14, 18);
    init_pair(2, 15, 19);

    init_pair(3, 16, 18);
    init_pair(4, 17, 19);

    init_pair(5, COLOR_CYAN, COLOR_BLACK);
    init_pair(6, COLOR_MAGENTA, COLOR_BLACK);

    let mut state = AppState::new(repo_path, files);
    let mut color_map: HashMap<syntect::highlighting::Color, i16> = HashMap::new();
    let mut pair_map: HashMap<(i16, i16), i16> = HashMap::new();
    let mut next_color_num = 20;
    let mut next_pair_num = 20;

    while state.running {
        render(
            &window,
            &state,
            &mut color_map,
            &mut pair_map,
            &mut next_color_num,
            &mut next_pair_num,
        );
        let input = window.getch();
        state = update_state(state, input, &window);
    }

    endwin();
}

pub fn run(repo_path: PathBuf) -> Result<()> {
    if !is_git_repository(&repo_path) {
        bail!("fatal: not a git repository (or any of the parent directories): .git");
    }

    let staged_diff_output = OsCommand::new("git")
        .arg("diff")
        .arg("--staged")
        .current_dir(&repo_path)
        .output()?;

    if staged_diff_output.stdout.is_empty() {
        OsCommand::new("git")
            .arg("add")
            .arg("-A")
            .current_dir(&repo_path)
            .output()?;
    }

    let files = get_diff(repo_path.clone());

    if files.is_empty() {
        bail!("No changes found.");
    }

    tui_loop(repo_path.clone(), files);

    Ok(())
}

fn is_git_repository(path: &Path) -> bool {
    path.join(".git").is_dir()
}
