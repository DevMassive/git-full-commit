use crate::command::{Command, FixupCommitCommand};
use crate::ui::main_screen::ListItem;

fn create_dummy_commit_item(message: &str, is_fixup: bool) -> ListItem {
    ListItem::PreviousCommitInfo {
        hash: String::new(),
        message: message.to_string(),
        is_on_remote: false,
        is_fixup,
    }
}

fn get_is_fixup(list: &[ListItem], index: usize) -> bool {
    if let ListItem::PreviousCommitInfo { is_fixup, .. } = &list[index] {
        *is_fixup
    } else {
        panic!("Expected a commit item at index {index}");
    }
}

#[test]
fn test_fixup_commit_command() {
    let mut list_items = vec![
        create_dummy_commit_item("third", false),
        create_dummy_commit_item("second", false),
        create_dummy_commit_item("first", false),
    ];
    let target_index = 1; // "second" commit

    // Check initial state
    assert!(!get_is_fixup(&list_items, target_index));

    // Execute the command in its own scope
    {
        let mut command = FixupCommitCommand::new(&mut list_items, target_index);
        assert!(command.execute());
    } // command is dropped, mutable borrow ends

    // Check state after execute
    assert!(get_is_fixup(&list_items, target_index));

    // Undo the command in its own scope
    {
        // For this specific command, undo is the same as execute,
        // so we can create a new command.
        let mut command = FixupCommitCommand::new(&mut list_items, target_index);
        command.undo();
    } // command is dropped, mutable borrow ends

    // Check state after undo
    assert!(!get_is_fixup(&list_items, target_index));
}

#[test]
fn test_fixup_commit_command_on_root_commit() {
    let mut list_items = vec![
        create_dummy_commit_item("second", false),
        create_dummy_commit_item("first", false), // Root commit
    ];
    let target_index = 1;

    // Check initial state
    assert!(!get_is_fixup(&list_items, target_index));

    // Attempt to execute
    {
        let mut command = FixupCommitCommand::new(&mut list_items, target_index);
        assert!(!command.execute());
    }

    // Verify state has not changed
    assert!(!get_is_fixup(&list_items, target_index));
}
