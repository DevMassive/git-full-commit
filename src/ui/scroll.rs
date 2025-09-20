use crate::app_state::AppState;
use pancurses::Input;

enum ScrollDirection {
    Up,
    Down,
}

enum ScrollAmount {
    Full,
    Half,
}

fn scroll_view(state: &mut AppState, direction: ScrollDirection, amount: ScrollAmount, max_y: i32) {
    let header_height = state.files.len() + 4;
    let content_height = (max_y as usize).saturating_sub(header_height);
    let num_files = state.files.len();
    let lines_count = if state.file_cursor > 0 && state.file_cursor <= num_files {
        state
            .files
            .get(state.file_cursor - 1)
            .map_or(0, |f| f.lines.len())
    } else if state.file_cursor == num_files + 2 {
        state
            .previous_commit_files
            .iter()
            .map(|f| f.lines.len())
            .sum()
    } else {
        0
    };

    if lines_count > 0 {
        let scroll_amount = match amount {
            ScrollAmount::Full => content_height,
            ScrollAmount::Half => (content_height / 2).max(1),
        };

        let max_line = lines_count.saturating_sub(1);

        match direction {
            ScrollDirection::Down => {
                let next_line_cursor = state.line_cursor.saturating_add(scroll_amount);
                state.line_cursor = next_line_cursor.min(max_line);
                if state.line_cursor >= state.scroll + content_height {
                    state.scroll = state.scroll.saturating_add(scroll_amount);
                }
            }
            ScrollDirection::Up => {
                let next_line_cursor = state.line_cursor.saturating_sub(scroll_amount);
                if next_line_cursor < state.scroll {
                    state.scroll = state.scroll.saturating_sub(scroll_amount);
                }
                state.line_cursor = next_line_cursor;
            }
        }

    }
}

pub fn handle_scroll(state: &mut AppState, input: Input, max_y: i32) {
    match input {
        Input::Character(' ') => {
            scroll_view(state, ScrollDirection::Down, ScrollAmount::Full, max_y);
        }
        Input::Character('b') => {
            scroll_view(state, ScrollDirection::Up, ScrollAmount::Full, max_y);
        }
        Input::Character('\u{4}') => {
            scroll_view(state, ScrollDirection::Down, ScrollAmount::Half, max_y);
        }
        Input::Character('\u{15}') => {
            scroll_view(state, ScrollDirection::Up, ScrollAmount::Half, max_y);
        }
        _ => {}
    }
}
