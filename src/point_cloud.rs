use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

use crate::material::{PointCloudBlend, PointCloudShape};

/// Per-point GPU data: position (vec3) + size (f32) + color (vec4) = 32 bytes, std430.
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

/// Spawn this component — the plugin handles mesh and GPU buffer creation.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct PointCloud {
    #[reflect(ignore)]
    pub points: Vec<PointData>,
    pub capacity: usize,
}

impl PointCloud {
    pub fn new(points: Vec<PointData>) -> Self {
        let capacity = points.len();
        Self { points, capacity }
    }

    pub fn with_capacity(points: Vec<PointData>, capacity: usize) -> Self {
        Self {
            capacity: capacity.max(points.len()),
            points,
        }
    }
}

/// Optional rendering settings. When absent, defaults apply.
#[derive(Component, Clone, Debug, Reflect)]
pub struct PointCloudSettings {
    pub blend: PointCloudBlend,
    /// When true, size is in world units; when false (default), screen pixels.
    pub size_attenuation: bool,
    /// Global opacity multiplier (0.0–1.0), on top of per-point alpha.
    pub opacity: f32,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_data_new() {
        let pd = PointData::new(Vec3::new(1.0, 2.0, 3.0), 5.0, Vec4::new(0.1, 0.2, 0.3, 0.4));
        assert_eq!(pd.position, [1.0, 2.0, 3.0]);
        assert_eq!(pd.size, 5.0);
        assert_eq!(pd.color, [0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn point_cloud_new_sets_capacity_to_len() {
        let points = vec![PointData::default(); 10];
        let cloud = PointCloud::new(points);
        assert_eq!(cloud.points.len(), 10);
        assert_eq!(cloud.capacity, 10);
    }

    #[test]
    fn point_cloud_with_capacity_respects_hint() {
        let points = vec![PointData::default(); 5];
        let cloud = PointCloud::with_capacity(points, 100);
        assert_eq!(cloud.points.len(), 5);
        assert_eq!(cloud.capacity, 100);
    }

    #[test]
    fn point_cloud_with_capacity_clamps_to_len() {
        let points = vec![PointData::default(); 50];
        let cloud = PointCloud::with_capacity(points, 10);
        assert_eq!(cloud.capacity, 50);
    }
}
