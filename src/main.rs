use anyhow::{Result, bail};
use clap::Parser;
use git_full_commit::run;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the git repository
    #[arg(short, long)]
    repo: Option<PathBuf>,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.debug {
        // Truncate the log file
        let _ = std::fs::File::create("debug.log");
    }
    let repo_path = match args.repo {
        Some(path) => path,
        None => {
            let output = Command::new("git")
                .arg("rev-parse")
                .arg("--show-toplevel")
                .output()?;
            if !output.status.success() {
                bail!("fatal: not a git repository (or any of the parent directories): .git");
            }
            PathBuf::from(String::from_utf8(output.stdout)?.trim())
        }
    };
    run(repo_path, args.debug)?;
    Ok(())
}
