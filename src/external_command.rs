#[cfg(not(test))]
use std::env;
#[cfg(not(test))]
use std::process::Command;

#[cfg(not(test))]
fn is_command_available(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(not(test))]
pub fn open_editor(file_path: &str, line_number: Option<usize>) -> std::io::Result<()> {
    let mut cmd;

    if is_command_available("code") {
        cmd = Command::new("code");
        if let Some(line) = line_number {
            cmd.arg("-g").arg(format!("{file_path}:{line}"));
        } else {
            cmd.arg(file_path);
        }
    } else {
        let editor_cmd = env::var("EDITOR");
        cmd = match editor_cmd {
            Ok(editor) if !editor.is_empty() => {
                // User has $EDITOR set. We already know `code` is not available as a command.
                // But user might have `EDITOR=/path/to/code`
                let mut inner_cmd = Command::new(&editor);
                if editor.contains("code") {
                    if let Some(line) = line_number {
                        inner_cmd.arg("-g").arg(format!("{file_path}:{line}"));
                    } else {
                        inner_cmd.arg(file_path);
                    }
                } else {
                    // Handle other editors like vi, vim, nvim, nano
                    if let Some(line) = line_number {
                        inner_cmd.arg(format!("+{line}"));
                    }
                    inner_cmd.arg(file_path);
                }
                inner_cmd
            }
            _ => {
                // $EDITOR is not set or is empty, use platform defaults
                if cfg!(target_os = "macos") {
                    let mut command = Command::new("open");
                    command.arg(file_path);
                    command
                } else {
                    // Default to `vi` on other systems if `code` is not found
                    let mut command = Command::new("vi");
                    if let Some(line) = line_number {
                        command.arg(format!("+{line}"));
                    }
                    command.arg(file_path);
                    command
                }
            }
        };
    }

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
