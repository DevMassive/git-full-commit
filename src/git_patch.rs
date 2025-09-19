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

    let relative_line_index = line_index - (hunk.start_line + 1);

    let old_line_offset = hunk.lines[1..=relative_line_index]
        .iter()
        .filter(|l| !l.starts_with('+'))
        .count();

    let new_line_offset = hunk.lines[1..=relative_line_index]
        .iter()
        .filter(|l| !l.starts_with('-'))
        .count();

    let patch_old_line = old_start + old_line_offset as u32;
    let patch_new_line = new_start + new_line_offset as u32;

    let new_hunk_header = if line_to_unstage.starts_with('-') {
        format!("@@ -{patch_old_line},1 +{patch_new_line},0 @@")
    } else {
        format!("@@ -{patch_old_line},0 +{patch_new_line},1 @@")
    };

    let mut patch = String::new();
    patch.push_str(&format!("diff --git a/{0} b/{0}\n", file.file_name));
    patch.push_str(&format!("--- a/{0}\n", file.file_name));
    patch.push_str(&format!("+++ b/{0}\n", file.file_name));
    patch.push_str(&new_hunk_header);
    patch.push('\n');
    patch.push_str(line_to_unstage);
    patch.push('\n');

    eprintln!("Generated patch:\n{patch}");

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
