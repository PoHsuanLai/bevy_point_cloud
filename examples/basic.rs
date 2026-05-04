//! Basic splat example — 10K points in a sphere with two materials.
//!
//! Run: cargo run --example basic

#[path = "common/mod.rs"]
mod common;

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_splat::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_splat — basic".into(),
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

fn setup(
    mut commands: Commands,
    mut splats: ResMut<Assets<Splat>>,
    mut materials: ResMut<Assets<SplatMaterial>>,
) {
    let num_points = 10_000;
    let mut points = Vec::with_capacity(num_points);

    for i in 0..num_points {
        let golden = (1.0 + 5.0_f32.sqrt()) / 2.0;
        let theta = 2.0 * std::f32::consts::PI * (i as f32 / golden);
        let phi = (1.0 - 2.0 * (i as f32 + 0.5) / num_points as f32).acos();

        let radius = 5.0;
        let x = radius * phi.sin() * theta.cos();
        let y = radius * phi.sin() * theta.sin();
        let z = radius * phi.cos();

        let r = 0.3 + 0.7 * ((x / radius + 1.0) * 0.5);
        let g = 0.2 + 0.6 * ((y / radius + 1.0) * 0.5);
        let b = 0.4 + 0.6 * ((z / radius + 1.0) * 0.5);

        points.push(SplatPoint::new(
            Vec3::new(x, y, z),
            3.0,
            Vec4::new(r, g, b, 0.8),
        ));
    }

    // Additive blend (default material) — size in pixels.
    commands.spawn(Splat3d(splats.add(Splat::new(points.clone()))));

    // Alpha-blended sphere with size attenuation — size in world units.
    let world_points: Vec<SplatPoint> = (0..2_000)
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
            SplatPoint::new(Vec3::new(x, y, z), 0.15, Vec4::new(r, g, b, 0.9))
        })
        .collect();
    commands.spawn((
        Splat3d(splats.add(Splat::new(world_points))),
        SplatMaterial3d(materials.add(SplatMaterial {
            blend: SplatBlend::Alpha,
            size_attenuation: true,
            ..default()
        })),
        Transform::from_xyz(12.0, 0.0, 0.0),
    ));

    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Transform::from_xyz(6.0, 0.0, 25.0).looking_at(Vec3::new(6.0, 0.0, 0.0), Vec3::Y),
        PanOrbitCamera::default(),
        NoIndirectDrawing,
    ));
}
