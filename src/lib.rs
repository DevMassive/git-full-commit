use anyhow::Result;
use std::path::PathBuf;

pub mod app_state;
pub mod command;
mod commit_storage;
pub mod cursor_state;
pub mod external_command;
pub mod git;
pub mod git_patch;
pub mod ui;
pub mod util;

pub fn run(repo_path: PathBuf, debug: bool) -> Result<()> {
    let staged_diff_output = git::get_staged_diff_output(&repo_path)?;

    if staged_diff_output.stdout.is_empty() {
        git::add_all(&repo_path)?;
    }

    let files = git::get_diff(repo_path.clone());
    ui::tui_loop(repo_path.clone(), files, debug);

    Ok(())
}
