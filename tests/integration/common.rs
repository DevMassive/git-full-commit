pub use crate::git_test::common::TestRepo;
use git_full_commit::app_state::AppState;
use git_full_commit::git;
use git_full_commit::ui::main_screen::ListItem;

pub fn select_commit_in_log(state: &mut AppState, index: usize) {
    let commit_count = state.previous_commits.len();
    let commit_input_index = state
        .main_screen
        .list_items
        .iter()
        .position(|item| matches!(item, ListItem::CommitMessageInput))
        .unwrap();
    state.main_screen.file_cursor = commit_input_index + 1 + (commit_count - 1 - index);
}

pub fn assert_commit_list(list_items: &[ListItem], expected: &[&str]) {
    let mut actual = Vec::new();
    for item in list_items {
        match item {
            ListItem::PreviousCommitInfo { message, .. } => {
                actual.push(message.clone());
            }
            ListItem::EditingReorderCommit { current_text, .. } => {
                actual.push(current_text.clone());
            }
            _ => {}
        }
    }
    assert_eq!(actual, expected);
}

pub fn get_log(repo_path: &std::path::Path) -> Vec<git::CommitInfo> {
    git::get_local_commits(repo_path).unwrap()
}

impl TestRepo {
    pub fn create_initial_state(&self) -> AppState {
        let files = git::get_diff(self.path.clone());
        AppState::new(self.path.clone(), files)
    }
}
