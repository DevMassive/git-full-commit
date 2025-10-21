use super::Command;
use crate::cursor_state::CursorState;
use crate::ui::main_screen::ListItem;

pub struct DiscardCommitCommand {
    pub list_items: *mut Vec<ListItem>,
    pub index: usize,
    removed_item: Option<ListItem>,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl DiscardCommitCommand {
    pub fn new(list_items: *mut Vec<ListItem>, index: usize) -> Self {
        Self {
            list_items,
            index,
            removed_item: None,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for DiscardCommitCommand {
    fn execute(&mut self) -> bool {
        unsafe {
            self.removed_item = Some((*self.list_items).remove(self.index));
        }
        true
    }

    fn undo(&mut self) {
        if let Some(item) = self.removed_item.take() {
            unsafe {
                (*self.list_items).insert(self.index, item);
            }
        }
    }

    command_impl!();
}
