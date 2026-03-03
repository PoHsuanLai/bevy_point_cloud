//! Basic point cloud example — 10K random points in a sphere.
//!
//! Run: cargo run --example basic

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_point_cloud::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_point_cloud — basic".into(),
                resolution: bevy::window::WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PointCloudPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        .add_systems(Update, auto_screenshot)
        .run();
}

#[derive(Resource)]
struct ScreenshotTimer(u32);

fn setup(mut commands: Commands) {
    commands.insert_resource(ScreenshotTimer(0));

    let num_points = 10_000;
    let mut points = Vec::with_capacity(num_points);

    // Generate points on a sphere surface with varied colors
    for i in 0..num_points {
        // Fibonacci sphere distribution
        let golden = (1.0 + 5.0_f32.sqrt()) / 2.0;
        let theta = 2.0 * std::f32::consts::PI * (i as f32 / golden);
        let phi = (1.0 - 2.0 * (i as f32 + 0.5) / num_points as f32).acos();

        let radius = 5.0;
        let x = radius * phi.sin() * theta.cos();
        let y = radius * phi.sin() * theta.sin();
        let z = radius * phi.cos();

        // Color: warm gradient based on position
        let r = 0.3 + 0.7 * ((x / radius + 1.0) * 0.5);
        let g = 0.2 + 0.6 * ((y / radius + 1.0) * 0.5);
        let b = 0.4 + 0.6 * ((z / radius + 1.0) * 0.5);

        points.push(PointData::new(
            Vec3::new(x, y, z),
            3.0,
            Vec4::new(r, g, b, 0.8),
        ));
    }

    // Just spawn PointCloud — the plugin handles mesh + material creation
    commands.spawn(PointCloud::new(points));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        PanOrbitCamera::default(),
    ));
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
            .observe(save_to_disk("/tmp/point_cloud_basic.png"));
        info!("Screenshot → /tmp/point_cloud_basic.png");
    }
    if timer.0 == 20 {
        exit.write(AppExit::Success);
    }
}
