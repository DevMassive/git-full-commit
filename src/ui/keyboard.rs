use pancurses::Input;

/// Returns true when the input should move the selection upwards within a list.
pub fn is_move_up(input: &Input) -> bool {
    matches!(input, Input::KeyUp | Input::Character('\u{10}'))
}

/// Returns true when the input should move the selection downwards within a list.
pub fn is_move_down(input: &Input) -> bool {
    matches!(input, Input::KeyDown | Input::Character('\u{e}'))
}

/// Convenience helper for checks that need to react to either up or down navigation.
pub fn is_vertical_navigation(input: &Input) -> bool {
    is_move_up(input) || is_move_down(input)
}

/// Returns true when the input should move the diff cursor upwards.
pub fn is_diff_move_up(input: &Input) -> bool {
    matches!(input, Input::Character('k'))
}

/// Returns true when the input should move the diff cursor downwards.
pub fn is_diff_move_down(input: &Input) -> bool {
    matches!(input, Input::Character('j'))
}

/// Returns true when the input should scroll the horizontal content to the left.
pub fn is_horizontal_left(input: &Input) -> bool {
    matches!(input, Input::KeyLeft)
}

/// Returns true when the input should scroll the horizontal content to the right.
pub fn is_horizontal_right(input: &Input) -> bool {
    matches!(input, Input::KeyRight)
}

/// Returns true when the input represents the primary staging/unstaging action.
pub fn is_stage_toggle(input: &Input) -> bool {
    matches!(input, Input::Character('\n') | Input::Character('u'))
}
