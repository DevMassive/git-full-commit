use crate::git::{FileDiff, Hunk};

pub fn find_hunk(file: &FileDiff, line_index: usize) -> Option<&Hunk> {
    file.hunks.iter().find(|hunk| {
        let hunk_start = hunk.start_line;
        let hunk_end = hunk_start + hunk.lines.len();
        line_index >= hunk_start && line_index < hunk_end
    })
}

pub fn create_unstage_line_patch(
    file: &FileDiff,
    line_index: usize,
    is_unstaging: bool,
) -> Option<String> {
    let line_to_unstage = file.lines.get(line_index)?;

    if !line_to_unstage.starts_with('+') && !line_to_unstage.starts_with('-') {
        return None;
    }

    let hunk = find_hunk(file, line_index)?;

    let relative_line_index = line_index - hunk.start_line;

    let (old_line_num, new_line_num) = hunk.line_numbers[relative_line_index];
    let line_num = if is_unstaging || line_to_unstage.starts_with('-') {
        new_line_num
    } else {
        old_line_num + 1
    };

    let new_hunk_header = if line_to_unstage.starts_with('+') {
        format!("@@ -{},0 +{},1 @@", line_num - 1, line_num)
    } else {
        format!("@@ -{},1 +{},0 @@", line_num + 1, line_num + 1)
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

// For staging, the logic is identical to unstaging
pub fn create_stage_line_patch(file: &FileDiff, line_index: usize) -> Option<String> {
    create_unstage_line_patch(file, line_index, false)
}

pub fn create_stage_hunk_patch(file: &FileDiff, hunk: &Hunk) -> String {
    create_unstage_hunk_patch(file, hunk)
}

pub fn create_patch_for_new_file(file_name: &str, content: &str) -> String {
    let mut patch = String::new();
    patch.push_str(&format!("diff --git a/{file_name} b/{file_name}\n"));
    patch.push_str("new file mode 100644\n");
    // The index hash is not critical for applying, a dummy one is fine.
    patch.push_str("index 0000000..e69de29\n");
    patch.push_str("--- /dev/null\n");
    patch.push_str(&format!("+++ b/{file_name}\n"));

    let lines: Vec<&str> = content.lines().collect();
    patch.push_str(&format!("@@ -0,0 +1,{} @@\n", lines.len()));

    for (i, line) in lines.iter().enumerate() {
        patch.push_str(&format!("+{line}"));
        if i < lines.len() - 1 || content.ends_with('\n') {
            patch.push('\n');
        }
    }
    patch
}

pub fn get_line_number(file: &FileDiff, line_index: usize) -> Option<usize> {
    let line_content = file.lines.get(line_index)?;
    if line_content.starts_with("@@") {
        return None;
    }

    let hunk = find_hunk(file, line_index)?;
    let relative_line_index = line_index - hunk.start_line;

    let (_, new_line_num) = hunk.line_numbers.get(relative_line_index)?;

    if line_content.starts_with('-') {
        // It's a deleted line. The stored `new_line_num` is the line number
        // of the line *before* the deletion. We want to point to the line
        // that now occupies that space.
        Some(new_line_num + 1)
    } else {
        Some(*new_line_num)
    }
}
