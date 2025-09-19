use crate::git::{FileDiff, Hunk};

pub fn find_hunk(file: &FileDiff, line_index: usize) -> Option<&Hunk> {
    file.hunks.iter().find(|hunk| {
        let hunk_start = hunk.start_line;
        let hunk_end = hunk_start + hunk.lines.len();
        line_index >= hunk_start && line_index < hunk_end
    })
}

pub fn create_unstage_line_patch(file: &FileDiff, line_index: usize) -> Option<String> {
    let line_to_unstage = file.lines.get(line_index)?;

    if !line_to_unstage.starts_with('+') && !line_to_unstage.starts_with('-') {
        return None;
    }

    let hunk = find_hunk(file, line_index)?;

    let relative_line_index = line_index - hunk.start_line;

    let (old_line, new_line) = hunk.line_numbers[relative_line_index];

    let new_hunk_header = if line_to_unstage.starts_with('+') {
        let new_line_num = new_line.unwrap();
        let old_line_num = hunk
            .line_numbers
            .iter()
            .find_map(|(ol, nl)| {
                if nl.is_some() && nl.unwrap() == new_line_num - 1 {
                    *ol
                } else {
                    None
                }
            })
            .unwrap_or(0);
        format!("@@ -{old_line_num},0 +{new_line_num},1 @@")
    } else {
        let old_line_num = old_line.unwrap();
        let new_line_num = hunk
            .line_numbers
            .iter()
            .find_map(|(ol, nl)| {
                if ol.is_some() && ol.unwrap() == old_line_num - 1 {
                    *nl
                } else {
                    None
                }
            })
            .unwrap_or(0);
        format!("@@ -{old_line_num},1 +{new_line_num},0 @@")
    };

    let mut patch = String::new();
    patch.push_str(&format!("diff --git a/{0} b/{0}\n", file.file_name));
    patch.push_str(&format!("--- a/{0}\n", file.file_name));
    patch.push_str(&format!("+++ b/{0}\n", file.file_name));
    patch.push_str(&new_hunk_header);
    patch.push('\n');
    patch.push_str(line_to_unstage);
    patch.push('\n');

    Some(patch)
}
pub fn create_unstage_hunk_patch(file: &FileDiff, hunk: &Hunk) -> String {
    let mut patch = String::new();
    patch.push_str(&format!("diff --git a/{0} b/{0}\n", file.file_name));
    patch.push_str(&format!("--- a/{0}\n", file.file_name));
    patch.push_str(&format!("+++ b/{0}\n", file.file_name));
    patch.push_str(&hunk.lines.join("\n"));
    patch.push('\n');
    patch
}
