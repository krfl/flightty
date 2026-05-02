# fpv

A physics-based FPV drone simulator that runs in the terminal. Written in Rust.

![fpv](https://img.shields.io/badge/rust-terminal--sim-orange)

![screenshot](screenshot.png)

## Features

- **Physics simulation** — quaternion-based orientation, Betaflight-style rate curves, linear drag, motor response modeling
- **Two render modes** — HalfBlock (unicode characters, works everywhere) and Kitty Graphics Protocol (real pixels, double-buffered)
- **Radio controller support** — plug in a RadioMaster TX16S/TX15 or any USB gamepad (PS5, Xbox) via `gilrs`
- **Keyboard controls** — smoothed analog-style input from binary keys
- **Freestyle course** — pillars, ramps, walls, gates, a bando, and slalom obstacles
- **Collision detection** — AABB-based obstacle collision with bounce/crash
- **Bitmap HUD** — throttle, altitude, speed, flight timer rendered as pixels into the framebuffer
- **Near-plane clipping** — proper Sutherland-Hodgman triangle clipping for close-up rendering
- **Frustum culling** — bounding sphere culling skips off-screen meshes

## Controls

### Keyboard

| Key | Action |
|-----|--------|
| `W` / `S` | Throttle up / down |
| `A` / `D` | Yaw left / right |
| `I` / `K` | Pitch forward / back |
| `J` / `L` | Roll left / right |
| `R` | Reset after crash |
| `Tab` | Toggle render mode (HalfBlock / Kitty) |
| `Q` / `Esc` | Quit |

### Radio Controller / Gamepad

Plug in via USB. Mode 2 mapping (standard):
- Left stick Y = throttle, X = yaw
- Right stick Y = pitch, X = roll

## Build & Run

```sh
cargo run --release
```

## Render Modes

**HalfBlock** — uses `▀` characters with foreground/background colors. Works in any terminal with truecolor support. 1x2 pixels per cell.

**Kitty Graphics Protocol** — sends real pixel data to the terminal. Double-buffered for flicker-free rendering. Best experience in [Kitty terminal](https://sw.kovidgoyal.net/kitty/). Also works in WezTerm, Ghostty, and Foot.

For the best Kitty mode experience, reduce your terminal font size for higher resolution.

## Dependencies

- [ratatui](https://ratatui.rs/) + [crossterm](https://docs.rs/crossterm/) — terminal UI
- [nalgebra](https://nalgebra.org/) — linear algebra, quaternions, projections
- [gilrs](https://gitlab.com/gilrs-project/gilrs) — gamepad/joystick input
- [base64](https://docs.rs/base64/) — Kitty graphics protocol encoding
