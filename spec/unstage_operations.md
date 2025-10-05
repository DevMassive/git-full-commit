# Application Specification: Unstage Operations

This document describes the application's functionality for unstaging changes from a user's perspective. "Unstaging" refers to the process of removing changes from the Git index (the staging area) and moving them back into the working directory.

## 1. General Context

All unstage operations are initiated from the **Main Screen**, which displays the "Staged changes" list. The user navigates this list using the `Up` and `Down` arrow keys. The selected item is highlighted.

A user can switch to the "Unstaged changes" screen by pressing `Tab` at any time, provided there is at least one unstaged change or one untracked file.

## 2. Unstage Granularities

The application supports four levels of granularity for unstaging changes.

### 2.1. Unstage All Changes

- **User Action:**
  1. Navigate to the **Main Screen**.
  2. Select the **"Staged changes"** header at the top of the list.
  3. Press the `u` key or the `Enter` key.
- **Expected Outcome:**
  - All files and changes currently in the staging area are unstaged.
  - Staged additions (`A`) become untracked files (`??`).
  - Staged modifications (`M`) become unstaged modifications (` M`).
  - The "Staged changes" list becomes empty.
- **Cursor Movement:**
  - The cursor remains on the **"Staged changes"** header.

### 2.2. Unstage an Entire File

- **User Action:**
  1. Navigate to the **Main Screen**.
  2. Select a specific file from the "Staged changes" list (e.g., `M  modified_file.txt`).
  3. Ensure the cursor focus is on the file list (not the diff view below).
  4. Press the `u` key or the `Enter` key.
- **Expected Outcome:**
  - All staged changes for the selected file are unstaged.
  - The file is removed from the "Staged changes" list.
- **Cursor Movement:**
  - The cursor moves to the next item in the list.
  - If the unstaged file was the last one in the list, the cursor moves to the "Commit Message" input field below the list.

### 2.3. Unstage a Hunk

A "hunk" is a contiguous block of changes within a file, as determined by Git.

- **User Action:**
  1. Navigate to the **Main Screen** and select a file with changes.
  2. Move the cursor focus into the diff panel below by pressing `j` (down) or `k` (up).
  3. Navigate the cursor to any line within the hunk you wish to unstage.
  4. Press the `u` key or the `Enter` key.
- **Expected Outcome:**
  - Only the selected hunk of changes is removed from the index and moved to the working directory.
  - The file remains in the "Staged changes" list, but the diff view updates to show that the selected hunk is no longer staged.
- **Cursor Movement:**
  - The file selection cursor remains on the same file.
  - The line cursor in the diff view attempts to stay at the same numerical position, but will move up if the lines it was on were removed.

### 2.4. Unstage a Single Line

- **User Action:**
  1. Navigate to the **Main Screen** and select a file with changes.
  2. Move the cursor focus into the diff panel below by pressing `j` (down) or `k` (up).
  3. Navigate the cursor to the specific line (an addition or modification) you wish to unstage.
  4. Press the `1` key.
- **Expected Outcome:**
  - Only the single selected line is removed from the index and moved to the working directory.
  - The file remains staged, but the diff view is updated to reflect the change.
- **Cursor Movement:**
  - The file selection cursor remains on the same file.
  - The line cursor in the diff view will attempt to stay at the same line index, but will move to the new last line of the diff if the line it was on was the last one and is now removed.