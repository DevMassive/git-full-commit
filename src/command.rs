use std::path::PathBuf;
use std::process::Command as OsCommand;

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

pub struct ApplyPatchCommand {
    pub repo_path: PathBuf,
    pub patch: String,
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

pub struct CheckoutFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    pub patch: String,
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

pub struct RemoveFileCommand {
    pub repo_path: PathBuf,
    pub file_name: String,
    pub patch: String,
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
