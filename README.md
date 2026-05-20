# flightty

A physics-based FPV drone simulator that runs in the terminal. Written in Rust.

![flightty](https://img.shields.io/badge/rust-terminal--sim-orange)

![screenshot](screenshot.png)

## Features

- **Physics simulation**: quaternion-based orientation, Betaflight-style rate curves, quadratic air drag
- **Two render modes**: HalfBlock (unicode characters, works everywhere) and Kitty Graphics Protocol (real pixels, double-buffered)
- **Auto-detection**: automatically selects Kitty graphics when running in a supported terminal (Kitty, WezTerm, Ghostty)
- **Radio controller support**: plug in a RadioMaster TX16S/TX15 or any USB gamepad (PS5, Xbox) via `gilrs`
- **Per-controller axis profiles**: axis mappings saved per controller name, auto-loaded on connect
- **Keyboard controls**: smoothed analog-style input from binary keys
- **Procedural courses**: randomly generated each time you play, with gates, pillars, walls, ramps, and obstacle clusters
- **Collision detection**: AABB-based obstacle collision with bounce/crash (gates are fly-through)
- **Bitmap HUD**: throttle, altitude, speed, flight timer, and render mode rendered as pixels into the framebuffer
- **Sky gradient and fog**: horizon-to-zenith sky gradient with distance-based fog
- **Music**: embedded MP3 soundtrack with mute toggle
- **Near-plane clipping**: Sutherland-Hodgman triangle clipping for close-up rendering
- **Frustum culling**: bounding sphere culling skips off-screen meshes
- **Menu system**: play, music toggle, axis mapping, and quit

## Controls

### Keyboard

| Key | Action |
|-----|--------|
| `W` / `S` | Throttle up / down |
| `A` / `D` | Yaw left / right |
| `I` / `K` | Pitch forward / back |
| `J` / `L` | Roll left / right |
| `R` | Reset after crash |
| `Tab` | Toggle render mode |
| `Esc` | Back to menu |

### Radio Controller / Gamepad

Plug in via USB. Default Mode 2 mapping:
- Left stick Y = throttle, X = yaw
- Right stick Y = pitch, X = roll

Axes can be remapped and inverted via the Axis Mapping screen in the menu. Mappings are saved per controller.

## Build & Run

```sh
cargo run --release
```

## Render Modes

**HalfBlock** uses `▀` characters with foreground/background colors. Works in any terminal with truecolor support.

**Kitty Graphics Protocol** sends real pixel data to the terminal. Double-buffered for flicker-free rendering. Auto-selected when running in [Kitty](https://sw.kovidgoyal.net/kitty/), [WezTerm](https://wezfurlong.org/wezterm/), or [Ghostty](https://ghostty.org/). Reduce your terminal font size for higher resolution.

## Music

Music crafted specifically for this game.

## Dependencies

- [ratatui](https://ratatui.rs/) + [crossterm](https://docs.rs/crossterm/) for terminal UI
- [nalgebra](https://nalgebra.org/) for linear algebra, quaternions, projections
- [gilrs](https://gitlab.com/gilrs-project/gilrs) for gamepad/joystick input
- [rodio](https://docs.rs/rodio/) for audio playback
- [base64](https://docs.rs/base64/) for Kitty graphics protocol encoding
- [rand](https://docs.rs/rand/) for procedural course generation
