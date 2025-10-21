use super::Command;
use crate::cursor_state::CursorState;
use crate::ui::main_screen::ListItem;

pub struct SwapCommitCommand {
    pub list_items: *mut Vec<ListItem>,
    pub index1: usize,
    pub index2: usize,
    cursor_before_execute: Option<CursorState>,
    cursor_before_undo: Option<CursorState>,
}

impl SwapCommitCommand {
    pub fn new(list_items: *mut Vec<ListItem>, index1: usize, index2: usize) -> Self {
        Self {
            list_items,
            index1,
            index2,
            cursor_before_execute: None,
            cursor_before_undo: None,
        }
    }
}

impl Command for SwapCommitCommand {
    fn execute(&mut self) -> bool {
        unsafe {
            (*self.list_items).swap(self.index1, self.index2);
        }
        true
    }

    fn undo(&mut self) {
        unsafe {
            (*self.list_items).swap(self.index1, self.index2);
        }
    }

    command_impl!();
}
