use bevy::camera::visibility::NoFrustumCulling;
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;

use crate::material::make_point_cloud_mesh;
use crate::point_cloud::PointCloud;

/// Auto-initializes newly spawned `PointCloud` entities with the required
/// mesh and `NoFrustumCulling`. The render pipeline handles GPU buffers.
pub fn init_point_clouds(
    mut commands: Commands,
    query: Query<(Entity, Has<Mesh3d>), Added<PointCloud>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, has_mesh) in &query {
        let mut cmds = commands.entity(entity);

        if !has_mesh {
            let mesh = meshes.add(make_point_cloud_mesh());
            cmds.insert(Mesh3d(mesh));
        }

        cmds.insert((Visibility::default(), NoFrustumCulling));
    }
}

/// Add `NoIndirectDrawing` to cameras so instanced draw calls work correctly.
/// Without this, Bevy's GPU preprocessing remaps instance indices.
pub fn setup_cameras(
    mut commands: Commands,
    cameras: Query<Entity, (With<Camera3d>, Without<NoIndirectDrawing>)>,
) {
    for entity in &cameras {
        commands.entity(entity).insert(NoIndirectDrawing);
    }
}
