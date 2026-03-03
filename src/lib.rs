pub mod material;
pub mod point_cloud;
mod systems;

pub use material::{PointCloudBlend, PointCloudMaterial, PointCloudParams};
pub use point_cloud::{PointCloud, PointData};

use bevy::prelude::*;

/// Plugin that enables point cloud rendering.
///
/// Spawn entities with just a `PointCloud` component — the plugin
/// auto-creates the mesh and material:
///
/// ```ignore
/// commands.spawn(PointCloud::new(points));
/// ```
pub struct PointCloudPlugin;

impl Plugin for PointCloudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<PointCloudMaterial>::default())
            .add_systems(
                Update,
                (systems::init_point_clouds, systems::sync_point_clouds).chain(),
            );
    }
}
