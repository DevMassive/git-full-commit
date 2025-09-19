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

    let mut old_line_counter = hunk.old_start;
    let mut new_line_counter = hunk.new_start;

    for i in 0..relative_line_index {
        let l = &hunk.lines[i + 1];
        if l.starts_with('+') {
            new_line_counter += 1;
        } else if l.starts_with('-') {
            old_line_counter += 1;
        } else {
            old_line_counter += 1;
            new_line_counter += 1;
        }
    }

    let (patch_old_line, patch_new_line) = if line_to_unstage.starts_with('+') {
        (old_line_counter - 1, new_line_counter)
    } else {
        // '-'
        (old_line_counter, new_line_counter - 1)
    };

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

    eprintln!("line_index: {line_index}");
    eprintln!("line_to_unstage: {line_to_unstage}");
    eprintln!("hunk_header: {hunk_header}");
    eprintln!("old_start: {old_start}, new_start: {new_start}");
    eprintln!("relative_line_index: {relative_line_index}");
    // eprintln!("old_line_offset: {old_line_offset}, new_line_offset: {new_line_offset}");
    eprintln!("patch_old_line: {patch_old_line}, patch_new_line: {patch_new_line}");
    eprintln!("new_hunk_header: {new_hunk_header}");
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
