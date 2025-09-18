use anyhow::{Result, bail};
use pancurses::{COLOR_BLACK, COLOR_PAIR, Input, Window, curs_set, endwin, init_color, init_pair, initscr, noecho, start_color};

use std::path::{Path, PathBuf};
use std::process::Command as OsCommand;
use unicode_width::UnicodeWidthStr;

mod commit_storage;

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

struct ApplyPatchCommand {
    repo_path: PathBuf,
    patch: String,
}

impl Command for ApplyPatchCommand {
    fn execute(&mut self) {
        self.apply_patch(true);
    }

    fn undo(&mut self) {
        self.apply_patch(false);
    }
}

impl ApplyPatchCommand {
    fn apply_patch(&self, reverse: bool) {
        use std::io::Write;
        use std::process::{Command as OsCommand, Stdio};

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
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn git apply process.");

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(self.patch.as_bytes())
                .expect("Failed to write to stdin.");
        }

        let output = child.wait_with_output().expect("Failed to wait for git apply process.");
        if !output.status.success() {
            eprintln!(
                "git apply failed for patch (reverse={}):\n{}\n--- stderr ---\n{}\n---",
                reverse, self.patch, String::from_utf8_lossy(&output.stderr)
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
    pub old_start: usize,
    pub new_start: usize,
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
        Some(Input::Character('1')) => {
            if let Some(file) = state.files.get(state.file_cursor) {
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
                        if let Some(file) = state.files.get(state.file_cursor) {
                            state.line_cursor = old_line_cursor.min(file.lines.len().saturating_sub(1));
                            let header_height = if state.files.is_empty() { 0 } else { state.files.len() + 2 };
                            let content_height = (max_y as usize).saturating_sub(header_height);
                            if state.line_cursor >= state.scroll + content_height {
                                state.scroll = state.line_cursor - content_height + 1;
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
            let header_height = if state.files.is_empty() {
                0
            } else {
                state.files.len() + 2
            };
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

fn render(window: &Window, state: &AppState) {
    window.clear();
    let (max_y, max_x) = window.get_max_yx();

    let num_files = state.files.len();

    // Render sticky header
    if !state.files.is_empty() {
        for (i, file) in state.files.iter().enumerate() {
            let is_selected_file = i == state.file_cursor;
            let pair = if is_selected_file { 5 } else { 1 };
            window.attron(COLOR_PAIR(pair));
            window.mv(i as i32, 0);
            window.clrtoeol();
            window.addstr(&file.file_name);
            window.attroff(COLOR_PAIR(pair));
        }
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
        window.attron(COLOR_PAIR(5));
        window.addstr(prefix);
        window.attroff(COLOR_PAIR(5));
    } else {
        window.addstr(prefix);
    }
    window.addstr(message);

    // Render separator
    window.mv((num_files + 1) as i32, 0);
    window.hline(pancurses::ACS_HLINE(), max_x);

    if state.file_cursor >= num_files {
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
        window.refresh();
        return;
    }

    let header_height = num_files + 2;
    let content_height = (max_y as usize).saturating_sub(header_height);

    let selected_file = &state.files[state.file_cursor];
    let lines = &selected_file.lines;

    let cursor_position = state.get_cursor_line_index();

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

    for (i, line) in lines
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
            line_index_in_file,
            i as i32 + header_height as i32,
            cursor_position,
            old_line_num,
            new_line_num,
        );
    }

    window.refresh();
}

fn render_line(
    window: &Window,
    _state: &AppState,
    line: &str,
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

    let line_num_str = format!(
        "{:>4} {:>4}",
        old_line_num.map_or(String::new(), |n| n.to_string()),
        new_line_num.map_or(String::new(), |n| n.to_string())
    );
    let line_content_offset = 10;

    window.mv(line_render_index, 0);
    window.clrtoeol();

    if line.starts_with("--- ") || line.starts_with("+++ ") {
        window.attron(COLOR_PAIR(deletion_pair));
        window.mvaddstr(line_render_index, 0, &line_num_str);
        window.addstr(" ");
        window.addstr(line);
        window.attroff(COLOR_PAIR(deletion_pair));
    } else if line.starts_with('+') {
        window.attron(COLOR_PAIR(addition_pair));
        window.mvaddstr(line_render_index, 0, &line_num_str);
        window.mvaddstr(line_render_index, line_content_offset, line);
        window.attroff(COLOR_PAIR(addition_pair));
    } else if line.starts_with('-') {
        window.attron(COLOR_PAIR(deletion_pair));
        window.mvaddstr(line_render_index, 0, &line_num_str);
        window.mvaddstr(line_render_index, line_content_offset, line);
        window.attroff(COLOR_PAIR(deletion_pair));
    } else if line.starts_with("@@ ") {
        let mut parts = line.splitn(2, "@@");
        let at_at = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("");
        let mut rest_parts = rest.splitn(2, " ");
        let func = rest_parts.next().unwrap_or("");

        window.attron(COLOR_PAIR(hunk_header_pair));
        window.mvaddstr(line_render_index, 0, &line_num_str);
        window.addstr(" ");
        window.addstr(at_at);
        window.addstr("@@");
        window.attroff(COLOR_PAIR(hunk_header_pair));

        window.attron(COLOR_PAIR(addition_pair));
        window.addstr(func);
        window.attroff(COLOR_PAIR(addition_pair));
    } else if line.starts_with("diff --git ") {
        window.attron(COLOR_PAIR(default_pair));
        window.mvaddstr(line_render_index, 0, line);
        window.attroff(COLOR_PAIR(default_pair));
    } else {
        window.attron(COLOR_PAIR(default_pair));
        window.mvaddstr(line_render_index, 0, &line_num_str);
        window.addstr(" ");
        window.addstr(line);
        window.attroff(COLOR_PAIR(default_pair));
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

            let parts: Vec<&str> = line.split(' ').collect();
            let old_start = parts
                .get(1)
                .and_then(|s| s.split(',').next())
                .and_then(|s| s.trim_start_matches('-').parse::<usize>().ok())
                .unwrap_or(0);
            let new_start = parts
                .get(2)
                .and_then(|s| s.split(',').next())
                .and_then(|s| s.trim_start_matches('+').parse::<usize>().ok())
                .unwrap_or(0);

            current_hunk = Some(Hunk {
                start_line: current_file_line_index,
                lines: vec![line.to_string()],
                old_start,
                new_start,
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
    // Base colors
    let color_white = 20;
    let color_red = 21;
    let color_green = 22;
    let color_cyan = 23;
    let color_selected_bg = 24;

    init_color(color_white, 968, 968, 941); // #F7F7F0
    init_color(color_red, 1000, 0, 439); // #FF0070
    init_color(color_green, 525, 812, 0); // #86CF00
    init_color(color_cyan, 0, 769, 961); // #00C4F5
    init_color(color_selected_bg, 133, 133, 133); // #222222

    // Color pairs
    init_pair(1, color_white, COLOR_BLACK); // Default: White on Black
    init_pair(2, color_red, COLOR_BLACK); // Deletion: Red on Black
    init_pair(3, color_green, COLOR_BLACK); // Addition: Green on Black
    init_pair(4, color_cyan, COLOR_BLACK); // Hunk Header: Cyan on Black

    // Selected line pairs
    init_pair(5, color_white, color_selected_bg); // Default: White on #222222
    init_pair(6, color_red, color_selected_bg); // Deletion: Red on #222222
    init_pair(7, color_green, color_selected_bg); // Addition: Green on #222222
    init_pair(8, color_cyan, color_selected_bg); // Hunk Header: Cyan on #222222

    let mut state = AppState::new(repo_path, files);

    while state.running {
        render(&window, &state);
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
