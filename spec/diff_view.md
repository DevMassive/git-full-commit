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
  - **On the Main Screen and Unstaged Screen:** The view scrolls horizontally by a dynamic amount calculated based on the terminal width (specifically, `terminal_width - 10` characters). This allows for rapid movement across wide lines.

### 3.3. Page Scrolling (Vertical)

Page scrolling follows a two-step logic: first the cursor moves, then the view scrolls only if necessary.

- **User Action (Page Down):** Press the `space` bar or `Ctrl+V`.
- **Expected Outcome:**
  1.  The line cursor moves down by one page (the height of the diff view), but does not exceed the last line of the content.
  2.  If the new cursor position is below the visible area of the view, the view scrolls down by exactly one page. This can result in blank lines being shown at the bottom if scrolling near the end of the content.

- **User Action (Page Up):** Press the `b` key or `Ctrl+B`.
- **Expected Outcome:**
  1.  The line cursor moves up by one page.
  2.  If the new cursor position is above the visible area of the view, the view scrolls up by exactly one page. The view will not scroll past the beginning of the content (no blank lines are shown at the top).

### 3.4. Half-Page Scrolling (Vertical)



Half-page scrolling follows the same two-step logic as full-page scrolling, but with half the page height.



- **User Action (Half Page Down):** Press `Ctrl+D`.

- **Expected Outcome:** The cursor moves down by half a page, and the view scrolls by half a page if the cursor moves off-screen.



- **User Action (Half Page Up):** Press `Ctrl+U`.

- **Expected Outcome:** The cursor moves up by half a page, and the view scrolls by half a page if the cursor moves off-screen.



## 4. Navigation from Stat Summary



When viewing a commit's diff, the header often includes a stat summary (a list of changed files with their respective change counts).



- **User Action:** Navigate the diff cursor (using `j`/`k`) to a line in the stat summary and press `Enter`.

- **Expected Outcome:** The diff view scrolls to the beginning of the patch for the corresponding file. The cursor is moved to the `diff --git` line of that file.
