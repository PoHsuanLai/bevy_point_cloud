pub mod material;
pub mod point_cloud;
pub mod render;
mod systems;

pub use material::{PointCloudBlend, PointCloudShape};
pub use point_cloud::{PointCloud, PointCloudSettings, PointData};

use bevy::prelude::*;

/// Instanced point cloud rendering for Bevy.
pub struct PointCloudPlugin;

impl Plugin for PointCloudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(render::PointCloudRenderPlugin)
            .add_systems(Update, (systems::init_point_clouds, systems::setup_cameras));
    }
}
