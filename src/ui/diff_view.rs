use similar::TextDiff;
use unicode_width::UnicodeWidthChar;

// Represents a line of text with word-level diff information.
// Each element in the vector is a tuple of (text, is_changed).
#[derive(Debug, Clone, PartialEq)]
pub struct WordDiffLine(pub Vec<(String, bool)>);

pub const LINE_CONTENT_OFFSET: usize = 10;

pub fn get_scrolled_line(full_line: &str, scroll_offset: usize) -> &str {
    if scroll_offset == 0 {
        return full_line;
    }

    let mut current_width = 0;
    let mut start_byte_index = 0;

    for (byte_index, ch) in full_line.char_indices() {
        if current_width >= scroll_offset {
            start_byte_index = byte_index;
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
        let mut parts = text.split_inclusive('\n').peekable();
        while let Some(part) = parts.next() {
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
        let mut parts = text.split_inclusive('\n').peekable();
        while let Some(part) = parts.next() {
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
