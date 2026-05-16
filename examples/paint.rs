//! Paint into a `GridSplat` with the mouse.
//!
//! Click-drag the 64×64 grid to raise cell values; the splat brightens
//! and lifts as you paint. Right-click + drag to lower (a tap-toggle on
//! [`BrushOp`]). Press `R` to reset.
//!
//! Demonstrates the recommended pipeline: `bevy_splat` emits
//! `GridCellHit` events; `bevy_cad` provides the brush math + ambient
//! `ActiveBrush`. You translate hits into cells, look up neighbors as
//! needed, and call `apply_brush`.
//!
//! Run: cargo run --example paint

#[path = "common/mod.rs"]
mod common;

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::picking::PickingPlugin;
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;
use bevy_cad::{ActiveBrush, BrushOp, CadPlugin, apply_brush, cells_in_radius};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_splat::*;

const GRID_W: u32 = 64;
const GRID_H: u32 = 64;
const CELL: f32 = 0.4;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_splat — paint".into(),
                resolution: bevy::window::WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PickingPlugin)
        .add_plugins(SplatPlugin)
        .add_plugins(CadPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_brush_on_right_click, handle_brush, reset_on_r))
        .run();
}

#[derive(Resource)]
struct PaintGrid(Handle<GridSplat>);

fn setup(
    mut commands: Commands,
    mut grids: ResMut<Assets<GridSplat>>,
    mut materials: ResMut<Assets<SplatMaterial>>,
    mut brush: ResMut<ActiveBrush>,
) {
    let mut grid = GridSplat::new(GRID_W, GRID_H, Vec2::splat(CELL));
    grid.height_scale = 4.0;
    grid.point_size = 0.06;
    grid.color_value_max = 4.0;
    grid.colormap = Some(vec![
        Vec4::new(0.05, 0.05, 0.10, 0.4),
        Vec4::new(0.20, 0.30, 0.55, 0.7),
        Vec4::new(0.55, 0.75, 0.90, 0.9),
        Vec4::new(1.00, 1.00, 1.00, 1.0),
    ]);
    grid.fill(0.0); // start at floor; painting raises toward 1.0+

    let handle = grids.add(grid);
    commands.insert_resource(PaintGrid(handle.clone()));

    let material = materials.add(SplatMaterial {
        blend: SplatBlend::Alpha,
        size_attenuation: true,
        ..default()
    });

    brush.0.op = BrushOp::Add { delta: 1.0 };
    brush.0.radius_cells = 4.0;
    brush.0.strength = 0.25;

    let center = Vec3::new(GRID_W as f32 * CELL * 0.5, 0.0, GRID_H as f32 * CELL * 0.5);

    commands.spawn((
        GridSplat3d(handle),
        SplatMaterial3d(material),
        Name::new("PaintGrid"),
    ));

    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Transform::from_xyz(center.x, 12.0, center.z + 18.0).looking_at(center, Vec3::Y),
        PanOrbitCamera {
            focus: center,
            ..default()
        },
        NoIndirectDrawing,
    ));
}

/// Right mouse button flips the brush between deposit (`Add`) and
/// scoop-back-toward-zero (`PullTo { target: 0.0 }`).
fn toggle_brush_on_right_click(
    buttons: Res<ButtonInput<MouseButton>>,
    mut brush: ResMut<ActiveBrush>,
) {
    let want_pull = buttons.pressed(MouseButton::Right);
    brush.0.op = if want_pull {
        BrushOp::PullTo { target: 0.0 }
    } else {
        BrushOp::Add { delta: 1.0 }
    };
}

fn handle_brush(
    mut events: MessageReader<GridCellHit>,
    brush: Res<ActiveBrush>,
    paint_grid: Res<PaintGrid>,
    mut grids: ResMut<Assets<GridSplat>>,
) {
    for ev in events.read() {
        if !matches!(ev.phase, PointerPhase::Begin | PointerPhase::Continue) {
            continue;
        }
        let Some(grid) = grids.get_mut(&paint_grid.0) else {
            continue;
        };
        let center = Vec2::new(ev.cell.x as f32 + 0.5, ev.cell.y as f32 + 0.5);
        let updates: Vec<(u32, u32, f32)> =
            cells_in_radius(center, brush.0.radius_cells, grid.dims())
                .map(|(x, y, falloff)| {
                    let prior = grid.get(x, y);
                    let next = apply_brush(brush.0.op, brush.0.strength * falloff, prior, prior)
                        .clamp(0.0, 4.0);
                    (x, y, next)
                })
                .collect();
        grid.set_many(updates);
    }
}

fn reset_on_r(
    keys: Res<ButtonInput<KeyCode>>,
    paint_grid: Res<PaintGrid>,
    mut grids: ResMut<Assets<GridSplat>>,
) {
    if keys.just_pressed(KeyCode::KeyR)
        && let Some(grid) = grids.get_mut(&paint_grid.0)
    {
        grid.fill(0.0);
    }
}
