# Application Specification: Ignore Operations

This document describes the application's functionality for ignoring files, which adds them to the `.gitignore` file.

## 1. General Context

Ignore operations can be initiated from two primary screens: the **Main Screen** (for staged files) and the **Unstaged Screen** (for unstaged or untracked files).

If a `.gitignore` file does not exist in the repository's root, it will be created automatically when the first file is ignored.

## 2. Ignoring from the Main Screen

This action is for ignoring files that are already staged.

-   **User Action:**
    1.  Navigate to the **Main Screen**.
    2.  Select a specific file from the "Staged changes" list.
    3.  Press the `i` key.
-   **Expected Outcome:**
    -   The selected file's name is appended to the `.gitignore` file.
    -   The `.gitignore` file is automatically staged.
    -   The original file is unstaged and removed from the "Staged changes" list, now appearing as ignored.

## 3. Ignoring from the Unstaged Screen

This screen handles two types of files: unstaged tracked files and untracked files.

### 3.1. Ignoring an Unstaged Tracked File

-   **User Action:**
    1.  Navigate to the **Unstaged Screen**.
    2.  Select a file from the "Unstaged changes" list.
    3.  Press the `i` key.
-   **Expected Outcome:**
    -   The file's name is appended to `.gitignore`.
    -   `.gitignore` is staged.
    -   The file is removed from the unstaged list and becomes ignored.

### 3.2. Ignoring an Untracked File

-   **User Action:**
    1.  Navigate to the **Unstaged Screen**.
    2.  Select a file from the "Untracked files" list.
    3.  Press the `i` key.
-   **Expected Outcome:**
    -   The file's name is appended to `.gitignore`.
    -   `.gitignore` is staged.
    -   The file is removed from the untracked list.