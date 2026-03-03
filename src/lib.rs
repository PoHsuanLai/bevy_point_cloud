pub mod material;
pub mod point_cloud;
pub mod render;
mod systems;

pub use material::PointCloudBlend;
pub use point_cloud::{PointCloud, PointCloudSettings, PointData};

use bevy::prelude::*;

/// Plugin that enables point cloud rendering.
///
/// Spawn entities with just a `PointCloud` component — the plugin
/// auto-creates the mesh and handles GPU buffer management:
///
/// ```ignore
/// commands.spawn(PointCloud::new(points));
/// ```
pub struct PointCloudPlugin;

impl Plugin for PointCloudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(render::PointCloudRenderPlugin)
            .add_systems(Update, (systems::init_point_clouds, systems::setup_cameras));
    }
}
