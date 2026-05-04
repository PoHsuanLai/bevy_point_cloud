//! Ryoji Ikeda / Wave_ref style data dashboard — dense white particles on black.
//!
//! Composites multiple point cloud elements: 3D terrain, grid lines,
//! waveforms, bar charts, and scatter data.
//!
//! Run: cargo run --example dashboard
//!
//! Controls:
//!   - Orbit camera with mouse
//!   - Press S to take a screenshot

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
                title: "bevy_splat — dashboard".into(),
                resolution: bevy::window::WindowResolution::new(1280, 900),
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

fn terrain_height(x_frac: f32) -> f32 {
    let center = 0.35;
    let sigma = 0.14;
    let dx = x_frac - center;
    let central = (-dx * dx / (2.0 * sigma * sigma)).exp();

    let center2 = 0.55;
    let sigma2 = 0.10;
    let dx2 = x_frac - center2;
    let secondary = 0.45 * (-dx2 * dx2 / (2.0 * sigma2 * sigma2)).exp();

    let center3 = 0.72;
    let sigma3 = 0.08;
    let dx3 = x_frac - center3;
    let tertiary = 0.25 * (-dx3 * dx3 / (2.0 * sigma3 * sigma3)).exp();

    let t = x_frac * std::f32::consts::TAU;
    let ridges = 0.6
        + 0.25 * (t * 8.0).sin().abs()
        + 0.12 * (t * 15.0 + 0.7).sin().abs()
        + 0.08 * (t * 27.0 + 2.1).sin().abs()
        + 0.04 * (t * 50.0 + 0.3).sin().abs();

    // Soft envelope: saturates to 1.0 quickly, trails off gently at edges
    let fade_in = (x_frac / 0.08).min(1.0).powi(2);
    let fade_out = ((1.0 - x_frac) / 0.1).min(1.0).powi(2);
    let envelope = fade_in * fade_out;
    // Small floor so edges still have sparse particles instead of nothing
    let floor = 0.04 * (0.5 - (x_frac - 0.5).abs()).max(0.0);
    (central + secondary + tertiary) * ridges * envelope + floor
}

// ── Waveform generators ──

fn waveform_a(x_frac: f32) -> f32 {
    let t = x_frac * std::f32::consts::TAU;
    let base = (t * 3.0).sin() * 0.4
        + (t * 7.0 + 1.0).sin() * 0.2
        + (t * 13.0 + 2.5).sin() * 0.1
        + (t * 29.0).sin() * 0.05;
    // Modulated envelope
    let env = (t * 1.5).sin().abs().powf(0.5);
    base * env
}

fn waveform_b(x_frac: f32) -> f32 {
    let t = x_frac * std::f32::consts::TAU;
    // Spiky, noisy waveform
    let spike = (t * 5.0).sin().powi(3) * 0.6;
    let noise = hash(x_frac * 200.0, 42.0) * 0.3 - 0.15;
    let slow = (t * 0.8 + 0.5).sin() * 0.2;
    (spike + noise + slow) * (1.0 - (x_frac - 0.5).abs() * 1.5).max(0.0)
}

fn waveform_c(x_frac: f32) -> f32 {
    // Stepped / quantized waveform
    let t = x_frac * 20.0;
    let step = t.floor();
    let level = hash(step, 77.0) * 2.0 - 1.0;
    level * 0.5 * (1.0 - (x_frac - 0.3).abs()).max(0.0)
}

// ── Point generators ──

#[allow(clippy::too_many_arguments)]
fn make_grid(
    x0: f32,
    y0: f32,
    width: f32,
    height: f32,
    h_lines: usize,
    v_lines: usize,
    density: usize,
    brightness: f32,
) -> Vec<SplatPoint> {
    let mut pts = Vec::new();
    let alpha = brightness * 0.8;

    // Horizontal lines
    for i in 0..=h_lines {
        let y = y0 + height * i as f32 / h_lines as f32;
        for j in 0..density {
            let x = x0 + width * j as f32 / (density - 1) as f32;
            pts.push(SplatPoint::new(
                Vec3::new(x, y, 0.0),
                0.8,
                Vec4::new(brightness, brightness, brightness, alpha),
            ));
        }
    }

    // Vertical lines
    for i in 0..=v_lines {
        let x = x0 + width * i as f32 / v_lines as f32;
        for j in 0..density {
            let y = y0 + height * j as f32 / (density - 1) as f32;
            pts.push(SplatPoint::new(
                Vec3::new(x, y, 0.0),
                0.8,
                Vec4::new(brightness, brightness, brightness, alpha),
            ));
        }
    }

    pts
}

#[allow(clippy::too_many_arguments)]
fn make_waveform(
    x0: f32,
    y0: f32,
    width: f32,
    height: f32,
    samples: usize,
    f: impl Fn(f32) -> f32,
    brightness: f32,
    point_size: f32,
) -> Vec<SplatPoint> {
    let mut pts = Vec::with_capacity(samples);
    let mid_y = y0 + height * 0.5;

    for i in 0..samples {
        let x_frac = i as f32 / (samples - 1) as f32;
        let x = x0 + x_frac * width;
        let val = f(x_frac);
        let y = mid_y + val * height * 0.5;

        let b = brightness * (0.6 + 0.4 * val.abs().min(1.0));
        pts.push(SplatPoint::new(
            Vec3::new(x, y, 0.0),
            point_size,
            Vec4::new(b, b, b * 1.05, brightness),
        ));
    }
    pts
}

fn make_bar_chart(
    x0: f32,
    y0: f32,
    width: f32,
    height: f32,
    num_bars: usize,
    density_per_bar: usize,
    seed: f32,
) -> Vec<SplatPoint> {
    let mut pts = Vec::new();
    let bar_width = width / (num_bars as f32 * 1.5);
    let gap = bar_width * 0.5;

    for bar in 0..num_bars {
        let bar_x = x0 + (bar_width + gap) * bar as f32;
        let bar_height = hash(bar as f32 * 0.37 + seed, seed * 1.7) * height;

        for i in 0..density_per_bar {
            let frac = i as f32 / (density_per_bar - 1) as f32;
            let y = y0 + frac * bar_height;
            let x = bar_x + hash(i as f32, bar as f32 + seed) * bar_width;

            let h_ratio = frac;
            let brightness = 0.3 + 0.7 * h_ratio;
            pts.push(SplatPoint::new(
                Vec3::new(x, y, 0.0),
                1.0,
                Vec4::new(brightness, brightness, brightness, 0.9),
            ));
        }
    }
    pts
}

fn make_scatter(
    x0: f32,
    y0: f32,
    width: f32,
    height: f32,
    count: usize,
    seed: f32,
) -> Vec<SplatPoint> {
    let mut pts = Vec::with_capacity(count);

    for i in 0..count {
        let rx = hash(i as f32 * 0.123 + seed, seed + 0.456);
        let ry = hash2(i as f32 * 0.789 + seed, seed + 0.321);
        let x = x0 + rx * width;
        // Clustered distribution — bias toward a curve
        let curve = (rx * std::f32::consts::PI).sin() * 0.6;
        let y = y0 + (curve + (ry - 0.5) * 0.4 + 0.5).clamp(0.0, 1.0) * height;

        let brightness = 0.3 + 0.5 * hash(i as f32, seed * 3.0);
        pts.push(SplatPoint::new(
            Vec3::new(x, y, 0.0),
            1.2,
            Vec4::new(brightness, brightness * 1.1, brightness, 0.7),
        ));
    }
    pts
}

fn make_terrain_3d() -> Vec<SplatPoint> {
    let terrain_height_scale = 10.0;
    let terrain_width = 24.0;
    let terrain_depth = 3.5;
    let x_offset = -12.0;
    let y_offset = 10.0;

    let mut points = Vec::with_capacity(250_000);

    // Surface grid — keep all points, dim the low ones instead of skipping
    let cols = 800;
    let rows = 140;
    for col in 0..cols {
        let x_frac = col as f32 / (cols - 1) as f32;
        let x = x_offset + x_frac * terrain_width;
        let base_h = terrain_height(x_frac) * terrain_height_scale;

        for row in 0..rows {
            let z_frac = row as f32 / (rows - 1) as f32;
            let z = (z_frac - 0.5) * terrain_depth;
            let z_norm = 2.0 * z_frac - 1.0;
            let taper = (-z_norm * z_norm * 3.0).exp();
            let scatter = hash(col as f32 * 0.73, row as f32 * 1.17);
            let h = base_h * taper * (0.75 + scatter * 0.25);
            let y = y_offset + h;

            // Dim low points instead of skipping — creates trailing falloff
            let h_ratio = (h / terrain_height_scale).min(1.0);
            let brightness = h_ratio.powf(0.3) * 0.85 + 0.15;
            let alpha = if h < 0.3 {
                // Very low points: sparse/dim, stochastic skip
                let keep_chance = (h / 0.3).max(0.05);
                if scatter > keep_chance {
                    continue;
                }
                brightness * 0.4
            } else {
                brightness
            };

            points.push(SplatPoint::new(
                Vec3::new(x, y, z),
                1.2,
                Vec4::new(brightness, brightness, brightness * 1.02, alpha),
            ));
        }
    }

    // Volume fill — also keep low-height areas with reduced density
    for i in 0..120_000 {
        let r1 = hash(i as f32 * 0.1234, i as f32 * 0.5678);
        let r2 = hash2(i as f32 * 0.9876, i as f32 * 0.3456);
        let r3 = hash(i as f32 * 0.2468 + 100.0, i as f32 * 0.1357);
        let r4 = hash2(i as f32 * 0.8642, i as f32 * 0.7531 + 50.0);

        let x_frac = r1;
        let x = x_offset + x_frac * terrain_width;
        let base_h = terrain_height(x_frac) * terrain_height_scale;

        // Allow low-height areas but with reduced probability
        if base_h < 0.3 && r4 > 0.15 {
            continue;
        }

        let z = (r2 - 0.5) * terrain_depth;
        let z_norm = 2.0 * r2 - 1.0;
        let taper = (-z_norm * z_norm * 3.0).exp();
        let h = r3 * r3 * base_h.max(0.1) * taper * 1.05;
        let y = y_offset + h;

        if h < 0.01 {
            continue;
        }

        let height_ratio = h / (base_h * taper).max(0.01);
        if r4 > (1.0 - height_ratio * 0.3).max(0.15) {
            continue;
        }

        let h_ratio = (h / terrain_height_scale).min(1.0);
        let brightness = h_ratio.powf(0.5) * 0.7 + 0.1;

        points.push(SplatPoint::new(
            Vec3::new(x, y, z),
            0.9,
            Vec4::new(brightness, brightness, brightness * 1.03, brightness * 0.85),
        ));
    }

    // Edge scatter — sparse particles trailing off at the sides
    for i in 0..15_000 {
        let r1 = hash(i as f32 * 0.567 + 700.0, i as f32 * 0.891);
        let r2 = hash2(i as f32 * 0.234 + 800.0, i as f32 * 0.654);
        let r3 = hash(i as f32 * 0.345 + 900.0, i as f32 * 0.987);

        let x_frac = r1;
        let x = x_offset + x_frac * terrain_width;
        let base_h = terrain_height(x_frac) * terrain_height_scale;

        // Bias toward edges: higher chance of keeping if base_h is low
        let edge_factor = 1.0 - (base_h / terrain_height_scale).min(1.0);
        if r2 > edge_factor * 0.5 + 0.1 {
            continue;
        }

        let z = (r2 - 0.5) * terrain_depth * 1.3;
        let y = y_offset + r3 * base_h.max(0.5) * 0.6;

        let brightness = 0.08 + r3 * 0.2;
        points.push(SplatPoint::new(
            Vec3::new(x, y, z),
            0.7,
            Vec4::new(brightness, brightness, brightness, brightness * 0.6),
        ));
    }

    points
}

fn setup(mut commands: Commands, mut splats: ResMut<Assets<Splat>>) {
    // Layout constants (world units, Z=0 plane for 2D elements)
    let panel_w = 24.0;
    let left = -12.0;

    // ── 3D Terrain (top, Y=10..22) ──
    let terrain = make_terrain_3d();
    commands.spawn(Splat3d(splats.add(Splat::new(terrain))));

    // ── Panel 1: Waveform A with grid (Y=5..9) ──
    let mut panel1 = make_grid(left, 5.0, panel_w, 4.0, 4, 12, 300, 0.15);
    panel1.extend(make_waveform(
        left, 5.0, panel_w, 4.0, 2000, waveform_a, 0.9, 1.3,
    ));
    commands.spawn(Splat3d(splats.add(Splat::new(panel1))));

    // ── Panel 2: Two overlaid waveforms (Y=0..4) ──
    let mut panel2 = make_grid(left, 0.0, panel_w, 4.0, 4, 12, 300, 0.12);
    panel2.extend(make_waveform(
        left, 0.0, panel_w, 4.0, 2000, waveform_b, 0.8, 1.2,
    ));
    panel2.extend(make_waveform(
        left, 0.0, panel_w, 4.0, 2000, waveform_c, 0.5, 1.0,
    ));
    commands.spawn(Splat3d(splats.add(Splat::new(panel2))));

    // ── Panel 3: Bar chart + scatter (Y=-6..-1) ──
    // Left half: bar chart
    let mut panel3 = make_grid(left, -6.0, panel_w, 5.0, 5, 12, 250, 0.1);
    panel3.extend(make_bar_chart(left + 0.5, -6.0, 10.0, 4.5, 24, 80, 1.0));
    // Right half: scatter plot
    panel3.extend(make_scatter(left + 13.0, -6.0, 10.0, 5.0, 3000, 2.0));
    commands.spawn(Splat3d(splats.add(Splat::new(panel3))));

    // ── Panel 4: Dense waveform strip (Y=-8..-7) ──
    let mut panel4 = Vec::new();
    // Multiple thin waveforms stacked
    for row in 0..5 {
        let y_base = -8.0 + row as f32 * 0.35;
        let seed = row as f32 * 3.7;
        for i in 0..1500 {
            let x_frac = i as f32 / 1499.0;
            let x = left + x_frac * panel_w;
            let val = (x_frac * std::f32::consts::TAU * (4.0 + seed)).sin()
                * 0.12
                * (1.0 - (x_frac - 0.5).abs() * 1.8).max(0.0);
            let y = y_base + val;
            let brightness = 0.3 + 0.5 * hash(i as f32 + seed, seed);
            panel4.push(SplatPoint::new(
                Vec3::new(x, y, 0.0),
                0.9,
                Vec4::new(brightness, brightness, brightness, 0.7),
            ));
        }
    }
    commands.spawn(Splat3d(splats.add(Splat::new(panel4))));

    // ── Horizontal separator lines ──
    let mut separators = Vec::new();
    for &y in &[9.5, 4.5, -0.5, -6.5] {
        for i in 0..600 {
            let x = left + panel_w * i as f32 / 599.0;
            separators.push(SplatPoint::new(
                Vec3::new(x, y, 0.0),
                0.6,
                Vec4::new(0.25, 0.25, 0.25, 0.5),
            ));
        }
    }
    commands.spawn(Splat3d(splats.add(Splat::new(separators))));

    let total = 200_000 // terrain approx
        + 2000 + 300 * 17 // panel1
        + 4000 + 300 * 17 // panel2
        + 3000 + 24 * 80 + 250 * 18 // panel3
        + 5 * 1500 // panel4
        + 4 * 600; // separators
    info!("Dashboard total points: ~{total}");

    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Transform::from_xyz(0.0, 6.0, 35.0).looking_at(Vec3::new(0.0, 6.0, 0.0), Vec3::Y),
        PanOrbitCamera {
            target_focus: Vec3::new(0.0, 6.0, 0.0),
            ..default()
        },
        NoIndirectDrawing,
    ));
}
