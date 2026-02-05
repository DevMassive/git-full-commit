use crate::app_state::{AppState, EditorRequest, FocusedPane};
use crate::command::{
    ApplyPatchCommand, CheckoutFileCommand, CommandHistory, DeleteUntrackedFileCommand,
    DiscardCommitCommand, DiscardFileCommand, DiscardHunkCommand, DiscardUnstagedHunkCommand,
    IgnoreFileCommand, IgnoreUnstagedTrackedFileCommand, IgnoreUntrackedFileCommand,
    StageAllCommand, StageFileCommand, StagePatchCommand, StageUnstagedCommand,
    StageUntrackedCommand, UnstageAllCommand, UnstageFileCommand,
};
use crate::commit_storage;
use crate::git::{self, FileStatus};
use crate::ui::commit_view;
use crate::ui::diff_view;
use crate::ui::diff_view::LINE_CONTENT_OFFSET;
use crate::ui::scroll;
use pancurses::Input;

use super::keyboard::{
    is_diff_move_down, is_diff_move_up, is_horizontal_left, is_horizontal_right, is_move_down,
    is_move_up, is_stage_toggle, is_vertical_navigation,
};
use crate::git_patch;
use pancurses::{COLOR_PAIR, Window};

fn is_binary(content: &[u8]) -> bool {
    content.contains(&0x00)
}

#[derive(Debug, Clone)]
pub enum UnstagedListItem {
    UnstagedChangesHeader,
    File(crate::git::FileDiff),
    UntrackedFilesHeader,
    UntrackedFile(String),
}

#[derive(Debug, Clone)]
pub enum ListItem {
    StagedChangesHeader,
    File(crate::git::FileDiff),
    CommitMessageInput,
    PreviousCommitInfo {
        hash: String,
        message: String,
        is_on_remote: bool,
        is_fixup: bool,
    },
    AmendingCommitMessageInput {
        hash: String,
        message: String,
    },
    EditingReorderCommit {
        hash: String,
        original_message: String,
        current_text: String,
        cursor: usize,
        scroll_offset: usize,
        scroll_extra_space: bool,
        is_on_remote: bool,
        is_fixup: bool,
    },
}

pub fn render(window: &Window, state: &AppState) {
    let (max_y, max_x) = window.get_max_yx();
    let mut main_pane_offset = 0;

    if state.main_screen.has_unstaged_changes && !state.main_screen.is_reordering_commits {
        let unstaged_pane_height = render_unstaged_pane(window, state, max_y, max_x);
        main_pane_offset = unstaged_pane_height;
    }

    let (main_pane_carret_y, main_pane_carret_x) =
        render_main_pane(window, state, max_y, max_x, main_pane_offset);

    if state.main_screen.is_reordering_commits {
        window.attron(COLOR_PAIR(1));
        let title = " Commit Reordering (Up/Down: move, Enter: confirm, Esc/q: cancel) ";
        let title_x = (max_x - title.len() as i32) / 2;
        window.mvaddstr(0, title_x, title);
        window.attroff(COLOR_PAIR(1));
    }

    let main_pane_height = state.main_header_height(max_y).0;
    let diff_view_top = main_pane_offset + main_pane_height;
    render_diff_view(window, state, max_y, diff_view_top);

    let is_editing_commit = state.is_in_input_mode();

    let (carret_y, carret_x) = if state.focused_pane == FocusedPane::Main {
        (main_pane_carret_y, main_pane_carret_x)
    } else {
        (0, 0) // Unstaged pane does not have a text input
    };

    window.mv(carret_y, carret_x);
    if is_editing_commit {
        #[cfg(not(test))]
        pancurses::curs_set(1);
    } else {
        #[cfg(not(test))]
        pancurses::curs_set(0);
    }

    if let Some(error) = &state.error_message {
        let error_y = max_y - 1;
        window.attron(COLOR_PAIR(10));
        for x in 0..max_x {
            window.mvaddch(error_y, x, ' ');
        }
        window.mvaddstr(error_y, 0, error);
        window.attroff(COLOR_PAIR(10));
    }
}

fn render_unstaged_pane(window: &Window, state: &AppState, max_y: i32, max_x: i32) -> usize {
    let (file_list_height, file_list_total_items) = state.unstaged_header_height(max_y);
    let is_focused = state.focused_pane == FocusedPane::Unstaged;

    for i in 0..file_list_height {
        let item_index = state.unstaged_pane.scroll + i;
        if item_index >= file_list_total_items {
            break;
        }
        let line_y = i as i32;
        let is_selected = is_focused && state.unstaged_pane.cursor == item_index;

        let item = &state.unstaged_pane.list_items[item_index];

        match item {
            UnstagedListItem::UnstagedChangesHeader => {
                let pair = if is_selected { 5 } else { 1 };
                window.attron(COLOR_PAIR(pair));
                if is_selected {
                    for x in 0..max_x {
                        window.mvaddch(line_y, x, ' ');
                    }
                }
                window.mv(line_y, 0);
                window.addstr(" Unstaged changes");
                window.attroff(COLOR_PAIR(pair));
            }
            UnstagedListItem::File(file) => {
                let pair = if is_selected { 5 } else { 1 };
                let status_pair = if is_selected { 6 } else { 2 };

                window.attron(COLOR_PAIR(pair));
                if is_selected {
                    for x in 0..max_x {
                        window.mvaddch(line_y, x, ' ');
                    }
                }
                window.mv(line_y, 0);
                window.attroff(COLOR_PAIR(pair));

                let status_char = match file.status {
                    FileStatus::Added => 'A',
                    FileStatus::Modified => 'M',
                    FileStatus::Renamed => 'R',
                    FileStatus::Deleted => 'D',
                };
                window.attron(COLOR_PAIR(pair));
                window.addstr("   ");
                window.attroff(COLOR_PAIR(pair));
                window.attron(COLOR_PAIR(status_pair));
                window.addstr(format!("{status_char}"));
                window.attroff(COLOR_PAIR(status_pair));
                window.attron(COLOR_PAIR(pair));
                if file.status == FileStatus::Renamed {
                    window.addstr(format!(" {} -> {}", file.old_file_name, file.file_name));
                } else {
                    window.addstr(format!(" {}", file.file_name));
                }
                window.attroff(COLOR_PAIR(pair));
            }
            UnstagedListItem::UntrackedFilesHeader => {
                let pair = if is_selected { 5 } else { 1 };
                window.attron(COLOR_PAIR(pair));
                if is_selected {
                    for x in 0..max_x {
                        window.mvaddch(line_y, x, ' ');
                    }
                }
                window.mv(line_y, 0);
                window.addstr(" Untracked files");
                window.attroff(COLOR_PAIR(pair));
            }
            UnstagedListItem::UntrackedFile(file_name) => {
                let pair = if is_selected { 5 } else { 1 };
                window.attron(COLOR_PAIR(pair));
                if is_selected {
                    for x in 0..max_x {
                        window.mvaddch(line_y, x, ' ');
                    }
                }
                window.mv(line_y, 0);
                window.addstr(format!("    ? {file_name}"));
                window.attroff(COLOR_PAIR(pair));
            }
        }
    }
    file_list_height
}

fn render_main_pane(
    window: &Window,
    state: &AppState,
    _max_y: i32,
    max_x: i32,
    top_offset: usize,
) -> (i32, i32) {
    let (max_y, _) = window.get_max_yx();
    let (file_list_height, file_list_total_items) = state.main_header_height(max_y);
    let is_focused = state.focused_pane == FocusedPane::Main;
    let mut carret_y = 0;
    let mut carret_x = 0;

    for i in 0..file_list_height {
        let item_index = state.main_screen.file_list_scroll + i;
        if item_index >= file_list_total_items {
            break;
        }
        let line_y = top_offset as i32 + i as i32;
        let is_selected = is_focused && state.main_screen.file_cursor == item_index;

        let item = &state.main_screen.list_items[item_index];

        match item {
            ListItem::StagedChangesHeader => {
                let pair = if is_selected { 5 } else { 1 };
                window.attron(COLOR_PAIR(pair));
                if is_selected {
                    for x in 0..max_x {
                        window.mvaddch(line_y, x, ' ');
                    }
                }
                window.mv(line_y, 0);
                window.addstr(" Staged changes");
                window.attroff(COLOR_PAIR(pair));
            }
            ListItem::File(file) => {
                let pair = if is_selected { 5 } else { 1 };
                let status_pair = if is_selected { 6 } else { 2 };

                window.attron(COLOR_PAIR(pair));
                if is_selected {
                    for x in 0..max_x {
                        window.mvaddch(line_y, x, ' ');
                    }
                }
                window.mv(line_y, 0);
                window.attroff(COLOR_PAIR(pair));

                let status_char = match file.status {
                    FileStatus::Added => 'A',
                    FileStatus::Modified => 'M',
                    FileStatus::Renamed => 'R',
                    FileStatus::Deleted => 'D',
                };
                window.attron(COLOR_PAIR(pair));
                window.addstr("   ");
                window.attroff(COLOR_PAIR(pair));
                window.attron(COLOR_PAIR(status_pair));
                window.addstr(format!("{status_char}"));
                window.attroff(COLOR_PAIR(status_pair));
                window.attron(COLOR_PAIR(pair));
                if file.status == FileStatus::Renamed {
                    window.addstr(format!(" {} -> {}", file.old_file_name, file.file_name));
                } else {
                    window.addstr(format!(" {}", file.file_name));
                }
                window.attroff(COLOR_PAIR(pair));
            }
            ListItem::CommitMessageInput => {
                if state.main_screen.amending_commit_hash.is_none() {
                    (carret_x, carret_y) =
                        commit_view::render(window, state, is_selected, line_y, max_x);
                } else if is_selected {
                    let pair = 5;
                    window.attron(COLOR_PAIR(pair));
                    for x in 0..max_x {
                        window.mvaddch(line_y, x, ' ');
                    }
                    window.attroff(COLOR_PAIR(pair));
                }
            }
            ListItem::PreviousCommitInfo {
                hash: _,
                message,
                is_on_remote,
                is_fixup,
            } => {
                let pair = if is_selected { 5 } else { 1 };
                window.attron(COLOR_PAIR(pair));
                if is_selected {
                    for x in 0..max_x {
                        window.mvaddch(line_y, x, ' ');
                    }
                }
                window.attroff(COLOR_PAIR(pair));

                window.mv(line_y, 0);
                let status_pair = if *is_on_remote {
                    if is_selected { 8 } else { 4 }
                } else if is_selected {
                    7
                } else {
                    3
                };
                window.attron(COLOR_PAIR(status_pair));
                window.addstr(" ● ");
                window.attroff(COLOR_PAIR(status_pair));

                window.attron(COLOR_PAIR(pair));
                if *is_fixup {
                    window.addstr("fixup!");
                } else {
                    use unicode_width::UnicodeWidthStr;
                    let prefix_width = " ● ".width();
                    let available_width = (max_x as usize).saturating_sub(prefix_width);
                    let mut truncated_message = String::new();
                    let mut current_width = 0;
                    for ch in message.chars() {
                        let char_width = ch.to_string().width();
                        if current_width + char_width > available_width {
                            break;
                        }
                        truncated_message.push(ch);
                        current_width += char_width;
                    }
                    window.addstr(&truncated_message);
                }
                window.attroff(COLOR_PAIR(pair));
            }
            ListItem::AmendingCommitMessageInput { .. } => {
                (carret_x, carret_y) =
                    commit_view::render(window, state, is_selected, line_y, max_x);
            }
            ListItem::EditingReorderCommit {
                current_text,
                cursor,
                scroll_offset,
                scroll_extra_space,
                ..
            } => {
                (carret_x, carret_y) = commit_view::render_editor(
                    window,
                    current_text,
                    *cursor,
                    is_selected,
                    line_y,
                    max_x,
                    " ● ",
                    *scroll_offset,
                    *scroll_extra_space,
                );
            }
        }
    }
    (carret_y, carret_x)
}

fn render_diff_view(window: &Window, state: &AppState, max_y: i32, top_offset: usize) {
    let content_height = (max_y as usize).saturating_sub(top_offset);

    match state.focused_pane {
        FocusedPane::Main => {
            let cursor_position = state.get_cursor_line_index();
            match state.current_main_item() {
                Some(ListItem::StagedChangesHeader) => {
                    // "Staged changes" is selected, do nothing for now.
                }
                Some(ListItem::PreviousCommitInfo { .. }) => {
                    diff_view::render_multiple(
                        window,
                        &state.selected_commit_files,
                        content_height,
                        state.main_screen.diff_scroll,
                        state.main_screen.horizontal_scroll,
                        top_offset,
                        cursor_position,
                        state.main_screen.is_diff_cursor_active,
                    );
                }
                Some(ListItem::File(selected_file)) => {
                    diff_view::render(
                        window,
                        selected_file,
                        content_height,
                        state.main_screen.diff_scroll,
                        state.main_screen.horizontal_scroll,
                        top_offset,
                        cursor_position,
                        state.main_screen.is_diff_cursor_active,
                    );
                }
                _ => {}
            }
        }
        FocusedPane::Unstaged => {
            let cursor_position = state.main_screen.line_cursor;
            match state
                .unstaged_pane
                .list_items
                .get(state.unstaged_pane.cursor)
            {
                Some(UnstagedListItem::File(selected_file)) => {
                    diff_view::render(
                        window,
                        selected_file,
                        content_height,
                        state.unstaged_pane.diff_scroll,
                        state.unstaged_pane.horizontal_scroll,
                        top_offset,
                        cursor_position,
                        state.unstaged_pane.is_diff_cursor_active,
                    );
                }
                Some(UnstagedListItem::UntrackedFile(file_name)) => {
                    let lines = match git::read_file_content(&state.repo_path, file_name) {
                        Ok((content, size)) => {
                            if is_binary(&content) {
                                vec![format!("  Binary file (size: {} bytes)", size)]
                            } else {
                                String::from_utf8_lossy(&content)
                                    .lines()
                                    .map(|l| format!(" {l}"))
                                    .collect()
                            }
                        }
                        Err(e) => vec![format!("  Error reading file: {}", e)],
                    };

                    diff_view::render_plain(
                        window,
                        lines,
                        content_height,
                        state.unstaged_pane.diff_scroll,
                        state.unstaged_pane.horizontal_scroll,
                        top_offset,
                        cursor_position,
                        state.unstaged_pane.is_diff_cursor_active,
                    );
                }
                _ => {}
            }
        }
    }
}

pub fn handle_alt_input(state: &mut AppState, input: Input, _max_y: i32, max_x: i32) {
    if let Some(item) = state
        .main_screen
        .list_items
        .get_mut(state.main_screen.file_cursor)
    {
        if let ListItem::EditingReorderCommit {
            current_text,
            cursor,
            scroll_offset,
            scroll_extra_space,
            ..
        } = item
        {
            commit_view::handle_generic_text_input_with_alt(current_text, cursor, input);
            let (offset, extra_space) = commit_view::compute_scroll_for_prefix(
                current_text.as_str(),
                *cursor,
                max_x,
                " ● ",
            );
            *scroll_offset = offset;
            *scroll_extra_space = extra_space;
        } else {
            commit_view::handle_commit_input_with_alt(state, input, max_x);
        }
    }
}

pub fn handle_input(state: &mut AppState, input: Input, max_y: i32, max_x: i32) {
    match state.focused_pane {
        crate::app_state::FocusedPane::Main => {
            handle_main_pane_input(state, input, max_y, max_x);
        }
        crate::app_state::FocusedPane::Unstaged => {
            handle_unstaged_pane_input(state, input, max_y, max_x);
        }
    }
}

fn handle_unstaged_pane_input(state: &mut AppState, input: Input, max_y: i32, max_x: i32) {
    let (file_list_height, unstaged_items_count) = state.unstaged_header_height(max_y);

    if handle_unstaged_quit(state, &input) {
        return;
    }

    if handle_unstaged_vertical_navigation(state, &input, file_list_height, unstaged_items_count) {
        return;
    }

    if handle_unstaged_diff_navigation(state, &input, max_y) {
        return;
    }

    if handle_unstaged_horizontal_scroll(state, &input, max_x) {
        return;
    }

    if handle_unstaged_stage_action(state, &input, max_y) {
        return;
    }

    if handle_unstaged_stage_line(state, &input, max_y) {
        return;
    }

    if handle_unstaged_stage_all(state, &input) {
        return;
    }

    if handle_unstaged_open_editor(state, &input) {
        return;
    }

    if handle_unstaged_discard(state, &input) {
        return;
    }

    if handle_unstaged_ignore(state, &input) {
        return;
    }

    scroll::handle_scroll(state, input, max_y);
}

fn handle_unstaged_quit(state: &mut AppState, input: &Input) -> bool {
    if matches!(input, Input::Character('q')) {
        if state.unstaged_pane.is_diff_cursor_active {
            state.unstaged_pane.is_diff_cursor_active = false;
        } else {
            let _ = commit_storage::save_commit_message(
                &state.repo_path,
                &state.main_screen.commit_message,
            );
            state.running = false;
        }
        return true;
    }
    false
}

fn handle_unstaged_vertical_navigation(
    state: &mut AppState,
    input: &Input,
    file_list_height: usize,
    total_items: usize,
) -> bool {
    if is_move_up(input) {
        state.unstaged_pane.cursor = state.unstaged_pane.cursor.saturating_sub(1);
        state.unstaged_pane.diff_scroll = 0;
        state.main_screen.line_cursor = 0;
        state.unstaged_pane.is_diff_cursor_active = false;
        if state.unstaged_pane.cursor < state.unstaged_pane.scroll {
            state.unstaged_pane.scroll = state.unstaged_pane.cursor;
        }
        return true;
    }

    if is_move_down(input) {
        if total_items > 0
            && state.unstaged_pane.cursor == total_items - 1
            && !state.main_screen.list_items.is_empty()
        {
            state.focused_pane = FocusedPane::Main;
            state.main_screen.file_cursor = 0;
            state.main_screen.file_list_scroll = 0;
            return true;
        }

        state.unstaged_pane.cursor = state
            .unstaged_pane
            .cursor
            .saturating_add(1)
            .min(total_items.saturating_sub(1));
        state.unstaged_pane.diff_scroll = 0;
        state.main_screen.line_cursor = 0;
        state.unstaged_pane.is_diff_cursor_active = false;
        if state.unstaged_pane.cursor >= state.unstaged_pane.scroll + file_list_height {
            state.unstaged_pane.scroll = state.unstaged_pane.cursor - file_list_height + 1;
        }
        return true;
    }

    false
}

fn handle_unstaged_diff_navigation(state: &mut AppState, input: &Input, max_y: i32) -> bool {
    if is_diff_move_up(input) {
        state.unstaged_pane.is_diff_cursor_active = true;
        state.main_screen.line_cursor = state.main_screen.line_cursor.saturating_sub(1);
        if state.main_screen.line_cursor < state.unstaged_pane.diff_scroll {
            state.unstaged_pane.diff_scroll = state.main_screen.line_cursor;
        }
        return true;
    }

    if is_diff_move_down(input) {
        state.unstaged_pane.is_diff_cursor_active = true;
        let file_lines_count = match state
            .unstaged_pane
            .list_items
            .get(state.unstaged_pane.cursor)
        {
            Some(UnstagedListItem::File(file)) => file.lines.len(),
            Some(UnstagedListItem::UntrackedFile(file_name)) => {
                if let Ok((content, _)) = git::read_file_content(&state.repo_path, file_name) {
                    if is_binary(&content) {
                        1
                    } else {
                        String::from_utf8_lossy(&content).lines().count()
                    }
                } else {
                    1
                }
            }
            _ => 0,
        };

        if state.main_screen.line_cursor < file_lines_count.saturating_sub(1) {
            state.main_screen.line_cursor += 1;

            let mut main_pane_offset = 0;
            if state.main_screen.has_unstaged_changes {
                main_pane_offset = state.unstaged_header_height(max_y).0 + 1;
            }
            let main_pane_height = state.main_header_height(max_y).0;
            let diff_view_top = main_pane_offset + main_pane_height;
            let content_height = (max_y as usize).saturating_sub(diff_view_top);

            if state.main_screen.line_cursor >= state.unstaged_pane.diff_scroll + content_height {
                state.unstaged_pane.diff_scroll =
                    state.main_screen.line_cursor - content_height + 1;
            }
        }
        return true;
    }

    false
}

fn handle_unstaged_horizontal_scroll(state: &mut AppState, input: &Input, max_x: i32) -> bool {
    if is_horizontal_left(input) {
        let scroll_amount = (max_x as usize).saturating_sub(diff_view::LINE_CONTENT_OFFSET);
        state.unstaged_pane.horizontal_scroll = state
            .unstaged_pane
            .horizontal_scroll
            .saturating_sub(scroll_amount);
        return true;
    }

    if is_horizontal_right(input) {
        let scroll_amount = (max_x as usize).saturating_sub(diff_view::LINE_CONTENT_OFFSET);
        state.unstaged_pane.horizontal_scroll = state
            .unstaged_pane
            .horizontal_scroll
            .saturating_add(scroll_amount);
        return true;
    }

    false
}

fn handle_unstaged_stage_action(state: &mut AppState, input: &Input, max_y: i32) -> bool {
    if !is_stage_toggle(input) {
        return false;
    }

    match state
        .unstaged_pane
        .list_items
        .get(state.unstaged_pane.cursor)
    {
        Some(UnstagedListItem::UnstagedChangesHeader) => {
            let command = Box::new(StageUnstagedCommand::new(state.repo_path.clone()));
            state.execute_and_refresh(command);
        }
        Some(UnstagedListItem::File(file)) => {
            if state.unstaged_pane.is_diff_cursor_active {
                if let Some(hunk) = git_patch::find_hunk(file, state.main_screen.line_cursor) {
                    let patch = git_patch::create_stage_hunk_patch(file, hunk);
                    let command = Box::new(StagePatchCommand::new(state.repo_path.clone(), patch));

                    let old_line_cursor = state.main_screen.line_cursor;
                    state.execute_and_refresh(command);

                    if let Some(updated_file) = state.get_unstaged_file() {
                        state.main_screen.line_cursor =
                            old_line_cursor.min(updated_file.lines.len().saturating_sub(1));
                        let (file_list_height, _) = state.unstaged_header_height(max_y);
                        let content_height = (max_y as usize).saturating_sub(file_list_height + 1);
                        if state.main_screen.line_cursor
                            >= state.unstaged_pane.diff_scroll + content_height
                        {
                            state.unstaged_pane.diff_scroll =
                                state.main_screen.line_cursor - content_height + 1;
                        }
                    } else {
                        state.main_screen.line_cursor = 0;
                    }
                } else {
                    let command = Box::new(StageFileCommand::new(
                        state.repo_path.clone(),
                        file.file_name.clone(),
                    ));
                    state.execute_and_refresh(command);
                }
            } else {
                let command = Box::new(StageFileCommand::new(
                    state.repo_path.clone(),
                    file.file_name.clone(),
                ));
                state.execute_and_refresh(command);
            }
        }
        Some(UnstagedListItem::UntrackedFilesHeader) => {
            let command = Box::new(StageUntrackedCommand::new(state.repo_path.clone()));
            state.execute_and_refresh(command);
        }
        Some(UnstagedListItem::UntrackedFile(file_name)) => {
            let command = Box::new(StageFileCommand::new(
                state.repo_path.clone(),
                file_name.clone(),
            ));
            state.execute_and_refresh(command);
        }
        _ => {}
    }

    true
}

fn handle_unstaged_stage_line(state: &mut AppState, input: &Input, max_y: i32) -> bool {
    if !matches!(input, Input::Character('1')) {
        return false;
    }

    if let Some(UnstagedListItem::File(file)) = state
        .unstaged_pane
        .list_items
        .get(state.unstaged_pane.cursor)
    {
        if let Some(patch) = git_patch::create_stage_line_patch(file, state.main_screen.line_cursor)
        {
            let command = Box::new(StagePatchCommand::new(state.repo_path.clone(), patch));

            let old_line_cursor = state.main_screen.line_cursor;
            state.execute_and_refresh(command);

            if let Some(updated_file) = state.get_unstaged_file() {
                state.main_screen.line_cursor =
                    old_line_cursor.min(updated_file.lines.len().saturating_sub(1));
                let (file_list_height, _) = state.unstaged_header_height(max_y);
                let content_height = (max_y as usize).saturating_sub(file_list_height + 1);
                if state.main_screen.line_cursor >= state.unstaged_pane.diff_scroll + content_height
                {
                    state.unstaged_pane.diff_scroll =
                        state.main_screen.line_cursor - content_height + 1;
                }
            } else {
                state.main_screen.line_cursor = 0;
            }
        }
    }

    true
}

fn handle_unstaged_stage_all(state: &mut AppState, input: &Input) -> bool {
    if matches!(input, Input::Character('R')) {
        let command = Box::new(StageAllCommand::new(state.repo_path.clone()));
        state.execute_and_refresh(command);
        return true;
    }
    false
}

fn handle_unstaged_open_editor(state: &mut AppState, input: &Input) -> bool {
    if !matches!(input, Input::Character('e')) {
        return false;
    }

    match state
        .unstaged_pane
        .list_items
        .get(state.unstaged_pane.cursor)
    {
        Some(UnstagedListItem::File(file)) => {
            let line_number = git_patch::get_line_number(file, state.main_screen.line_cursor);
            let file_path = state.repo_path.join(&file.file_name);
            if let Some(path_str) = file_path.to_str() {
                state.editor_request = Some(EditorRequest {
                    file_path: path_str.to_string(),
                    line_number,
                });
            }
        }
        Some(UnstagedListItem::UntrackedFile(file_name)) => {
            let file_path = state.repo_path.join(file_name);
            if let Some(path_str) = file_path.to_str() {
                state.editor_request = Some(EditorRequest {
                    file_path: path_str.to_string(),
                    line_number: None,
                });
            }
        }
        _ => {}
    }

    true
}

fn handle_unstaged_discard(state: &mut AppState, input: &Input) -> bool {
    if !matches!(input, Input::Character('!')) {
        return false;
    }

    match state
        .unstaged_pane
        .list_items
        .get(state.unstaged_pane.cursor)
    {
        Some(UnstagedListItem::File(file)) => {
            if state.unstaged_pane.is_diff_cursor_active {
                if let Some(hunk) = git_patch::find_hunk(file, state.main_screen.line_cursor) {
                    let patch = git_patch::create_unstage_hunk_patch(file, hunk);
                    let command = Box::new(DiscardUnstagedHunkCommand::new(
                        state.repo_path.clone(),
                        patch,
                    ));
                    state.execute_and_refresh(command);
                }
            } else {
                let patch = git::get_unstaged_file_diff_patch(&state.repo_path, &file.file_name)
                    .unwrap_or_default();
                let command = Box::new(CheckoutFileCommand::new(
                    state.repo_path.clone(),
                    file.file_name.clone(),
                    patch,
                ));
                state.execute_and_refresh(command);
            }
        }
        Some(UnstagedListItem::UntrackedFile(file_name)) => {
            if let Ok((content, _)) = git::read_file_content(&state.repo_path, file_name) {
                if is_binary(&content) {
                    return true;
                }
                let command = Box::new(DeleteUntrackedFileCommand::new(
                    state.repo_path.clone(),
                    file_name.clone(),
                    content,
                ));
                state.execute_and_refresh(command);
            }
        }
        _ => {}
    }

    true
}

fn handle_unstaged_ignore(state: &mut AppState, input: &Input) -> bool {
    if !matches!(input, Input::Character('i')) {
        return false;
    }

    let mut file_to_ignore: Option<String> = None;
    let mut is_tracked = false;

    match state
        .unstaged_pane
        .list_items
        .get(state.unstaged_pane.cursor)
    {
        Some(UnstagedListItem::File(file)) => {
            file_to_ignore = Some(file.file_name.clone());
            is_tracked = true;
        }
        Some(UnstagedListItem::UntrackedFile(file_name)) => {
            file_to_ignore = Some(file_name.clone());
            is_tracked = false;
        }
        _ => {}
    }

    if let Some(file_name) = file_to_ignore {
        if file_name != ".gitignore" {
            let command: Box<dyn crate::command::Command> = if is_tracked {
                Box::new(IgnoreUnstagedTrackedFileCommand::new(
                    state.repo_path.clone(),
                    file_name,
                ))
            } else {
                Box::new(IgnoreUntrackedFileCommand::new(
                    state.repo_path.clone(),
                    file_name,
                ))
            };
            state.execute_and_refresh(command);
        }
    }

    true
}

fn handle_main_pane_input(state: &mut AppState, input: Input, max_y: i32, max_x: i32) {
    if state.main_screen.is_reordering_commits {
        handle_reorder_mode_input(state, input, max_y, max_x);
        return;
    }

    if state.is_in_input_mode() {
        if is_vertical_navigation(&input) {
            handle_navigation(state, input, max_y, max_x);
        } else {
            // Other keys go to the text editor
            commit_view::handle_commit_input(state, input, max_y, max_x);
        }
    } else if !handle_commands(state, &input, max_y) {
        handle_navigation(state, input, max_y, max_x);
    }
}

fn unstage_line(state: &mut AppState, max_y: i32) {
    if let Some(file) = state.current_main_file() {
        let line_index = state.main_screen.line_cursor;
        if let Some(patch) = git_patch::create_unstage_line_patch(file, line_index, true) {
            let command = Box::new(ApplyPatchCommand::new(state.repo_path.clone(), patch));
            let old_line_cursor = state.main_screen.line_cursor;
            state.execute_and_refresh(command);

            if let Some(file) = state.current_main_file() {
                state.main_screen.line_cursor =
                    old_line_cursor.min(file.lines.len().saturating_sub(1));
                let header_height = state.main_header_height(max_y).0;
                let content_height = (max_y as usize).saturating_sub(header_height);
                if state.main_screen.line_cursor >= state.main_screen.diff_scroll + content_height {
                    state.main_screen.diff_scroll =
                        state.main_screen.line_cursor - content_height + 1;
                }
            }
        }
    }
}

fn handle_commands(state: &mut AppState, input: &Input, max_y: i32) -> bool {
    if handle_main_quit(state, input) {
        return true;
    }

    if handle_main_ignore(state, input) {
        return true;
    }

    if handle_main_discard(state, input) {
        return true;
    }

    if handle_main_stage_toggle(state, input) {
        return true;
    }

    if handle_main_stage_line(state, input, max_y) {
        return true;
    }

    if handle_main_stage_all(state, input) {
        return true;
    }

    if handle_main_open_editor(state, input) {
        return true;
    }

    false
}

fn handle_main_quit(state: &mut AppState, input: &Input) -> bool {
    if matches!(input, Input::Character('q')) {
        if state.main_screen.is_diff_cursor_active {
            state.main_screen.is_diff_cursor_active = false;
        } else {
            let _ = commit_storage::save_commit_message(
                &state.repo_path,
                &state.main_screen.commit_message,
            );
            state.running = false;
        }
        return true;
    }
    false
}

fn handle_main_ignore(state: &mut AppState, input: &Input) -> bool {
    if !matches!(input, Input::Character('i')) {
        return false;
    }

    if let Some(file) = state.current_main_file().cloned() {
        if file.file_name != ".gitignore" {
            let command = Box::new(IgnoreFileCommand::new(
                state.repo_path.clone(),
                file.file_name.clone(),
            ));
            state.execute_and_refresh(command);
        }
    }

    true
}

fn handle_main_discard(state: &mut AppState, input: &Input) -> bool {
    if !matches!(input, Input::Character('!')) {
        return false;
    }

    if state.main_screen.is_diff_cursor_active {
        if let Some(file) = state.current_main_file() {
            let line_index = state.main_screen.line_cursor;
            if let Some(hunk) = git_patch::find_hunk(file, line_index) {
                let patch = git_patch::create_unstage_hunk_patch(file, hunk);
                let command = Box::new(DiscardHunkCommand::new(state.repo_path.clone(), patch));
                state.execute_and_refresh(command);
            }
        }
    } else if let Some(file) = state.current_main_file().cloned() {
        let is_new = file.status == FileStatus::Added;
        let command = Box::new(DiscardFileCommand::new(
            state.repo_path.clone(),
            file.file_name.clone(),
            is_new,
        ));
        state.execute_and_refresh(command);
    }

    true
}

fn handle_main_stage_toggle(state: &mut AppState, input: &Input) -> bool {
    if !is_stage_toggle(input) {
        return false;
    }

    match state
        .main_screen
        .list_items
        .get(state.main_screen.file_cursor)
        .cloned()
    {
        Some(ListItem::StagedChangesHeader) => {
            let command = Box::new(UnstageAllCommand::new(state.repo_path.clone()));
            state.execute_and_refresh(command);
        }
        Some(ListItem::File(file)) => {
            let line_index = state.main_screen.line_cursor;
            if let Some(hunk) = git_patch::find_hunk(&file, line_index) {
                let patch = git_patch::create_unstage_hunk_patch(&file, hunk);
                let command = Box::new(ApplyPatchCommand::new(state.repo_path.clone(), patch));
                state.execute_and_refresh(command);
            } else {
                let command = Box::new(UnstageFileCommand::new(
                    state.repo_path.clone(),
                    file.file_name.clone(),
                ));
                state.execute_and_refresh(command);
            }
        }
        Some(ListItem::PreviousCommitInfo {
            hash,
            message,
            is_on_remote,
            is_fixup: _,
        }) => {
            if state.main_screen.is_diff_cursor_active {
                if state.jump_to_file_in_diff() {
                    return true;
                }
            }
            if !is_on_remote {
                state.main_screen.amending_commit_hash = Some(hash.clone());

                let current_index = state.main_screen.file_cursor;
                if let Some(item) = state.main_screen.list_items.get_mut(current_index) {
                    *item = ListItem::AmendingCommitMessageInput {
                        hash: hash.clone(),
                        message: message.clone(),
                    };
                }

                if let Some(commit_input_index) = state
                    .main_screen
                    .list_items
                    .iter()
                    .position(|item| matches!(item, ListItem::CommitMessageInput))
                {
                    if let Some(item) = state.main_screen.list_items.get_mut(commit_input_index) {
                        if let ListItem::CommitMessageInput = item {
                            state.main_screen.commit_message.clear();
                        }
                    }
                }
                state.main_screen.commit_cursor = message.chars().count();
            }
        }
        _ => {}
    }

    true
}

fn handle_main_stage_line(state: &mut AppState, input: &Input, max_y: i32) -> bool {
    if !matches!(input, Input::Character('1')) {
        return false;
    }
    unstage_line(state, max_y);
    true
}

fn handle_main_stage_all(state: &mut AppState, input: &Input) -> bool {
    if matches!(input, Input::Character('R')) {
        let command = Box::new(StageAllCommand::new(state.repo_path.clone()));
        state.execute_and_refresh(command);
        return true;
    }
    false
}

fn handle_main_open_editor(state: &mut AppState, input: &Input) -> bool {
    if !matches!(input, Input::Character('e')) {
        return false;
    }

    if let Some(file) = state.current_main_file() {
        let line_number = if state.main_screen.is_diff_cursor_active {
            git_patch::get_line_number(file, state.main_screen.line_cursor)
        } else {
            None
        };
        let file_path = state.repo_path.join(&file.file_name);
        if let Some(path_str) = file_path.to_str() {
            state.editor_request = Some(EditorRequest {
                file_path: path_str.to_string(),
                line_number,
            });
        }
    }

    true
}

fn handle_reorder_mode_input(state: &mut AppState, input: Input, max_y: i32, max_x: i32) {
    if let Some(item) = state
        .main_screen
        .list_items
        .get_mut(state.main_screen.file_cursor)
    {
        if let ListItem::EditingReorderCommit {
            current_text,
            cursor,
            original_message,
            hash,
            is_on_remote,
            is_fixup,
            scroll_offset,
            scroll_extra_space,
        } = item
        {
            match input {
                Input::Character('\n') => {
                    // Enter
                    *item = ListItem::PreviousCommitInfo {
                        hash: hash.clone(),
                        message: current_text.clone(),
                        is_on_remote: *is_on_remote,
                        is_fixup: *is_fixup, // Preserve fixup status
                    };
                }
                Input::Character('\u{1b}') | Input::Character('\u{3}') => {
                    // Esc or Ctrl+C
                    *item = ListItem::PreviousCommitInfo {
                        hash: hash.clone(),
                        message: original_message.clone(),
                        is_on_remote: *is_on_remote,
                        is_fixup: *is_fixup, // Preserve fixup status
                    };
                }
                _ => {
                    commit_view::handle_generic_text_input(current_text, cursor, input);
                    let (offset, extra_space) = commit_view::compute_scroll_for_prefix(
                        current_text.as_str(),
                        *cursor,
                        max_x,
                        " ● ",
                    );
                    *scroll_offset = offset;
                    *scroll_extra_space = extra_space;
                }
            }
            return;
        }
    }

    match input {
        Input::Character('q') => {
            let current_hash = if let Some(item) = state
                .main_screen
                .list_items
                .get(state.main_screen.file_cursor)
            {
                match item {
                    ListItem::PreviousCommitInfo { hash, .. } => Some(hash.clone()),
                    ListItem::EditingReorderCommit { hash, .. } => Some(hash.clone()),
                    _ => None,
                }
            } else {
                None
            };

            state.main_screen.list_items =
                state.main_screen.original_list_items_for_reorder.clone();

            if let Some(hash) = current_hash {
                if let Some(pos) = state.main_screen.list_items.iter().position(|item| {
                    if let ListItem::PreviousCommitInfo { hash: h, .. } = item {
                        h == &hash
                    } else {
                        false
                    }
                }) {
                    state.main_screen.file_cursor = pos;
                }
            }

            state.main_screen.is_reordering_commits = false;
            state.reorder_command_history = None;
        }
        Input::Character('\n') => {
            // Enter
            let original_commits =
                get_commits_from_list(&state.main_screen.original_list_items_for_reorder);
            let reordered_commits = get_commits_from_list(&state.main_screen.list_items);

            if original_commits != reordered_commits {
                let command = Box::new(crate::command::ReorderCommitsCommand::new(
                    state.repo_path.clone(),
                    original_commits,
                    reordered_commits,
                ));
                state.execute_and_refresh(command);
            }
            state.main_screen.is_reordering_commits = false;
            state.reorder_command_history = None;
        }
        Input::KeyUp | Input::Character('\u{10}') => {
            state.main_screen.file_cursor = state.main_screen.file_cursor.saturating_sub(1);
            state.main_screen.diff_scroll = 0;
            state.main_screen.line_cursor = 0;
            state.debounce_diff_update();
        }
        Input::KeyDown | Input::Character('\u{e}') => {
            let item_count = state.main_screen.list_items.len();
            if item_count > 0 {
                state.main_screen.file_cursor = state
                    .main_screen
                    .file_cursor
                    .saturating_add(1)
                    .min(item_count - 1);
            }
            state.main_screen.diff_scroll = 0;
            state.main_screen.line_cursor = 0;
            state.debounce_diff_update();
        }
        Input::Character('f') => {
            let cursor = state.main_screen.file_cursor;
            if let Some(ListItem::PreviousCommitInfo { .. }) =
                state.main_screen.list_items.get(cursor)
            {
                let command = Box::new(crate::command::FixupCommitCommand::new(
                    &mut state.main_screen.list_items as *mut _,
                    cursor,
                ));
                state.execute_reorder_command(command);
            }
        }
        Input::Character('!') => {
            let cursor = state.main_screen.file_cursor;
            if let Some(ListItem::PreviousCommitInfo { .. }) =
                state.main_screen.list_items.get(cursor)
            {
                let command = Box::new(DiscardCommitCommand::new(
                    &mut state.main_screen.list_items,
                    cursor,
                ));
                state.execute_reorder_command(command);
                if cursor >= state.main_screen.list_items.len()
                    && !state.main_screen.list_items.is_empty()
                {
                    state.main_screen.file_cursor = state.main_screen.list_items.len() - 1;
                }
            }
        }
        Input::Character('<') => {
            let cursor_state = crate::cursor_state::CursorState::from_app_state(state);
            if let Some(history) = &mut state.reorder_command_history {
                if let Some(cursor) = history.undo(cursor_state) {
                    cursor.apply_to_app_state(state);
                }
            }
        }
        Input::Character('>') => {
            let cursor_state = crate::cursor_state::CursorState::from_app_state(state);
            if let Some(history) = &mut state.reorder_command_history {
                if let Some(cursor) = history.redo(cursor_state) {
                    cursor.apply_to_app_state(state);
                }
            }
        }
        _ => handle_navigation(state, input, max_y, max_x),
    }
}

fn get_commits_from_list(list: &[ListItem]) -> Vec<crate::git::CommitInfo> {
    list.iter()
        .filter_map(|item| match item {
            ListItem::PreviousCommitInfo {
                hash,
                message,
                is_on_remote,
                is_fixup,
            } => Some(crate::git::CommitInfo {
                hash: hash.clone(),
                message: message.clone(),
                is_on_remote: *is_on_remote,
                is_fixup: *is_fixup,
            }),
            ListItem::EditingReorderCommit {
                hash,
                current_text,
                is_on_remote,
                ..
            } => Some(crate::git::CommitInfo {
                hash: hash.clone(),
                message: current_text.clone(),
                is_on_remote: *is_on_remote,
                is_fixup: false, // Editing resets fixup status
            }),
            _ => None,
        })
        .collect()
}

pub fn start_reorder_mode(state: &mut AppState) {
    if state.main_screen.is_reordering_commits {
        return;
    }

    let current_cursor = state.main_screen.file_cursor;
    let current_item_hash = if let Some(item) = state.main_screen.list_items.get(current_cursor) {
        match item {
            ListItem::PreviousCommitInfo { hash, .. } => Some(hash.clone()),
            ListItem::EditingReorderCommit { hash, .. } => Some(hash.clone()),
            _ => None,
        }
    } else {
        None
    };

    state.main_screen.original_list_items_for_reorder = state.main_screen.list_items.clone();

    state.main_screen.list_items.retain(|item| {
        matches!(
            item,
            ListItem::PreviousCommitInfo { .. } | ListItem::EditingReorderCommit { .. }
        )
    });

    if let Some(hash) = current_item_hash {
        let new_cursor = state
            .main_screen
            .list_items
            .iter()
            .position(|item| match item {
                ListItem::PreviousCommitInfo { hash: h, .. } => h == &hash,
                ListItem::EditingReorderCommit { hash: h, .. } => h == &hash,
                _ => false,
            })
            .unwrap_or(0);
        state.main_screen.file_cursor = new_cursor;
    } else {
        state.main_screen.file_cursor = 0;
    }

    state.main_screen.is_reordering_commits = true;
    state.reorder_command_history = Some(CommandHistory::new());
}

fn handle_navigation(state: &mut AppState, input: Input, max_y: i32, max_x: i32) {
    if let Some(hash) = state.main_screen.amending_commit_hash.clone() {
        if let Some(index) = state
            .main_screen
            .list_items
            .iter()
            .position(|item| matches!(item, ListItem::AmendingCommitMessageInput { .. }))
        {
            if let Some(commit) = state.previous_commits.iter().find(|c| c.hash == hash) {
                state.main_screen.list_items[index] = ListItem::PreviousCommitInfo {
                    hash: commit.hash.clone(),
                    message: commit.message.clone(),
                    is_on_remote: commit.is_on_remote,
                    is_fixup: commit.is_fixup,
                };
            }
        }
        state.main_screen.amending_commit_hash = None;
    }

    if is_move_up(&input) {
        if handle_main_move_up(state, max_y) {
            return;
        }
        state.debounce_diff_update();
        return;
    }

    if is_move_down(&input) {
        handle_main_move_down(state, max_y);
        state.debounce_diff_update();
        return;
    }

    if is_diff_move_up(&input) {
        handle_main_diff_move_up(state);
        return;
    }

    if is_diff_move_down(&input) {
        handle_main_diff_move_down(state, max_y);
        return;
    }

    if is_horizontal_left(&input) {
        handle_main_horizontal_scroll_left(state, max_x);
        return;
    }

    if is_horizontal_right(&input) {
        handle_main_horizontal_scroll_right(state, max_x);
        return;
    }

    if matches!(
        state
            .main_screen
            .list_items
            .get(state.main_screen.file_cursor),
        Some(ListItem::CommitMessageInput)
    ) {
        commit_view::handle_commit_input(state, input, max_y, max_x);
    } else {
        scroll::handle_scroll(state, input, max_y);
    }
}

fn handle_main_move_up(state: &mut AppState, max_y: i32) -> bool {
    if state.main_screen.file_cursor == 0 && state.main_screen.has_unstaged_changes {
        let unstaged_items_count = state.unstaged_pane.list_items.len();
        if unstaged_items_count > 0 {
            state.focused_pane = FocusedPane::Unstaged;
            state.unstaged_pane.cursor = unstaged_items_count - 1;

            let (file_list_height, _) = state.unstaged_header_height(max_y);
            if state.unstaged_pane.cursor >= state.unstaged_pane.scroll + file_list_height {
                state.unstaged_pane.scroll = state.unstaged_pane.cursor - file_list_height + 1;
            }
            return true;
        }
    }

    state.main_screen.file_cursor = state.main_screen.file_cursor.saturating_sub(1);
    state.main_screen.diff_scroll = 0;
    state.main_screen.line_cursor = 0;
    state.main_screen.is_diff_cursor_active = false;

    if state.main_screen.file_cursor < state.main_screen.file_list_scroll {
        state.main_screen.file_list_scroll = state.main_screen.file_cursor;
    }
    false
}

fn handle_main_move_down(state: &mut AppState, max_y: i32) {
    if state.main_screen.file_cursor < state.main_screen.list_items.len().saturating_sub(1) {
        state.main_screen.file_cursor += 1;
        state.main_screen.diff_scroll = 0;
        state.main_screen.line_cursor = 0;
    }
    state.main_screen.is_diff_cursor_active = false;

    let file_list_height = state.main_header_height(max_y).0;

    if state.main_screen.file_cursor >= state.main_screen.file_list_scroll + file_list_height {
        state.main_screen.file_list_scroll = state.main_screen.file_cursor - file_list_height + 1;
    }
}

fn handle_main_diff_move_up(state: &mut AppState) {
    state.main_screen.is_diff_cursor_active = true;
    state.main_screen.line_cursor = state.main_screen.line_cursor.saturating_sub(1);
    let cursor_line = state.get_cursor_line_index();
    if cursor_line < state.main_screen.diff_scroll {
        state.main_screen.diff_scroll = cursor_line;
    }
}

fn handle_main_diff_move_down(state: &mut AppState, max_y: i32) {
    state.main_screen.is_diff_cursor_active = true;
    let lines_count = match state
        .main_screen
        .list_items
        .get(state.main_screen.file_cursor)
    {
        Some(ListItem::File(file)) => file.lines.len(),
        Some(ListItem::PreviousCommitInfo { .. }) => state
            .selected_commit_files
            .iter()
            .map(|f| f.lines.len())
            .sum(),
        _ => 0,
    };

    if lines_count > 0 && state.main_screen.line_cursor < lines_count.saturating_sub(1) {
        state.main_screen.line_cursor += 1;

        let mut main_pane_offset = 0;
        if state.main_screen.has_unstaged_changes {
            main_pane_offset = state.unstaged_header_height(max_y).0 + 1;
        }
        let main_pane_height = state.main_header_height(max_y).0;
        let diff_view_top = main_pane_offset + main_pane_height;
        let content_height = (max_y as usize).saturating_sub(diff_view_top);

        let cursor_line = state.get_cursor_line_index();

        if cursor_line >= state.main_screen.diff_scroll + content_height {
            state.main_screen.diff_scroll = cursor_line - content_height + 1;
        }
    }
}

fn handle_main_horizontal_scroll_left(state: &mut AppState, max_x: i32) {
    let scroll_amount = (max_x as usize).saturating_sub(LINE_CONTENT_OFFSET);
    state.main_screen.horizontal_scroll = state
        .main_screen
        .horizontal_scroll
        .saturating_sub(scroll_amount);
}

fn handle_main_horizontal_scroll_right(state: &mut AppState, max_x: i32) {
    let scroll_amount = (max_x as usize).saturating_sub(LINE_CONTENT_OFFSET);
    state.main_screen.horizontal_scroll = state
        .main_screen
        .horizontal_scroll
        .saturating_add(scroll_amount);
}
