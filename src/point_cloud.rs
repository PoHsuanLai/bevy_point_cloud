use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

/// Per-point data uploaded to the GPU via storage buffer.
///
/// Layout: position (vec3) + size (f32) + color (vec4) = 32 bytes, std430 compatible.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct PointData {
    pub position: [f32; 3],
    pub size: f32,
    pub color: [f32; 4],
}

impl PointData {
    pub fn new(position: Vec3, size: f32, color: Vec4) -> Self {
        Self {
            position: position.to_array(),
            size,
            color: color.to_array(),
        }
    }
}

/// Component holding point cloud data. Attach to an entity alongside
/// `Mesh3d` and `MeshMaterial3d<PointCloudMaterial>`.
#[derive(Component, Clone, Debug, Default)]
pub struct PointCloud {
    pub points: Vec<PointData>,
}
