//! Wave_ref style waveform terrain — dense white particles on black.
//!
//! Run: cargo run --example terrain

#[path = "common/mod.rs"]
mod common;

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_splat::*;
use common::{hash, hash2};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_splat — terrain".into(),
                resolution: bevy::window::WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(SplatPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        .add_systems(Update, common::take_screenshot)
        .run();
}

/// Terrain height function — dramatic central mountain with ridges.
fn terrain_height(x_frac: f32) -> f32 {
    // Dominant central peak (left of center, tall and narrow)
    let center = 0.35;
    let sigma = 0.14;
    let dx = x_frac - center;
    let central = (-dx * dx / (2.0 * sigma * sigma)).exp();

    // Secondary peaks creating ridge structure
    let center2 = 0.55;
    let sigma2 = 0.10;
    let dx2 = x_frac - center2;
    let secondary = 0.45 * (-dx2 * dx2 / (2.0 * sigma2 * sigma2)).exp();

    let center3 = 0.72;
    let sigma3 = 0.08;
    let dx3 = x_frac - center3;
    let tertiary = 0.25 * (-dx3 * dx3 / (2.0 * sigma3 * sigma3)).exp();

    // Multi-frequency ridges for texture
    let t = x_frac * std::f32::consts::TAU;
    let ridges = 0.6
        + 0.25 * (t * 8.0).sin().abs()
        + 0.12 * (t * 15.0 + 0.7).sin().abs()
        + 0.08 * (t * 27.0 + 2.1).sin().abs()
        + 0.04 * (t * 50.0 + 0.3).sin().abs();

    // Envelope: fade in/out at edges
    let envelope = (x_frac * 10.0).min(1.0) * ((1.0 - x_frac) * 8.0).min(1.0);

    (central + secondary + tertiary) * ridges * envelope
}

fn setup(mut commands: Commands, mut splats: ResMut<Assets<Splat>>) {
    let terrain_height_scale = 14.0;
    let terrain_width = 26.0;
    let terrain_depth = 4.0;

    let mut points = Vec::with_capacity(300_000);

    // === Layer 1: Dense surface grid (~120K) ===
    let cols = 900;
    let rows = 180;
    for col in 0..cols {
        let x_frac = col as f32 / (cols - 1) as f32;
        let x = x_frac * terrain_width - terrain_width / 2.0;
        let base_h = terrain_height(x_frac) * terrain_height_scale;

        for row in 0..rows {
            let z_frac = row as f32 / (rows - 1) as f32;
            let z = (z_frac - 0.5) * terrain_depth;

            // Sharper ridge taper: gaussian cross-section instead of parabolic
            let z_norm = 2.0 * z_frac - 1.0;
            let taper = (-z_norm * z_norm * 3.0).exp();

            let scatter = hash(col as f32 * 0.73, row as f32 * 1.17);
            let y = base_h * taper * (0.75 + scatter * 0.25);

            if y < 0.05 {
                continue;
            }

            // Brighter at higher elevations — push toward white at peaks
            let h_ratio = (y / terrain_height_scale).min(1.0);
            let brightness = h_ratio.powf(0.3) * 0.85 + 0.15;

            points.push(SplatPoint::new(
                Vec3::new(x, y, z),
                1.3,
                Vec4::new(brightness, brightness, brightness * 1.02, brightness),
            ));
        }
    }

    // === Layer 2: Volume fill (~120K) ===
    let volume_count = 150_000;
    for i in 0..volume_count {
        let r1 = hash(i as f32 * 0.1234, i as f32 * 0.5678);
        let r2 = hash2(i as f32 * 0.9876, i as f32 * 0.3456);
        let r3 = hash(i as f32 * 0.2468 + 100.0, i as f32 * 0.1357);
        let r4 = hash2(i as f32 * 0.8642, i as f32 * 0.7531 + 50.0);

        let x_frac = r1;
        let x = x_frac * terrain_width - terrain_width / 2.0;
        let base_h = terrain_height(x_frac) * terrain_height_scale;

        if base_h < 0.3 {
            continue;
        }

        let z_frac = r2;
        let z = (z_frac - 0.5) * terrain_depth;
        let z_norm = 2.0 * z_frac - 1.0;
        let taper = (-z_norm * z_norm * 3.0).exp();

        // Bias Y toward the surface: use r3^2 to concentrate near surface
        let y = r3 * r3 * base_h * taper * 1.05;
        if y < 0.05 {
            continue;
        }

        // Probability falloff: more likely to keep points near surface
        let height_ratio = y / (base_h * taper).max(0.01);
        if r4 > (1.0 - height_ratio * 0.3).max(0.15) {
            continue;
        }

        let h_ratio = (y / terrain_height_scale).min(1.0);
        let brightness = h_ratio.powf(0.5) * 0.7 + 0.1;

        points.push(SplatPoint::new(
            Vec3::new(x, y, z),
            1.0,
            Vec4::new(brightness, brightness, brightness * 1.03, brightness * 0.85),
        ));
    }

    // === Layer 3: Spray/mist (~25K) ===
    let spray_count = 40_000;
    for i in 0..spray_count {
        let r1 = hash(i as f32 * 0.3141 + 500.0, i as f32 * 0.2718);
        let r2 = hash2(i as f32 * 0.1618 + 200.0, i as f32 * 0.4142);
        #[allow(clippy::approx_constant)]
        let r3 = hash(i as f32 * 0.7071 + 300.0, i as f32 * 0.5774);
        let r4 = hash2(i as f32 * 0.4321 + 400.0, i as f32 * 0.8765);

        let x_frac = r1;
        let x = x_frac * terrain_width - terrain_width / 2.0;
        let base_h = terrain_height(x_frac) * terrain_height_scale;

        if base_h < 0.8 {
            continue;
        }

        let z = (r2 - 0.5) * terrain_depth * 1.5; // wider Z spread

        // Y: extends above the surface, biased toward top
        let y = base_h * (0.6 + r3 * 0.8);

        // Sparser further from peak
        if r4 > (base_h / terrain_height_scale).powf(0.3) {
            continue;
        }

        let brightness = 0.1 + r3 * 0.3;

        points.push(SplatPoint::new(
            Vec3::new(x, y, z),
            0.7,
            Vec4::new(brightness, brightness, brightness, brightness * 0.5),
        ));
    }

    info!("Total points: {}", points.len());

    commands.spawn(Splat3d(splats.add(Splat::new(points))));

    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Transform::from_xyz(2.0, 5.0, 20.0).looking_at(Vec3::new(-1.0, 5.0, 0.0), Vec3::Y),
        PanOrbitCamera {
            target_focus: Vec3::new(-1.0, 5.0, 0.0),
            ..default()
        },
        NoIndirectDrawing,
    ));
}
