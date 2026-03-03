//! GPU-instanced point cloud rendering for Bevy.
//!
//! Renders point clouds as camera-facing billboard quads via a single instanced
//! draw call per entity. Per-point data (position, size, color) is stored in a
//! GPU SSBO — no per-point mesh overhead.
//!
//! # Usage
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_point_cloud::*;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(PointCloudPlugin)
//!     .add_systems(Startup, |mut commands: Commands| {
//!         commands.spawn(PointCloud::new(vec![
//!             PointData::new(Vec3::ZERO, 5.0, Vec4::ONE),
//!         ]));
//!     })
//!     .run();
//! ```

pub mod material;
pub mod point_cloud;
pub mod render;
mod systems;

pub use material::{PointCloudBlend, PointCloudShape};
pub use point_cloud::{PointCloud, PointCloudSettings, PointData};

use bevy::prelude::*;

/// Instanced point cloud rendering for Bevy.
///
/// Cameras that render point clouds must have `NoIndirectDrawing` inserted,
/// otherwise Bevy's GPU preprocessing remaps instance indices.
pub struct PointCloudPlugin;

impl Plugin for PointCloudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(render::PointCloudRenderPlugin)
            .add_systems(Update, systems::init_point_clouds);
    }
}
