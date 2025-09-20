use crate::git::FileDiff;
use pancurses::{A_REVERSE, COLOR_PAIR, Window, chtype};
use similar::TextDiff;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

// Represents a line of text with word-level diff information.
// Each element in the vector is a tuple of (text, is_changed).
#[derive(Debug, Clone, PartialEq)]
pub struct WordDiffLine(pub Vec<(String, bool)>);

pub const LINE_CONTENT_OFFSET: usize = 10;

pub fn render_diff_view(
    window: &Window,
    file: &FileDiff,
    content_height: usize,
    scroll: usize,
    horizontal_scroll: usize,
    header_height: usize,
    cursor_position: usize,
    is_diff_cursor_active: bool,
) {
    let lines = &file.lines;

    let mut line_numbers: Vec<(usize, usize)> = vec![(0, 0); lines.len()];
    for hunk in &file.hunks {
        for (hunk_line_index, (old, new)) in hunk.line_numbers.iter().enumerate() {
            let line_index = hunk.start_line + hunk_line_index;
            if line_index >= lines.len() {
                continue;
            }
            line_numbers[line_index] = (*old, *new);
        }
    }

    let mut i = 0;
    let mut render_index = 0;
    while i < lines.len() {
        if render_index >= content_height {
            break;
        }

        let line = &lines[i];

        if line.starts_with('-') && !line.starts_with("--- ") {
            let mut minus_lines_indices = Vec::new();
            let mut current_pos = i;
            while current_pos < lines.len()
                && lines[current_pos].starts_with('-')
                && !lines[current_pos].starts_with("--- ")
            {
                minus_lines_indices.push(current_pos);
                current_pos += 1;
            }

            let mut plus_lines_indices = Vec::new();
            let mut next_pos = current_pos;
            while next_pos < lines.len()
                && lines[next_pos].starts_with('+')
                && !lines[next_pos].starts_with("+++ ")
            {
                plus_lines_indices.push(next_pos);
                next_pos += 1;
            }

            if !plus_lines_indices.is_empty() {
                let old_text = minus_lines_indices
                    .iter()
                    .map(|&idx| &lines[idx][1..])
                    .collect::<Vec<_>>()
                    .join("\n");
                let new_text = plus_lines_indices
                    .iter()
                    .map(|&idx| &lines[idx][1..])
                    .collect::<Vec<_>>()
                    .join("\n");

                let (old_word_diffs, new_word_diffs) = compute_word_diffs(&old_text, &new_text);

                for (k, &idx) in minus_lines_indices.iter().enumerate() {
                    if idx < scroll {
                        continue;
                    }
                    if render_index >= content_height {
                        break;
                    }
                    let (old_line_num, new_line_num) = line_numbers[idx];
                    render_line(
                        window,
                        &lines[idx],
                        old_word_diffs.get(k),
                        idx,
                        render_index as i32 + header_height as i32,
                        cursor_position,
                        old_line_num,
                        new_line_num,
                        horizontal_scroll,
                        is_diff_cursor_active,
                    );
                    render_index += 1;
                }

                for (k, &idx) in plus_lines_indices.iter().enumerate() {
                    if idx < scroll {
                        continue;
                    }
                    if render_index >= content_height {
                        break;
                    }
                    let (old_line_num, new_line_num) = line_numbers[idx];
                    render_line(
                        window,
                        &lines[idx],
                        new_word_diffs.get(k),
                        idx,
                        render_index as i32 + header_height as i32,
                        cursor_position,
                        old_line_num,
                        new_line_num,
                        horizontal_scroll,
                        is_diff_cursor_active,
                    );
                    render_index += 1;
                }
                i = next_pos;
            } else {
                for &idx in &minus_lines_indices {
                    if idx < scroll {
                        continue;
                    }
                    if render_index >= content_height {
                        break;
                    }
                    let (old_line_num, new_line_num) = line_numbers[idx];
                    render_line(
                        window,
                        &lines[idx],
                        None,
                        idx,
                        render_index as i32 + header_height as i32,
                        cursor_position,
                        old_line_num,
                        new_line_num,
                        horizontal_scroll,
                        is_diff_cursor_active,
                    );
                    render_index += 1;
                }
                i = current_pos;
            }
        } else {
            if i >= scroll {
                let (old_line_num, new_line_num) = line_numbers[i];
                render_line(
                    window,
                    line,
                    None,
                    i,
                    render_index as i32 + header_height as i32,
                    cursor_position,
                    old_line_num,
                    new_line_num,
                    horizontal_scroll,
                    is_diff_cursor_active,
                );
                render_index += 1;
            }
            i += 1;
        }
    }
}

pub fn get_scrolled_line(full_line: &str, scroll_offset: usize) -> &str {
    if scroll_offset == 0 {
        return full_line;
    }

    let mut current_width = 0;

    for (byte_index, ch) in full_line.char_indices() {
        if current_width >= scroll_offset {
            let start_byte_index = byte_index;
            return &full_line[start_byte_index..];
        }
        current_width += UnicodeWidthChar::width(ch).unwrap_or(0);
    }

    ""
}

pub fn compute_word_diffs(old: &str, new: &str) -> (Vec<WordDiffLine>, Vec<WordDiffLine>) {
    if old.trim().is_empty() || new.trim().is_empty() {
        let old_lines = old
            .lines()
            .map(|l| WordDiffLine(vec![(l.to_string(), false)]))
            .collect();
        let new_lines = new
            .lines()
            .map(|l| WordDiffLine(vec![(l.to_string(), false)]))
            .collect();
        return (old_lines, new_lines);
    }

    let diff = TextDiff::from_unicode_words(old, new);

    if diff.ratio() < 0.7 {
        let old_lines = old
            .lines()
            .map(|l| WordDiffLine(vec![(l.to_string(), false)]))
            .collect();
        let new_lines = new
            .lines()
            .map(|l| WordDiffLine(vec![(l.to_string(), false)]))
            .collect();
        return (old_lines, new_lines);
    }

    let mut old_line_parts = Vec::new();
    let mut new_line_parts = Vec::new();

    for change in diff.iter_all_changes() {
        let text = change.value().to_string();
        match change.tag() {
            similar::ChangeTag::Delete => old_line_parts.push((text, true)),
            similar::ChangeTag::Insert => new_line_parts.push((text, true)),
            similar::ChangeTag::Equal => {
                old_line_parts.push((text.clone(), false));
                new_line_parts.push((text, false));
            }
        }
    }

    let mut old_lines = Vec::new();
    let mut current_line = WordDiffLine(Vec::new());
    for (text, changed) in old_line_parts {
        let parts = text.split_inclusive('\n').peekable();
        for part in parts {
            let content = part.strip_suffix('\n').unwrap_or(part);
            if !content.is_empty() {
                current_line.0.push((content.to_string(), changed));
            }
            if part.ends_with('\n') {
                old_lines.push(current_line);
                current_line = WordDiffLine(Vec::new());
            }
        }
    }
    if !current_line.0.is_empty() {
        old_lines.push(current_line);
    }

    let mut new_lines = Vec::new();
    current_line = WordDiffLine(Vec::new());
    for (text, changed) in new_line_parts {
        let parts = text.split_inclusive('\n').peekable();
        for part in parts {
            let content = part.strip_suffix('\n').unwrap_or(part);
            if !content.is_empty() {
                current_line.0.push((content.to_string(), changed));
            }
            if part.ends_with('\n') {
                new_lines.push(current_line);
                current_line = WordDiffLine(Vec::new());
            }
        }
    }
    if !current_line.0.is_empty() {
        new_lines.push(current_line);
    }

    (old_lines, new_lines)
}

pub fn render_line(
    window: &Window,
    line: &str,
    word_diff_line: Option<&WordDiffLine>,
    line_index_in_file: usize,
    line_render_index: i32,
    cursor_position: usize,
    old_line_num: usize,
    new_line_num: usize,
    horizontal_scroll: usize,
    is_diff_cursor_active: bool,
) {
    let is_cursor_line = line_index_in_file == cursor_position;

    let (default_pair, deletion_pair, addition_pair, hunk_header_pair, grey_pair) =
        if is_cursor_line {
            if is_diff_cursor_active {
                (5, 6, 7, 8, 10) // Active cursor pairs
            } else {
                (11, 12, 13, 14, 15) // Inactive cursor pairs
            }
        } else {
            (1, 2, 3, 4, 9) // Non-cursor pairs
        };

    let line_num_str = format!(
        "{:<4} {:<4}",
        if line.starts_with('+') || old_line_num == 0 {
            "".to_string()
        } else {
            old_line_num.to_string()
        },
        if line.starts_with('-') || new_line_num == 0 {
            "".to_string()
        } else {
            new_line_num.to_string()
        }
    );
    let line_content_offset = LINE_CONTENT_OFFSET as i32;

    window.mv(line_render_index, 0);
    window.clrtoeol();

    if is_cursor_line {
        window.attron(COLOR_PAIR(default_pair));
        for i in 0..window.get_max_x() {
            window.mvaddch(line_render_index, i, ' ');
        }
        window.attroff(COLOR_PAIR(default_pair));
    }

    let (base_pair, line_prefix) = if line.starts_with("--- ") || line.starts_with("+++ ") {
        (grey_pair, "")
    } else if line.starts_with('+') {
        (addition_pair, "+")
    } else if line.starts_with('-') {
        (deletion_pair, "-")
    } else if line.starts_with("@@ ") {
        (hunk_header_pair, "")
    } else if line.starts_with("diff --git ") {
        (default_pair, "")
    } else {
        (default_pair, " ")
    };

    let num_pair = if line.starts_with('+') || line.starts_with('-') {
        base_pair
    } else {
        grey_pair
    };

    if (line.starts_with(' ') || line.starts_with('+') || line.starts_with('-'))
        && (!line.starts_with("@@ ") && !line.starts_with("+++") && !line.starts_with("---"))
    {
        window.attron(COLOR_PAIR(num_pair));
        window.mvaddstr(line_render_index, 0, &line_num_str);
        window.attroff(COLOR_PAIR(num_pair));
    }

    window.mv(line_render_index, line_content_offset);

    let mut remaining_scroll = horizontal_scroll;

    let render_part = |win: &Window,
                       text: &str,
                       pair: chtype,
                       attr: pancurses::chtype,
                       remaining_scroll: &mut usize| {
        if *remaining_scroll == 0 {
            win.attron(COLOR_PAIR(pair));
            win.attron(attr);
            win.addstr(text);
            win.attroff(attr);
            win.attroff(COLOR_PAIR(pair));
        } else {
            let width = UnicodeWidthStr::width(text);
            if *remaining_scroll < width {
                let scrolled_text = get_scrolled_line(text, *remaining_scroll);
                win.attron(COLOR_PAIR(pair));
                win.attron(attr);
                win.addstr(scrolled_text);
                win.attroff(attr);
                win.attroff(COLOR_PAIR(pair));
                *remaining_scroll = 0;
            } else {
                *remaining_scroll -= width;
            }
        }
    };

    if line.starts_with("@@ ") {
        window.attroff(COLOR_PAIR(base_pair));
        window.attron(COLOR_PAIR(grey_pair));
        window.mvaddstr(line_render_index, 0, &line_num_str);
        window.attroff(COLOR_PAIR(grey_pair));

        if let Some(hunk_end_pos) = line.rfind("@@") {
            let hunk_header = &line[..hunk_end_pos + 2];
            let function_signature = &line[hunk_end_pos + 2..];

            render_part(
                window,
                hunk_header,
                hunk_header_pair,
                0,
                &mut remaining_scroll,
            );
            render_part(
                window,
                function_signature,
                addition_pair,
                0,
                &mut remaining_scroll,
            );
        } else {
            render_part(window, line, hunk_header_pair, 0, &mut remaining_scroll);
        }
    } else if let Some(word_diff) = word_diff_line {
        render_part(window, line_prefix, base_pair, 0, &mut remaining_scroll);
        for (text, is_changed) in &word_diff.0 {
            let attr = if *is_changed { A_REVERSE } else { 0 };
            render_part(window, text, base_pair, attr, &mut remaining_scroll);
        }
    } else {
        render_part(window, line, base_pair, 0, &mut remaining_scroll);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_word_diffs() {
        let old = "The quick brown fox\njumps over the lazy dog";
        let new = "The slow brown cat\njumps over the lazy dog";

        let (old_diff, new_diff) = compute_word_diffs(old, new);

        assert_eq!(old_diff.len(), 2);
        assert_eq!(new_diff.len(), 2);

        let expected_old_line1 = WordDiffLine(vec![
            ("The".to_string(), false),
            (" ".to_string(), false),
            ("quick".to_string(), true),
            (" ".to_string(), false),
            ("brown".to_string(), false),
            (" ".to_string(), false),
            ("fox".to_string(), true),
        ]);
        assert_eq!(old_diff[0], expected_old_line1);

        let expected_new_line1 = WordDiffLine(vec![
            ("The".to_string(), false),
            (" ".to_string(), false),
            ("slow".to_string(), true),
            (" ".to_string(), false),
            ("brown".to_string(), false),
            (" ".to_string(), false),
            ("cat".to_string(), true),
        ]);
        assert_eq!(new_diff[0], expected_new_line1);

        let expected_line2 = WordDiffLine(vec![
            ("jumps".to_string(), false),
            (" ".to_string(), false),
            ("over".to_string(), false),
            (" ".to_string(), false),
            ("the".to_string(), false),
            (" ".to_string(), false),
            ("lazy".to_string(), false),
            (" ".to_string(), false),
            ("dog".to_string(), false),
        ]);
        assert_eq!(old_diff[1], expected_line2);
        assert_eq!(new_diff[1], expected_line2);
    }

    #[test]
    fn test_compute_word_diffs_empty() {
        let old = "";
        let new = "a";
        let (old_diff, new_diff) = compute_word_diffs(old, new);
        assert_eq!(old_diff.len(), 0);
        assert_eq!(new_diff.len(), 1);
        assert_eq!(new_diff[0], WordDiffLine(vec![("a".to_string(), false)]));

        let old = "a";
        let new = "";
        let (old_diff, new_diff) = compute_word_diffs(old, new);
        assert_eq!(old_diff.len(), 1);
        assert_eq!(new_diff.len(), 0);
        assert_eq!(old_diff[0], WordDiffLine(vec![("a".to_string(), false)]));
    }

    #[test]
    fn test_compute_word_diffs_low_similarity() {
        let old = "completely different";
        let new = "something else entirely";

        let (old_diff, new_diff) = compute_word_diffs(old, new);

        // Expect no word-level highlighting due to low similarity
        let expected_old = WordDiffLine(vec![("completely different".to_string(), false)]);
        let expected_new = WordDiffLine(vec![("something else entirely".to_string(), false)]);

        assert_eq!(old_diff.len(), 1);
        assert_eq!(old_diff[0], expected_old);
        assert_eq!(new_diff.len(), 1);
        assert_eq!(new_diff[0], expected_new);
    }
}
