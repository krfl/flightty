use std::io;

use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

mod app;
mod audio;
mod game_loop;
mod input;
mod physics;
mod render;
mod ui;
mod world;

fn main() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // Try to enable keyboard enhancement for key release events.
    // Not all terminals support this, so fall back gracefully.
    let keyboard_enhanced = execute!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
    )
    .is_ok();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Create app (starts in menu state with a default scene)
    let scene = world::scene::test_course();
    let mut app = app::App::new(scene);
    // Clean up kitty images on exit
    let _ = render::terminal::cleanup_kitty();

    // Resize framebuffer to terminal size
    let size = terminal.size()?;
    app.resize_framebuffer(size.width, size.height);

    // Run game loop
    let result = game_loop::run(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    if keyboard_enhanced {
        let _ = execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags);
    }
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;
    terminal.show_cursor()?;

    result
}
