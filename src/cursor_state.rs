use crate::app_state::{AppState, Screen};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorState {
    // Screen
    pub screen: Screen,

    // Main screen
    pub file_cursor: usize,
    pub line_cursor: usize,
    pub scroll: usize,
    pub file_list_scroll: usize,
    pub horizontal_scroll: usize,

    // Unstaged screen
    pub unstaged_cursor: usize,
    pub unstaged_scroll: usize,
    pub unstaged_diff_scroll: usize,
    pub unstaged_horizontal_scroll: usize,
}

impl CursorState {
    pub fn from_app_state(state: &AppState) -> Self {
        Self {
            screen: state.screen,
            file_cursor: state.main_screen.file_cursor,
            line_cursor: state.main_screen.line_cursor,
            scroll: state.main_screen.diff_scroll,
            file_list_scroll: state.main_screen.file_list_scroll,
            horizontal_scroll: state.main_screen.horizontal_scroll,
            unstaged_cursor: state.unstaged_screen.unstaged_cursor,
            unstaged_scroll: state.unstaged_screen.unstaged_scroll,
            unstaged_diff_scroll: state.unstaged_screen.unstaged_diff_scroll,
            unstaged_horizontal_scroll: state.unstaged_screen.unstaged_horizontal_scroll,
        }
    }

    pub fn apply_to_app_state(&self, state: &mut AppState) {
        state.screen = self.screen;

        // Restore main screen cursors
        state.main_screen.file_cursor = self.file_cursor.min(state.files.len() + 2);
        if let Some(file) = state.current_file() {
            state.main_screen.line_cursor = self.line_cursor.min(file.lines.len().saturating_sub(1));
        } else {
            state.main_screen.line_cursor = 0;
        }
        state.main_screen.diff_scroll = self.scroll;
        state.main_screen.file_list_scroll = self.file_list_scroll;
        state.main_screen.horizontal_scroll = self.horizontal_scroll;

        // Restore unstaged screen cursors
        state.unstaged_screen.unstaged_cursor = self.unstaged_cursor.min(state.unstaged_screen.unstaged_files.len() + 1);
        // This seems to be unused in the current implementation, but we restore it anyway.
        if let Some(_file) = state.get_unstaged_file() {
            // Unstaged view doesn't have a line_cursor in the same way, it's implicit
        }
        state.unstaged_screen.unstaged_scroll = self.unstaged_scroll;
        state.unstaged_screen.unstaged_diff_scroll = self.unstaged_diff_scroll;
        state.unstaged_screen.unstaged_horizontal_scroll = self.unstaged_horizontal_scroll;
    }
}
