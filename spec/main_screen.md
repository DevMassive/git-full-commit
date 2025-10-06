# Application Specification: Main Screen

This document specifies the layout, content, and behavior of the application's Main Screen.

## 1. General Context

The Main Screen is the initial and primary view of the application. It is a composite screen divided into multiple panes, providing a comprehensive overview of the repository's status and enabling core Git operations.

## 2. Screen Layout and Content

The Main Screen is composed of three primary sections, arranged vertically:

1.  **Top Pane: Unstaged & Untracked Files**
2.  **Bottom Pane: Staged Files, Commit Input & Log**
3.  **Diff View**

### 2.1. Top Pane: Unstaged & Untracked Files

This pane displays files with changes in the working directory that have not yet been staged.

- **Visibility:**
  - This pane is only displayed if there is at least one unstaged change or one untracked file. Otherwise, it is hidden entirely.

- **Layout and Sizing:**
  - The pane's height is dynamic, with a maximum size of one-third of the terminal's height.
  - If the content exceeds this height, the pane becomes vertically scrollable.
- **Content:**
  - It contains two sections, each with a header:
    1.  **Unstaged changes:** A list of modified files.
    2.  **Untracked files:** A list of new files not yet tracked by Git.

### 2.2. Bottom Pane: Staged Files, Commit Input & Log

This pane displays staged changes and the commit history.

- **Content Order:**
  1.  **Staged Changes:** A header followed by a list of staged files.
  2.  **Commit Message Input:** A text input field. (*Details in `spec/commit_input_view.md`*)
  3.  **Commit Log:** A list of commits. (*Details in `spec/commit_log_view.md`*)

### 2.3. Diff View

- This view always occupies the bottom-most portion of the screen, below the other panes.
- Its content is dynamic, showing the diff for the item selected in the currently active pane (either Top or Bottom).
- *Note: All interactions within the Diff View are detailed in `spec/diff_view.md`.*

## 3. Navigation and Command Model

Navigation is split between the two main panes (Top and Bottom). The `Tab` key switches focus between them.

- **Pane Switching:** See `spec/pane_switching.md`.
- **Cursor:** The `Up` and `Down` arrow keys move the cursor within the currently focused pane. The cursor is hidden in the inactive pane.

### 3.1. Operations in the Top Pane (Unstaged/Untracked)

When the Top Pane is focused, the user can perform the following operations:

- **Staging:** See `spec/stage_operations.md`
- **Discarding:** See `spec/discard_operations.md`
- **Ignoring:** See `spec/ignore_operations.md`

### 3.2. Operations in the Bottom Pane (Staged/Commit)

When the Bottom Pane is focused, the user can perform the following operations:

- **Unstaging:** See `spec/unstage_operations.md`
- **Discarding:** See `spec/discard_operations.md`
- **Committing:** See `spec/commit_input_view.md` and `spec/commit_log_view.md`

### 3.3. Diff Interaction

- The `j` and `k` keys are used to activate and move the cursor within the Diff View, regardless of which pane is focused. This allows for hunk-level operations.
- The target of commands like staging (`u`) or discarding (`!`) depends on whether the diff cursor is active.

## 4. Initial State

- When the application starts, the Main Screen is displayed.
- Focus is initially on the **Bottom Pane** (Staged/Commit), regardless of whether the Top Pane is visible.
- The cursor is positioned on the first item in the Bottom Pane.
