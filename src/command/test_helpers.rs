use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command as OsCommand;
use tempfile::TempDir;

pub struct TestRepo {
    pub path: PathBuf,
    _temp_dir: TempDir,
}

impl Default for TestRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl TestRepo {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();
        run_git(&path, &["init"]);
        run_git(&path, &["config", "user.name", "Test"]);
        run_git(&path, &["config", "user.email", "test@example.com"]);
        TestRepo {
            path,
            _temp_dir: temp_dir,
        }
    }

    pub fn create_file(&self, name: &str, content: &str) {
        fs::write(self.path.join(name), content).unwrap();
    }

    pub fn append_file(&self, name: &str, content: &str) {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(self.path.join(name))
            .unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    pub fn add_file(&self, name: &str) {
        run_git(&self.path, &["add", name]);
    }

    pub fn add_all(&self) {
        run_git(&self.path, &["add", "-A"]);
    }

    pub fn commit(&self, msg: &str) {
        run_git(&self.path, &["commit", "--allow-empty", "-m", msg]);
    }

    pub fn get_status(&self) -> String {
        let output = OsCommand::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(&self.path)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    pub fn get_file_content_at_commit(&self, file_name: &str, hash: &str) -> String {
        let output = OsCommand::new("git")
            .arg("show")
            .arg(format!("{hash}:{file_name}"))
            .current_dir(&self.path)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).to_string()
    }
}

pub fn create_file(repo_path: &Path, name: &str, content: &str) {
    fs::write(repo_path.join(name), content).unwrap();
}

pub fn commit(repo_path: &Path, msg: &str) {
    run_git(repo_path, &["add", "-A"]);
    run_git(repo_path, &["commit", "--allow-empty", "-m", msg]);
}

pub fn get_log(repo_path: &Path) -> Vec<crate::git::CommitInfo> {
    crate::git::get_local_commits(repo_path).unwrap()
}

pub fn run_git(dir: &Path, args: &[&str]) {
    let output = OsCommand::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run git command");

    if !output.status.success() {
        panic!(
            "git command failed: {:?}\nstdout: {}\nstderr: {}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
