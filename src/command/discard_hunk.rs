use std::path::PathBuf;

use lazy_static::lazy_static;
use regex::Regex;

use super::Command;
use crate::cursor_state::CursorState;
use crate::git;

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

fn get_file_name_from_patch(patch: &str) -> Option<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"^diff --git a/("[^"]+"|\S+)"#).unwrap();
    }
    patch.lines().next().and_then(|line| {
        RE.captures(line).and_then(|caps| {
            caps.get(1)
                .map(|m| m.as_str().trim_matches('"').to_string())
        })
    })
}
