use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::cursor_state::CursorState;
use crate::git;

pub trait Command {
    fn execute(&mut self) -> bool;
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
    fn execute(&mut self) -> bool {
        git::unstage_file(&self.repo_path, &self.file_name).expect("Failed to unstage file.");
        true
    }

    fn undo(&mut self) {
        git::stage_file(&self.repo_path, &self.file_name).expect("Failed to stage file.");
    }

    command_impl!();
}

pub struct IgnoreUnstagedTrackedFileCommand {
    pub repo_path: std::path::PathBuf,
    pub file_name: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl IgnoreUnstagedTrackedFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String) -> Self {
        Self {
            repo_path,
            file_name,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for IgnoreUnstagedTrackedFileCommand {
    fn execute(&mut self) -> bool {
        let gitignore_path = self.repo_path.join(".gitignore");
        let mut gitignore = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(gitignore_path)
            .expect("Failed to open .gitignore");
        writeln!(gitignore, "{}", self.file_name).expect("Failed to write to .gitignore");

        git::stage_path(&self.repo_path, ".gitignore").expect("Failed to stage .gitignore");
        git::rm_cached(&self.repo_path, &self.file_name).expect("Failed to unstage file");
        true
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
                git::rm_cached(&self.repo_path, ".gitignore")
                    .expect("Failed to remove .gitignore from index");
            } else {
                fs::write(&gitignore_path, new_content + "\n")
                    .expect("Failed to write to .gitignore");
                git::stage_path(&self.repo_path, ".gitignore").expect("Failed to stage .gitignore");
            }
        }

        // Re-track the file, then unstage it to restore original state
        git::stage_file(&self.repo_path, &self.file_name).expect("Failed to re-track file");
        git::unstage_file(&self.repo_path, &self.file_name)
            .expect("Failed to unstage file to restore state");
    }

    command_impl!();
}

pub struct DeleteUntrackedFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    content: Vec<u8>,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl DeleteUntrackedFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String, content: Vec<u8>) -> Self {
        Self {
            repo_path,
            file_name,
            content,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for DeleteUntrackedFileCommand {
    fn execute(&mut self) -> bool {
        fs::remove_file(self.repo_path.join(&self.file_name)).expect("Failed to delete file");
        true
    }

    fn undo(&mut self) {
        fs::write(self.repo_path.join(&self.file_name), &self.content)
            .expect("Failed to restore file");
    }

    command_impl!();
}

pub struct IgnoreUntrackedFileCommand {
    pub repo_path: std::path::PathBuf,
    pub file_name: String,
    was_empty_before: bool,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl IgnoreUntrackedFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String) -> Self {
        let gitignore_path = repo_path.join(".gitignore");
        let was_empty_before = !gitignore_path.exists()
            || fs::read_to_string(gitignore_path)
                .map(|c| c.trim().is_empty())
                .unwrap_or(true);
        Self {
            repo_path,
            file_name,
            was_empty_before,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for IgnoreUntrackedFileCommand {
    fn execute(&mut self) -> bool {
        let gitignore_path = self.repo_path.join(".gitignore");
        let mut gitignore = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(gitignore_path)
            .expect("Failed to open .gitignore");
        writeln!(gitignore, "{}", self.file_name).expect("Failed to write to .gitignore");

        git::stage_path(&self.repo_path, ".gitignore").expect("Failed to stage .gitignore");
        true
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
                if !self.was_empty_before {
                    // If the file was not empty before, we need to remove it from the index
                    git::rm_cached(&self.repo_path, ".gitignore")
                        .expect("Failed to remove .gitignore from index");
                }
            } else {
                fs::write(&gitignore_path, new_content + "\n")
                    .expect("Failed to write to .gitignore");
                git::stage_path(&self.repo_path, ".gitignore").expect("Failed to stage .gitignore");
            }
        }
    }

    command_impl!();
}

pub struct DiscardUnstagedHunkCommand {
    pub repo_path: PathBuf,
    pub patch: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl DiscardUnstagedHunkCommand {
    pub fn new(repo_path: PathBuf, patch: String) -> Self {
        Self {
            repo_path,
            patch,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for DiscardUnstagedHunkCommand {
    fn execute(&mut self) -> bool {
        git::apply_patch(&self.repo_path, &self.patch, true, false)
            .expect("Failed to discard hunk from working tree.");
        true
    }

    fn undo(&mut self) {
        git::apply_patch(&self.repo_path, &self.patch, false, false)
            .expect("Failed to re-apply hunk to working tree.");
    }

    command_impl!();
}

pub struct UnstageAllCommand {
    pub repo_path: PathBuf,
    patch: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl UnstageAllCommand {
    pub fn new(repo_path: PathBuf) -> Self {
        let patch = git::get_staged_diff_patch(&repo_path).unwrap_or_default();
        Self {
            repo_path,
            patch,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for UnstageAllCommand {
    fn execute(&mut self) -> bool {
        git::unstage_all(&self.repo_path).expect("Failed to unstage all files.");
        true
    }

    fn undo(&mut self) {
        if !self.patch.is_empty() {
            git::apply_patch(&self.repo_path, &self.patch, false, true)
                .expect("Failed to apply patch for unstage undo.");
        }
    }

    command_impl!();
}

pub struct StageUnstagedCommand {
    pub repo_path: PathBuf,
    files_to_stage: Vec<String>,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl StageUnstagedCommand {
    pub fn new(repo_path: PathBuf) -> Self {
        let files_to_stage = git::get_unstaged_files(&repo_path).unwrap_or_default();
        Self {
            repo_path,
            files_to_stage,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for StageUnstagedCommand {
    fn execute(&mut self) -> bool {
        for file in &self.files_to_stage {
            git::stage_file(&self.repo_path, file).expect("Failed to stage file.");
        }
        true
    }

    fn undo(&mut self) {
        for file in &self.files_to_stage {
            git::unstage_file(&self.repo_path, file).expect("Failed to unstage file.");
        }
    }

    command_impl!();
}

pub struct StageUntrackedCommand {
    pub repo_path: PathBuf,
    untracked_files: Vec<String>,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl StageUntrackedCommand {
    pub fn new(repo_path: PathBuf) -> Self {
        let untracked_files = git::get_untracked_files(&repo_path).unwrap_or_default();
        Self {
            repo_path,
            untracked_files,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for StageUntrackedCommand {
    fn execute(&mut self) -> bool {
        for file in &self.untracked_files {
            git::stage_file(&self.repo_path, file).expect("Failed to stage untracked file.");
        }
        true
    }

    fn undo(&mut self) {
        for file in &self.untracked_files {
            git::rm_cached(&self.repo_path, file).expect("Failed to unstage untracked file.");
        }
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
    fn execute(&mut self) -> bool {
        git::stage_file(&self.repo_path, &self.file_name).expect("Failed to stage file.");
        true
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
    fn execute(&mut self) -> bool {
        git::apply_patch(&self.repo_path, &self.patch, true, true)
            .expect("Failed to apply patch in reverse.");
        true
    }

    fn undo(&mut self) {
        git::apply_patch(&self.repo_path, &self.patch, false, true)
            .expect("Failed to apply patch.");
    }

    command_impl!();
}

fn get_file_name_from_patch(patch: &str) -> Option<String> {
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r#"^diff --git a/("[^"]+"|\S+)"#).unwrap();
    }
    patch.lines().next().and_then(|line| {
        RE.captures(line).and_then(|caps| {
            caps.get(1)
                .map(|m| m.as_str().trim_matches('"').to_string())
        })
    })
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
    fn execute(&mut self) -> bool {
        git::apply_patch(&self.repo_path, &self.patch, false, true)
            .expect("Failed to apply patch.");
        true
    }

    fn undo(&mut self) {
        git::apply_patch(&self.repo_path, &self.patch, true, true)
            .expect("Failed to apply patch in reverse.");
    }

    command_impl!();
}

pub struct DiscardFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    staged_patch: String,
    is_new_file: bool,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl DiscardFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String, is_new_file: bool) -> Self {
        let staged_patch = git::get_file_diff_patch(&repo_path, &file_name).unwrap_or_default();
        Self {
            repo_path,
            file_name,
            staged_patch,
            is_new_file,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for DiscardFileCommand {
    fn execute(&mut self) -> bool {
        if git::has_unstaged_changes_in_file(&self.repo_path, &self.file_name).unwrap_or(true) {
            // Don't discard if there are unstaged changes
            return false;
        }

        if self.is_new_file {
            git::rm_cached(&self.repo_path, &self.file_name)
                .expect("Failed to remove file from index");
            fs::remove_file(self.repo_path.join(&self.file_name))
                .expect("Failed to delete new file");
        } else {
            git::unstage_file(&self.repo_path, &self.file_name).expect("Failed to unstage file.");
            git::checkout_file(&self.repo_path, &self.file_name).expect("Failed to checkout file.");
        }
        true
    }

    fn undo(&mut self) {
        if self.is_new_file {
            git::apply_patch(&self.repo_path, &self.staged_patch, false, true)
                .expect("Failed to re-apply patch for new file.");
            git::checkout_file(&self.repo_path, &self.file_name)
                .expect("Failed to checkout file after undoing discard.");
        } else {
            git::apply_patch(&self.repo_path, &self.staged_patch, false, false)
                .expect("Failed to re-apply patch to working tree for undo.");
            git::apply_patch(&self.repo_path, &self.staged_patch, false, true)
                .expect("Failed to re-apply patch to index for undo.");
        }
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
    fn execute(&mut self) -> bool {
        if let Some(file_name) = get_file_name_from_patch(&self.patch) {
            if git::has_unstaged_changes_in_file(&self.repo_path, &file_name).unwrap_or(true) {
                // Don't discard if there are unstaged changes
                return false;
            }
        }

        // Unstage
        git::apply_patch(&self.repo_path, &self.patch, true, true)
            .expect("Failed to unstage hunk.");
        // Discard from working tree
        git::apply_patch(&self.repo_path, &self.patch, true, false)
            .expect("Failed to discard hunk from working tree.");
        true
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
    fn execute(&mut self) -> bool {
        git::checkout_file(&self.repo_path, &self.file_name).expect("Failed to checkout file.");
        true
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
    fn execute(&mut self) -> bool {
        let gitignore_path = self.repo_path.join(".gitignore");
        let mut gitignore = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(gitignore_path)
            .expect("Failed to open .gitignore");
        writeln!(gitignore, "{}", self.file_name).expect("Failed to write to .gitignore");

        git::stage_path(&self.repo_path, ".gitignore").expect("Failed to stage .gitignore");

        // For staged files, we need to unstage them.
        git::rm_cached(&self.repo_path, &self.file_name).expect("Failed to unstage file");
        true
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
    fn execute(&mut self) -> bool {
        git::rm_file(&self.repo_path, &self.file_name).expect("Failed to remove file.");
        true
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
    fn execute(&mut self) -> bool {
        git::add_all(&self.repo_path).expect("Failed to stage all files.");
        true
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
        if command.execute() {
            self.undo_stack.push(command);
            self.redo_stack.clear();
        }
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
