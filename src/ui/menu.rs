use crate::app::MenuOption;
use crate::render::font;
use crate::render::framebuffer::{Color, Framebuffer};

/// Render the main menu into the framebuffer using the bitmap font.
pub fn render_menu(fb: &mut Framebuffer, selection: usize, options: &[MenuOption], muted: bool) {
    fb.color.fill(Color::new(10, 10, 25));

    let w = fb.width as i32;
    let h = fb.height as i32;
    if w < 16 || h < 16 {
        return;
    }

    // Scale to fit: widest content must fit in framebuffer width
    // "  RANDOM COURSE" = 15 chars is the widest menu item
    // Title "FLIGHTTY" = 8 chars at title_scale = scale+1
    let max_chars = 17; // widest line including prefix
    let scale: u32 = ((w as u32 * 36 / 100) / (max_chars * 8)).clamp(1, 3);
    let title_scale = scale;
    let char_w = 8 * scale as i32;
    let title_char_w = 8 * title_scale as i32;
    let line_h = 8 * scale as i32 + 4;

    // Title: "FLIGHTTY"
    let title = "FLIGHTTY";
    let title_px = title.len() as i32 * title_char_w;
    let title_x = (w - title_px) / 2;
    let title_y = h / 8;
    font::draw_string(fb, title, title_x, title_y, Color::new(255, 200, 50), None, title_scale);

    // Subtitle
    let sub = "TERMINAL EDITION";
    let sub_px = sub.len() as i32 * char_w;
    let sub_y = title_y + title_scale as i32 * 9;
    font::draw_string(fb, sub, (w - sub_px) / 2, sub_y, Color::new(80, 80, 110), None, scale);

    // Menu options, vertically centered in remaining space
    let menu_height = options.len() as i32 * line_h;
    let menu_start = (sub_y + scale as i32 * 12 + h - menu_height) / 2;

    for (i, option) in options.iter().enumerate() {
        let label = match option {
            MenuOption::ToggleMusic => if muted { "MUSIC: OFF" } else { "MUSIC: ON" },
            MenuOption::Play => "PLAY",
            MenuOption::AxisMapping => "AXIS MAPPING",
            MenuOption::Quit => "QUIT",
        };
        let prefix = if i == selection { "> " } else { "  " };
        let text = format!("{}{}", prefix, label);
        let text_px = text.len() as i32 * char_w;
        let x = (w - text_px) / 2;
        let y = menu_start + i as i32 * line_h;

        let (color, bg) = if i == selection {
            (Color::new(255, 255, 255), Some(Color::new(40, 40, 60)))
        } else {
            (Color::new(90, 90, 110), None)
        };

        font::draw_string(fb, &text, x, y, color, bg, scale);
    }

    // Controls hint at bottom
    let hint_scale = scale.max(2) - 1;
    let hint = "I/K NAVIGATE  ENTER SELECT";
    let hint_px = hint.len() as i32 * 8 * hint_scale as i32;
    font::draw_string(
        fb,
        hint,
        (w - hint_px) / 2,
        h - hint_scale as i32 * 10,
        Color::new(50, 50, 70),
        None,
        hint_scale,
    );
}
