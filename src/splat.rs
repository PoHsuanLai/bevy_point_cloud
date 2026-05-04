use bevy::asset::Asset;
use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

/// Per-point GPU data: position (vec3) + size (f32) + color (vec4) = 32 bytes.
///
/// Laid out for direct upload as an instance vertex buffer.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct SplatPoint {
    pub position: [f32; 3],
    pub size: f32,
    pub color: [f32; 4],
}

impl SplatPoint {
    pub fn new(position: Vec3, size: f32, color: Vec4) -> Self {
        Self {
            position: position.to_array(),
            size,
            color: color.to_array(),
        }
    }
}

/// Asset holding a list of splat points plus an optional capacity hint.
///
/// `capacity` lets callers pre-allocate a larger GPU instance buffer than
/// `points.len()` to avoid reallocation when the point count grows over time
/// (e.g. animated visualizations).
#[derive(Asset, Clone, Debug, Default, Reflect)]
#[reflect(Default)]
pub struct Splat {
    #[reflect(ignore)]
    pub points: Vec<SplatPoint>,
    pub capacity: usize,
}

impl Splat {
    pub fn new(points: Vec<SplatPoint>) -> Self {
        let capacity = points.len();
        Self { points, capacity }
    }

    pub fn with_capacity(points: Vec<SplatPoint>, capacity: usize) -> Self {
        Self {
            capacity: capacity.max(points.len()),
            points,
        }
    }
}

/// Component for entities that render a splat asset, mirroring `Mesh3d`.
#[derive(Component, Clone, Debug, Default, Reflect, PartialEq, Eq)]
#[reflect(Component, Default)]
#[require(Transform, Visibility)]
pub struct Splat3d(pub Handle<Splat>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splat_point_new() {
        let p = SplatPoint::new(Vec3::new(1.0, 2.0, 3.0), 5.0, Vec4::new(0.1, 0.2, 0.3, 0.4));
        assert_eq!(p.position, [1.0, 2.0, 3.0]);
        assert_eq!(p.size, 5.0);
        assert_eq!(p.color, [0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn splat_new_sets_capacity_to_len() {
        let splat = Splat::new(vec![SplatPoint::default(); 10]);
        assert_eq!(splat.points.len(), 10);
        assert_eq!(splat.capacity, 10);
    }

    #[test]
    fn splat_with_capacity_respects_hint() {
        let splat = Splat::with_capacity(vec![SplatPoint::default(); 5], 100);
        assert_eq!(splat.points.len(), 5);
        assert_eq!(splat.capacity, 100);
    }

    #[test]
    fn splat_with_capacity_clamps_to_len() {
        let splat = Splat::with_capacity(vec![SplatPoint::default(); 50], 10);
        assert_eq!(splat.capacity, 50);
    }
}
