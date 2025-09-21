#[cfg(not(test))]
use std::env;
#[cfg(not(test))]
use std::process::Command;

#[cfg(not(test))]
pub fn open_editor(file_path: &str, line_number: Option<usize>) -> std::io::Result<()> {
    let editor_cmd = env::var("EDITOR");

    let mut cmd = match editor_cmd {
        Ok(editor) if !editor.is_empty() => {
            // User has $EDITOR set
            
            if editor.contains("code") {
                // Handle VS Code and its variants
                let mut cmd = Command::new(&editor);
                if let Some(line) = line_number {
                    cmd.arg("-g").arg(format!("{file_path}:{line}"));
                } else {
                    cmd.arg(file_path);
                }
                cmd
            } else {
                // Handle other editors like vi, vim, nvim, nano
                let mut cmd = Command::new(&editor);
                if let Some(line) = line_number {
                    // This syntax works for vim/nvim, but might not for others.
                    // It's a reasonable default.
                    cmd.arg(format!("+{line}"));
                }
                cmd.arg(file_path);
                cmd
            }
        }
        _ => {
            // $EDITOR is not set or is empty, use platform defaults
            if cfg!(target_os = "macos") {
                let mut command = Command::new("open");
                command.arg(file_path);
                command
            } else {
                // Default to `code` on other systems (e.g., Linux)
                let mut command = Command::new("code");
                if let Some(line) = line_number {
                    command.arg("-g").arg(format!("{file_path}:{line}"));
                } else {
                    command.arg(file_path);
                }
                command
            }
        }
    };

    cmd.status().map(|_| ()).map_err(|e| {
        // If the command fails (e.g. `code` not found), we can add a fallback here in the future.
        // For now, just return the error to the caller.
        eprintln!("Failed to open editor: {e}");
        e
    })
}

#[cfg(test)]
pub use mock::open_editor;

#[cfg(test)]
pub mod mock {
    use std::sync::Mutex;

    lazy_static::lazy_static! {
        pub static ref CALLS: Mutex<Vec<(String, Option<usize>)>> = Mutex::new(Vec::new());
    }

    pub fn open_editor(file_path: &str, line_number: Option<usize>) -> std::io::Result<()> {
        CALLS
            .lock()
            .unwrap()
            .push((file_path.to_string(), line_number));
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_calls() -> Vec<(String, Option<usize>)> {
        CALLS.lock().unwrap().clone()
    }

    #[allow(dead_code)]
    pub fn clear_calls() {
        CALLS.lock().unwrap().clear();
    }
}
