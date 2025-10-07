# Application Specification: Pane Switching

This document describes how users navigate between different panes within the Main Screen.

## 1. General Context

The Main Screen is divided into two primary, vertically-stacked panes:
- **Top Pane:** Displays unstaged changes and untracked files.
- **Bottom Pane:** Displays staged changes, the commit message input, and the commit log.

Navigation between these panes is handled by the `Tab` key and arrow keys.

## 2. Switching Actions

### 2.1. Tab Key

- **User Action:**
  - Press the `Tab` key.
- **Expected Outcome:**
  - The application toggles the focus between the Top Pane and the Bottom Pane.

### 2.2. Arrow Keys

- **User Action:**
  - Press the `Up Arrow` key or `Ctrl+P` when the cursor is at the top of the Bottom Pane.
- **Expected Outcome:**
  - The focus switches to the Top Pane, and the cursor is positioned on the last item.
- **User Action:**
  - Press the `Down Arrow` key or `Ctrl+N` when the cursor is at the bottom of the Top Pane.
- **Expected Outcome:**
  - The focus switches to the Bottom Pane, and the cursor is positioned on the first item.

## 3. Switching Conditions

Pane switching is not always available. The action is blocked in certain contexts.

- **Blocked Contexts:**
  - **Commit Message Editing:** Switching is disabled when the user is actively editing a commit message in the Bottom Pane. In this mode, the `Tab` key press is ignored.

## 4. Visual Indication of Focus

The currently active pane is indicated by the presence of the list cursor.

- **Active Pane:** The list cursor (highlighting the selected file or item) is visible.
- **Inactive Pane:** The list cursor is hidden.

## 5. Cursor Behavior on Switch

### 5.1. Tab Key Switching
When switching with the `Tab` key, the application attempts to maintain context by intelligently positioning the cursor after a pane switch. The position is determined by the following priority:

1.  **Match Current File:** If the user has a file selected in the source pane, the application will search for the *same file* in the destination pane. If a match is found, the cursor in the destination pane is moved directly to that file.

2.  **Restore Last Position:** If no file match is found, the application restores the cursor to its last known position in the destination pane.

3.  **Default to Initial Position:** If the destination pane has not been focused before, the cursor is placed at the default initial position for that pane.

### 5.2. Arrow Key Switching
When switching with arrow keys, the cursor is moved to the first or last item of the destination pane as described in section 2.2.

## 6. Automatic Focus Switching

If an action causes the Top Pane (Unstaged/Untracked) to become empty (e.g., staging the last file), the focus automatically switches to the Bottom Pane. This transition follows the same cursor behavior rules as a manual `Tab` key press.