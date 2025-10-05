# Application Specification: Discard Operations

This document describes the application's functionality for discarding changes. "Discarding" refers to permanently deleting changes from either the staging area or the working directory.

## 1. General Context

Discard operations can be initiated from both the **Main Screen** (for staged items) and the **Unstaged Screen** (for unstaged or untracked items).

## 2. Discarding from the Main Screen (Staged Changes)

These operations discard changes that have been staged.

- **Safety Check:** A discard operation on a staged file will be blocked if the same file also has unstaged changes. This prevents accidental loss of unstaged work.

### 2.1. Discard an Entire File

- **User Action:**
  1. Navigate to the **Main Screen**.
  2. Select a specific file from the "Staged changes" list.
  3. Ensure cursor focus is on the file list (not the diff view).
  4. Press the `!` key.
- **Expected Outcome:**
  - **For a modified file:** All staged changes are reverted, and the file is removed from the "Staged changes" list. Its contents are restored to the state of the last commit.
  - **For a new file (Added):** The file is unstaged and deleted from the filesystem.
  - The file is removed from the "Staged changes" list.
- **Cursor Movement:**
  - The cursor moves to the next item in the list.
  - If the discarded file was the last one in the list, the cursor moves to the "Commit Message" input field below the list.

### 2.2. Discard a Hunk

- **User Action:**
  1. Navigate to the **Main Screen** and select a file.
  2. Move focus into the diff panel by pressing `j` (down) or `k` (up).
  3. Navigate to any line within the hunk to be discarded.
  4. Press the `!` key.
- **Expected Outcome:**
  - The selected hunk is reverted from both the staging area and the working directory.
  - The diff view updates to show the hunk has been removed.
  - The file remains in the "Staged changes" list if it has other staged changes.
- **Cursor Movement:**
  - The file selection cursor remains on the same file.
  - The line cursor in the diff view attempts to stay at the same numerical position but will move up if the lines it was on were removed.

## 3. Discarding from the Unstaged Screen

These operations discard changes from the working directory.

### 3.1. Discard Unstaged Changes to a File

- **User Action:**
  1. Navigate to the **Unstaged Screen**.
  2. Select a file from the "Unstaged changes" list.
  3. Ensure cursor focus is on the file list.
  4. Press the `!` key.
- **Expected Outcome:**
  - All unstaged changes for the selected file are reverted.
  - The file is removed from the "Unstaged changes" list.
- **Cursor Movement:**
  - The cursor moves to the next item in the list. If the discarded file was the last one in the list, the cursor moves to the section header.

### 3.2. Discard an Unstaged Hunk

- **User Action:**
  1. Navigate to the **Unstaged Screen** and select a file.
  2. Move focus into the diff panel.
  3. Navigate to any line within the hunk to be discarded.
  4. Press the `!` key.
- **Expected Outcome:**
  - The selected hunk is reverted from the working directory.
  - The diff view updates to reflect the change.
- **Cursor Movement:**
  - The file selection cursor remains on the same file.
  - The line cursor in the diff view attempts to stay at the same numerical position but will move up if the lines it was on were removed.

### 3.3. Delete an Untracked File

- **User Action:**
  1. Navigate to the **Unstaged Screen**.
  2. Select a file from the "Untracked files" list.
  3. Press the `!` key.
- **Expected Outcome:**
  - The selected file is permanently deleted from the filesystem.
  - The file is removed from the "Untracked files" list.
  - This action cannot be performed on binary files as a safety measure.
- **Cursor Movement:**
  - The cursor moves to the next item in the list. If the deleted file was the last one in the list, the cursor moves to the section header.