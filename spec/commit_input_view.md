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
| `KeyBackspace`, `\x7f`, `\x08`        | Deletes the character immediately before the cursor. |
| `KeyDC` (`Delete`)                    | Deletes the character at the cursor position.     |
| `KeyLeft`                             | Moves the cursor one character to the left.       |
| `KeyRight`                            | Moves the cursor one character to the right.      |
| `Ctrl-A` (`\u{1}`)                     | Moves the cursor to the beginning of the line.    |
| `Ctrl-E` (`\u{5}`)                     | Moves the cursor to the end of the line.          |
| `Ctrl-K` (`\u{b}`)                     | Deletes all text from the cursor to the end of the line. |
| `Up Arrow`, `Down Arrow`              | Moves selection out of the input field, deactivating the text cursor and committing the user to list navigation mode. |
| `Meta-Left`                           | Moves the cursor to the beginning of the previous word. |
| `Meta-Right`                          | Moves the cursor to the beginning of the next word. |
| `Meta-Backspace`                      | Deletes the word immediately before the cursor.   |

## 3. State and Workflow

### 3.1. Draft Message Persistence

- **Storage Location:** For normal (non-amend) commits, the draft message is automatically saved on every modification. It is stored in a central application directory (`~/.git-reset-pp/`), with a unique filename generated from a hash of the repository's path. This prevents losing work if the application closes unexpectedly.
- **Cleanup:** This saved draft is deleted after a successful commit.
- **Amend Mode:** When amending, the message is held in memory but is **not** persisted to the file system until the operation is finalized.

### 3.2. Finalizing a Commit

- **Normal Commit:** Pressing `Enter` with a non-empty message executes `git commit`.
- **Amending a Commit:** The behavior depends on whether there are staged changes:
  - **No Staged Changes:** `git reword` is used to change only the commit message.
  - **With Staged Changes:** `git commit --amend` is used to include the staged changes in the amended commit.

### 3.3. Post-Commit Workflow

- After any successful commit (normal or amend), the following actions occur automatically:
  1. The application's undo/redo history is cleared.
  2. The application executes `git add -A` to stage all remaining unstaged changes.
  3. The application checks the repository status:
     - If there are no more staged changes, the application exits.
     - If staged changes remain, the screen is refreshed to show the new state, with the cursor moved to the top of the list.