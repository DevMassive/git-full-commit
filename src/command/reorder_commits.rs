use std::path::PathBuf;

use super::Command;
use crate::cursor_state::CursorState;
use crate::git::{self, CommitInfo};

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
