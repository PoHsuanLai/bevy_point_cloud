#![allow(dead_code, clippy::excessive_precision)]

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

pub fn hash(x: f32, y: f32) -> f32 {
    let h = x * 127.1 + y * 311.7;
    (h.sin() * 43758.5453).fract().abs()
}

pub fn hash2(x: f32, y: f32) -> f32 {
    let h = x * 269.5 + y * 183.3;
    (h.sin() * 28947.7134).fract().abs()
}

pub fn take_screenshot(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::KeyS) {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk("/tmp/point_cloud_*.png"));
        info!("Screenshot -> /tmp/point_cloud_*.png");
    }
}
