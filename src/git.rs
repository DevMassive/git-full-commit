use std::path::{Path, PathBuf};
use std::process::Command as OsCommand;
use anyhow::Result;

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
                if let Some(hunk) = current_hunk.take() {
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
        } else if line.starts_with("@@ ") {
            if let Some(hunk) = current_hunk.take() {
                if let Some(file) = current_file.as_mut() {
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
        if let Some(hunk) = current_hunk.take() {
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

pub fn get_status(repo_path: PathBuf) -> Vec<FileDiff> {
    let output = OsCommand::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(&repo_path)
        .output()
        .expect("Failed to execute git status");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();

    for line in stdout.lines() {
        let status_str = &line[..2];
        let file_name = &line[3..];

        let _file_status = match status_str.chars().next().unwrap() {
            'A' => FileStatus::Added,
            'M' => FileStatus::Modified,
            'D' => FileStatus::Deleted,
            'R' => FileStatus::Renamed,
            '?' => FileStatus::Added, // Untracked
            _ => continue,
        };

        let output = OsCommand::new("git")
            .arg("diff")
            .arg("--staged")
            .arg(file_name)
            .current_dir(&repo_path)
            .output()
            .expect("Failed to execute git diff");
        let diff_str = String::from_utf8_lossy(&output.stdout);
        if let Some(file_diff) = parse_diff(&diff_str).pop() {
            files.push(file_diff);
        }
    }

    files
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
