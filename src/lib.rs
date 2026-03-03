pub mod material;
pub mod point_cloud;
mod systems;

pub use material::{PointCloudMaterial, make_point_cloud_mesh};
pub use point_cloud::{PointCloud, PointData};
pub use systems::points_to_ssbo;

use bevy::prelude::*;

/// Plugin that enables point cloud rendering.
///
/// Add this to your app, then spawn entities with:
/// - `PointCloud` component (your point data)
/// - `Mesh3d(meshes.add(make_point_cloud_mesh(num_points)))`
/// - `MeshMaterial3d(materials.add(PointCloudMaterial { points: vec![...] }))`
pub struct PointCloudPlugin;

impl Plugin for PointCloudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<PointCloudMaterial>::default())
            .add_systems(Update, systems::sync_point_clouds);
    }
}
