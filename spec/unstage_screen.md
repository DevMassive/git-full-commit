# Application Specification: Unstaged Screen

This document specifies the layout, content, and behavior of the application's Unstaged Screen, based on a direct analysis of the source code.

## 1. General Context

The Unstaged Screen is where users view and manage changes in the working directory that have not yet been staged. This includes modified files and new, untracked files.

## 2. Screen Layout and Content

The screen is composed of two primary panels separated by a horizontal line:

### 2.1. Top Panel: File List

- Displays two groups of files:
  1.  **Unstaged Changes:** A list of files that have been modified but not yet staged.
  2.  **Untracked Files:** A list of files that are new to the repository.

### 2.2. Bottom Panel: Diff View

- The content of this view is dynamic and depends on the item selected in the Top Panel.
- **For Modified Files:** The Diff View shows the unstaged changes for the selected file.
- **For Untracked Files:**
  - The Diff View shows the full content of the new file.
  - If the file is detected as a binary file, the Diff View instead displays a message indicating this (e.g., "Binary file (size: ... bytes)").
- *Note: All specific interactions within the Diff View (scrolling, highlighting, etc.) are detailed in `spec/diff_view.md`.*

## 3. Navigation and Command Model

Navigation and command execution on this screen are governed by a "Diff Cursor State," which determines whether actions apply to the selected file as a whole or to a specific part of its diff.

### 3.1. File List Navigation (Diff Cursor Inactive)

- **User Action:**
  - Press the `Up` or `Down` arrow key.
- **Expected Outcome:**
  - The cursor moves between items in the File List (top panel).
  - **The Diff Cursor state is set to INACTIVE.** This means any subsequent commands will target the entire selected file.
  - **The Diff View is reset.** When the file selection is changed via the arrow keys, the Diff View's scroll position and line cursor are reset to 0.

### 3.2. Diff View Navigation (Diff Cursor Active)

- **User Action:**
  - Press the `j` or `k` key.
- **Expected Outcome:**
  - **The Diff Cursor state is set to ACTIVE.** This means any subsequent commands (like staging or discarding) will target the specific line or hunk currently selected by the cursor within the Diff View (bottom panel).
  - The cursor moves line-by-line within the Diff View.

### 3.3. Command Logic: File vs. Hunk

The target of commands like staging (`u`) or discarding (`!`) depends entirely on the Diff Cursor State.

- **When Diff Cursor is INACTIVE:** The command applies to the **entire file** selected in the list.
- **When Diff Cursor is ACTIVE:** The command applies to the **hunk** currently selected by the cursor in the Diff View.

## 4. Screen Switching and Cursor Position

- **Switching to Unstaged Screen:** When the user presses `Tab` on the Main Screen, the application switches to the Unstaged Screen.
- **Cursor Behavior:** The application attempts to maintain context:
  - If the cursor on the Main Screen was on a file that also has unstaged changes, the cursor on the Unstaged Screen will be positioned on that same file.
  - Otherwise, the cursor defaults to the first item in the list.
- **Initial State:** The Diff Cursor state is initially INACTIVE upon switching to this screen.

## 5. Operations

The following operations can be performed from the Unstaged Screen:

- **Staging:** See `spec/stage_operations.md`
- **Discarding:** See `spec/discard_operations.md`
- **Ignoring:** See `spec/ignore_operations.md`