# Application Specification: Stage Operations

This document describes the application's functionality for staging changes, moving them from the working directory to the Git index.

## 1. General Context

All staging operations are initiated from the **Top Pane of the Main Screen**, which displays "Unstaged changes" and "Untracked files." The user navigates these lists using the `Up` and `Down` arrow keys when the pane is focused.

A user can switch focus to the **Bottom Pane** by pressing `Tab`. See `spec/pane_switching.md`.

## 2. Staging from the Top Pane

The application supports staging at four levels of granularity.

### 2.1. Stage All Unstaged Changes

- **User Action:**
  1. Select the **"Unstaged changes"** header in the Top Pane.
  3. Press the `u` key or the `Enter` key.
- **Expected Outcome:**
  - All modified files in the "Unstaged changes" list are staged.
  - The "Unstaged changes" list becomes empty.
- **Cursor Movement:**
  - The cursor remains on the **"Unstaged changes"** header.

### 2.2. Stage All Untracked Files

- **User Action:**
  1. Select the **"Untracked files"** header in the Top Pane.
  3. Press the `u` key or the `Enter` key.
- **Expected Outcome:**
  - All files in the "Untracked files" list are staged.
  - The "Untracked files" list becomes empty.
- **Cursor Movement:**
  - The cursor remains on the **"Untracked files"** header.

### 2.3. Stage an Entire File

- **Condition:** The diff cursor is **inactive**.
- **User Action:**
  1. Select a file from the "Unstaged changes" or "Untracked files" list in the Top Pane.
  2. Press the `u` key or the `Enter` key.
- **Expected Outcome:**
  - The selected file is staged and removed from its list.
- **Cursor Movement:**
  - The cursor moves to the next item in the list. If the staged file was the last one in its section, the cursor moves to the section header above it.

### 2.4. Stage a Hunk

- **Condition:** The diff cursor is **active**.
- **User Action:**
  1. Select a file and activate the diff cursor (`j` or `k`).
  2. Navigate to any line within the hunk to be staged.
  3. Press the `u` key or the `Enter` key.
- **Expected Outcome:**
  - The selected hunk is staged.
  - The diff view updates to show the hunk is no longer unstaged.
  - The file remains in the "Unstaged changes" list if it has other unstaged changes.
- **Cursor Movement:**
  - The file selection cursor remains on the same file.
  - The line cursor in the diff view attempts to stay at the same numerical position but will move up if the lines it was on were removed.

### 2.5. Stage a Single Line

- **User Action:**
  1. In the Top Pane, select a file.
  2. Move focus to the diff panel.
  3. Navigate to the specific line to be staged.
  4. Press the `1` key.
- **Expected Outcome:**
  - The selected line is staged.
  - The diff view updates to reflect the change.
  - The file remains in the "Unstaged changes" list.
- **Cursor Movement:**
  - The file selection cursor remains on the same file.
  - The line cursor in the diff view will attempt to stay at the same line index.

## 3. Staging All from Main Screen

As a shortcut, it is also possible to stage all unstaged and untracked files directly from the main screen.

- **User Action:**
  1. Navigate to the **Main Screen**.
  2. Press the `R` key.
- **Expected Outcome:**
  - All unstaged and untracked files are staged.
  - The corresponding items appear in the "Staged changes" list.
- **Cursor Movement:**
  - The cursor position on the Main Screen is not explicitly changed. The list of staged files is updated, which may cause the item under the cursor to change.