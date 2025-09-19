use crate::git::{FileDiff, Hunk};

pub fn create_unstage_line_patch(file: &FileDiff, line_index: usize) -> Option<String> {
    let line_to_unstage = file.lines.get(line_index)?;

    if !line_to_unstage.starts_with('+') && !line_to_unstage.starts_with('-') {
        return None;
    }

    let hunk = file.hunks.iter().find(|hunk| {
        let hunk_start = hunk.start_line;
        let hunk_end = hunk_start + hunk.lines.len();
        line_index >= hunk_start && line_index < hunk_end
    })?;

    let hunk_header = &hunk.lines[0];
    let mut parts = hunk_header.split(' ');
    let old_range = parts.nth(1).unwrap();
    let new_range = parts.next().unwrap();

    let mut old_range_parts = old_range.split(',');
    let old_start: u32 = old_range_parts
        .next()
        .unwrap()
        .trim_start_matches('-')
        .parse()
        .unwrap();

    let mut new_range_parts = new_range.split(',');
    let new_start: u32 = new_range_parts
        .next()
        .unwrap()
        .trim_start_matches('+')
        .parse()
        .unwrap();

    let mut current_old_line = old_start;
    let mut current_new_line = new_start;
    let mut patch_old_line = 0;
    let mut patch_new_line = 0;

    for (i, line) in hunk.lines.iter().skip(1).enumerate() {
        let current_line_index_in_file = hunk.start_line + 1 + i;

        if current_line_index_in_file == line_index {
            patch_old_line = current_old_line;
            patch_new_line = current_new_line;
            break;
        }

        if line.starts_with('-') {
            current_old_line += 1;
        } else if line.starts_with('+') {
            current_new_line += 1;
        } else {
            current_old_line += 1;
            current_new_line += 1;
        }
    }

    let new_hunk_header = if line_to_unstage.starts_with('-') {
        format!("@@ -{},1 +{},0 @@", patch_old_line, patch_new_line)
    } else {
        format!("@@ -{},0 +{},1 @@", patch_old_line, patch_new_line)
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
