use crate::git_test::common::TestRepo;
use git_full_commit::app_state::AppState;
use git_full_commit::git;
use git_full_commit::ui::update::update_state;
use pancurses::Input;

fn setup_long_file_repo() -> (TestRepo, AppState) {
    let repo = TestRepo::new();
    let initial_content: String = (0..100)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    repo.create_file("a.txt", &initial_content);
    repo.add_all();
    repo.commit("initial");

    let modified_content: String = (0..100)
        .map(|i| format!("changed {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    repo.create_file("a.txt", &modified_content);
    repo.add_all();

    let files = git::get_diff(repo.path.clone());
    let app_state = AppState::new(repo.path.clone(), files);
    (repo, app_state)
}

#[test]
fn test_diff_view_scrolling_j_k() {
    let (_repo, mut app_state) = setup_long_file_repo();

    app_state.main_screen.file_cursor = 1;
    app_state.main_screen.is_diff_cursor_active = true;

    app_state = update_state(app_state, Some(Input::Character('j')), 80, 80);
    assert_eq!(app_state.main_screen.line_cursor, 1);

    app_state = update_state(app_state, Some(Input::Character('j')), 80, 80);
    assert_eq!(app_state.main_screen.line_cursor, 2);

    app_state = update_state(app_state, Some(Input::Character('k')), 80, 80);
    assert_eq!(app_state.main_screen.line_cursor, 1);
}

#[test]
fn test_diff_view_scrolling_page_down() {
    let (_repo, mut app_state) = setup_long_file_repo();
    let max_y = 30;
    let header_height = app_state.main_header_height(max_y).0;
    let page_size = max_y as usize - header_height;

    app_state.main_screen.file_cursor = 1;
    app_state.main_screen.is_diff_cursor_active = true;

    app_state = update_state(app_state, Some(Input::Character(' ')), max_y, 80);
    assert_eq!(app_state.main_screen.line_cursor, page_size);
    assert_eq!(app_state.main_screen.diff_scroll, page_size);
}

#[test]
fn test_diff_view_scrolling_horizontal() {
    let (_repo, mut app_state) = setup_long_file_repo();
    let max_x = 80;
    let scroll_amount = max_x as usize - 10;

    app_state.main_screen.file_cursor = 1;

    app_state = update_state(app_state, Some(Input::KeyRight), 80, max_x);
    assert_eq!(app_state.main_screen.horizontal_scroll, scroll_amount);

    app_state = update_state(app_state, Some(Input::KeyLeft), 80, max_x);
    assert_eq!(app_state.main_screen.horizontal_scroll, 0);
}
