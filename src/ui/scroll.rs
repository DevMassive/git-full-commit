use crate::app_state::{AppState, FocusedPane};
use crate::git;
use pancurses::Input;

enum ScrollDirection {
    Up,
    Down,
}

enum ScrollAmount {
    Full,
    Half,
}

fn scroll_content(
    line_cursor: usize,
    scroll: usize,
    content_height: usize,
    lines_count: usize,
    direction: ScrollDirection,
    amount: ScrollAmount,
) -> (usize, usize) {
    if lines_count == 0 {
        return (line_cursor, scroll);
    }

    let scroll_amount = match amount {
        ScrollAmount::Full => content_height,
        ScrollAmount::Half => (content_height / 2).max(1),
    };

    let max_line = lines_count.saturating_sub(1);

    let mut new_line_cursor = line_cursor;
    let mut new_scroll = scroll;

    match direction {
        ScrollDirection::Down => {
            new_line_cursor = line_cursor.saturating_add(scroll_amount).min(max_line);

            if new_line_cursor >= scroll + content_height {
                new_scroll = scroll.saturating_add(scroll_amount);
            }
        }
        ScrollDirection::Up => {
            new_line_cursor = line_cursor.saturating_sub(scroll_amount);

            if new_line_cursor < scroll {
                new_scroll = scroll.saturating_sub(scroll_amount);
            }
        }
    }

    (new_line_cursor, new_scroll)
}

fn scroll_view(state: &mut AppState, direction: ScrollDirection, amount: ScrollAmount, max_y: i32) {
    let mut main_pane_offset = 0;
    if state.main_screen.has_unstaged_changes {
        main_pane_offset = state.unstaged_header_height(max_y).0 + 1;
    }
    let main_pane_height = state.main_header_height(max_y).0;
    let diff_view_top = main_pane_offset + main_pane_height;
    let content_height = (max_y as usize).saturating_sub(diff_view_top);

    let num_files = state.files.len();
    let lines_count =
        if state.main_screen.file_cursor > 0 && state.main_screen.file_cursor <= num_files {
            state
                .files
                .get(state.main_screen.file_cursor - 1)
                .map_or(0, |f| f.lines.len())
        } else if state.main_screen.file_cursor > num_files + 1 {
            state
                .selected_commit_files
                .iter()
                .map(|f| f.lines.len())
                .sum()
        } else {
            0
        };

    let (new_line_cursor, new_scroll) = scroll_content(
        state.main_screen.line_cursor,
        state.main_screen.diff_scroll,
        content_height,
        lines_count,
        direction,
        amount,
    );
    state.main_screen.line_cursor = new_line_cursor;
    state.main_screen.diff_scroll = new_scroll;
}

fn scroll_unstaged_diff_view(
    state: &mut AppState,
    direction: ScrollDirection,
    amount: ScrollAmount,
    max_y: i32,
) {
    let unstaged_file_count = state.unstaged_pane.unstaged_files.len();
    let untracked_file_count = state.unstaged_pane.untracked_files.len();

    let lines_count =
        if state.unstaged_pane.cursor > 0 && state.unstaged_pane.cursor <= unstaged_file_count {
            state.get_unstaged_file().map_or(0, |f| f.lines.len())
        } else if state.unstaged_pane.cursor > unstaged_file_count + 1
            && state.unstaged_pane.cursor <= unstaged_file_count + 1 + untracked_file_count
        {
            let file_index = state.unstaged_pane.cursor - unstaged_file_count - 2;
            if let Some(file_path) = state.unstaged_pane.untracked_files.get(file_index) {
                if let Ok((content, _)) = git::read_file_content(&state.repo_path, file_path) {
                    if content.contains(&0x00) {
                        1
                    } else {
                        String::from_utf8_lossy(&content).lines().count()
                    }
                } else {
                    1
                }
            } else {
                0
            }
        } else {
            0
        };

    if lines_count > 0 {
        let mut main_pane_offset = 0;
        if state.main_screen.has_unstaged_changes {
            main_pane_offset = state.unstaged_header_height(max_y).0 + 1;
        }
        let main_pane_height = state.main_header_height(max_y).0;
        let diff_view_top = main_pane_offset + main_pane_height;
        let content_height = (max_y as usize).saturating_sub(diff_view_top);

        let (new_line_cursor, new_scroll) = scroll_content(
            state.main_screen.line_cursor,
            state.unstaged_pane.diff_scroll,
            content_height,
            lines_count,
            direction,
            amount,
        );
        state.main_screen.line_cursor = new_line_cursor;
        state.unstaged_pane.diff_scroll = new_scroll;
    }
}

pub fn handle_scroll(state: &mut AppState, input: Input, max_y: i32) {
    let (direction, amount) = match input {
        Input::Character(' ') | Input::Character('\u{16}') => {
            (ScrollDirection::Down, ScrollAmount::Full)
        }
        Input::Character('b') | Input::Character('\u{2}') => {
            (ScrollDirection::Up, ScrollAmount::Full)
        }
        Input::Character('\u{4}') => (ScrollDirection::Down, ScrollAmount::Half),
        Input::Character('\u{15}') => (ScrollDirection::Up, ScrollAmount::Half),
        _ => return,
    };

    match state.focused_pane {
        FocusedPane::Main => scroll_view(state, direction, amount, max_y),
        FocusedPane::Unstaged => scroll_unstaged_diff_view(state, direction, amount, max_y),
    }
}
