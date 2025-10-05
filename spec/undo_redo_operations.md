# Application Specification: Undo and Redo Operations

This document specifies the application's undo and redo functionality, which allows users to reverse and restore actions.

## 1. General Context

The application maintains a history of state-changing commands. The undo and redo operations are available globally across different screens, except when in a mode that captures text input (e.g., editing a commit message).

- **Undo:** Reverts the most recent command.
- **Redo:** Re-applies the most recently undone command.

## 2. Undo Operation

- **User Action:**
  - Press the `<` key (less-than sign).
- **Expected Outcome:**
  - The application's state is reverted to what it was before the last command was executed.
  - For example, if the last action was staging a file, an undo will unstage that file.
  - The cursor's position is restored to where it was before the undone action was performed.

## 3. Redo Operation

- **User Action:**
  - Press the `>` key (greater-than sign).
- **Expected Outcome:**
  - The most recently undone command is re-executed.
  - For example, if a file staging action was undone, a redo will stage that file again.
  - The cursor's position is restored to where it was after the redone action was originally performed.

## 4. History Management

The undo/redo history is cleared, and all previous actions can no longer be undone or redone, after certain irreversible operations are performed. This occurs after:

- **Executing a commit:** After a new commit is successfully created.
- **Amending a commit:** After a commit is successfully amended or reworded.

## 5. Edge Cases

- **No History:**
  - If no actions have been taken, pressing `<` will have no effect.
  - If no action has been undone, pressing `>` will have no effect.
- **End of History:**
  - Once all actions in the history have been undone, further presses of `<` will have no effect.
  - Once all undone actions have been redone, further presses of `>` will have no effect.