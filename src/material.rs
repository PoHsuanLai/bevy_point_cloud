use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};

/// GPU material for point cloud rendering.
///
/// Uses a storage buffer of point data and a dummy mesh whose vertex count
/// triggers the right number of shader invocations. The vertex shader expands
/// each point into a camera-facing billboard quad.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct PointCloudMaterial {
    #[storage(0, read_only, visibility(vertex))]
    pub buffer: Handle<ShaderStorageBuffer>,
}

impl Material for PointCloudMaterial {
    fn vertex_shader() -> bevy::shader::ShaderRef {
        "shaders/point_cloud.wgsl".into()
    }

    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/point_cloud.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        // Disable backface culling — billboard quads are expanded in clip space,
        // so their geometric face normal is arbitrary.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// Create a dummy mesh that triggers `num_points * 6` vertex shader invocations.
///
/// The mesh has no meaningful geometry — the vertex shader reads positions from
/// the storage buffer and expands each point into a billboard quad. Sentinel
/// vertices are placed at extreme positions to produce a large AABB that
/// prevents frustum culling.
pub fn make_point_cloud_mesh(num_points: usize) -> Mesh {
    let num_verts = num_points * 6;

    // Sentinel positions for a large AABB (prevents frustum culling).
    let mut positions = Vec::with_capacity(num_verts);
    let corners: [[f32; 3]; 8] = [
        [-1000.0, -1000.0, -1000.0], [ 1000.0, -1000.0, -1000.0],
        [-1000.0,  1000.0, -1000.0], [ 1000.0,  1000.0, -1000.0],
        [-1000.0, -1000.0,  1000.0], [ 1000.0, -1000.0,  1000.0],
        [-1000.0,  1000.0,  1000.0], [ 1000.0,  1000.0,  1000.0],
    ];
    for (i, corner) in corners.iter().enumerate() {
        if i < num_verts {
            positions.push(*corner);
        }
    }
    while positions.len() < num_verts {
        positions.push([0.0, 0.0, 0.0]);
    }

    let normals = vec![[0.0_f32, 1.0, 0.0]; num_verts];
    let uvs = vec![[0.0_f32, 0.0]; num_verts];
    let indices: Vec<u32> = (0..num_verts as u32).collect();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
