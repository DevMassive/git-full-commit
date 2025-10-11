pub use crate::git_test::common::TestRepo;
use git_full_commit::app_state::AppState;
use git_full_commit::git;
use git_full_commit::ui::main_screen::ListItem;
use git_full_commit::ui::update::update_state;
use pancurses::Input;
use std::collections::VecDeque;

pub struct Pancurses {
    inputs: VecDeque<Input>,
}

impl Pancurses {
    pub fn new() -> Self {
        Self {
            inputs: VecDeque::new(),
        }
    }

    pub fn send_input(&mut self, input: Input) {
        self.inputs.push_back(input);
    }
}

pub fn generate_test_repo_and_pancurses(_width: i32, _height: i32) -> (TestRepo, Pancurses) {
    let repo = TestRepo::new();
    repo.commit("commit 0");
    repo.commit("commit 1");
    repo.commit("commit 2");
    (repo, Pancurses::new())
}

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
        if let ListItem::PreviousCommitInfo { message, .. } = item {
            actual.push(message.clone());
        }
    }
    assert_eq!(actual, expected);
}

impl TestRepo {
    pub fn create_initial_state(&self) -> AppState {
        let files = git::get_diff(self.path.clone());
        AppState::new(self.path.clone(), files)
    }

    pub fn update_state(&self, state: AppState, pancurses: &mut Pancurses) -> AppState {
        let input = pancurses.inputs.pop_front();
        update_state(state, input, 80, 80)
    }
}
