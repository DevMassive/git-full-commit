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
  5.  When the amended message is confirmed, only the selected commit is rewritten—the newest commit (HEAD) and any other commits retain their original messages.

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
  3.  In this mode, the Unstaged and Staged panes are hidden. The screen consists of two panes:
      1.  **Commit List:** A list of local commits that can be reordered.
      2.  **Diff View:** Shows the diff for the currently selected commit.
  4.  The commit list has a maximum height of one-third of the terminal height.
  5.  A header is displayed at the top, indicating that the user is in reordering mode and showing the available commands.

- **Reordering Mode Commands:**
  - `Up/Down Arrow`: Moves the selection in the commit list.
  - `Alt+Up/Down Arrow`: Swaps the currently selected commit with its neighbor.
  - `j`/`k`, page up/down, etc.: Navigates the diff view.
  - `f`: Toggles the "fixup" status of the selected commit. When a commit is marked as a fixup, its message will be visually replaced with "fixup!". Upon execution, this commit will be squashed into its preceding commit, and its message will be discarded. Pressing `f` again will revert it to a normal commit.
  - `!`: Discards the currently selected commit. This is a visual change only; the commit is not actually discarded until the reordering is confirmed.
  - `<`: Undoes the last action (swap or discard).
  - `>`: Redoes the last undone action.
  - `Enter`: Confirms the new commit order and exits reordering mode. The application will then perform a safe rebase operation in the background. If any conflicts are detected, the operation is aborted, and the commit order remains unchanged.
  - `Esc` or `q`: Cancels the reordering, reverts the commit list to its original order, and exits reordering mode.

### 3.3. Editing Commit Messages While Reordering

- Pressing `Alt+Enter` (`Meta+Enter`) while a local commit is selected replaces the row with an inline editor for that commit message.
- The editor uses the same single-line scrolling behaviour as the commit input field:
  - Once the cursor reaches `available_width - 5` cells (after the ` ● ` prefix), the view scrolls forward so the cursor remains at `available_width - 4`.
  - The first visible glyph becomes `…` or `… ` to indicate hidden content; the extra space is used when required to keep the glyph grid aligned around double-width characters.
  - Moving the cursor back toward the left edge scrolls the view backwards, stopping at the beginning of the string if further scrolling would go negative.
