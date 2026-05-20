use gilrs::{Axis, EventType, Gilrs};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::controls::StickState;

/// Assignment of a gilrs axis to a drone control.
#[derive(Clone)]
pub struct AxisAssignment {
    pub axis: Axis,
    pub inverted: bool,
}

/// Configurable mapping from gamepad axes to drone controls.
pub struct AxisMapping {
    pub throttle: AxisAssignment,
    pub yaw: AxisAssignment,
    pub pitch: AxisAssignment,
    pub roll: AxisAssignment,
}

impl Default for AxisMapping {
    /// Mode 2 defaults (most common RC layout).
    fn default() -> Self {
        Self {
            throttle: AxisAssignment { axis: Axis::LeftStickY, inverted: false },
            yaw: AxisAssignment { axis: Axis::LeftStickX, inverted: false },
            pitch: AxisAssignment { axis: Axis::RightStickY, inverted: true },
            roll: AxisAssignment { axis: Axis::RightStickX, inverted: false },
        }
    }
}

fn config_path() -> PathBuf {
    let mut path = dirs_home().unwrap_or_else(|| PathBuf::from("."));
    path.push(".config");
    path.push("flightty");
    path
}


fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn axis_to_str(axis: &Axis) -> String {
    match axis {
        Axis::LeftStickX => "LeftStickX".into(),
        Axis::LeftStickY => "LeftStickY".into(),
        Axis::RightStickX => "RightStickX".into(),
        Axis::RightStickY => "RightStickY".into(),
        Axis::LeftZ => "LeftZ".into(),
        Axis::RightZ => "RightZ".into(),
        Axis::DPadX => "DPadX".into(),
        Axis::DPadY => "DPadY".into(),
        Axis::Unknown => "Unknown".into(),
    }
}

fn str_to_axis(s: &str) -> Option<Axis> {
    match s.trim() {
        "LeftStickX" => Some(Axis::LeftStickX),
        "LeftStickY" => Some(Axis::LeftStickY),
        "RightStickX" => Some(Axis::RightStickX),
        "RightStickY" => Some(Axis::RightStickY),
        "LeftZ" => Some(Axis::LeftZ),
        "RightZ" => Some(Axis::RightZ),
        "DPadX" => Some(Axis::DPadX),
        "DPadY" => Some(Axis::DPadY),
        "Unknown" => Some(Axis::Unknown),
        _ => None,
    }
}

impl AxisMapping {
    /// Get assignment by index (0=throttle, 1=yaw, 2=pitch, 3=roll).
    pub fn get_mut(&mut self, index: usize) -> &mut AxisAssignment {
        match index {
            0 => &mut self.throttle,
            1 => &mut self.yaw,
            2 => &mut self.pitch,
            3 => &mut self.roll,
            _ => panic!("invalid axis index"),
        }
    }

    /// Save axis mapping for a specific controller.
    pub fn save(&self, controller_name: &str) {
        let dir = config_path();
        let _ = fs::create_dir_all(&dir);

        let content = format!(
            "throttle={},{}\nyaw={},{}\npitch={},{}\nroll={},{}\n",
            axis_to_str(&self.throttle.axis), self.throttle.inverted,
            axis_to_str(&self.yaw.axis), self.yaw.inverted,
            axis_to_str(&self.pitch.axis), self.pitch.inverted,
            axis_to_str(&self.roll.axis), self.roll.inverted,
        );

        let safe_name = sanitize_filename(controller_name);
        let _ = fs::write(config_path().join(format!("{}.conf", safe_name)), content);
    }

    /// Load axis mapping for a specific controller. Returns default if not found.
    pub fn load(controller_name: &str) -> Self {
        let safe_name = sanitize_filename(controller_name);
        let path = config_path().join(format!("{}.conf", safe_name));
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        let mut mapping = Self::default();

        for line in content.lines() {
            let Some((key, rest)) = line.split_once('=') else { continue };
            let Some((axis_str, inv_str)) = rest.rsplit_once(',') else { continue };
            let Some(axis) = str_to_axis(axis_str) else { continue };
            let inverted = inv_str.trim() == "true";
            let assignment = AxisAssignment { axis, inverted };

            match key.trim() {
                "throttle" => mapping.throttle = assignment,
                "yaw" => mapping.yaw = assignment,
                "pitch" => mapping.pitch = assignment,
                "roll" => mapping.roll = assignment,
                _ => {}
            }
        }

        mapping
    }
}

/// Sanitize controller name for use as a filename.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
        .collect()
}

/// All axes we check when listening for stick input.
#[allow(dead_code)]
pub const ALL_AXES: [Axis; 8] = [
    Axis::LeftStickX,
    Axis::LeftStickY,
    Axis::RightStickX,
    Axis::RightStickY,
    Axis::LeftZ,
    Axis::RightZ,
    Axis::DPadX,
    Axis::DPadY,
];

/// Manages gamepad/radio controller input via gilrs.
pub struct GamepadInput {
    gilrs: Gilrs,
    pub connected: bool,
    pub name: String,
    pub sticks: StickState,
    pub mapping: AxisMapping,
    /// Last known values for ALL axes seen via events (including unmapped ones).
    pub live_axes: HashMap<Axis, f32>,
}

impl GamepadInput {
    pub fn new() -> Option<Self> {
        let gilrs = Gilrs::new().ok()?;

        let mut connected = false;
        let mut name = String::new();
        if let Some((_id, gamepad)) = gilrs.gamepads().next() {
            connected = true;
            name = gamepad.name().to_string();
        }

        let mapping = if connected { AxisMapping::load(&name) } else { AxisMapping::default() };

        Some(Self {
            gilrs,
            connected,
            name,
            sticks: StickState::default(),
            mapping,
            live_axes: HashMap::new(),
        })
    }

    fn read_axis(&self, assignment: &AxisAssignment) -> f32 {
        // Read from live_axes (populated by events). Works for all axes including Unknown.
        let val = self.live_axes.get(&assignment.axis).copied().unwrap_or(0.0);
        if assignment.inverted { -val } else { val }
    }

    /// Poll gamepad events and update stick state.
    pub fn poll(&mut self) -> bool {
        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                EventType::Connected => {
                    let gp = self.gilrs.gamepad(event.id);
                    self.connected = true;
                    self.name = gp.name().to_string();
                    self.mapping = AxisMapping::load(&self.name);
                    self.live_axes.clear();
                }
                EventType::Disconnected => {
                    self.connected = false;
                    self.name.clear();
                }
                EventType::AxisChanged(axis, value, _) => {
                    self.live_axes.insert(axis, value);
                }
                _ => {}
            }
        }

        if !self.connected {
            return false;
        }

        // Read all axes from event-based live values (works for Unknown axes too)
        let raw_throttle = self.read_axis(&self.mapping.throttle.clone());
        self.sticks.throttle = (raw_throttle * 0.5 + 0.5).clamp(0.0, 1.0);
        self.sticks.yaw = self.read_axis(&self.mapping.yaw.clone());
        self.sticks.pitch = self.read_axis(&self.mapping.pitch.clone());
        self.sticks.roll = self.read_axis(&self.mapping.roll.clone());

        true
    }

    /// Get all axis values for the debug display (from events, catches everything).
    pub fn all_axis_values(&self) -> Vec<(Axis, f32)> {
        let mut result: Vec<(Axis, f32)> = self
            .live_axes
            .iter()
            .filter(|(_, v)| v.abs() > 0.01)
            .map(|(&a, &v)| (a, v))
            .collect();
        result.sort_by(|a, b| format!("{:?}", a.0).cmp(&format!("{:?}", b.0)));
        result
    }

    /// Find the axis with the largest deflection (from events, catches everything).
    pub fn detect_axis(&self) -> Option<Axis> {
        let mut best_axis = None;
        let mut best_val = 0.5f32;

        for (&axis, &value) in &self.live_axes {
            if value.abs() > best_val {
                best_val = value.abs();
                best_axis = Some(axis);
            }
        }

        best_axis
    }
}
