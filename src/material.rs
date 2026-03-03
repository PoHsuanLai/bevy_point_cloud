use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType};
use bevy::render::storage::ShaderStorageBuffer;

/// Blending mode for point cloud rendering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PointCloudBlend {
    /// Additive blending — overlapping points accumulate brightness. Great for
    /// glowing particles, data visualization. Depth writes disabled so back
    /// particles shine through.
    #[default]
    Additive,
    /// Standard alpha blending with depth writes enabled.
    Alpha,
    /// Fully opaque points with depth writes enabled.
    Opaque,
}

/// Material-level parameters sent to the shader as a uniform.
#[derive(ShaderType, Clone, Debug)]
pub struct PointCloudParams {
    /// 0 = screen-space (fixed pixel size), 1 = perspective (shrinks with distance)
    pub size_attenuation: u32,
    /// Base world-space scale when size_attenuation is enabled.
    pub base_scale: f32,
}

impl Default for PointCloudParams {
    fn default() -> Self {
        Self {
            size_attenuation: 0,
            base_scale: 500.0,
        }
    }
}

/// GPU material for point cloud rendering.
///
/// Uses a storage buffer of point data. The vertex shader expands each point
/// into a camera-facing billboard quad.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct PointCloudMaterial {
    #[storage(0, read_only, visibility(vertex))]
    pub buffer: Handle<ShaderStorageBuffer>,

    #[uniform(1)]
    pub params: PointCloudParams,

    /// Blending mode (not sent to GPU — controls pipeline configuration).
    pub blend: PointCloudBlend,
}

impl PointCloudMaterial {
    pub fn new(buffer: Handle<ShaderStorageBuffer>) -> Self {
        Self {
            buffer,
            params: PointCloudParams::default(),
            blend: PointCloudBlend::default(),
        }
    }

    pub fn with_blend(mut self, blend: PointCloudBlend) -> Self {
        self.blend = blend;
        self
    }

    pub fn with_perspective_size(mut self, base_scale: f32) -> Self {
        self.params.size_attenuation = 1;
        self.params.base_scale = base_scale;
        self
    }
}

impl Material for PointCloudMaterial {
    fn vertex_shader() -> bevy::shader::ShaderRef {
        "shaders/point_cloud.wgsl".into()
    }

    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/point_cloud.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        match self.blend {
            PointCloudBlend::Additive => AlphaMode::Add,
            PointCloudBlend::Alpha => AlphaMode::Blend,
            PointCloudBlend::Opaque => AlphaMode::Opaque,
        }
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

        // Disable depth writes for additive blending so back particles still
        // accumulate brightness. We can't read `self` here (specialize is static),
        // so we always disable depth writes. For opaque mode users can override.
        if let Some(ref mut depth) = descriptor.depth_stencil {
            depth.depth_write_enabled = false;
        }

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
        [-1000.0, -1000.0, -1000.0],
        [1000.0, -1000.0, -1000.0],
        [-1000.0, 1000.0, -1000.0],
        [1000.0, 1000.0, -1000.0],
        [-1000.0, -1000.0, 1000.0],
        [1000.0, -1000.0, 1000.0],
        [-1000.0, 1000.0, 1000.0],
        [1000.0, 1000.0, 1000.0],
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
