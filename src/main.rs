use anyhow::Result;
use clap::Parser;
use git_reset_pp::run;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the git repository
    #[arg(short, long, default_value = ".")]
    repo: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    run(args.repo)?;
    Ok(())
}
