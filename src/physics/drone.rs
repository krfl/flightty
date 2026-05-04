use nalgebra::{Point3, UnitQuaternion, Vector3};

use super::integrator;
use crate::input::controls::StickState;
use crate::input::rate_curve::apply_rate_curve;
use crate::world::mesh::Aabb;

pub struct DroneConfig {
    pub mass: f32,
    pub max_thrust: f32,
    pub drag_coeff: f32,
    pub angular_drag_coeff: f32,
    pub rate_tracking_speed: f32,
    pub camera_tilt_deg: f32,
    pub max_rate_deg_s: f32,
    pub rc_rate: f32,
    pub expo: f32,
    pub super_rate: f32,
}

impl Default for DroneConfig {
    fn default() -> Self {
        Self {
            mass: 0.250,       // 250g
            max_thrust: 7.0,   // hover at ~35% throttle (3:1 thrust-to-weight)
            drag_coeff: 0.012, // drag force: freefall ~14m/s, full thrust ~20m/s
            angular_drag_coeff: 0.04,
            rate_tracking_speed: 25.0,
            camera_tilt_deg: 30.0,
            max_rate_deg_s: 250.0, // faster flips and rolls
            rc_rate: 0.9,
            expo: 0.35,
            super_rate: 0.6,
        }
    }
}

pub struct DroneState {
    pub position: Point3<f32>,
    pub velocity: Vector3<f32>,
    pub orientation: UnitQuaternion<f32>,
    pub angular_velocity: Vector3<f32>,
    pub crashed: bool,
}

impl DroneState {
    pub fn new(position: Point3<f32>, orientation: UnitQuaternion<f32>) -> Self {
        Self {
            position,
            velocity: Vector3::zeros(),
            orientation,
            angular_velocity: Vector3::zeros(),
            crashed: false,
        }
    }
}

const GRAVITY: Vector3<f32> = Vector3::new(0.0, -9.81, 0.0);

pub fn physics_step(state: &mut DroneState, config: &DroneConfig, sticks: &StickState, colliders: &[Aabb], dt: f32) {
    if state.crashed {
        return;
    }

    // Convert stick inputs to desired angular rates via Betaflight rate curves
    let desired_roll = apply_rate_curve(sticks.roll, config.rc_rate, config.expo, config.super_rate)
        * config.max_rate_deg_s.to_radians();
    let desired_pitch =
        apply_rate_curve(sticks.pitch, config.rc_rate, config.expo, config.super_rate)
            * config.max_rate_deg_s.to_radians();
    let desired_yaw =
        apply_rate_curve(sticks.yaw, config.rc_rate, config.expo, config.super_rate)
            * config.max_rate_deg_s.to_radians()
            * 0.7;

    // Rate tracking: blend angular velocity toward desired rates
    // This simulates the flight controller's PID loop
    // Drone faces -Z, so: pitch=X rotation, yaw=-Y rotation, roll=-Z rotation
    let desired_omega = Vector3::new(desired_pitch, -desired_yaw, -desired_roll);
    let blend = (config.rate_tracking_speed * dt).min(1.0);
    state.angular_velocity = state.angular_velocity * (1.0 - blend) + desired_omega * blend;

    // Apply angular drag
    state.angular_velocity -= state.angular_velocity * config.angular_drag_coeff * dt;

    // Integrate quaternion orientation
    state.orientation =
        integrator::integrate_quaternion(&state.orientation, &state.angular_velocity, dt);

    // Thrust: throttle controls total force along body Y-up axis
    let thrust_magnitude = sticks.throttle * config.max_thrust;
    let thrust_body = Vector3::new(0.0, thrust_magnitude, 0.0);
    let thrust_world = state.orientation * thrust_body;

    // Quadratic drag as force: F_drag = -k * v * |v|, then divide by mass for acceleration
    let speed = state.velocity.norm();
    let drag_force = -state.velocity * config.drag_coeff * speed;
    let acceleration = (thrust_world + drag_force) / config.mass + GRAVITY;

    // Semi-implicit Euler
    state.velocity += acceleration * dt;
    state.position += state.velocity * dt;

    // Ground collision
    if state.position.y < 0.05 {
        state.position.y = 0.05;
        if state.velocity.y < -3.0 {
            state.crashed = true;
            state.velocity = Vector3::zeros();
            state.angular_velocity = Vector3::zeros();
        } else {
            state.velocity.y = state.velocity.y.abs() * 0.2;
            state.velocity.x *= 0.9;
            state.velocity.z *= 0.9;
        }
    }

    // Obstacle collision
    let drone_radius = 0.08; // ~65mm whoop radius
    let speed = state.velocity.norm();
    for aabb in colliders {
        if let Some(push) = aabb.push_out(&state.position, drone_radius) {
            // Push drone out of obstacle
            state.position += push;

            if speed > 3.0 {
                // Hard crash at speed
                state.crashed = true;
                state.velocity = Vector3::zeros();
                state.angular_velocity = Vector3::zeros();
                return;
            } else {
                // Bounce off: reflect velocity along push direction
                let push_norm = push.normalize();
                let vel_into = state.velocity.dot(&push_norm);
                if vel_into < 0.0 {
                    state.velocity -= push_norm * vel_into * 1.5; // bounce
                }
                state.velocity *= 0.7; // lose energy
            }
        }
    }
}
