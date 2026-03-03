# bevy_point_cloud

[![CI](https://github.com/PoHsuanLai/bevy_point_cloud/actions/workflows/ci.yml/badge.svg)](https://github.com/PoHsuanLai/bevy_point_cloud/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/bevy_point_cloud.svg)](https://crates.io/crates/bevy_point_cloud)
[![Docs.rs](https://docs.rs/bevy_point_cloud/badge.svg)](https://docs.rs/bevy_point_cloud)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

GPU-instanced point cloud rendering for [Bevy](https://bevyengine.org/) 0.17.

Renders millions of points as camera-facing billboard quads via a single instanced draw call per entity. Points are stored in a GPU SSBO — no per-point mesh overhead.

## Features

- **Instanced rendering** — one draw call per point cloud entity, scales to millions of points
- **Billboard quads** — always face the camera, no manual orientation needed
- **Per-point attributes** — position, size, and RGBA color per point
- **Blend modes** — additive, alpha, or opaque blending
- **Point shapes** — circle (soft-edged) or square
- **Size modes** — screen-space pixels or world-unit sizing with perspective projection
- **Global opacity** — per-entity opacity multiplier
- **Transform support** — standard Bevy `Transform` / `GlobalTransform` applied on GPU
- **Change detection** — only re-uploads to GPU when `PointCloud`, `PointCloudSettings`, or `GlobalTransform` changes
- **Buffer reuse** — GPU buffers are reused across frames with 25% over-allocation to reduce reallocations

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy_point_cloud = "0.1"
```

Spawn a point cloud:

```rust
use bevy::prelude::*;
use bevy_point_cloud::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PointCloudPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    let points = vec![
        PointData::new(Vec3::new(0.0, 0.0, 0.0), 5.0, Vec4::new(1.0, 1.0, 1.0, 1.0)),
        PointData::new(Vec3::new(1.0, 0.0, 0.0), 3.0, Vec4::new(1.0, 0.0, 0.0, 0.8)),
        PointData::new(Vec3::new(0.0, 1.0, 0.0), 3.0, Vec4::new(0.0, 1.0, 0.0, 0.8)),
    ];

    commands.spawn(PointCloud::new(points));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
```

## Configuration

Add `PointCloudSettings` for non-default rendering:

```rust
commands.spawn((
    PointCloud::new(points),
    PointCloudSettings {
        blend: PointCloudBlend::Alpha,
        size_attenuation: true,  // size in world units instead of pixels
        opacity: 0.8,
        shape: PointCloudShape::Square,
    },
    Transform::from_xyz(5.0, 0.0, 0.0),
));
```

### Size Modes

- `size_attenuation: false` (default) — `PointData::size` is in **screen pixels**. Points stay the same size regardless of distance.
- `size_attenuation: true` — `PointData::size` is in **world units**. Points shrink with distance, matching the 3D scene scale.

### Blend Modes

| Mode | Depth Write | Use Case |
|------|-------------|----------|
| `Additive` (default) | No | Glowing particles, energy effects |
| `Alpha` | No | Semi-transparent, natural layering |
| `Opaque` | Yes | Solid points, z-correct occlusion |

### Dynamic Updates

Mutate the `PointCloud` component to trigger a GPU re-upload:

```rust
fn animate(mut query: Query<&mut PointCloud>, time: Res<Time>) {
    for mut cloud in &mut query {
        for point in &mut cloud.points {
            point.position[1] += (time.elapsed_secs()).sin() * 0.01;
        }
    }
}
```

## Examples

```bash
cargo run --example basic       # Two spheres: additive vs alpha blend
cargo run --example terrain     # 3D terrain surface
cargo run --example audio       # Live audio spectrum waterfall (needs WAV file)
cargo run --example dashboard   # Composite data visualization panels
```

## Bevy Compatibility

| bevy_point_cloud | Bevy |
|------------------|------|
| 0.1              | 0.17 |

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
