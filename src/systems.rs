use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;

use crate::material::{PointCloudMaterial, make_point_cloud_mesh};
use crate::point_cloud::{PointCloud, PointData};

/// Auto-initializes newly spawned `PointCloud` entities with the required
/// mesh and material components. Users just need to spawn `PointCloud`.
pub fn init_point_clouds(
    mut commands: Commands,
    query: Query<(Entity, &PointCloud), Added<PointCloud>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PointCloudMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    for (entity, cloud) in &query {
        let ssbo = points_to_ssbo(&cloud.points);
        let buffer = buffers.add(ssbo);
        let mat = materials.add(PointCloudMaterial::new(buffer));
        let mesh = meshes.add(make_point_cloud_mesh(cloud.capacity));

        commands
            .entity(entity)
            .insert((Mesh3d(mesh), MeshMaterial3d(mat)));
    }
}

/// Syncs `PointCloud` component data into the GPU storage buffer.
/// Only rebuilds the mesh when the point count exceeds current capacity.
pub fn sync_point_clouds(
    mut query: Query<
        (
            &mut PointCloud,
            &MeshMaterial3d<PointCloudMaterial>,
            &Mesh3d,
        ),
        Changed<PointCloud>,
    >,
    materials: Res<Assets<PointCloudMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (mut cloud, mat_handle, mesh_handle) in &mut query {
        let Some(mat) = materials.get(mat_handle) else {
            continue;
        };

        if let Some(ssbo) = buffers.get_mut(&mat.buffer) {
            let bytes: &[u8] = bytemuck::cast_slice(&cloud.points);
            ssbo.data = Some(bytes.to_vec());

            // Only rebuild mesh if points exceed allocated capacity
            if cloud.points.len() > cloud.capacity {
                cloud.capacity = cloud.points.len();
                if let Some(mesh) = meshes.get_mut(mesh_handle) {
                    *mesh = make_point_cloud_mesh(cloud.capacity);
                }
            }
        }
    }
}

/// Create a `ShaderStorageBuffer` from point data.
pub(crate) fn points_to_ssbo(points: &[PointData]) -> ShaderStorageBuffer {
    let bytes: &[u8] = bytemuck::cast_slice(points);
    ShaderStorageBuffer::new(bytes, RenderAssetUsages::default())
}
