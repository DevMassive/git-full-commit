use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[cfg(test)]
mod reorder_commits_command_test;
#[cfg(test)]
mod stage_all_command_test;
#[cfg(test)]
mod stage_file_command_test;
#[cfg(test)]
mod stage_unstaged_command_test;
#[cfg(test)]
mod stage_untracked_command_test;
#[cfg(test)]
mod test_helpers;
#[cfg(test)]
mod unstage_all_command_test;
#[cfg(test)]
mod unstage_file_command_test;

mod fixup_commit;
#[cfg(test)]
mod fixup_commit_test;

use crate::cursor_state::CursorState;
use crate::git::{self, CommitInfo};
use crate::ui::main_screen::ListItem;

pub use fixup_commit::FixupCommitCommand;

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
    patch: String,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl UnstageFileCommand {
    pub fn new(repo_path: PathBuf, file_name: String) -> Self {
        let patch = git::get_file_diff_patch(&repo_path, &file_name).unwrap_or_default();
        Self {
            repo_path,
            file_name,
            patch,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

pub struct ReorderCommitsCommand {
    pub repo_path: PathBuf,
    pub original_commits: Vec<CommitInfo>,
    pub reordered_commits: Vec<CommitInfo>,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl ReorderCommitsCommand {
    pub fn new(
        repo_path: PathBuf,
        original_commits: Vec<CommitInfo>,
        reordered_commits: Vec<CommitInfo>,
    ) -> Self {
        Self {
            repo_path,
            original_commits,
            reordered_commits,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for ReorderCommitsCommand {
    fn execute(&mut self) -> bool {
        if self.original_commits.len() < 2 {
            return false;
        }
        if self.original_commits == self.reordered_commits {
            return true;
        }

        // --- Get original state ---
        let original_branch = match git::get_current_branch_name(&self.repo_path) {
            Ok(name) => name,
            Err(_) => return false,
        };
        let stashed = match git::stash_unstaged_changes(&self.repo_path) {
            Ok(stashed) => stashed,
            Err(_) => return false,
        };

        // --- Find the base and the commits to re-order ---
        let mut original_chrono = self.original_commits.clone();
        original_chrono.reverse();
        let mut reordered_chrono = self.reordered_commits.clone();
        reordered_chrono.reverse();

        let mut base_commit_opt: Option<&CommitInfo> = None;
        let mut first_diverged_idx = 0;
        for (i, (original, reordered)) in original_chrono
            .iter()
            .zip(reordered_chrono.iter())
            .enumerate()
        {
            if original == reordered {
                base_commit_opt = Some(original);
                first_diverged_idx = i + 1;
            } else {
                break;
            }
        }

        let commits_to_pick = &reordered_chrono[first_diverged_idx..];
        let temp_branch = format!("reorder-temp-{}", chrono::Utc::now().timestamp());

        // --- Create temp branch ---
        if let Some(base_commit) = base_commit_opt {
            // Normal case: create branch from the last common commit
            if git::create_branch_at(&self.repo_path, &temp_branch, &base_commit.hash).is_err() {
                if stashed {
                    let _ = git::pop_stash(&self.repo_path);
                }
                return false;
            }
            if git::checkout_branch(&self.repo_path, &temp_branch).is_err() {
                let _ = git::delete_branch(&self.repo_path, &temp_branch, true);
                if stashed {
                    let _ = git::pop_stash(&self.repo_path);
                }
                return false;
            }
        } else {
            // Root reorder case: create an orphan branch
            if git::checkout_orphan_branch(&self.repo_path, &temp_branch).is_err() {
                if stashed {
                    let _ = git::pop_stash(&self.repo_path);
                }
                return false;
            }
        }

        // --- Re-create history on temp branch ---
        let rebase_failed = |_e: anyhow::Error| {
            let _ = git::cherry_pick_abort(&self.repo_path);
            let _ = git::checkout_branch(&self.repo_path, &original_branch);
            let _ = git::delete_branch(&self.repo_path, &temp_branch, true);
            if stashed {
                let _ = git::pop_stash(&self.repo_path);
            }
        };

        // Iterate through chronological list of commits to apply
        for commit in commits_to_pick {
            if commit.is_fixup {
                // If it's a fixup, we apply its changes to the staging area, then amend them into the previous commit.
                if let Err(e) = git::cherry_pick_no_commit(&self.repo_path, &commit.hash) {
                    rebase_failed(e);
                    return false;
                }
                if let Err(e) = git::commit_amend_no_edit(&self.repo_path) {
                    rebase_failed(e);
                    return false;
                }
            } else {
                // Otherwise, just pick the commit
                if let Err(e) = git::cherry_pick(&self.repo_path, &commit.hash) {
                    rebase_failed(e);
                    return false;
                }
            }

            // Check if the message needs to be amended
            let original_commit = self.original_commits.iter().find(|c| c.hash == commit.hash);

            if let Some(original) = original_commit {
                if original.message != commit.message {
                    if let Err(e) = git::commit_amend_with_message(&self.repo_path, &commit.message)
                    {
                        rebase_failed(e);
                        return false;
                    }
                }
            }
        }

        // --- Update original branch ---
        if git::checkout_branch(&self.repo_path, &original_branch).is_err() {
            // Recovery is hard here. Leave temp branch for manual recovery.
            return false;
        }
        if git::reset_hard(&self.repo_path, &temp_branch).is_err() {
            return false;
        }

        // --- Final cleanup ---
        let _ = git::delete_branch(&self.repo_path, &temp_branch, true);
        if stashed {
            let _ = git::pop_stash(&self.repo_path);
        }

        true
    }

    fn undo(&mut self) {
        let _ = git::reset_hard(&self.repo_path, "HEAD@{1}");
    }

    command_impl!();
}

pub struct DiscardCommitCommand {
    pub list_items: *mut Vec<ListItem>,
    pub index: usize,
    removed_item: Option<ListItem>,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl DiscardCommitCommand {
    pub fn new(list_items: *mut Vec<ListItem>, index: usize) -> Self {
        Self {
            list_items,
            index,
            removed_item: None,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for DiscardCommitCommand {
    fn execute(&mut self) -> bool {
        unsafe {
            self.removed_item = Some((*self.list_items).remove(self.index));
        }
        true
    }

    fn undo(&mut self) {
        if let Some(item) = self.removed_item.take() {
            unsafe {
                (*self.list_items).insert(self.index, item);
            }
        }
    }

    command_impl!();
}

pub struct SwapCommitCommand {
    pub list_items: *mut Vec<ListItem>,
    pub index1: usize,
    pub index2: usize,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl SwapCommitCommand {
    pub fn new(list_items: *mut Vec<ListItem>, index1: usize, index2: usize) -> Self {
        Self {
            list_items,
            index1,
            index2,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for SwapCommitCommand {
    fn execute(&mut self) -> bool {
        unsafe {
            (*self.list_items).swap(self.index1, self.index2);
        }
        true
    }

    fn undo(&mut self) {
        unsafe {
            (*self.list_items).swap(self.index1, self.index2);
        }
    }

    command_impl!();
}

impl Command for UnstageFileCommand {
    fn execute(&mut self) -> bool {
        git::unstage_file(&self.repo_path, &self.file_name).expect("Failed to unstage file.");
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
