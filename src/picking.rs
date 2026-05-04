//! Picking glue — translates `Pointer<Press/Move/Release>` on a
//! `GridSplat3d` entity into [`GridBrush`] events tagged with the cell
//! the pointer hit.
//!
//! `GridSplat3d` renders as splats which `bevy_picking` can't ray-cast
//! directly. So when an entity gains `GridSplat3d`, we attach an
//! invisible quad mesh sized to the grid's footprint as the picking
//! proxy. `MeshPickingPlugin` raycasts that, and our observers convert
//! the world-space hit into a cell coordinate.

use bevy::picking::events::{Move, Pointer, Press, Release};
use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::picking::Pickable;
use bevy::prelude::*;

use crate::grid::{GridSplat, GridSplat3d};

/// Brush event emitted from grid pointer interaction.
///
/// `Begin` fires on press, `Continue` while the pointer moves with a
/// button held, `End` on release. `cell` is the discrete grid cell the
/// pointer hit; `world_pos` is the world-space ray hit (useful for
/// brushes that need fractional positions, e.g. for falloff).
#[derive(Message, Debug, Clone, Copy)]
pub struct GridBrush {
    pub entity: Entity,
    pub cell: UVec2,
    pub world_pos: Vec3,
    pub phase: BrushPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushPhase {
    Begin,
    Continue,
    End,
}

/// Marker on a `GridSplat3d` entity to track that we've attached the
/// picking proxy + observers. Avoids re-attaching every frame.
#[derive(Component)]
pub(crate) struct GridPickerAttached;

/// Per-entity flag set when a stroke is in progress. `Move` events only
/// emit `BrushPhase::Continue` while this is present; cleared on `Release`.
#[derive(Component)]
pub(crate) struct GridBrushActive;

/// Plugin that wires up grid picking. Added by `SplatPlugin` so users
/// don't have to think about it.
pub(crate) struct GridPickingPlugin;

impl Plugin for GridPickingPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<MeshPickingPlugin>() {
            app.add_plugins(MeshPickingPlugin);
        }
    }
}

/// Attach the picking proxy mesh + observers to any new `GridSplat3d`.
/// Runs every `Update` and is a no-op once `GridPickerAttached` is
/// present on the entity.
pub(crate) fn attach_grid_pickers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    grids: Res<Assets<GridSplat>>,
    query: Query<(Entity, &GridSplat3d), Without<GridPickerAttached>>,
) {
    for (entity, grid3d) in &query {
        let Some(grid) = grids.get(&grid3d.0) else {
            continue;
        };

        // Plane sized to the grid's footprint, centered on its midpoint.
        let w = grid.width as f32 * grid.cell_size.x;
        let h = grid.height as f32 * grid.cell_size.y;
        let mid = Vec3::new(
            grid.origin.x + w * 0.5,
            grid.origin.y,
            grid.origin.z + h * 0.5,
        );
        // `Plane3d::default()` faces +Y; subdivisions: 0 = single quad.
        let mesh = meshes.add(Plane3d::default().mesh().size(w, h).build());

        let mut e = commands.entity(entity);
        e.insert((
            GridPickerAttached,
            Mesh3d(mesh),
            Transform::from_translation(mid),
            Visibility::Visible,
            // Mesh stays invisible (no MeshMaterial3d) but is hittable.
            Pickable::default(),
        ));
        e.observe(on_press);
        e.observe(on_move);
        e.observe(on_release);
    }
}

fn on_press(
    event: On<Pointer<Press>>,
    mut commands: Commands,
    grids: Res<Assets<GridSplat>>,
    grid_q: Query<(&GridSplat3d, &GlobalTransform)>,
    mut writer: MessageWriter<GridBrush>,
) {
    let entity = event.entity;
    let Some(world_pos) = event.event.hit.position else {
        return;
    };
    let Ok((grid3d, gxform)) = grid_q.get(entity) else {
        return;
    };
    let Some(grid) = grids.get(&grid3d.0) else {
        return;
    };
    let Some(cell) = world_to_cell(grid, gxform, world_pos) else {
        return;
    };
    commands.entity(entity).insert(GridBrushActive);
    writer.write(GridBrush {
        entity,
        cell,
        world_pos,
        phase: BrushPhase::Begin,
    });
}

fn on_move(
    event: On<Pointer<Move>>,
    grids: Res<Assets<GridSplat>>,
    grid_q: Query<(&GridSplat3d, &GlobalTransform, Has<GridBrushActive>)>,
    mut writer: MessageWriter<GridBrush>,
) {
    let entity = event.entity;
    let Ok((grid3d, gxform, active)) = grid_q.get(entity) else {
        return;
    };
    if !active {
        return;
    }
    let Some(world_pos) = event.event.hit.position else {
        return;
    };
    let Some(grid) = grids.get(&grid3d.0) else {
        return;
    };
    let Some(cell) = world_to_cell(grid, gxform, world_pos) else {
        return;
    };
    writer.write(GridBrush {
        entity,
        cell,
        world_pos,
        phase: BrushPhase::Continue,
    });
}

fn on_release(
    event: On<Pointer<Release>>,
    mut commands: Commands,
    grids: Res<Assets<GridSplat>>,
    grid_q: Query<(&GridSplat3d, &GlobalTransform, Has<GridBrushActive>)>,
    mut writer: MessageWriter<GridBrush>,
) {
    let entity = event.entity;
    let Ok((grid3d, gxform, active)) = grid_q.get(entity) else {
        return;
    };
    if !active {
        return;
    }
    commands.entity(entity).remove::<GridBrushActive>();
    let Some(world_pos) = event.event.hit.position else {
        return;
    };
    let Some(grid) = grids.get(&grid3d.0) else {
        return;
    };
    let Some(cell) = world_to_cell(grid, gxform, world_pos) else {
        return;
    };
    writer.write(GridBrush {
        entity,
        cell,
        world_pos,
        phase: BrushPhase::End,
    });
}

/// Project `world_pos` into the grid's local space and ask
/// `GridSplat::world_to_cell`. The picking backend reports world-space
/// hits even though the grid renders splats; we need to undo the
/// entity's transform first.
fn world_to_cell(grid: &GridSplat, gxform: &GlobalTransform, world_pos: Vec3) -> Option<UVec2> {
    let local = gxform.affine().inverse().transform_point3(world_pos);
    grid.world_to_cell(Vec2::new(local.x, local.z))
}
