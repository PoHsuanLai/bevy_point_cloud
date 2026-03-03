use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Blending mode for point cloud rendering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PointCloudBlend {
    #[default]
    Additive,
    Alpha,
    Opaque,
}

/// Point shape for rendering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PointCloudShape {
    /// Soft circular dot with smooth edges.
    #[default]
    Circle,
    /// Square (no discard, no smoothstep — cheapest).
    Square,
}

/// Create a minimal quad mesh (6 vertices, 2 triangles) used as the billboard
/// template for instanced point cloud rendering. Each point is one instance.
pub fn make_point_cloud_mesh() -> Mesh {
    let positions: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]; 6];
    let normals = vec![[0.0_f32, 1.0, 0.0]; 6];
    let uvs = vec![[0.0_f32, 0.0]; 6];
    let indices: Vec<u32> = vec![0, 1, 2, 3, 4, 5];

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
