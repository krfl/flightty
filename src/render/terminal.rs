use std::io::{self, Write};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color as RatColor;
use ratatui::widgets::Widget;

use crate::render::framebuffer::Framebuffer;

#[derive(Clone, Copy, PartialEq)]
pub enum RenderMode {
    HalfBlock,
    Kitty,
}

impl RenderMode {
    /// Pixel dimensions per terminal cell.
    pub fn cell_pixels(self) -> (u32, u32) {
        match self {
            RenderMode::HalfBlock => (1, 2),
            RenderMode::Kitty => (2, 4),
        }
    }

    pub fn toggle(self, kitty_supported: bool) -> Self {
        match self {
            RenderMode::HalfBlock => {
                if kitty_supported { RenderMode::Kitty } else { RenderMode::HalfBlock }
            }
            RenderMode::Kitty => RenderMode::HalfBlock,
        }
    }
}

/// Detect if the terminal supports the Kitty graphics protocol via env vars.
pub fn detect_kitty_support() -> bool {
    if let Ok(term) = std::env::var("TERM") {
        if term.contains("kitty") {
            return true;
        }
    }
    if let Ok(prog) = std::env::var("TERM_PROGRAM") {
        let prog_lower = prog.to_lowercase();
        if prog_lower.contains("wezterm")
            || prog_lower.contains("ghostty")
            || prog_lower.contains("kitty")
        {
            return true;
        }
    }
    false
}

/// Widget that renders a Framebuffer to the terminal.
pub struct FpvView<'a> {
    pub framebuffer: &'a Framebuffer,
    pub mode: RenderMode,
}

impl Widget for FpvView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.mode {
            RenderMode::HalfBlock => render_halfblock(self.framebuffer, area, buf),
            RenderMode::Kitty => {} // handled separately
        }
    }
}

/// Render framebuffer via Kitty Graphics Protocol directly to stdout.
/// Bypasses ratatui by writing escape sequences directly.
/// Delete all kitty graphics images (call when leaving kitty mode or exiting).
pub fn cleanup_kitty() -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    write!(stdout, "\x1b_Ga=d,d=A,q=2\x1b\\")?;
    stdout.flush()
}

/// Reset the kitty frame counter (call on mode switch).
pub fn reset_kitty_frame_counter() {
    KITTY_FRAME.store(0, std::sync::atomic::Ordering::Relaxed);
}

/// Frame counter for double-buffered kitty rendering.
static KITTY_FRAME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Render framebuffer via Kitty Graphics Protocol directly to stdout.
/// HUD is already rendered into the framebuffer as pixels.
/// Uses double buffering: alternates between image IDs 1 and 2 so the old
/// frame stays visible until the new one is fully transmitted and placed.
pub fn render_kitty_frame(
    fb: &Framebuffer,
    term_cols: u16,
    term_rows: u16,
) -> io::Result<()> {
    let frame_num = KITTY_FRAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let new_id = if frame_num.is_multiple_of(2) { 1 } else { 2 };
    let old_id = if frame_num.is_multiple_of(2) { 2 } else { 1 };

    // Build RGB byte buffer from framebuffer
    let pixel_count = (fb.width * fb.height) as usize;
    let mut rgb_bytes = Vec::with_capacity(pixel_count * 3);
    for i in 0..pixel_count {
        let c = fb.color[i];
        rgb_bytes.push(c.r);
        rgb_bytes.push(c.g);
        rgb_bytes.push(c.b);
    }

    // Base64 encode the pixel data
    let encoded = STANDARD.encode(&rgb_bytes);
    let encoded_bytes = encoded.as_bytes();

    // Buffer everything before writing to stdout
    let mut out = Vec::with_capacity(encoded_bytes.len() + 512);

    // Move cursor to top-left
    out.extend_from_slice(b"\x1b[H");

    // Step 1: Transmit new image data silently (a=t, no display yet)
    const CHUNK_SIZE: usize = 4096;
    let total_chunks = encoded_bytes.len().div_ceil(CHUNK_SIZE);

    for (i, chunk) in encoded_bytes.chunks(CHUNK_SIZE).enumerate() {
        let is_last = i == total_chunks - 1;
        let m = if is_last { 0 } else { 1 };

        if i == 0 {
            write!(
                out,
                "\x1b_Ga=t,i={},f=24,s={},v={},q=2,m={};",
                new_id, fb.width, fb.height, m,
            )?;
        } else {
            write!(out, "\x1b_Gm={};", m)?;
        }
        out.extend_from_slice(chunk);
        out.extend_from_slice(b"\x1b\\");
    }

    // Step 2: Place new image, atomically replacing the old placement
    write!(
        out,
        "\x1b_Ga=p,i={},p=1,c={},r={},q=2,C=1\x1b\\",
        new_id, term_cols, term_rows,
    )?;

    // Step 3: Delete old image data (frees memory, no visual effect)
    if frame_num > 0 {
        write!(out, "\x1b_Ga=d,d=i,i={},q=2\x1b\\", old_id)?;
    }

    // Single write + flush
    let mut stdout = io::stdout().lock();
    stdout.write_all(&out)?;
    stdout.flush()
}

/// Half-block rendering: each terminal cell = 1x2 pixels.
fn render_halfblock(fb: &Framebuffer, area: Rect, buf: &mut Buffer) {
    let fb_w = fb.width;
    let fb_h = fb.height;
    let cols = area.width.min(fb_w as u16);
    let rows = area.height.min((fb_h / 2) as u16);

    for cy in 0..rows {
        let py_top = cy as u32 * 2;
        let py_bot = py_top + 1;
        let top_row = (py_top * fb_w) as usize;
        let bot_row = if py_bot < fb_h {
            (py_bot * fb_w) as usize
        } else {
            top_row
        };

        for cx in 0..cols {
            let px = cx as usize;
            let top = fb.color[top_row + px];
            let bot = if py_bot < fb_h {
                fb.color[bot_row + px]
            } else {
                crate::render::framebuffer::Color::BLACK
            };

            if let Some(cell) = buf.cell_mut((area.x + cx, area.y + cy)) {
                cell.set_char('▀');
                cell.set_fg(RatColor::Rgb(top.r, top.g, top.b));
                cell.set_bg(RatColor::Rgb(bot.r, bot.g, bot.b));
            }
        }
    }
}

