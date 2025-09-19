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
    let header_height = state.files.len() + 3;
    let content_height = (max_y as usize).saturating_sub(header_height);
    let lines_count = if state.file_cursor == 0 {
        state
            .previous_commit_files
            .iter()
            .map(|f| f.lines.len())
            .sum()
    } else if state.file_cursor > 0 && state.file_cursor <= state.files.len() {
        state
            .files
            .get(state.file_cursor - 1)
            .map_or(0, |f| f.lines.len())
    } else {
        0
    };

    if lines_count > 0 {
        let scroll_amount = match amount {
            ScrollAmount::Full => content_height,
            ScrollAmount::Half => (content_height / 2).max(1),
        };

        let old_scroll = state.scroll;
        match direction {
            ScrollDirection::Down => {
                let max_scroll = lines_count.saturating_sub(content_height).max(0);
                let new_scroll = state.scroll.saturating_add(scroll_amount).min(max_scroll);
                state.scroll = new_scroll;
                let scrolled_by = new_scroll - old_scroll;
                state.line_cursor = state
                    .line_cursor
                    .saturating_add(scrolled_by)
                    .min(lines_count.saturating_sub(1));
            }
            ScrollDirection::Up => {
                state.scroll = state.scroll.saturating_sub(scroll_amount);
                let scrolled_by = old_scroll - state.scroll;
                state.line_cursor = state.line_cursor.saturating_sub(scrolled_by);
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
