# Application Specification: Commit Log View

This document specifies the behavior of the Commit Log View, which is part of the scrollable list on the Main Screen.

## 1. Visual Representation

- Each commit in the log is displayed as a single-line entry.
- The entry consists of a status indicator followed by the commit message.
- If the commit message is too long to fit within the window width, it will be truncated, not wrapped.

### 1.1. Status Indicator

- The status indicator is a `●` character, and its color signifies the commit's state.
- The coloring logic is as follows:

| State                                 | Foreground Color |
| ------------------------------------- | ---------------- |
| **Selected** and **on remote**        | Bright Cyan (Blue) |
| **Selected** and **local only**       | Bright Green       |
| Not selected and **on remote**        | Cyan (Blue)        |
| Not selected and **local only**       | Green              |

- A commit is considered "on remote" if it has been pushed to a remote branch.

### 1.2. Highlighting

- When a commit log entry is selected with the cursor, its entire line is highlighted with a different background color to indicate focus.

## 2. Navigation

- Standard `Up Arrow` and `Down Arrow` keys are used to navigate up and down the list of commits.
- As the selection changes, the Diff View in the bottom panel updates to show the diff corresponding to the newly selected commit.

## 3. Interaction and Commands

### 3.1. Amending/Rewording a Commit

- **Trigger:**
  - Pressing the `Enter` key while a **local only** commit is selected.
  - This action is ignored if the selected commit is already on a remote.

- **Outcome:**
  1.  The standard **Commit Message Input** field moves to the position of the selected commit, replacing it in the list.
  2.  The input field is pre-populated with the message from the commit being amended.
  3.  The cursor is placed at the end of the commit message, ready for editing.
  4.  The user is now in "amend mode," and all interactions are handled by the commit input view. (See `spec/commit_input_view.md` for details on editing).

- **Canceling Amend Mode:**
  - Navigating away from the input field using the `Up` or `Down` arrow keys cancels the operation.
  - The input field disappears, and the original commit log entry is restored in its place.

### 3.2. Reordering Commits

- **Trigger:**
  - Pressing `Meta + Up Arrow` or `Meta + Down Arrow` while a **local only** commit is selected.
  - This action is ignored if the selected commit is already on a remote.

- **Outcome:**
  1.  The application enters "Commit Reordering Mode."
  2.  The selected commit is swapped with its adjacent commit in the direction of the arrow key.
  3.  In this mode, the Unstaged and Staged panes are hidden, and the Commit Log View expands to fill the screen.
  4.  A header is displayed at the top, indicating that the user is in reordering mode and showing the available commands.

- **Reordering Mode Commands:**
  - `Up/Down Arrow`: Swaps the currently selected commit with its neighbor.
  - `!`: Discards the currently selected commit. This is a visual change only; the commit is not actually discarded until the reordering is confirmed.
  - `<`: Undoes the last action (swap or discard).
  - `>`: Redoes the last undone action.
  - `Enter`: Confirms the new commit order and exits reordering mode. The application will then perform a safe rebase operation in the background. If any conflicts are detected, the operation is aborted, and the commit order remains unchanged.
  - `Esc` or `q`: Cancels the reordering, reverts the commit list to its original order, and exits reordering mode.