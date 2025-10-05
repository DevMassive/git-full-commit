# Application Specification: Screen Switching

This document describes how users navigate between the application's primary screens, based on a direct analysis of the source code.

## 1. General Context

The application has two main screens for managing Git state:
- **Main Screen:** Displays staged changes and the commit log.
- **Unstaged Screen:** Displays unstaged changes and untracked files.

Navigation between these screens is handled by the `Tab` key.

## 2. Switching Action

- **User Action:**
  - Press the `Tab` key.
- **Expected Outcome:**
  - The application toggles between the Main Screen and the Unstaged Screen.

## 3. Switching Conditions

Screen switching is not always available. The action is blocked in certain contexts.

- **Blocked Contexts:**
  - **Commit Message Editing:** Switching is disabled when the user is actively editing a commit message on the Main Screen. In this mode, `Tab` inserts a tab character.
  - **No Unstaged Changes:** Switching from the Main Screen to the Unstaged Screen is disabled if there are no unstaged changes and no untracked files.

## 4. Cursor Behavior on Switch

The application attempts to maintain context by intelligently positioning the cursor after a screen switch. The position is determined by the following priority:

1.  **Match Current File:**
    - If the user has a file selected on the source screen, the application will search for the *same file* on the destination screen.
    - If a match is found, the cursor on the destination screen is moved directly to that file.

2.  **Restore Last Position:**
    - If no file match is found, the application restores the cursor to its last known position on the destination screen.

3.  **Default to Initial Position:**
    - If the destination screen has not been visited before, the cursor is placed at the default initial position.

## 5. Diff View State Persistence

The state of the Diff View for each screen is preserved independently.

- **Example:**
  1. A user on the **Main Screen** activates the Diff View cursor (`j`/`k`) and scrolls to line 50.
  2. The user switches to the **Unstaged Screen** (`Tab`) and performs some actions.
  3. When the user switches back to the **Main Screen** (`Tab`), the Diff View will still be active and scrolled to line 50, exactly as it was left.