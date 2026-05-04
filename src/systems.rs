use bevy::asset::AssetEvent;
use bevy::camera::primitives::Aabb;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

use crate::splat::{Splat, Splat3d};
use crate::splat_material::{SplatMaterial, SplatMaterial3d};

/// Auto-attach a default `SplatMaterial3d` to entities that only have
/// `Splat3d`. Lets users spawn `Splat3d(handle)` standalone and still get
/// rendered — mirrors the ergonomics of the previous `PointCloud`-only spawn.
pub fn ensure_default_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<SplatMaterial>>,
    query: Query<Entity, (With<Splat3d>, Without<SplatMaterial3d>)>,
) {
    if query.is_empty() {
        return;
    }
    let default_handle = materials.add(SplatMaterial::default());
    for entity in &query {
        commands
            .entity(entity)
            .insert(SplatMaterial3d(default_handle.clone()));
    }
}

/// Recompute `Aabb` for entities whose `Splat` asset changed.
///
/// Replaces the old `±1000` sentinel-vertex hack: we now know the actual
/// bounds from the point list, so frustum culling works correctly.
pub fn update_splat_aabb(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<Splat>>,
    splats: Res<Assets<Splat>>,
    query: Query<(Entity, &Splat3d)>,
) {
    let mut changed = HashSet::<AssetId<Splat>>::default();
    for event in events.read() {
        match event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::LoadedWithDependencies { id } => {
                changed.insert(*id);
            }
            _ => {}
        }
    }
    if changed.is_empty() {
        return;
    }

    for (entity, splat3d) in &query {
        let id = splat3d.0.id();
        if !changed.contains(&id) {
            continue;
        }
        let Some(splat) = splats.get(id) else {
            continue;
        };
        let Some(aabb) = compute_aabb(&splat.points) else {
            continue;
        };
        commands.entity(entity).insert(aabb);
    }
}

fn compute_aabb(points: &[crate::splat::SplatPoint]) -> Option<Aabb> {
    if points.is_empty() {
        return None;
    }
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for p in points {
        let pos = Vec3::from_array(p.position);
        min = min.min(pos);
        max = max.max(pos);
        // Inflate by point size so big screen-space splats at the edge of
        // the cloud aren't clipped — sizes are in pixels by default but we
        // can't know view-space size here, so inflate by world size as a
        // safe bound for size_attenuation=true.
        let half_size = Vec3::splat(p.size.max(0.0));
        min = min.min(pos - half_size);
        max = max.max(pos + half_size);
    }
    Some(Aabb::from_min_max(min, max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::splat::SplatPoint;

    #[test]
    fn aabb_handles_empty() {
        assert!(compute_aabb(&[]).is_none());
    }

    #[test]
    fn aabb_covers_all_points() {
        let points = vec![
            SplatPoint::new(Vec3::new(-1.0, -2.0, -3.0), 0.0, Vec4::ONE),
            SplatPoint::new(Vec3::new(4.0, 5.0, 6.0), 0.0, Vec4::ONE),
        ];
        let aabb = compute_aabb(&points).unwrap();
        let min = aabb.center - aabb.half_extents;
        let max = aabb.center + aabb.half_extents;
        assert!(min.x <= -1.0 && min.y <= -2.0 && min.z <= -3.0);
        assert!(max.x >= 4.0 && max.y >= 5.0 && max.z >= 6.0);
    }

    #[test]
    fn aabb_inflates_by_point_size() {
        let points = vec![SplatPoint::new(Vec3::ZERO, 2.0, Vec4::ONE)];
        let aabb = compute_aabb(&points).unwrap();
        // With size=2 inflation, half-extents should be at least 2.
        assert!(aabb.half_extents.x >= 2.0);
        assert!(aabb.half_extents.y >= 2.0);
        assert!(aabb.half_extents.z >= 2.0);
    }
}
