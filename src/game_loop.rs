use std::io::{Stdout, Write};
use std::time::{Duration, Instant};

use crossterm::event::KeyCode;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::{App, AppState, MenuOption, MENU_OPTIONS};
use crate::audio::music::Track;
use crate::input::controls::StickState;
use crate::physics::drone::{physics_step, DroneState};
use crate::render::camera;
use crate::render::font;
use crate::render::framebuffer::{Color, Framebuffer};
use crate::render::rasterizer;
use crate::render::terminal::{self, FpvView, RenderMode};
use crate::ui::axis_mapping;
use crate::ui::menu;

const PHYSICS_HZ: f32 = 240.0;
const PHYSICS_DT: f32 = 1.0 / PHYSICS_HZ;
const MAX_FRAME_TIME: f32 = 0.1;
const TARGET_FPS: u64 = 24;

pub fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> std::io::Result<()> {
    let mut accumulator = 0.0f32;
    let mut last_time = Instant::now();
    let frame_duration = Duration::from_millis(1000 / TARGET_FPS);

    // Spawn dedicated input thread
    let input_shared = app.input.shared.clone();
    let _input_handle = crate::input::controls::spawn_input_thread(input_shared);

    loop {
        let now = Instant::now();
        let frame_time = now.duration_since(last_time).as_secs_f32().min(MAX_FRAME_TIME);
        last_time = now;
        accumulator += frame_time;

        // Sync input from thread
        app.input.sync_from_shared();

        // Poll gamepad
        if let Some(ref mut gp) = app.gamepad {
            gp.poll();
        }

        // Get terminal size + resize framebuffer
        let size = terminal.size()?;
        app.resize_framebuffer(size.width, size.height);

        // State dispatch
        // Music follows state (unless muted)
        if !app.muted {
            if let Some(ref mut sound) = app.sound {
                match app.state {
                    AppState::Menu | AppState::AxisMapping => sound.play(Track::Menu),
                    AppState::Flying => sound.play(Track::Flight),
                }
            }
        }

        // Save state before input handling to detect transitions
        let state_before = app.state;

        match app.state {
            AppState::Menu => {
                handle_menu_input(app);
                if !app.running {
                    break;
                }
            }
            AppState::Flying => {
                handle_flying_input(app, frame_time, &mut accumulator);
            }
            AppState::AxisMapping => {
                handle_axis_mapping_input(app);
            }
        }

        // If state changed, skip rendering this frame (clear already handled)
        if app.state != state_before {
            continue;
        }

        // Render current state
        match app.state {
            AppState::Menu => {
                menu::render_menu(&mut app.framebuffer, app.menu_selection, MENU_OPTIONS, app.muted);
                render_frame(terminal, app, size)?;
            }
            AppState::Flying => {
                run_flying_frame(app, &mut accumulator);
                render_frame(terminal, app, size)?;
            }
            AppState::AxisMapping => {
                axis_mapping::render_axis_mapping(
                    &mut app.framebuffer,
                    app.axis_map_selection,
                    app.axis_map_listening,
                    app.gamepad.as_ref(),
                );
                render_frame(terminal, app, size)?;
            }
        }

        // Frame rate limiting
        let elapsed = now.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }

    Ok(())
}

fn handle_menu_input(app: &mut App) {
    let num_options = MENU_OPTIONS.len();

    // Keyboard navigation
    if (app.input.was_pressed(KeyCode::Char('i')) || app.input.was_pressed(KeyCode::Up))
        && app.menu_selection > 0
    {
        app.menu_selection -= 1;
    }
    if (app.input.was_pressed(KeyCode::Char('k')) || app.input.was_pressed(KeyCode::Down))
        && app.menu_selection < num_options - 1
    {
        app.menu_selection += 1;
    }


    let selected = app.input.was_pressed(KeyCode::Enter);

    if selected {
        match MENU_OPTIONS[app.menu_selection] {
            MenuOption::Play => {
                app.scene = crate::world::procedural::random_course();
                app.reset();
                app.state = AppState::Flying;
                app.needs_clear = true;
            }
            MenuOption::AxisMapping => {
                app.axis_map_selection = 0;
                app.axis_map_listening = false;
                app.state = AppState::AxisMapping;
                app.needs_clear = true;
            }
            MenuOption::ToggleMusic => {
                app.muted = !app.muted;
                if let Some(ref mut sound) = app.sound {
                    if app.muted {
                        sound.stop();
                    }
                    // If unmuted, the music loop at the top of the frame will restart it
                }
            }
            MenuOption::Quit => {
                app.running = false;
            }
        }
    }
}

fn handle_flying_input(app: &mut App, frame_time: f32, _accumulator: &mut f32) {
    if app.input.was_pressed(KeyCode::Esc) || app.input.was_pressed(KeyCode::Char('q')) {
        app.state = AppState::Menu;
        app.needs_clear = true;
        return;
    }

    // Reset
    if app.input.was_pressed(KeyCode::Char('r')) {
        app.reset();
    }

    // Render mode toggle
    if app.input.was_pressed(KeyCode::Tab) {
        if app.render_mode == RenderMode::Kitty {
            let _ = terminal::cleanup_kitty();
        }
        app.render_mode = app.render_mode.toggle(app.kitty_supported);
        if app.render_mode == RenderMode::Kitty {
            terminal::reset_kitty_frame_counter();
        }
        app.needs_clear = true;
    }

    // Update sticks: gamepad priority
    // Only accept gamepad input once throttle has been at zero after entering flight.
    // This prevents the drone launching immediately if throttle was held during menu.
    let use_gamepad = app.gamepad.as_ref().is_some_and(|gp| gp.connected);
    if use_gamepad {
        let gp = app.gamepad.as_ref().unwrap();
        if !app.flight_controls_armed {
            // Wait for throttle near zero before arming
            if gp.sticks.throttle < 0.05 {
                app.flight_controls_armed = true;
            }
        }
        if app.flight_controls_armed {
            app.input.sticks.throttle = gp.sticks.throttle;
            app.input.sticks.yaw = gp.sticks.yaw;
            app.input.sticks.pitch = gp.sticks.pitch;
            app.input.sticks.roll = gp.sticks.roll;
        }
    } else {
        app.input.update_sticks(frame_time);
    }
}

fn run_flying_frame(app: &mut App, accumulator: &mut f32) {
    // Fixed timestep physics
    while *accumulator >= PHYSICS_DT {
        physics_step(
            &mut app.drone,
            &app.config,
            &app.input.sticks,
            &app.scene.colliders,
            PHYSICS_DT,
        );
        if !app.drone.crashed {
            app.flight_time += PHYSICS_DT;
        }
        *accumulator -= PHYSICS_DT;
    }

    // Rasterize scene
    let view = camera::fpv_view_matrix(&app.drone, &app.config);
    let aspect = app.framebuffer.width as f32 / app.framebuffer.height as f32;
    let proj = camera::projection_matrix(120.0, aspect);
    let view_proj = proj * view;

    let zenith = Color::new(15, 20, 60);
    let horizon = crate::render::rasterizer::FOG_COLOR;
    app.framebuffer.clear_gradient(zenith, horizon);

    for mesh in &app.scene.meshes {
        rasterizer::rasterize_mesh(&mut app.framebuffer, mesh, &view_proj);
    }

    // Pixel HUD
    render_pixel_hud(
        &mut app.framebuffer,
        &app.input.sticks,
        &app.drone,
        app.flight_time,
        app.gamepad
            .as_ref()
            .filter(|gp| gp.connected)
            .map(|gp| gp.name.as_str()),
        app.render_mode,
    );
}

fn handle_axis_mapping_input(app: &mut App) {
    if app.input.was_pressed(KeyCode::Esc) || app.input.was_pressed(KeyCode::Char('q')) {
        if app.axis_map_listening {
            app.axis_map_listening = false;
        } else {
            app.state = AppState::Menu;
            app.needs_clear = true;
        }
        return;
    }

    if app.axis_map_listening {
        // Detect axis movement
        if let Some(ref gp) = app.gamepad {
            if let Some(axis) = gp.detect_axis() {
                if let Some(ref mut gp) = app.gamepad {
                    gp.mapping.get_mut(app.axis_map_selection).axis = axis;
                    gp.mapping.save(&gp.name);
                }
                app.axis_map_listening = false;
            }
        }
        return;
    }

    // Navigation
    if (app.input.was_pressed(KeyCode::Char('i')) || app.input.was_pressed(KeyCode::Up))
        && app.axis_map_selection > 0
    {
        app.axis_map_selection -= 1;
    }
    if (app.input.was_pressed(KeyCode::Char('k')) || app.input.was_pressed(KeyCode::Down))
        && app.axis_map_selection < 3
    {
        app.axis_map_selection += 1;
    }

    if app.input.was_pressed(KeyCode::Enter)
        && app.gamepad.as_ref().is_some_and(|gp| gp.connected)
    {
        app.axis_map_listening = true;
    }

    // I key for invert (when not used for navigation; use 'v' instead)
    if app.input.was_pressed(KeyCode::Char('v')) {
        if let Some(ref mut gp) = app.gamepad {
            let a = gp.mapping.get_mut(app.axis_map_selection);
            a.inverted = !a.inverted;
            gp.mapping.save(&gp.name);
        }
    }
}

/// Render the framebuffer to the terminal (handles both modes).
fn render_frame(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    size: ratatui::layout::Size,
) -> std::io::Result<()> {
    // Clear screen on mode/state transitions to remove previous render artifacts
    if app.needs_clear {
        let _ = terminal::cleanup_kitty();
        // Clear terminal screen and reset ratatui state
        write!(std::io::stdout(), "\x1b[2J\x1b[H")?;
        std::io::stdout().flush()?;
        terminal.clear()?;
        app.needs_clear = false;
    }

    match app.render_mode {
        RenderMode::Kitty => {
            terminal::render_kitty_frame(&app.framebuffer, size.width, size.height)?;
        }
        RenderMode::HalfBlock => {
            terminal.draw(|frame| {
                let area = frame.area();
                let fpv = FpvView {
                    framebuffer: &app.framebuffer,
                    mode: app.render_mode,
                };
                frame.render_widget(fpv, area);
            })?;
        }
    }
    Ok(())
}

/// Render HUD directly into the framebuffer as pixels.
fn render_pixel_hud(
    fb: &mut Framebuffer,
    sticks: &StickState,
    drone: &DroneState,
    flight_time: f32,
    gamepad_name: Option<&str>,
    render_mode: RenderMode,
) {
    let w = fb.width as i32;
    let h = fb.height as i32;

    // Scale font so it covers the same number of terminal cells in both modes.
    // Kitty has 2x pixels per cell vs halfblock, so we multiply scale by px_w
    // to compensate. This makes the font the same visual size on screen.
    let (px_w, _) = render_mode.cell_pixels();
    let base_scale = (w as u32 / 1600).clamp(1, 3);
    let scale = base_scale * px_w;
    let char_w = 8 * scale as i32;
    let pad = scale as i32 * 2;

    let white = Color::new(240, 240, 240);
    let green = Color::new(80, 255, 80);
    let yellow = Color::new(255, 255, 80);
    let gray = Color::new(160, 160, 160);
    let red = Color::new(255, 60, 60);
    let cyan = Color::new(80, 220, 255);
    let bg = Some(Color::new(15, 15, 15));

    // Flight timer (top center)
    let secs = flight_time as u32;
    let timer = format!("{:02}:{:02}", secs / 60, secs % 60);
    let tx = w / 2 - (timer.len() as i32 * char_w) / 2;
    font::draw_string(fb, &timer, tx, pad, white, bg, scale);

    // Render mode (top right)
    let mode_label = match render_mode {
        RenderMode::HalfBlock => "BLOCK",
        RenderMode::Kitty => "KITTY",
    };
    let mode_px = mode_label.len() as i32 * char_w;
    font::draw_string(fb, mode_label, w - mode_px - pad, pad, gray, bg, scale);

    // Controls / gamepad name (top left)
    if let Some(name) = gamepad_name {
        let label: String = name.chars().take(16).collect();
        font::draw_string(fb, &label, pad, pad, cyan, bg, scale);
    } else {
        font::draw_string(fb, "W/S THR", pad, pad, gray, bg, scale);
        font::draw_string(fb, "IJKL FLY", pad, pad + 8 * scale as i32 + 2, gray, bg, scale);
    }

    // Bottom row
    let bot_y = h - 8 * scale as i32 - pad;

    // Throttle (bottom left)
    let throttle_pct = (sticks.throttle * 100.0) as u32;
    let thr_str = format!("THR {:3}%", throttle_pct);
    font::draw_string(fb, &thr_str, pad, bot_y, green, bg, scale);

    // Altitude (bottom center)
    let alt_str = format!("ALT {:.1}M", drone.position.y);
    let ax = w / 2 - (alt_str.len() as i32 * char_w) / 2;
    font::draw_string(fb, &alt_str, ax, bot_y, white, bg, scale);

    // Speed (bottom right)
    let spd_str = format!("{:.1} M/S", drone.velocity.norm());
    let sx = w - spd_str.len() as i32 * char_w - pad;
    font::draw_string(fb, &spd_str, sx, bot_y, yellow, bg, scale);

    // Crosshair (center of screen)
    let cx = w / 2;
    let cy = h / 2;
    let arm = (4 * px_w as i32).max(3); // length of each arm
    let gap = (2 * px_w as i32).max(1); // gap in the center
    let crosshair_color = Color::new(255, 255, 255);

    // Horizontal arms
    for x in (cx - arm - gap)..=(cx - gap) {
        if x >= 0 && x < w { fb.color[(cy * w + x) as usize] = crosshair_color; }
    }
    for x in (cx + gap)..=(cx + arm + gap) {
        if x >= 0 && x < w { fb.color[(cy * w + x) as usize] = crosshair_color; }
    }
    // Vertical arms
    for y in (cy - arm - gap)..=(cy - gap) {
        if y >= 0 && y < h { fb.color[(y * w + cx) as usize] = crosshair_color; }
    }
    for y in (cy + gap)..=(cy + arm + gap) {
        if y >= 0 && y < h { fb.color[(y * w + cx) as usize] = crosshair_color; }
    }

    // Crashed indicator
    if drone.crashed {
        let msg = "CRASHED - PRESS R";
        let cx = w / 2 - (msg.len() as i32 * char_w) / 2;
        let cy = h / 2 - 4 * scale as i32;
        font::draw_string(fb, msg, cx, cy, red, Some(Color::new(40, 0, 0)), scale);
    }
}
