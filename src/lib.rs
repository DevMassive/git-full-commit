use anyhow::{Result, bail};
use std::path::PathBuf;
use std::process::Command as OsCommand;

pub mod app_state;
pub mod command;
mod commit_storage;
pub mod git;
pub mod git_patch;
pub mod ui;

pub fn run(repo_path: PathBuf) -> Result<()> {
    if !git::is_git_repository(&repo_path) {
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

    let files = git::get_diff(repo_path.clone());

    if files.is_empty() {
        bail!("No changes found.");
    }

    ui::tui_loop(repo_path.clone(), files);

    Ok(())
}
