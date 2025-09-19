use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::{fs, io};

fn get_storage_dir() -> Result<PathBuf, io::Error> {
    dirs::home_dir()
        .map(|home| home.join(".git-reset-pp"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found"))
}

fn get_commit_message_file_path(repo_path: &Path) -> Result<PathBuf, io::Error> {
    let storage_dir = get_storage_dir()?;
    let mut hasher = DefaultHasher::new();
    repo_path.hash(&mut hasher);
    let repo_hash = hasher.finish();
    Ok(storage_dir.join(format!("{repo_hash}")))
}

pub fn save_commit_message(repo_path: &Path, message: &str) -> Result<(), io::Error> {
    let storage_dir = get_storage_dir()?;
    fs::create_dir_all(&storage_dir)?;
    let file_path = get_commit_message_file_path(repo_path)?;
    fs::write(file_path, message)
}

pub fn load_commit_message(repo_path: &Path) -> Result<String, io::Error> {
    let file_path = get_commit_message_file_path(repo_path)?;
    fs::read_to_string(file_path)
}

pub fn delete_commit_message(repo_path: &Path) -> Result<(), io::Error> {
    let file_path = get_commit_message_file_path(repo_path)?;
    if file_path.exists() {
        fs::remove_file(file_path)
    } else {
        Ok(())
    }
}
