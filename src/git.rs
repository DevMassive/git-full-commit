use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command as OsCommand;

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
    pub line_numbers: Vec<(Option<usize>, Option<usize>)>,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub file_name: String,
    pub hunks: Vec<Hunk>,
    pub lines: Vec<String>,
    pub status: FileStatus,
}

pub fn get_previous_commit_message(repo_path: &Path) -> Result<String> {
    let output = OsCommand::new("git")
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
    let output = OsCommand::new("git")
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

fn parse_diff(diff_str: &str) -> Vec<FileDiff> {
    let mut files = Vec::new();
    let mut current_file: Option<FileDiff> = None;
    let mut current_hunk: Option<Hunk> = None;
    let mut current_file_lines: Vec<String> = Vec::new();
    let mut current_file_line_index = 0;

    for line in diff_str.lines() {
        if line.starts_with("diff --git") {
            if let Some(mut file) = current_file.take() {
                if let Some(mut hunk) = current_hunk.take() {
                    let mut line_numbers = Vec::new();
                    let mut old_line_counter = hunk.old_start;
                    let mut new_line_counter = hunk.new_start;
                    for (i, hunk_line) in hunk.lines.iter().enumerate() {
                        if i == 0 {
                            // Skip hunk header
                            line_numbers.push((None, None));
                            continue;
                        }
                        if hunk_line.starts_with('+') {
                            line_numbers.push((None, Some(new_line_counter)));
                            new_line_counter += 1;
                        } else if hunk_line.starts_with('-') {
                            line_numbers.push((Some(old_line_counter), None));
                            old_line_counter += 1;
                        } else {
                            line_numbers.push((Some(old_line_counter), Some(new_line_counter)));
                            old_line_counter += 1;
                            new_line_counter += 1;
                        }
                    }
                    hunk.line_numbers = line_numbers;
                    file.hunks.push(hunk);
                }
                file.lines = current_file_lines;
                files.push(file);
                current_file_lines = Vec::new();
                current_file_line_index = 0;
            }
            let file_name_part = line.split(' ').nth(2).unwrap_or("");
            let file_name = if file_name_part.starts_with("a/") {
                &file_name_part[2..]
            } else {
                file_name_part
            };
            current_file = Some(FileDiff {
                file_name: file_name.to_string(),
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
        } else if line.starts_with("rename to") {
            let new_name = line.split(' ').nth(2).unwrap_or("").trim();
            if !new_name.is_empty() {
                if let Some(file) = current_file.as_mut() {
                    file.file_name = new_name.to_string();
                }
            }
        } else if line.starts_with("@@ ") {
            if let Some(mut hunk) = current_hunk.take() {
                if let Some(file) = current_file.as_mut() {
                    let mut line_numbers = Vec::new();
                    let mut old_line_counter = hunk.old_start;
                    let mut new_line_counter = hunk.new_start;
                    for (i, hunk_line) in hunk.lines.iter().enumerate() {
                        if i == 0 {
                            // Skip hunk header
                            line_numbers.push((None, None));
                            continue;
                        }
                        if hunk_line.starts_with('+') {
                            line_numbers.push((None, Some(new_line_counter)));
                            new_line_counter += 1;
                        } else if hunk_line.starts_with('-') {
                            line_numbers.push((Some(old_line_counter), None));
                            old_line_counter += 1;
                        } else {
                            line_numbers.push((Some(old_line_counter), Some(new_line_counter)));
                            old_line_counter += 1;
                            new_line_counter += 1;
                        }
                    }
                    hunk.line_numbers = line_numbers;
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
            let mut line_numbers = Vec::new();
            let mut old_line_counter = hunk.old_start;
            let mut new_line_counter = hunk.new_start;
            for (i, hunk_line) in hunk.lines.iter().enumerate() {
                if i == 0 {
                    // Skip hunk header
                    line_numbers.push((None, None));
                    continue;
                }
                if hunk_line.starts_with('+') {
                    line_numbers.push((None, Some(new_line_counter)));
                    new_line_counter += 1;
                } else if hunk_line.starts_with('-') {
                    line_numbers.push((Some(old_line_counter), None));
                    old_line_counter += 1;
                } else {
                    line_numbers.push((Some(old_line_counter), Some(new_line_counter)));
                    old_line_counter += 1;
                    new_line_counter += 1;
                }
            }
            hunk.line_numbers = line_numbers;
            file.hunks.push(hunk);
        }
        file.lines = current_file_lines;
        files.push(file);
    }

    files
}

pub fn get_diff(repo_path: PathBuf) -> Vec<FileDiff> {
    let output = OsCommand::new("git")
        .arg("diff")
        .arg("--staged")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to execute git diff");

    let diff_str = String::from_utf8_lossy(&output.stdout);
    parse_diff(&diff_str)
}

pub fn get_previous_commit_diff(repo_path: &Path) -> Result<Vec<FileDiff>> {
    let output = OsCommand::new("git")
        .arg("show")
        .arg("HEAD")
        .current_dir(repo_path)
        .output()?;

    let diff_str = String::from_utf8_lossy(&output.stdout);
    Ok(parse_diff(&diff_str))
}

pub fn is_git_repository(path: &Path) -> bool {
    path.join(".git").is_dir()
}

pub fn commit(repo_path: &Path, message: &str) -> Result<()> {
    OsCommand::new("git")
        .arg("commit")
        .arg("-m")
        .arg(message)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn amend_commit(repo_path: &Path, message: &str) -> Result<()> {
    OsCommand::new("git")
        .arg("commit")
        .arg("--amend")
        .arg("-m")
        .arg(message)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn add_all(repo_path: &Path) -> Result<()> {
    OsCommand::new("git")
        .arg("add")
        .arg("-A")
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn get_staged_diff_output(repo_path: &Path) -> Result<std::process::Output> {
    let output = OsCommand::new("git")
        .arg("diff")
        .arg("--staged")
        .current_dir(repo_path)
        .output()?;
    Ok(output)
}

pub fn get_file_diff_patch(repo_path: &Path, file_name: &str) -> Result<String> {
    let output = OsCommand::new("git")
        .arg("diff")
        .arg("--staged")
        .arg("--")
        .arg(file_name)
        .current_dir(repo_path)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn unstage_file(repo_path: &Path, file_name: &str) -> Result<()> {
    OsCommand::new("git")
        .arg("reset")
        .arg("HEAD")
        .arg("--")
        .arg(file_name)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn stage_file(repo_path: &Path, file_name: &str) -> Result<()> {
    OsCommand::new("git")
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

    let mut child = OsCommand::new("git")
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
    OsCommand::new("git")
        .arg("checkout")
        .arg("HEAD")
        .arg("--")
        .arg(file_name)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn rm_file(repo_path: &Path, file_name: &str) -> Result<()> {
    OsCommand::new("git")
        .arg("rm")
        .arg("-f")
        .arg(file_name)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn stage_path(repo_path: &Path, path: &str) -> Result<()> {
    OsCommand::new("git")
        .arg("add")
        .arg(path)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn rm_cached(repo_path: &Path, path: &str) -> Result<()> {
    OsCommand::new("git")
        .arg("rm")
        .arg("--cached")
        .arg(path)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn rm_file_from_index(repo_path: &Path, path: &str) -> Result<()> {
    OsCommand::new("git")
        .arg("rm")
        .arg(path)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}
