//! Basic point cloud example — 10K random points in a sphere.
//!
//! Run: cargo run --example basic

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;
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
        .add_systems(Update, take_screenshot)
        .run();
}

fn setup(mut commands: Commands) {
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

    // Additive blend (default) — size in pixels
    commands.spawn(PointCloud::new(points.clone()));

    // Alpha blend with size attenuation — size in world units
    // Use fewer, smaller points so individual dots are clearly visible
    let world_points: Vec<PointData> = (0..2_000)
        .map(|i| {
            let golden = (1.0 + 5.0_f32.sqrt()) / 2.0;
            let theta = 2.0 * std::f32::consts::PI * (i as f32 / golden);
            let phi = (1.0 - 2.0 * (i as f32 + 0.5) / 2_000.0).acos();
            let radius = 5.0;
            let x = radius * phi.sin() * theta.cos();
            let y = radius * phi.sin() * theta.sin();
            let z = radius * phi.cos();
            let r = 0.3 + 0.7 * ((x / radius + 1.0) * 0.5);
            let g = 0.2 + 0.6 * ((y / radius + 1.0) * 0.5);
            let b = 0.4 + 0.6 * ((z / radius + 1.0) * 0.5);
            PointData::new(Vec3::new(x, y, z), 0.15, Vec4::new(r, g, b, 0.9))
        })
        .collect();
    commands.spawn((
        PointCloud::new(world_points),
        PointCloudSettings {
            blend: PointCloudBlend::Alpha,
            size_attenuation: true,
            ..default()
        },
        Transform::from_xyz(12.0, 0.0, 0.0),
    ));

    // Camera — NoIndirectDrawing is required for point cloud instancing
    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Transform::from_xyz(6.0, 0.0, 25.0).looking_at(Vec3::new(6.0, 0.0, 0.0), Vec3::Y),
        PanOrbitCamera::default(),
        NoIndirectDrawing,
    ));
}

fn take_screenshot(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::KeyS) {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk("/tmp/point_cloud_basic.png"));
        info!("Screenshot → /tmp/point_cloud_basic.png");
    }
}
