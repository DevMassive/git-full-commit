use crate::command::Command;
use crate::cursor_state::CursorState;
use crate::ui::main_screen::ListItem;

pub struct FixupCommitCommand {
    list_items: *mut Vec<ListItem>,
    index: usize,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl FixupCommitCommand {
    pub fn new(list_items: *mut Vec<ListItem>, index: usize) -> Self {
        Self {
            list_items,
            index,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }

    fn toggle_fixup(&mut self) -> bool {
        unsafe {
            let list_items = &mut *self.list_items;
            let len = list_items.len();
            // The root commit is the last one in the list.
            if self.index >= len.saturating_sub(1) {
                return false;
            }

            if let Some(ListItem::PreviousCommitInfo { is_fixup, .. }) =
                list_items.get_mut(self.index)
            {
                *is_fixup = !*is_fixup;
                true
            } else {
                false
            }
        }
    }
}

impl Command for FixupCommitCommand {
    fn execute(&mut self) -> bool {
        self.toggle_fixup()
    }

    fn undo(&mut self) {
        self.toggle_fixup();
    }

    fn set_cursor_before_execute(&mut self, cursor: CursorState) {
        self.cursor_before_execute = Some(cursor);
    }

    fn set_cursor_before_undo(&mut self, cursor: CursorState) {
        self.cursor_before_undo = Some(cursor);
    }

    fn get_cursor_to_restore_on_undo(&self) -> Option<CursorState> {
        self.cursor_before_execute
    }

    fn get_cursor_to_restore_on_redo(&self) -> Option<CursorState> {
        self.cursor_before_undo
    }
}
