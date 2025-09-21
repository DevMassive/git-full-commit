use pancurses::{COLOR_BLACK, init_color, init_pair};

pub fn setup_colors() {
    // Base colors
    let color_white = 20;
    let color_red = 21;
    let color_green = 22;
    let color_cyan = 23;
    let color_grey = 24;
    let color_selected_bg = 25;
    let color_inactive_cursor_bg = 26;

    init_color(color_white, 968, 968, 941); // #F7F7F0
    init_color(color_red, 1000, 0, 439); // #FF0070
    init_color(color_green, 525, 812, 0); // #86CF00
    init_color(color_cyan, 0, 769, 961); // #00C4F5
    init_color(color_grey, 266, 266, 266); // #444444
    init_color(color_selected_bg, 150, 150, 150);
    init_color(color_inactive_cursor_bg, 80, 80, 80);

    // Color pairs
    init_pair(1, color_white, COLOR_BLACK); // Default: White on Black
    init_pair(2, color_red, COLOR_BLACK); // Deletion: Red on Black
    init_pair(3, color_green, COLOR_BLACK); // Addition: Green on Black
    init_pair(4, color_cyan, COLOR_BLACK); // Hunk Header: Cyan on Black
    init_pair(9, color_grey, COLOR_BLACK); // Grey on Black

    // Selected line pairs
    init_pair(5, color_white, color_selected_bg); // Default: White on selected
    init_pair(6, color_red, color_selected_bg); // Deletion: Red on selected
    init_pair(7, color_green, color_selected_bg); // Addition: Green on selected
    init_pair(8, color_cyan, color_selected_bg); // Hunk Header: Cyan on selected
    init_pair(10, color_grey, color_selected_bg); // Grey on selected

    // Inactive cursor pairs
    init_pair(11, color_white, color_inactive_cursor_bg);
    init_pair(12, color_red, color_inactive_cursor_bg);
    init_pair(13, color_green, color_inactive_cursor_bg);
    init_pair(14, color_cyan, color_inactive_cursor_bg);
    init_pair(15, color_grey, color_inactive_cursor_bg);
    init_pair(16, COLOR_BLACK, color_selected_bg); // Black on selected
}
