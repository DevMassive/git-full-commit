use crate::cursor_state::CursorState;

pub trait Command {
    fn execute(&mut self) -> bool;
    fn undo(&mut self);
    fn set_cursor_before_execute(&mut self, cursor: CursorState);
    fn set_cursor_before_undo(&mut self, cursor: CursorState);
    fn get_cursor_to_restore_on_undo(&self) -> Option<CursorState>;
    fn get_cursor_to_restore_on_redo(&self) -> Option<CursorState>;
}

macro_rules! command_impl {
    () => {
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
    };
}

mod apply_patch;
mod checkout_file;
mod delete_untracked_file;
mod discard_commit;
mod discard_file;
mod discard_hunk;
mod discard_unstaged_hunk;
mod fixup_commit;
mod ignore_file;
mod ignore_unstaged_tracked_file;
mod ignore_untracked_file;
mod remove_file;
mod reorder_commits;
mod stage_all;
mod stage_file;
mod stage_patch;
mod stage_unstaged;
mod stage_untracked;
mod swap_commit;
mod unstage_all;
mod unstage_file;

pub use apply_patch::ApplyPatchCommand;
pub use checkout_file::CheckoutFileCommand;
pub use delete_untracked_file::DeleteUntrackedFileCommand;
pub use discard_commit::DiscardCommitCommand;
pub use discard_file::DiscardFileCommand;
pub use discard_hunk::DiscardHunkCommand;
pub use discard_unstaged_hunk::DiscardUnstagedHunkCommand;
pub use fixup_commit::FixupCommitCommand;
pub use ignore_file::IgnoreFileCommand;
pub use ignore_unstaged_tracked_file::IgnoreUnstagedTrackedFileCommand;
pub use ignore_untracked_file::IgnoreUntrackedFileCommand;
pub use remove_file::RemoveFileCommand;
pub use reorder_commits::ReorderCommitsCommand;
pub use stage_all::StageAllCommand;
pub use stage_file::StageFileCommand;
pub use stage_patch::StagePatchCommand;
pub use stage_unstaged::StageUnstagedCommand;
pub use stage_untracked::StageUntrackedCommand;
pub use swap_commit::SwapCommitCommand;
pub use unstage_all::UnstageAllCommand;
pub use unstage_file::UnstageFileCommand;

pub struct CommandHistory {
    pub undo_stack: Vec<Box<dyn Command>>,
    pub redo_stack: Vec<Box<dyn Command>>,
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHistory {
    pub fn new() -> Self {
        CommandHistory {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn execute(&mut self, mut command: Box<dyn Command>, cursor_state: CursorState) {
        command.set_cursor_before_execute(cursor_state);
        if command.execute() {
            self.undo_stack.push(command);
            self.redo_stack.clear();
        }
    }

    pub fn undo(&mut self, cursor_state: CursorState) -> Option<CursorState> {
        if let Some(mut command) = self.undo_stack.pop() {
            command.set_cursor_before_undo(cursor_state);
            command.undo();
            let cursor_to_restore = command.get_cursor_to_restore_on_undo();
            self.redo_stack.push(command);
            cursor_to_restore
        } else {
            None
        }
    }

    pub fn redo(&mut self, cursor_state: CursorState) -> Option<CursorState> {
        if let Some(mut command) = self.redo_stack.pop() {
            let cursor_to_restore = command.get_cursor_to_restore_on_redo();

            command.set_cursor_before_execute(cursor_state);

            command.execute();

            self.undo_stack.push(command);
            cursor_to_restore
        } else {
            None
        }
    }
}

#[cfg(test)]
mod apply_patch_command_test;
#[cfg(test)]
mod checkout_file_command_test;
#[cfg(test)]
mod fixup_commit_test;
#[cfg(test)]
mod reorder_commits_command_test;
#[cfg(test)]
mod stage_all_command_test;
#[cfg(test)]
mod stage_file_command_test;
#[cfg(test)]
mod stage_patch_command_test;
#[cfg(test)]
mod stage_unstaged_command_test;
#[cfg(test)]
mod stage_untracked_command_test;
#[cfg(test)]
mod test_helpers;
#[cfg(test)]
mod unstage_all_command_test;
#[cfg(test)]
mod unstage_file_command_test;
