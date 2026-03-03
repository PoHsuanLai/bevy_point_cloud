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
#[derive(Component, Clone, Debug, Default)]
pub struct PointCloud {
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
#[derive(Component, Clone, Debug)]
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
