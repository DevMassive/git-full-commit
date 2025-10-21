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
    pub old_file_name: String,
    pub hunks: Vec<Hunk>,
    pub lines: Vec<String>,
    pub status: FileStatus,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub message: String,
    pub is_on_remote: bool,
    pub is_fixup: bool,
}

impl PartialEq for CommitInfo {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
            && self.message == other.message
            && self.is_on_remote == other.is_on_remote
            && self.is_fixup == other.is_fixup
    }
}

pub fn get_local_commits(repo_path: &Path) -> Result<Vec<CommitInfo>> {
    let output = git_command()
        .arg("log")
        .arg("--pretty=%h %s")
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in stdout.lines() {
        let mut parts = line.splitn(2, ' ');
        let hash = parts.next().unwrap_or("").to_string();
        let message = parts.next().unwrap_or("").to_string();

        let is_on_remote = is_commit_on_remote(repo_path, &hash)?;
        commits.push(CommitInfo {
            hash,
            message,
            is_on_remote,
            is_fixup: false,
        });

        if is_on_remote {
            break;
        }
    }

    Ok(commits)
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
    let mut header_lines: Vec<String> = Vec::new();

    let diff_line_re = Regex::new(r#"^diff --git a/(.+) b/(.+)"#).unwrap();

    for line in diff_str.lines() {
        if let Some(caps) = diff_line_re.captures(line) {
            if let Some(mut file) = current_file.take() {
                if let Some(mut hunk) = current_hunk.take() {
                    hunk.line_numbers = calc_line_numbers(&hunk);
                    file.hunks.push(hunk);
                }
                file.lines = current_file_lines;
                files.push(file);
            }

            current_file_lines = Vec::new();

            let old_file_name = caps
                .get(1)
                .map(|m| m.as_str().trim_matches('"'))
                .unwrap_or("")
                .to_string();
            let file_name = caps
                .get(2)
                .map(|m| m.as_str().trim_matches('"'))
                .unwrap_or("")
                .to_string();

            current_file = Some(FileDiff {
                file_name,
                old_file_name,
                hunks: Vec::new(),
                lines: Vec::new(), // Will be filled in later
                status: FileStatus::Modified,
            });

            if files.is_empty() {
                current_file_lines.append(&mut header_lines);
            }
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
                start_line: current_file_lines.len(),
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
        } else {
            header_lines.push(line.to_string());
        }
    }

    if let Some(mut file) = current_file.take() {
        if let Some(mut hunk) = current_hunk.take() {
            hunk.line_numbers = calc_line_numbers(&hunk);
            file.hunks.push(hunk);
        }
        file.lines = current_file_lines;
        files.push(file);
    } else if !header_lines.is_empty() {
        files.push(FileDiff {
            file_name: "".to_string(),
            old_file_name: "".to_string(),
            hunks: Vec::new(),
            lines: header_lines,
            status: FileStatus::Modified,
        });
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

pub fn get_commit_diff(repo_path: &Path, hash: &str) -> Result<Vec<FileDiff>> {
    let output = git_command()
        .arg("show")
        .arg("--stat")
        .arg("--patch")
        .arg(hash)
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

pub fn commit(repo_path: &Path, message: &str) -> Result<()> {
    git_command()
        .arg("commit")
        .arg("-m")
        .arg(message)
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn commit_amend_with_message(repo_path: &Path, message: &str) -> Result<()> {
    let output = git_command()
        .arg("commit")
        .arg("--amend")
        .arg("--allow-empty")
        .arg("-m")
        .arg(message)
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to amend commit with message. Stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

pub fn commit_amend_no_edit(repo_path: &Path) -> Result<()> {
    let output = git_command()
        .arg("commit")
        .arg("--amend")
        .arg("--no-edit")
        .arg("--allow-empty")
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to commit --amend --no-edit. Stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

pub fn amend_commit_with_staged_changes(
    repo_path: &Path,
    target_hash: &str,
    message: &str,
) -> Result<()> {
    let commits_before = get_local_commits(repo_path)?;
    let short_hash_len = commits_before.first().map(|c| c.hash.len()).unwrap_or(7);
    let target_hash_short: String = target_hash.chars().take(short_hash_len).collect();
    let target_index = commits_before
        .iter()
        .position(|c| c.hash == target_hash_short)
        .ok_or_else(|| anyhow::anyhow!("Target commit not found in local history"))?;

    // 1. Get the original message to check if it needs to be changed later
    let original_message_output = git_command()
        .arg("log")
        .arg("-1")
        .arg("--pretty=%s")
        .arg(target_hash)
        .current_dir(repo_path)
        .output()?;
    let original_message = String::from_utf8_lossy(&original_message_output.stdout);

    // 2. Create a fixup! commit for the staged changes
    let commit_output = git_command()
        .arg("commit")
        .arg("--fixup")
        .arg(target_hash)
        .current_dir(repo_path)
        .output()?;

    if !commit_output.status.success() {
        anyhow::bail!(
            "git commit --fixup failed. This usually means there are no staged changes. Stderr: {}",
            String::from_utf8_lossy(&commit_output.stderr)
        );
    }

    // 3. Rebase with autosquash
    let parent_hash_output = git_command()
        .arg("rev-parse")
        .arg(format!("{target_hash}^"))
        .current_dir(repo_path)
        .output()?;
    let is_root_commit = !parent_hash_output.status.success();

    let mut rebase_cmd = git_command();
    rebase_cmd.env("GIT_SEQUENCE_EDITOR", "true");
    rebase_cmd
        .arg("rebase")
        .arg("-i")
        .arg("--autosquash")
        .arg("--autostash");

    if is_root_commit {
        rebase_cmd.arg("--root");
    } else {
        let parent_hash = String::from_utf8_lossy(&parent_hash_output.stdout)
            .trim()
            .to_string();
        rebase_cmd.arg(&parent_hash);
    }

    let rebase_output = rebase_cmd.current_dir(repo_path).output()?;

    if !rebase_output.status.success() {
        git_command()
            .arg("rebase")
            .arg("--abort")
            .current_dir(repo_path)
            .output()?;
        anyhow::bail!("git rebase for fixup failed. Aborting.");
    }

    // 4. If the message has changed, amend the now-squashed commit
    if message.trim() != original_message.trim() {
        let commits_after = get_local_commits(repo_path)?;
        let rewritten_commit = commits_after
            .get(target_index)
            .ok_or_else(|| anyhow::anyhow!("Rewritten commit not found after amend rebase"))?;

        reword_commit(repo_path, &rewritten_commit.hash, message)?;
    }

    Ok(())
}

pub fn reword_commit(repo_path: &Path, commit_hash: &str, message: &str) -> Result<()> {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    // Write the new message to a temporary file
    let temp_message_path = repo_path.join(".git/COMMIT_EDITMSG_TEMP");
    std::fs::write(&temp_message_path, message)?;

    // Create an editor script that will overwrite the real commit message file
    let editor_script_path = repo_path.join(".git/reword_editor.sh");
    let script_content = format!(
        "#!/bin/sh\ncp '{}' \"$1\"",
        temp_message_path.to_str().unwrap()
    );
    std::fs::write(&editor_script_path, &script_content)?;
    #[cfg(unix)]
    std::fs::set_permissions(&editor_script_path, std::fs::Permissions::from_mode(0o755))?;

    let parent_hash_output = git_command()
        .arg("rev-parse")
        .arg(format!("{commit_hash}^"))
        .current_dir(repo_path)
        .output()?;
    let is_root_commit = !parent_hash_output.status.success();
    let parent_hash = String::from_utf8_lossy(&parent_hash_output.stdout)
        .trim()
        .to_string();

    let mut rebase_cmd = git_command();
    let short_hash = &commit_hash[0..7.min(commit_hash.len())];
    rebase_cmd.env(
        "GIT_SEQUENCE_EDITOR",
        format!("sed -i -e 's/^pick {short_hash}/reword {short_hash}/'"),
    );
    rebase_cmd.env("GIT_EDITOR", editor_script_path.to_str().unwrap());
    rebase_cmd.arg("rebase").arg("-i").arg("--autostash");

    if is_root_commit {
        rebase_cmd.arg("--root");
    } else {
        rebase_cmd.arg(&parent_hash);
    }

    let rebase_output = rebase_cmd.current_dir(repo_path).output()?;

    // Clean up temporary files
    let _ = std::fs::remove_file(&temp_message_path);
    let _ = std::fs::remove_file(&editor_script_path);

    if !rebase_output.status.success() {
        git_command()
            .arg("rebase")
            .arg("--abort")
            .current_dir(repo_path)
            .output()?;
        anyhow::bail!(
            "git rebase for reword failed. Stderr: {}. Aborting.",
            String::from_utf8_lossy(&rebase_output.stderr)
        );
    }

    Ok(())
}

pub fn fixup_and_rebase_autosquash(repo_path: &Path, fixup_commit_hash: &str) -> Result<()> {
    // 1. Create a fixup! commit
    let commit_output = git_command()
        .arg("commit")
        .arg("--no-edit")
        .arg("--fixup")
        .arg(fixup_commit_hash)
        .current_dir(repo_path)
        .output()?;

    if !commit_output.status.success() {
        // This can happen if there's nothing to commit. For now, we'll treat this as an error.
        anyhow::bail!(
            "git commit --fixup failed. This usually means there are no staged changes. Stderr: {}",
            String::from_utf8_lossy(&commit_output.stderr)
        );
    }

    // 2. Execute git rebase -i --autosquash
    let parent_hash_output = git_command()
        .arg("rev-parse")
        .arg(format!("{fixup_commit_hash}^"))
        .current_dir(repo_path)
        .output()?;
    let is_root_commit = !parent_hash_output.status.success();

    let mut rebase_cmd = git_command();
    rebase_cmd.env("GIT_SEQUENCE_EDITOR", "true");
    rebase_cmd
        .arg("rebase")
        .arg("-i")
        .arg("--autosquash")
        .arg("--autostash");

    if is_root_commit {
        rebase_cmd.arg("--root");
    } else {
        rebase_cmd.arg(fixup_commit_hash);
    }

    let rebase_output = rebase_cmd.current_dir(repo_path).output()?;

    if !rebase_output.status.success() {
        git_command()
            .arg("rebase")
            .arg("--abort")
            .current_dir(repo_path)
            .output()?;
        anyhow::bail!("git rebase for fixup failed. Aborting.");
    }

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

pub fn cherry_pick_no_commit(repo_path: &Path, commit_hash: &str) -> Result<()> {
    let output = git_command()
        .arg("cherry-pick")
        .arg("--no-commit")
        .arg(commit_hash)
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to cherry-pick -n {}. Stderr: {}",
            commit_hash,
            String::from_utf8_lossy(&output.stderr)
        );
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

pub fn has_unstaged_changes_in_file(repo_path: &Path, file_path: &str) -> Result<bool> {
    let output = git_command()
        .arg("diff")
        .arg("--quiet")
        .arg("--")
        .arg(file_path)
        .current_dir(repo_path)
        .status()?;

    // status() returns the exit status. For `git diff --quiet`,
    // it's 0 if no changes, 1 if there are changes.
    Ok(output.code() == Some(1))
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
    let output = git_command().arg("diff").current_dir(repo_path).output()?;
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

pub fn stash_unstaged_changes(repo_path: &Path) -> Result<bool> {
    let output = git_command()
        .arg("stash")
        .arg("push")
        .arg("-u")
        .arg("-m")
        .arg("git-branchless-reorder-stash")
        .current_dir(repo_path)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(!stdout.contains("No local changes to save"))
}

pub fn pop_stash(repo_path: &Path) -> Result<()> {
    let stash_list_output = git_command()
        .arg("stash")
        .arg("list")
        .current_dir(repo_path)
        .output()?;
    let stash_list = String::from_utf8_lossy(&stash_list_output.stdout);
    let stash_ref = stash_list
        .lines()
        .find(|line| line.contains("git-branchless-reorder-stash"))
        .and_then(|line| line.split(':').next())
        .map(|s| s.to_string());

    if let Some(stash_ref) = stash_ref {
        git_command()
            .arg("stash")
            .arg("pop")
            .arg(&stash_ref)
            .current_dir(repo_path)
            .output()?;
    }
    Ok(())
}

pub fn get_current_branch_name(repo_path: &Path) -> Result<String> {
    let output = git_command()
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("Failed to get current branch name");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn get_commit_parent(repo_path: &Path, commit_hash: &str) -> Result<String> {
    let output = git_command()
        .arg("rev-parse")
        .arg(format!("{commit_hash}^"))
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("Failed to get parent for commit {}", commit_hash);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn create_branch_at(repo_path: &Path, branch_name: &str, start_point: &str) -> Result<()> {
    let output = git_command()
        .arg("branch")
        .arg(branch_name)
        .arg(start_point)
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to create branch {} at {}. Stderr: {}",
            branch_name,
            start_point,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

pub fn checkout_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
    let output = git_command()
        .arg("checkout")
        .arg(branch_name)
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to checkout branch {}. Stderr: {}",
            branch_name,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

pub fn checkout_orphan_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
    let output = git_command()
        .arg("checkout")
        .arg("--orphan")
        .arg(branch_name)
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to checkout orphan branch {}. Stderr: {}",
            branch_name,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    // After checkout --orphan, the index is full and working dir is dirty.
    // We need to clear them to start fresh. `git rm` can fail on an empty index.
    // So we ignore the error.
    let _ = git_command()
        .arg("rm")
        .arg("-rf")
        .arg(".")
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn cherry_pick(repo_path: &Path, commit_hash: &str) -> Result<()> {
    let output = git_command()
        .arg("cherry-pick")
        .arg("--allow-empty")
        .arg(commit_hash)
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to cherry-pick {}. Stderr: {}",
            commit_hash,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

pub fn cherry_pick_abort(repo_path: &Path) -> Result<()> {
    git_command()
        .arg("cherry-pick")
        .arg("--abort")
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

pub fn reset_hard(repo_path: &Path, target: &str) -> Result<()> {
    let output = git_command()
        .arg("reset")
        .arg("--hard")
        .arg(target)
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to reset --hard to {}. Stderr: {}",
            target,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

pub fn run_git_command(repo_path: &Path, args: &[&str]) -> Result<String> {
    let output = git_command().args(args).current_dir(repo_path).output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Git command failed: {:?}\n---\nstdout:\n{}\n---\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn delete_branch(repo_path: &Path, branch_name: &str, force: bool) -> Result<()> {
    let mut cmd = git_command();
    cmd.arg("branch");
    if force {
        cmd.arg("-D");
    } else {
        cmd.arg("-d");
    }
    cmd.arg(branch_name);
    let output = cmd.current_dir(repo_path).output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to delete branch {}. Stderr: {}",
            branch_name,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
