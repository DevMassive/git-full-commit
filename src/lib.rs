use anyhow::{Result, bail};
use std::path::PathBuf;

pub mod app_state;
pub mod command;
mod commit_storage;
pub mod cursor_state;
pub mod external_command;
pub mod git;
pub mod git_patch;
pub mod ui;

pub fn run(repo_path: PathBuf) -> Result<()> {
    if !git::is_git_repository(&repo_path) {
        bail!("fatal: not a git repository (or any of the parent directories): .git");
    }

    let files = git::get_diff(repo_path.clone());
    ui::tui_loop(repo_path.clone(), files);

    Ok(())
}
