# Application Specification: Diff View

This document specifies the behavior of the diff view, which displays the differences between file versions. The behavior described here is based on a direct analysis of the application's source code.

## 1. General Context

The diff view is a component displayed on the **Unstaged Screen** and **Main Screen**. It appears when a file or commit with changes is selected, showing the specific modifications.

## 2. Visual Elements

The diff view contains several key visual elements to help users understand the changes.

### 2.1. Line Number Display

- A dedicated area on the left of the diff panel displays line numbers in a two-column format, similar to `git diff`.
- The format is `old_line_num new_line_num`, with each number padded to occupy 4 characters.
- **Added Lines (`+`):** Only the `new_line_num` is displayed. The old line number column is blank.
- **Removed Lines (`-`):** Only the `old_line_num` is displayed. The new line number column is blank.
- **Context Lines (` `):** Both `old_line_num` and `new_line_num` are displayed.

### 2.2. Word-Level Highlighting

- When a line has been modified, the application highlights the specific words that have changed.
- **Highlighting Method:** Changed characters or words within a modified line are rendered with a reverse-video effect (foreground and background colors are swapped), making them stand out from the rest of the line.

## 3. Scrolling the Diff View

The diff view can be scrolled vertically and horizontally to inspect all changes in a file.

### 3.1. Line-by-Line Scrolling (Vertical)

- **User Action (Scroll Down):** Press the `j` key.
- **Expected Outcome:** The line cursor moves down by one line. The view scrolls if necessary to keep the cursor visible.

- **User Action (Scroll Up):** Press the `k` key.
- **Expected Outcome:** The line cursor moves up by one line. The view scrolls if necessary.

### 3.2. Horizontal Scrolling

The scroll amount for horizontal movement depends on the current screen.

- **User Action:** Press the `Left` or `Right` arrow key.
- **Expected Outcome:**
  - **On the Main Screen:** The view scrolls horizontally by a dynamic amount calculated based on the terminal width (specifically, `terminal_width - 10` characters). This allows for rapid movement across wide lines.
  - **On the Unstaged Screen:** The view scrolls horizontally by a fixed amount of 10 characters per key press.

### 3.3. Page Scrolling (Vertical)

- **User Action (Page Down):** Press the `space` bar or `Ctrl+V`.
- **Expected Outcome:** The view and the line cursor scroll down by a "full page" (the height of the diff panel).

- **User Action (Page Up):** Press the `b` key or `Ctrl+B`.
- **Expected Outcome:** The view and the line cursor scroll up by one full page.

### 3.4. Half-Page Scrolling (Vertical)

- **User Action (Half Page Down):** Press `Ctrl+D`.
- **Expected Outcome:** The view and the line cursor scroll down by a "half page" (half the height of the diff panel).

- **User Action (Half Page Up):** Press `Ctrl+U`.
- **Expected Outcome:** The view and the line cursor scroll up by one half page.

## 4. Detailed Scrolling Mechanics

- **Cursor and View are Linked:** Page and half-page scroll actions modify both the `line_cursor` and the `scroll` offset to create a cohesive scrolling experience.
- **Boundary Conditions:** The cursor position is always clamped to stay within the bounds of the diff content.
- **Scrolling Logic:** The `scroll` offset is automatically adjusted to ensure the `line_cursor` remains visible within the viewport after a scroll action.