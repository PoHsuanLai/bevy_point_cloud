use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::storage::ShaderStorageBuffer;

use crate::material::{PointCloudMaterial, make_point_cloud_mesh};
use crate::point_cloud::{PointCloud, PointData};

/// Syncs `PointCloud` component data into the `ShaderStorageBuffer` and updates
/// the dummy mesh vertex count when the point count changes.
pub fn sync_point_clouds(
    mut query: Query<
        (&PointCloud, &MeshMaterial3d<PointCloudMaterial>, &Mesh3d),
        Changed<PointCloud>,
    >,
    materials: Res<Assets<PointCloudMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (cloud, mat_handle, mesh_handle) in &mut query {
        let Some(mat) = materials.get(mat_handle) else { continue };

        if let Some(ssbo) = buffers.get_mut(&mat.buffer) {
            let bytes: &[u8] = bytemuck::cast_slice(&cloud.points);
            ssbo.data = Some(bytes.to_vec());

            // Resize the dummy mesh if the point count changed
            let current_verts = meshes.get(mesh_handle)
                .map(|m| m.count_vertices())
                .unwrap_or(0);
            let needed_verts = cloud.points.len() * 6;
            if current_verts != needed_verts {
                if let Some(mesh) = meshes.get_mut(mesh_handle) {
                    *mesh = make_point_cloud_mesh(cloud.points.len());
                }
            }
        }
    }
}

/// Create a `ShaderStorageBuffer` from point data.
pub fn points_to_ssbo(points: &[PointData]) -> ShaderStorageBuffer {
    let bytes: &[u8] = bytemuck::cast_slice(points);
    ShaderStorageBuffer::new(bytes, RenderAssetUsages::default())
}
