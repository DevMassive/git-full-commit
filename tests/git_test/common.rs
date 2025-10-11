use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as OsCommand;
use tempfile::TempDir;

pub struct TestRepo {
    pub path: PathBuf,
    _temp_dir: TempDir,
    _remote_temp_dir: TempDir,
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

        let remote_temp_dir = TempDir::new().unwrap();
        let remote_path = remote_temp_dir.path().to_path_buf();
        run_git(&remote_path, &["init", "--bare"]);

        run_git(&path, &["init"]);
        run_git(&path, &["config", "user.name", "Test"]);
        run_git(&path, &["config", "user.email", "test@example.com"]);
        run_git(
            &path,
            &["remote", "add", "origin", remote_path.to_str().unwrap()],
        );

        TestRepo {
            path,
            _temp_dir: temp_dir,
            _remote_temp_dir: remote_temp_dir,
        }
    }

    pub fn create_file(&self, name: &str, content: &str) {
        fs::write(self.path.join(name), content).unwrap();
    }

    pub fn add_all(&self) {
        run_git(&self.path, &["add", "-A"]);
    }

    pub fn commit(&self, msg: &str) {
        run_git(&self.path, &["commit", "--allow-empty", "-m", msg]);
    }

    pub fn push(&self) {
        run_git(&self.path, &["push", "-u", "origin", "HEAD"]);
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

    pub fn get_log(&self, count: usize) -> String {
        let mut arg = String::from("-n");
        arg.push_str(&count.to_string());
        let output = OsCommand::new("git")
            .arg("log")
            .arg(arg)
            .current_dir(&self.path)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    pub fn get_commit_diff(&self, hash: &str) -> String {
        let output = OsCommand::new("git")
            .arg("show")
            .arg(hash)
            .current_dir(&self.path)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).to_string()
    }
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
