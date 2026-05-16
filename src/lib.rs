//! GPU-instanced splat (point cloud) rendering for Bevy.
//!
//! Renders large point sets as camera-facing billboard quads via a single
//! instanced draw call per entity. Per-point data (position, size, color)
//! lives in a per-instance vertex buffer; visual settings (blend, shape,
//! falloff) live in a shared `SplatMaterial` asset.
//!
//! # Usage
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_splat::*;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(SplatPlugin)
//!     .add_systems(Startup, |mut commands: Commands, mut splats: ResMut<Assets<Splat>>| {
//!         commands.spawn(Splat3d(splats.add(Splat::new(vec![
//!             SplatPoint::new(Vec3::ZERO, 5.0, Vec4::ONE),
//!         ]))));
//!     })
//!     .run();
//! ```

pub mod grid;
pub mod picking;
pub mod splat;
pub mod splat_material;
pub(crate) mod render;
mod systems;

pub use grid::{GridSplat, GridSplat3d};
pub use picking::{GridCellHit, PointerPhase};
pub use splat::{Splat, Splat3d, SplatPoint};
pub use splat_material::{SplatBlend, SplatMaterial, SplatMaterial3d, SplatShape};

use bevy::prelude::*;

/// Instanced splat rendering for Bevy.
///
/// Cameras that render splats must have `NoIndirectDrawing` inserted,
/// otherwise Bevy's GPU preprocessing remaps instance indices and
/// per-instance data is read from the wrong slot.
pub struct SplatPlugin;

impl Plugin for SplatPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "splat.wgsl");
        app.init_asset::<Splat>()
            .init_asset::<SplatMaterial>()
            .init_asset::<GridSplat>()
            .add_message::<GridCellHit>()
            .register_asset_reflect::<Splat>()
            .register_asset_reflect::<SplatMaterial>()
            .register_asset_reflect::<GridSplat>()
            .register_type::<Splat3d>()
            .register_type::<SplatMaterial3d>()
            .register_type::<SplatBlend>()
            .register_type::<SplatShape>()
            .register_type::<GridSplat3d>()
            .add_plugins(render::SplatRenderPlugin)
            .add_plugins(picking::GridPickingPlugin)
            .add_systems(
                Update,
                (
                    systems::ensure_default_material,
                    grid::init_grid_backing,
                    picking::attach_grid_pickers,
                ),
            )
            .add_systems(
                PostUpdate,
                (systems::update_splat_aabb, grid::sync_grid_to_splat),
            );
    }
}
