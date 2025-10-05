# Application Specification: Main Screen

This document specifies the layout, content, and behavior of the application's Main Screen, based on a direct analysis of the source code.

## 1. General Context

The Main Screen is the initial view when the application starts. It provides a summary of the repository's status, allows for new commits to be made, and displays diffs for both staged changes and previous commits.

## 2. Screen Layout and Content

The Main Screen is composed of two primary panels separated by a horizontal line:

### 2.1. Top Panel: List View

The top panel contains a single, continuous list of navigable items.

- **Layout and Sizing:**
  - The panel's height is dynamic, calculated as one-third of the terminal's height (min 3 lines).
  - If the content exceeds this calculated height, the panel becomes vertically scrollable.

- **Content Order:**
  1.  **Staged Changes:** A header followed by a list of staged files.
  2.  **Commit Message Input:** A text input field. (*Details in `spec/commit_input_view.md`*)
  3.  **Commit Log:** A list of commits. (*Details in `spec/commit_log_view.md`*)

### 2.2. Bottom Panel: Diff View

- The content of this view is dynamic, showing the diff for the selected staged file or commit.
- *Note: All interactions within the Diff View are detailed in `spec/diff_view.md`.*

## 3. Navigation and Command Model

Navigation and command execution are governed by a "Diff Cursor State," which determines whether actions apply to a selected file as a whole or to a specific part of its diff. This state is only relevant when a staged file is selected.

### 3.1. List Navigation (Diff Cursor Inactive)

- **User Action:**
  - Press the `Up` or `Down` arrow key.
- **Expected Outcome:**
  - The cursor moves between items in the List View (top panel).
  - **The Diff Cursor state is set to INACTIVE.** Any subsequent commands will target the entire selected file or commit.
  - **The Diff View is reset.** When the file selection is changed via the arrow keys, the Diff View's scroll position and line cursor are reset to 0.

### 3.2. Diff View Navigation (Diff Cursor Active)

- **User Action:**
  - When a staged file is selected, press the `j` or `k` key.
- **Expected Outcome:**
  - **The Diff Cursor state is set to ACTIVE.** Any subsequent commands (like unstaging) will target the specific line or hunk currently selected by the cursor within the Diff View.
  - The cursor moves line-by-line within the Diff View.

### 3.3. Command Execution Example (Unstaging)

- **When Diff Cursor is INACTIVE:** Pressing `u` (unstage) on a selected file will unstage the *entire file*.
- **When Diff Cursor is ACTIVE:** Pressing `u` (unstage) will unstage only the *hunk* currently under the cursor in the Diff View.

## 4. Initial State

- When the application starts, the Main Screen is displayed.
- The cursor is positioned at the top of the list in the Top Panel.
- The Diff Cursor state is initially INACTIVE.