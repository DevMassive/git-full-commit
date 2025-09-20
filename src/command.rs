use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::git;

pub trait Command {
    fn execute(&mut self);
    fn undo(&mut self);
}

pub struct UnstageFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
}

impl Command for UnstageFileCommand {
    fn execute(&mut self) {
        git::unstage_file(&self.repo_path, &self.file_name).expect("Failed to unstage file.");
    }

    fn undo(&mut self) {
        git::stage_file(&self.repo_path, &self.file_name).expect("Failed to stage file.");
    }
}

pub struct ApplyPatchCommand {
    pub repo_path: PathBuf,
    pub patch: String,
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
}

pub struct StagePatchCommand {
    pub repo_path: PathBuf,
    pub patch: String,
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
}

pub struct DiscardHunkCommand {
    pub repo_path: PathBuf,
    pub patch: String,
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
        git::apply_patch(&self.repo_path, &self.patch, false, true)
            .expect("Failed to stage hunk.");
    }
}

pub struct CheckoutFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    pub patch: String,
}

impl Command for CheckoutFileCommand {
    fn execute(&mut self) {
        git::checkout_file(&self.repo_path, &self.file_name).expect("Failed to checkout file.");
    }

    fn undo(&mut self) {
        git::apply_patch(&self.repo_path, &self.patch, false, false)
            .expect("Failed to apply patch for checkout undo.");
    }
}

pub struct IgnoreFileCommand {
    pub repo_path: std::path::PathBuf,
    pub file_name: String,
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
}

pub struct RemoveFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    pub patch: String,
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

    pub fn execute(&mut self, mut command: Box<dyn Command>) {
        command.execute();
        self.undo_stack.push(command);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(mut command) = self.undo_stack.pop() {
            command.undo();
            self.redo_stack.push(command);
        }
    }

    pub fn redo(&mut self) {
        if let Some(mut command) = self.redo_stack.pop() {
            command.execute();
            self.undo_stack.push(command);
        }
    }
}
