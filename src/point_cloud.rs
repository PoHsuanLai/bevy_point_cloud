use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

use crate::material::{PointCloudBlend, PointCloudShape};

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

/// Component holding point cloud data.
///
/// Just spawn this component — the plugin auto-creates the mesh and material:
/// ```ignore
/// commands.spawn(PointCloud::new(points));
/// ```
#[derive(Component, Clone, Debug, Default)]
pub struct PointCloud {
    pub points: Vec<PointData>,
    /// Pre-allocated capacity. The mesh/SSBO are sized for this many points
    /// to avoid rebuilds when the point count fluctuates.
    pub capacity: usize,
}

impl PointCloud {
    pub fn new(points: Vec<PointData>) -> Self {
        let capacity = points.len();
        Self { points, capacity }
    }

    /// Create with extra capacity so the mesh doesn't rebuild when points change count.
    pub fn with_capacity(points: Vec<PointData>, capacity: usize) -> Self {
        Self {
            capacity: capacity.max(points.len()),
            points,
        }
    }
}

/// Optional rendering settings for a point cloud entity.
///
/// Controls blend mode, size attenuation, opacity, and point shape.
/// When not present, defaults apply.
///
/// ```ignore
/// commands.spawn((
///     PointCloud::new(points),
///     PointCloudSettings {
///         blend: PointCloudBlend::Alpha,
///         size_attenuation: true,
///         opacity: 0.5,
///         shape: PointCloudShape::Square,
///         ..default()
///     },
/// ));
/// ```
#[derive(Component, Clone, Debug)]
pub struct PointCloudSettings {
    pub blend: PointCloudBlend,
    /// When true, `PointData::size` is in world units and shrinks with distance.
    /// When false (default), `PointData::size` is in screen pixels.
    pub size_attenuation: bool,
    /// Global opacity multiplier (0.0–1.0). Applied on top of per-point alpha.
    pub opacity: f32,
    /// Point shape: circle (soft dot) or square.
    pub shape: PointCloudShape,
}

impl Default for PointCloudSettings {
    fn default() -> Self {
        Self {
            blend: PointCloudBlend::default(),
            size_attenuation: false,
            opacity: 1.0,
            shape: PointCloudShape::default(),
        }
    }
}
