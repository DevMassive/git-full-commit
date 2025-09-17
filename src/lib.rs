use anyhow::{Result, bail};
use pancurses::{
    A_BOLD, A_DIM, A_REVERSE, COLOR_BLACK, COLOR_GREEN, COLOR_MAGENTA, COLOR_PAIR, COLOR_RED,
    Input, Window, curs_set, endwin, init_color, init_pair, initscr, noecho, start_color,
};
use std::path::{Path, PathBuf};
use std::process::Command as OsCommand;

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
            "diff --git a/{} b/{}
",
            self.file_name, self.file_name
        ));
        patch.push_str(&format!(
            "--- a/{}
",
            self.file_name
        ));
        patch.push_str(&format!(
            "+++ b/{}
",
            self.file_name
        ));
        patch.push_str(&self.hunk_lines.join(
            "
",
        ));
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
                "git apply failed for patch (reverse={reverse}):
{patch}"
            );
        }
    }
}

pub struct CommandHistory {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
}

impl CommandHistory {
    fn new() -> Self {
        CommandHistory {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
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
    pub start_line: usize,
    pub file_name: String,
    pub hunks: Vec<Hunk>,
}

pub enum CursorLevel {
    File,
    Hunk,
    Line,
}

pub struct AppState {
    pub repo_path: PathBuf,
    pub scroll: usize,
    pub running: bool,
    pub file_cursor: usize,
    pub hunk_cursor: usize,
    pub line_cursor: usize,
    pub files: Vec<FileDiff>,
    pub lines: Vec<String>,
    pub cursor_level: CursorLevel,
    pub command_history: CommandHistory,
}

impl AppState {
    pub fn new(repo_path: PathBuf, files: Vec<FileDiff>, lines: Vec<String>) -> Self {
        Self {
            repo_path,
            scroll: 0,
            running: true,
            file_cursor: 0,
            hunk_cursor: 0,
            line_cursor: 0,
            files,
            lines,
            cursor_level: CursorLevel::Line,
            command_history: CommandHistory::new(),
        }
    }

    pub fn get_cursor_line_index(&self) -> usize {
        if self.files.is_empty() {
            return 0;
        }
        match self.cursor_level {
            CursorLevel::File => self.files[self.file_cursor].start_line,
            CursorLevel::Hunk => {
                let file = &self.files[self.file_cursor];
                if file.hunks.is_empty() {
                    return file.start_line;
                }
                file.hunks[self.hunk_cursor].start_line
            }
            CursorLevel::Line => {
                let file = &self.files[self.file_cursor];
                if file.hunks.is_empty() {
                    return file.start_line;
                }
                let hunk = &file.hunks[self.hunk_cursor];
                if hunk.lines.is_empty() {
                    return hunk.start_line;
                }
                hunk.start_line + self.line_cursor
            }
        }
    }

    fn refresh_diff(&mut self) {
        let (files, lines) = get_diff(self.repo_path.clone());
        self.files = files;
        self.lines = lines;
        self.file_cursor = 0;
        self.hunk_cursor = 0;
        self.line_cursor = 0;
        self.scroll = 0;
    }
}

pub fn update_state(mut state: AppState, input: Option<Input>, window: &Window) -> AppState {
    let (max_y, _) = window.get_max_yx();

    match input {
        Some(Input::Character('q')) => state.running = false,
        Some(Input::Character('\n')) => {
            if let CursorLevel::File = state.cursor_level {
                if let Some(file) = state.files.get(state.file_cursor) {
                    let command = Box::new(UnstageFileCommand {
                        repo_path: state.repo_path.clone(),
                        file_name: file.file_name.clone(),
                    });
                    state.command_history.execute(command);
                    state.refresh_diff();
                }
            } else if let CursorLevel::Hunk = state.cursor_level {
                if let Some(file) = state.files.get(state.file_cursor) {
                    if let Some(hunk) = file.hunks.get(state.hunk_cursor) {
                        let command = Box::new(UnstageHunkCommand {
                            repo_path: state.repo_path.clone(),
                            file_name: file.file_name.clone(),
                            hunk_lines: hunk.lines.clone(),
                        });
                        state.command_history.execute(command);
                        state.refresh_diff();
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
        Some(Input::KeyUp) => match state.cursor_level {
            CursorLevel::File => {
                state.file_cursor = state.file_cursor.saturating_sub(1);
            }
            CursorLevel::Hunk => {
                if state.hunk_cursor > 0 {
                    state.hunk_cursor -= 1;
                } else if state.file_cursor > 0 {
                    state.file_cursor -= 1;
                    if let Some(file) = state.files.get(state.file_cursor) {
                        state.hunk_cursor = file.hunks.len().saturating_sub(1);
                    }
                }
            }
            CursorLevel::Line => {
                if state.line_cursor > 1 {
                    state.line_cursor -= 1;
                } else if state.hunk_cursor > 0 {
                    state.hunk_cursor -= 1;
                    if let Some(file) = state.files.get(state.file_cursor) {
                        if let Some(hunk) = file.hunks.get(state.hunk_cursor) {
                            state.line_cursor = hunk.lines.len().saturating_sub(1);
                        }
                    }
                } else if state.file_cursor > 0 {
                    state.file_cursor -= 1;
                    if let Some(file) = state.files.get(state.file_cursor) {
                        state.hunk_cursor = file.hunks.len().saturating_sub(1);
                        if let Some(hunk) = file.hunks.get(state.hunk_cursor) {
                            state.line_cursor = hunk.lines.len().saturating_sub(1);
                        } else {
                            state.line_cursor = 0;
                        }
                    }
                }
            }
        },
        Some(Input::KeyDown) => match state.cursor_level {
            CursorLevel::File => {
                if state.file_cursor < state.files.len().saturating_sub(1) {
                    state.file_cursor += 1;
                }
            }
            CursorLevel::Hunk => {
                if let Some(file) = state.files.get(state.file_cursor) {
                    if state.hunk_cursor < file.hunks.len().saturating_sub(1) {
                        state.hunk_cursor += 1;
                    } else if state.file_cursor < state.files.len().saturating_sub(1) {
                        state.file_cursor += 1;
                        state.hunk_cursor = 0;
                    }
                }
            }
            CursorLevel::Line => {
                if let Some(file) = state.files.get(state.file_cursor) {
                    if let Some(hunk) = file.hunks.get(state.hunk_cursor) {
                        if state.line_cursor < hunk.lines.len().saturating_sub(1) {
                            state.line_cursor += 1;
                        } else if state.hunk_cursor < file.hunks.len().saturating_sub(1) {
                            state.hunk_cursor += 1;
                            state.line_cursor = 1;
                        } else if state.file_cursor < state.files.len().saturating_sub(1) {
                            state.file_cursor += 1;
                            state.hunk_cursor = 0;
                            if !state.files[state.file_cursor].hunks.is_empty() {
                                state.line_cursor = 1;
                            } else {
                                state.line_cursor = 0;
                            }
                        }
                    }
                }
            }
        },
        Some(Input::KeyRight) => match state.cursor_level {
            CursorLevel::File => {
                if !state.files.is_empty() {
                    state.cursor_level = CursorLevel::Hunk;
                    state.hunk_cursor = 0;
                    state.line_cursor = 0;
                }
            }
            CursorLevel::Hunk => {
                if let Some(file) = state.files.get(state.file_cursor) {
                    if !file.hunks.is_empty() {
                        state.cursor_level = CursorLevel::Line;
                        state.line_cursor = 1; // Skip hunk header
                    }
                }
            }
            CursorLevel::Line => {
                // Do nothing
            }
        },
        Some(Input::KeyLeft) => match state.cursor_level {
            CursorLevel::File => {
                // Do nothing
            }
            CursorLevel::Hunk => {
                state.cursor_level = CursorLevel::File;
                state.hunk_cursor = 0;
                state.line_cursor = 0;
            }
            CursorLevel::Line => {
                state.cursor_level = CursorLevel::Hunk;
                state.line_cursor = 0;
            }
        },
        _ => {}
    }

    // Adjust scroll
    let cursor_position = state.get_cursor_line_index();

    match state.cursor_level {
        CursorLevel::File | CursorLevel::Hunk => {
            state.scroll = cursor_position;
        }
        CursorLevel::Line => {
            if cursor_position < state.scroll {
                state.scroll = cursor_position;
            }
            let window_height = max_y as usize;
            if cursor_position >= state.scroll + window_height {
                state.scroll = cursor_position - window_height + 1;
            }
        }
    }

    state
}

fn render(window: &Window, state: &AppState) {
    window.clear();
    let (max_y, _) = window.get_max_yx();
    let lines = &state.lines;

    let cursor_position = state.get_cursor_line_index();

    for (i, line) in lines
        .iter()
        .skip(state.scroll)
        .take(max_y as usize)
        .enumerate()
    {
        let line_index_in_full_list = i + state.scroll;
        let is_cursor_line = line_index_in_full_list == cursor_position
            && matches!(state.cursor_level, CursorLevel::Line);
        
        // TODO
        let is_selected = is_cursor_line;

        if is_cursor_line {
            window.attron(A_REVERSE);
        }

        if line.starts_with("--- ") {
            window.attron(A_DIM);
            window.mvaddstr(i as i32, 0, line);
            window.attroff(A_DIM);
        } else if line.starts_with("+++ ") {
            window.attron(A_DIM);
            window.mvaddstr(i as i32, 0, line);
            window.attroff(A_DIM);
        } else if line.starts_with("new file mode ") {
            window.mvaddstr(i as i32, 0, "[new]");
        } else if line.starts_with('+') {
            let attributes = if is_selected {
                COLOR_PAIR(1)
            } else {
                COLOR_PAIR(3)
            };
            window.attron(attributes);
            window.mvaddstr(i as i32, 0, line);
            window.attroff(attributes);
        } else if line.starts_with('-') {
            let attributes = if is_selected {
                COLOR_PAIR(2)
            } else {
                COLOR_PAIR(4)
            };
            window.attron(attributes);
            window.mvaddstr(i as i32, 0, line);
            window.attroff(attributes);
        } else if line.starts_with("@@ ") {
            window.attron(A_DIM);
            window.mvaddstr(i as i32, 0, line);
            window.attroff(A_DIM);
        } else if line.starts_with("diff --git") {
            let file_name_a_b = line.strip_prefix("diff --git ").unwrap();
            let file_name_a = file_name_a_b.split_whitespace().next().unwrap();
            let file_name = file_name_a.strip_prefix("a/").unwrap();
            window.attron(COLOR_PAIR(5));
            window.mvaddstr(i as i32, 0, file_name);
            window.attroff(COLOR_PAIR(5));
        } else if line.starts_with("index ") {
            let (_, max_x) = window.get_max_yx();
            window.mv(i as i32, 0);
            window.hline('-', max_x);
        } else {
            window.mvaddstr(i as i32, 0, line);
        }

        if is_cursor_line {
            window.attroff(A_REVERSE);
        }
    }
    window.refresh();
}

pub fn get_diff(repo_path: PathBuf) -> (Vec<FileDiff>, Vec<String>) {
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

    let mut current_line_index = 0;

    for line in diff_str.lines() {
        if line.starts_with("diff --git") {
            if let Some(mut file) = current_file.take() {
                if let Some(hunk) = current_hunk.take() {
                    file.hunks.push(hunk);
                }
                files.push(file);
            }
            let file_name_part = line.split(' ').nth(2).unwrap_or("");
            let file_name = if file_name_part.starts_with("a/") {
                &file_name_part[2..]
            } else {
                file_name_part
            };
            current_file = Some(FileDiff {
                start_line: current_line_index,
                file_name: file_name.to_string(),
                hunks: Vec::new(),
            });
        } else if line.starts_with("@@ ") {
            if let Some(hunk) = current_hunk.take() {
                if let Some(file) = current_file.as_mut() {
                    file.hunks.push(hunk);
                }
            }
            current_hunk = Some(Hunk {
                start_line: current_line_index,
                lines: vec![line.to_string()],
            });
        } else if let Some(hunk) = current_hunk.as_mut() {
            hunk.lines.push(line.to_string());
        }
        current_line_index += 1;
    }

    if let Some(mut file) = current_file.take() {
        if let Some(hunk) = current_hunk.take() {
            file.hunks.push(hunk);
        }
        files.push(file);
    }

    (files, diff_str.lines().map(String::from).collect())
}

pub fn tui_loop(repo_path: PathBuf, files: Vec<FileDiff>, lines: Vec<String>) {
    let window = initscr();
    window.keypad(true);
    noecho();
    curs_set(0);

    start_color();
    init_color(14, 0, 1000, 0);
    init_color(15, 1000, 0, 0);
    init_color(16, 0, 500, 0);
    init_color(17, 500, 0, 0);
    init_color(18, 0, 80, 0);
    init_color(19, 80, 0, 0);

    init_pair(1, 14, 18);
    init_pair(2, 15, 19);

    init_pair(3, 16, 18);
    init_pair(4, 17, 19);

    init_pair(5, COLOR_MAGENTA, COLOR_BLACK);

    let mut state = AppState::new(repo_path, files, lines);

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

    let (files, lines) = get_diff(repo_path.clone());

    if files.is_empty() {
        println!("No staged changes found.");
        return Ok(());
    }

    tui_loop(repo_path, files, lines);
    Ok(())
}

fn is_git_repository(path: &Path) -> bool {
    path.join(".git").is_dir()
}
