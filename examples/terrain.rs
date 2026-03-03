//! Wave_ref style waveform terrain — dense white particles on black.
//!
//! Run: cargo run --example terrain

use bevy::prelude::*;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::render::storage::ShaderStorageBuffer;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_point_cloud::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_point_cloud — terrain".into(),
                resolution: bevy::window::WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PointCloudPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        .add_systems(Update, (take_screenshot, auto_screenshot))
        .run();
}

#[derive(Resource)]
struct ScreenshotTimer(u32);

/// Simple hash for pseudo-random scatter.
fn hash(x: f32, y: f32) -> f32 {
    let h = x * 127.1 + y * 311.7;
    (h.sin() * 43758.5453).fract().abs()
}

/// Generate synthetic waveform peaks resembling audio.
fn generate_peaks(num_peaks: usize, freq: f32, noise: f32) -> Vec<f32> {
    (0..num_peaks)
        .map(|i| {
            let t = i as f32 / num_peaks as f32;
            let envelope = (t * 5.0).min(1.0) * ((1.0 - t) * 4.0).min(1.0);
            let s1 = (t * freq * std::f32::consts::TAU).sin().abs();
            let s2 = (t * freq * 2.3 * std::f32::consts::TAU + 0.7).sin().abs() * 0.4;
            let s3 = (t * freq * 0.5 * std::f32::consts::TAU + 1.3).sin().abs() * 0.3;
            let hash_val = ((i as f32 * 0.618033988) % 1.0 * 2.0 - 1.0).abs();
            ((s1 + s2 + s3 + hash_val * noise) * envelope * 0.5).clamp(0.0, 1.0)
        })
        .collect()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PointCloudMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    commands.insert_resource(ScreenshotTimer(0));

    let cols = 400;
    let rows = 80;
    let terrain_height = 6.0;
    let terrain_width = 30.0;
    let terrain_depth = 8.0;

    let peaks = generate_peaks(cols, 5.0, 0.3);
    let mut points = Vec::with_capacity(cols * rows);

    for col in 0..cols {
        let x_frac = col as f32 / (cols - 1) as f32;
        let x = x_frac * terrain_width - terrain_width / 2.0;
        let amplitude = peaks[col];

        for row in 0..rows {
            let z_frac = row as f32 / (rows - 1) as f32;
            let z = (z_frac - 0.5) * terrain_depth;

            // Ridge taper: peak at center, zero at edges
            let taper = 1.0 - (2.0 * z_frac - 1.0).powi(2);

            // Pseudo-random scatter for particle cloud look
            let scatter = hash(x_frac * 1000.0, z_frac * 1000.0);
            let y = amplitude * terrain_height * taper * (0.6 + scatter * 0.4);

            if y < 0.05 { continue; }

            let brightness = (y / terrain_height).powf(0.6) * 0.9 + 0.1;

            points.push(PointData::new(
                Vec3::new(x, y, z),
                2.5,
                Vec4::new(brightness, brightness, brightness * 1.05, brightness),
            ));
        }
    }

    let buffer = buffers.add(points_to_ssbo(&points));
    let mat = materials.add(PointCloudMaterial { buffer });
    let mesh = meshes.add(make_point_cloud_mesh(points.len()));

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        PointCloud { points },
    ));

    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Transform::from_xyz(0.0, 12.0, 20.0).looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y),
        PanOrbitCamera {
            target_focus: Vec3::new(0.0, 2.0, 0.0),
            ..default()
        },
    ));
}

fn take_screenshot(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::KeyS) {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk("/tmp/point_cloud_terrain.png"));
        info!("Screenshot → /tmp/point_cloud_terrain.png");
    }
}

fn auto_screenshot(
    mut commands: Commands,
    mut timer: ResMut<ScreenshotTimer>,
    mut exit: MessageWriter<AppExit>,
) {
    timer.0 += 1;
    if timer.0 == 10 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk("/tmp/point_cloud_terrain.png"));
        info!("Screenshot → /tmp/point_cloud_terrain.png");
    }
    if timer.0 == 20 {
        exit.write(AppExit::Success);
    }
}
