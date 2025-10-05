# Application Specification: Commit Message Input View

This document specifies the behavior of the Commit Message Input View, as found on the Main Screen.

## 1. Visual Representation

- The input field is rendered as a single line of text.
- It is prefixed with a circle character: ` ○ `.
- When the input field is empty, it displays placeholder text.
  - **Default Placeholder:** `Enter commit message...`
  - **Amend Mode Placeholder:** `Enter amend message...`
- The placeholder text is rendered with a distinct, dimmed color.
- When the input field is selected, its background color changes to highlight it.
- A block cursor is visible at the current text insertion point. The cursor is only shown when the view is active for editing.

## 2. Interaction and Keybindings

The following keybindings are active when the Commit Message Input view is selected.

| Key(s)                                | Action                                            |
| ------------------------------------- | ------------------------------------------------- |
| Any printable character               | Inserts the character at the cursor position.     |
| `Enter`                               | Finalizes the commit.                             |
|                                       | - If the message is empty, the action is ignored. |
|                                       | - For a normal commit, `git commit` is executed.  |
|                                       | - For an amend, `git commit --amend` is executed. |
| `Backspace`, `Ctrl-H`, (`\x08`, `\x7f`) | Deletes the character immediately before the cursor. |
| `Delete` (`Del`)                      | Deletes the character at the cursor position.     |
| `Left Arrow`                          | Moves the cursor one character to the left.       |
| `Right Arrow`                         | Moves the cursor one character to the right.      |
| `Ctrl-A`                              | Moves the cursor to the beginning of the line.    |
| `Ctrl-E`                              | Moves the cursor to the end of the line.          |
| `Ctrl-K`                              | Deletes all text from the cursor to the end of the line. |
| `Up Arrow`, `Down Arrow`              | Moves selection out of the input field, deactivating the text cursor and committing the user to list navigation mode. |

## 3. State Management

### 3.1. Normal Commit

- The text entered into the commit message field is persisted to a file within the `.git` directory (`.git/COMMIT_EDITMSG.bk`) upon every modification.
- This ensures that a draft commit message is not lost if the application is closed unexpectedly.
- After a successful commit, this backup file is deleted.

### 3.2. Amending a Commit

- When amending a commit (see `commit_log_view.md`), the input field is pre-populated with the message from the commit being amended.
- The placeholder text changes to "Enter amend message...".
- Unlike a normal commit, the message is **not** persisted to a backup file during editing. It is only used when the `Enter` key is pressed to finalize the amend operation.
- Finalizing the amend can result in either a `git reword` (if no files are staged) or a `git commit --amend` (if files are staged).