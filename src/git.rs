use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command as OsCommand;

fn git_command() -> OsCommand {
    let mut command = OsCommand::new("git");
    command.arg("-c").arg("core.quotepath=false");
    command
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Added,
    Modified,
    Renamed,
    Deleted,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub start_line: usize,
    pub lines: Vec<String>,
    pub old_start: usize,
    pub new_start: usize,
    pub line_numbers: Vec<(usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub file_name: String,
    pub hunks: Vec<Hunk>,
    pub lines: Vec<String>,
    pub status: FileStatus,
}

pub fn get_previous_commit_message(repo_path: &Path) -> Result<String> {
    let output = git_command()
        .arg("log")
        .arg("-1")
        .arg("--pretty=%s")
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        return Ok(String::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn get_previous_commit_info(repo_path: &Path) -> Result<(String, String)> {
    let output = git_command()
        .arg("log")
        .arg("-1")
        .arg("--pretty=%h %s")
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        return Ok((String::new(), String::new()));
    }
    let full_string = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let mut parts = full_string.splitn(2, ' ');
    let hash = parts.next().unwrap_or("").to_string();
    let message = parts.next().unwrap_or("").to_string();
    Ok((hash, message))
}

fn calc_line_numbers(hunk: &Hunk) -> Vec<(usize, usize)> {
    let mut line_numbers: Vec<(usize, usize)> = Vec::new();
    let mut old_line_counter: i32 = hunk.old_start as i32 - 1;
    let mut new_line_counter: i32 = hunk.new_start as i32 - 1;
    for (i, hunk_line) in hunk.lines.iter().enumerate() {
        if i == 0 {
            // Ignore the hunk header
            line_numbers.push((0, 0));
            continue;
        } else if hunk_line.starts_with('+') {
            new_line_counter += 1;
        } else if hunk_line.starts_with('-') {
            old_line_counter += 1;
        } else {
            old_line_counter += 1;
            new_line_counter += 1;
        }
        line_numbers.push((
            old_line_counter.max(0) as usize,
            new_line_counter.max(0) as usize,
        ));
    }
    line_numbers
}

fn parse_diff(diff_str: &str) -> Vec<FileDiff> {
    let mut files = Vec::new();
    let mut current_file: Option<FileDiff> = None;
    let mut current_hunk: Option<Hunk> = None;
    let mut current_file_lines: Vec<String> = Vec::new();
    let mut current_file_line_index = 0;

    let diff_line_re = Regex::new(r#"^diff --git a/("[^"]+"|\S+) b/("[^"]+"|\S+)"#).unwrap();

    for line in diff_str.lines() {
        if let Some(caps) = diff_line_re.captures(line) {
            if let Some(mut file) = current_file.take() {
                if let Some(mut hunk) = current_hunk.take() {
                    hunk.line_numbers = calc_line_numbers(&hunk);
                    file.hunks.push(hunk);
                }
                file.lines = current_file_lines;
                files.push(file);
                current_file_lines = Vec::new();
                current_file_line_index = 0;
            }

            let file_name = caps.get(2).map(|m| m.as_str().trim_matches('"')).unwrap_or("").to_string();

            current_file = Some(FileDiff {
                file_name,
                hunks: Vec::new(),
                lines: Vec::new(), // Will be filled in later
                status: FileStatus::Modified,
            });
        } else if line.starts_with("new file mode") {
            if let Some(file) = current_file.as_mut() {
                file.status = FileStatus::Added;
            }
        } else if line.starts_with("deleted file mode") {
            if let Some(file) = current_file.as_mut() {
                file.status = FileStatus::Deleted;
            }
        } else if line.starts_with("rename from") {
            if let Some(file) = current_file.as_mut() {
                file.status = FileStatus::Renamed;
            }

        } else if line.starts_with("@@ ") {
            if let Some(mut hunk) = current_hunk.take() {
                if let Some(file) = current_file.as_mut() {
                    hunk.line_numbers = calc_line_numbers(&hunk);
                    file.hunks.push(hunk);
                }
            }

            let parts: Vec<&str> = line.split(' ').collect();
            let old_start = parts
                .get(1)
                .and_then(|s| s.split(',').next())
                .and_then(|s| s.trim_start_matches('-').parse::<usize>().ok())
                .unwrap_or(0);
            let new_start = parts
                .get(2)
                .and_then(|s| s.split(',').next())
                .and_then(|s| s.trim_start_matches('+').parse::<usize>().ok())
                .unwrap_or(0);

            current_hunk = Some(Hunk {
                start_line: current_file_line_index,
                lines: vec![line.to_string()],
                old_start,
                new_start,
                line_numbers: Vec::new(),
            });
        } else if let Some(hunk) = current_hunk.as_mut() {
            hunk.lines.push(line.to_string());
        }

        if current_file.is_some() {
            current_file_lines.push(line.to_string());
            current_file_line_index += 1;
        }
    }

    if let Some(mut file) = current_file.take() {
        if let Some(mut hunk) = current_hunk.take() {
            hunk.line_numbers = calc_line_numbers(&hunk);
            file.hunks.push(hunk);
        }
        file.lines = current_file_lines;
        files.push(file);
    }

    files
}

pub fn get_diff(repo_path: PathBuf) -> Vec<FileDiff> {
    let output = git_command()
        .arg("diff")
        .arg("--staged")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to execute git diff");

    let diff_str = String::from_utf8_lossy(&output.stdout);
    parse_diff(&diff_str)
}

pub fn get_previous_commit_diff(repo_path: &Path) -> Result<Vec<FileDiff>> {
    let output = git_command()
        .arg("show")
        .arg("HEAD")
        .current_dir(repo_path)
        .output()?;

    let diff_str = String::from_utf8_lossy(&output.stdout);
    Ok(parse_diff(&diff_str))
}

pub fn has_unstaged_changes(repo_path: &Path) -> Result<bool> {
    let output = git_command()
        .arg("status")
        .arg("--porcelain")
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        return Ok(false);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let mut chars = line.chars();
        let x = chars.next();
        let y = chars.next();

        if x == Some('?') && y == Some('?') {
            return Ok(true); // Untracked
        }
        if y.is_some() && y != Some(' ') {
            return Ok(true); // Modified in work tree
        }
    }

    Ok(false)
}

pub fn is_git_repository(path: &Path) -> bool {
    path.join(".git").is_dir()
}

pub fn commit(repo_path: &Path, message: &str) -> Result<()> {
    git_command()
        .arg("commit")
        .arg("-m")
        .arg(message)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn amend_commit(repo_path: &Path, message: &str) -> Result<()> {
    git_command()
        .arg("commit")
        .arg("--amend")
        .arg("-m")
        .arg(message)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn add_all_with_size_limit(repo_path: &Path, size_limit: u64) -> Result<()> {
    // Stage all modified and deleted files
    git_command()
        .arg("add")
        .arg("--update")
        .current_dir(repo_path)
        .output()?;

    // Get untracked files
    let output = git_command()
        .arg("ls-files")
        .arg("--others")
        .arg("--exclude-standard")
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for file_name in stdout.lines() {
        if file_name.is_empty() {
            continue;
        }

        let file_path = repo_path.join(file_name);
        if let Ok(metadata) = std::fs::metadata(&file_path) {
            if metadata.len() <= size_limit {
                stage_file(repo_path, file_name)?;
            }
        }
    }
    Ok(())
}

pub fn add_all(repo_path: &Path) -> Result<()> {
    add_all_with_size_limit(repo_path, 100 * 1024 * 1024)
}

pub fn unstage_all(repo_path: &Path) -> Result<()> {
    git_command()
        .arg("reset")
        .arg("HEAD")
        .arg(".")
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn get_staged_diff_output(repo_path: &Path) -> Result<std::process::Output> {
    let output = git_command()
        .arg("diff")
        .arg("--staged")
        .current_dir(repo_path)
        .output()?;
    Ok(output)
}

pub fn get_unstaged_diff(repo_path: &Path) -> Vec<FileDiff> {
    let output = git_command()
        .arg("diff")
        .current_dir(repo_path)
        .output()
        .expect("Failed to execute git diff");

    let diff_str = String::from_utf8_lossy(&output.stdout);
    parse_diff(&diff_str)
}

pub fn get_unstaged_files(repo_path: &Path) -> Result<Vec<String>> {
    let output = git_command()
        .arg("diff")
        .arg("--name-only")
        .current_dir(repo_path)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(String::from).collect())
}

pub fn get_untracked_files(repo_path: &Path) -> Result<Vec<String>> {
    let output = git_command()
        .arg("ls-files")
        .arg("--others")
        .arg("--exclude-standard")
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let untracked_files = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect();

    Ok(untracked_files)
}

pub fn get_unstaged_diff_patch(repo_path: &Path) -> Result<String> {
    let output = git_command()
        .arg("diff")
        .current_dir(repo_path)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn get_staged_diff_patch(repo_path: &Path) -> Result<String> {
    let output = git_command()
        .arg("diff")
        .arg("--staged")
        .current_dir(repo_path)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn get_unstaged_file_diff_patch(repo_path: &Path, file_name: &str) -> Result<String> {
    let output = git_command()
        .arg("diff")
        .arg("--")
        .arg(file_name)
        .current_dir(repo_path)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn get_file_diff_patch(repo_path: &Path, file_name: &str) -> Result<String> {
    let output = git_command()
        .arg("diff")
        .arg("--staged")
        .arg("--")
        .arg(file_name)
        .current_dir(repo_path)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn unstage_file(repo_path: &Path, file_name: &str) -> Result<()> {
    git_command()
        .arg("reset")
        .arg("HEAD")
        .arg("--")
        .arg(file_name)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn stage_file(repo_path: &Path, file_name: &str) -> Result<()> {
    git_command()
        .arg("add")
        .arg(file_name)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn apply_patch(repo_path: &Path, patch: &str, reverse: bool, cached: bool) -> Result<()> {
    use std::io::Write;
    use std::process::Stdio;

    let mut args = vec!["apply"];
    if cached {
        args.push("--cached");
    }
    if reverse {
        args.push("--reverse");
    }
    args.push("--unidiff-zero");
    args.push("-");

    let mut child = git_command()
        .args(&args)
        .current_dir(repo_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(patch.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        anyhow::bail!(
            "git apply failed (reverse={}):
--- stderr ---
{}",
            reverse,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

pub fn checkout_file(repo_path: &Path, file_name: &str) -> Result<()> {
    git_command()
        .arg("checkout")
        .arg("HEAD")
        .arg("--")
        .arg(file_name)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn rm_file(repo_path: &Path, file_name: &str) -> Result<()> {
    git_command()
        .arg("rm")
        .arg("-f")
        .arg(file_name)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn stage_path(repo_path: &Path, path: &str) -> Result<()> {
    git_command()
        .arg("add")
        .arg(path)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn rm_cached(repo_path: &Path, path: &str) -> Result<()> {
    git_command()
        .arg("rm")
        .arg("--cached")
        .arg(path)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn read_file_content(repo_path: &Path, file_path: &str) -> Result<(Vec<u8>, usize)> {
    let full_path = repo_path.join(file_path);
    let metadata = std::fs::metadata(&full_path)?;
    let content = std::fs::read(&full_path)?;
    Ok((content, metadata.len() as usize))
}

pub fn rm_file_from_index(repo_path: &Path, path: &str) -> Result<()> {
    git_command()
        .arg("rm")
        .arg(path)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn is_commit_on_remote(repo_path: &Path, hash: &str) -> Result<bool> {
    if hash.is_empty() {
        return Ok(false);
    }
    let output = git_command()
        .arg("branch")
        .arg("-r")
        .arg("--contains")
        .arg(hash)
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        return Ok(false);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(!stdout.trim().is_empty())
}