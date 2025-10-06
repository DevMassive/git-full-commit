use crate::app_state::{AppState, FocusedPane};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorState {
    // Screen
    pub focused_pane: FocusedPane,

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
            focused_pane: state.focused_pane,
            file_cursor: state.main_screen.file_cursor,
            line_cursor: state.main_screen.line_cursor,
            scroll: state.main_screen.diff_scroll,
            file_list_scroll: state.main_screen.file_list_scroll,
            horizontal_scroll: state.main_screen.horizontal_scroll,
            unstaged_cursor: state.unstaged_pane.cursor,
            unstaged_scroll: state.unstaged_pane.scroll,
            unstaged_diff_scroll: state.unstaged_pane.diff_scroll,
            unstaged_horizontal_scroll: state.unstaged_pane.horizontal_scroll,
        }
    }

    pub fn apply_to_app_state(&self, state: &mut AppState) {
        state.focused_pane = self.focused_pane;

        // Restore main screen cursors
        state.main_screen.file_cursor = self.file_cursor.min(state.files.len() + 2);
        if let Some(file) = state.current_main_file() {
            state.main_screen.line_cursor =
                self.line_cursor.min(file.lines.len().saturating_sub(1));
        } else {
            state.main_screen.line_cursor = 0;
        }
        state.main_screen.diff_scroll = self.scroll;
        state.main_screen.file_list_scroll = self.file_list_scroll;
        state.main_screen.horizontal_scroll = self.horizontal_scroll;

        // Restore unstaged screen cursors
        state.unstaged_pane.cursor = self
            .unstaged_cursor
            .min(state.unstaged_pane.unstaged_files.len() + 1);
        // This seems to be unused in the current implementation, but we restore it anyway.
        if let Some(_file) = state.get_unstaged_file() {
            // Unstaged view doesn't have a line_cursor in the same way, it's implicit
        }
        state.unstaged_pane.scroll = self.unstaged_scroll;
        state.unstaged_pane.diff_scroll = self.unstaged_diff_scroll;
        state.unstaged_pane.horizontal_scroll = self.unstaged_horizontal_scroll;
    }
}
