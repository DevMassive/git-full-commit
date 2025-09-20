use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::cursor_state::CursorState;
use crate::git;

pub trait Command {
    fn execute(&mut self);
    fn undo(&mut self);
    fn set_cursor_before_execute(&mut self, cursor: CursorState);
    fn set_cursor_before_undo(&mut self, cursor: CursorState);
    fn get_cursor_to_restore_on_undo(&self) -> Option<CursorState>;
    fn get_cursor_to_restore_on_redo(&self) -> Option<CursorState>;
}

macro_rules! command_impl {
    () => {
        fn set_cursor_before_execute(&mut self, cursor: CursorState) {
            self.cursor_before_execute = Some(cursor);
        }

        fn set_cursor_before_undo(&mut self, cursor: CursorState) {
            self.cursor_before_undo = Some(cursor);
        }

        fn get_cursor_to_restore_on_undo(&self) -> Option<CursorState> {
            self.cursor_before_execute
        }

        fn get_cursor_to_restore_on_redo(&self) -> Option<CursorState> {
            self.cursor_before_undo
        }
    };
}

pub struct UnstageFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl UnstageFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String) -> Self {
        Self {
            repo_path,
            file_name,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for UnstageFileCommand {
    fn execute(&mut self) {
        git::unstage_file(&self.repo_path, &self.file_name).expect("Failed to unstage file.");
    }

    fn undo(&mut self) {
        git::stage_file(&self.repo_path, &self.file_name).expect("Failed to stage file.");
    }

    command_impl!();
}

pub struct StageFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl StageFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String) -> Self {
        Self {
            repo_path,
            file_name,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for StageFileCommand {
    fn execute(&mut self) {
        git::stage_file(&self.repo_path, &self.file_name).expect("Failed to stage file.");
    }

    fn undo(&mut self) {
        git::unstage_file(&self.repo_path, &self.file_name).expect("Failed to unstage file.");
    }

    command_impl!();
}

pub struct ApplyPatchCommand {
    pub repo_path: PathBuf,
    pub patch: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl ApplyPatchCommand {
    pub fn new(repo_path: PathBuf, patch: String) -> Self {
        Self {
            repo_path,
            patch,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for ApplyPatchCommand {
    fn execute(&mut self) {
        git::apply_patch(&self.repo_path, &self.patch, true, true)
            .expect("Failed to apply patch in reverse.");
    }

    fn undo(&mut self) {
        git::apply_patch(&self.repo_path, &self.patch, false, true)
            .expect("Failed to apply patch.");
    }

    command_impl!();
}

pub struct StagePatchCommand {
    pub repo_path: PathBuf,
    pub patch: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl StagePatchCommand {
    pub fn new(repo_path: PathBuf, patch: String) -> Self {
        Self {
            repo_path,
            patch,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for StagePatchCommand {
    fn execute(&mut self) {
        git::apply_patch(&self.repo_path, &self.patch, false, true)
            .expect("Failed to apply patch.");
    }

    fn undo(&mut self) {
        git::apply_patch(&self.repo_path, &self.patch, true, true)
            .expect("Failed to apply patch in reverse.");
    }

    command_impl!();
}

pub struct DiscardHunkCommand {
    pub repo_path: PathBuf,
    pub patch: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl DiscardHunkCommand {
    pub fn new(repo_path: PathBuf, patch: String) -> Self {
        Self {
            repo_path,
            patch,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for DiscardHunkCommand {
    fn execute(&mut self) {
        // Unstage
        git::apply_patch(&self.repo_path, &self.patch, true, true)
            .expect("Failed to unstage hunk.");
        // Discard from working tree
        git::apply_patch(&self.repo_path, &self.patch, true, false)
            .expect("Failed to discard hunk from working tree.");
    }

    fn undo(&mut self) {
        // Re-apply to working tree
        git::apply_patch(&self.repo_path, &self.patch, false, false)
            .expect("Failed to re-apply hunk to working tree.");
        // Stage
        git::apply_patch(&self.repo_path, &self.patch, false, true).expect("Failed to stage hunk.");
    }

    command_impl!();
}

pub struct CheckoutFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    pub patch: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl CheckoutFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String, patch: String) -> Self {
        Self {
            repo_path,
            file_name,
            patch,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for CheckoutFileCommand {
    fn execute(&mut self) {
        git::checkout_file(&self.repo_path, &self.file_name).expect("Failed to checkout file.");
    }

    fn undo(&mut self) {
        git::apply_patch(&self.repo_path, &self.patch, false, false)
            .expect("Failed to apply patch for checkout undo.");
    }

    command_impl!();
}

pub struct IgnoreFileCommand {
    pub repo_path: std::path::PathBuf,
    pub file_name: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl IgnoreFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String) -> Self {
        Self {
            repo_path,
            file_name,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for IgnoreFileCommand {
    fn execute(&mut self) {
        let gitignore_path = self.repo_path.join(".gitignore");
        let mut gitignore = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(gitignore_path)
            .expect("Failed to open .gitignore");
        writeln!(gitignore, "{}", self.file_name).expect("Failed to write to .gitignore");

        git::stage_path(&self.repo_path, ".gitignore").expect("Failed to stage .gitignore");

        git::rm_cached(&self.repo_path, &self.file_name).expect("Failed to unstage file");
    }

    fn undo(&mut self) {
        let gitignore_path = self.repo_path.join(".gitignore");
        if gitignore_path.exists() {
            let content = fs::read_to_string(&gitignore_path).expect("Failed to read .gitignore");
            let new_content: String = content
                .lines()
                .filter(|line| !line.trim().is_empty() && *line != self.file_name)
                .collect::<Vec<_>>()
                .join("\n");

            if new_content.is_empty() {
                fs::remove_file(&gitignore_path).expect("Failed to remove .gitignore");
                git::rm_file_from_index(&self.repo_path, ".gitignore")
                    .expect("Failed to remove .gitignore from index");
            } else {
                fs::write(&gitignore_path, new_content + "\n")
                    .expect("Failed to write to .gitignore");
                git::stage_path(&self.repo_path, ".gitignore").expect("Failed to stage .gitignore");
            }
        }

        git::stage_file(&self.repo_path, &self.file_name).expect("Failed to stage file");
    }

    command_impl!();
}

pub struct RemoveFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    pub patch: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl RemoveFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String, patch: String) -> Self {
        Self {
            repo_path,
            file_name,
            patch,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for RemoveFileCommand {
    fn execute(&mut self) {
        git::rm_file(&self.repo_path, &self.file_name).expect("Failed to remove file.");
    }

    fn undo(&mut self) {
        git::apply_patch(&self.repo_path, &self.patch, false, false)
            .expect("Failed to apply patch for remove undo.");

        git::stage_file(&self.repo_path, &self.file_name).expect("Failed to stage file.");
    }

    command_impl!();
}

pub struct StageAllCommand {
    pub repo_path: PathBuf,
    patch: String,
    untracked_files: Vec<String>,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl StageAllCommand {
    pub fn new(repo_path: PathBuf) -> Self {
        let patch = git::get_unstaged_diff_patch(&repo_path).unwrap_or_default();
        let untracked_files = git::get_untracked_files(&repo_path).unwrap_or_default();
        Self {
            repo_path,
            patch,
            untracked_files,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for StageAllCommand {
    fn execute(&mut self) {
        git::add_all(&self.repo_path).expect("Failed to stage all files.");
    }

    fn undo(&mut self) {
        // Untracked files are now tracked, so we need to unstage them.
        for file in &self.untracked_files {
            git::rm_cached(&self.repo_path, file).expect("Failed to unstage file.");
        }

        // For modified and deleted files, we apply the reverse of the patch.
        if !self.patch.is_empty() {
            git::apply_patch(&self.repo_path, &self.patch, true, true)
                .expect("Failed to apply patch in reverse.");
        }
    }

    command_impl!();
}

pub struct CommandHistory {
    pub undo_stack: Vec<Box<dyn Command>>,
    pub redo_stack: Vec<Box<dyn Command>>,
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHistory {
    pub fn new() -> Self {
        CommandHistory {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn execute(&mut self, mut command: Box<dyn Command>, cursor_state: CursorState) {
        command.set_cursor_before_execute(cursor_state);
        command.execute();
        self.undo_stack.push(command);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, cursor_state: CursorState) -> Option<CursorState> {
        if let Some(mut command) = self.undo_stack.pop() {
            command.set_cursor_before_undo(cursor_state);
            command.undo();
            let cursor_to_restore = command.get_cursor_to_restore_on_undo();
            self.redo_stack.push(command);
            cursor_to_restore
        } else {
            None
        }
    }

    pub fn redo(&mut self, cursor_state: CursorState) -> Option<CursorState> {
        if let Some(mut command) = self.redo_stack.pop() {
            let cursor_to_restore = command.get_cursor_to_restore_on_redo();

            command.set_cursor_before_execute(cursor_state);

            command.execute();

            self.undo_stack.push(command);
            cursor_to_restore
        } else {
            None
        }
    }
}
