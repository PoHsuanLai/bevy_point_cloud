# bevy_splat

[![CI](https://github.com/PoHsuanLai/bevy_splat/actions/workflows/ci.yml/badge.svg)](https://github.com/PoHsuanLai/bevy_splat/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/bevy_splat.svg)](https://crates.io/crates/bevy_splat)
[![Docs.rs](https://docs.rs/bevy_splat/badge.svg)](https://docs.rs/bevy_splat)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

GPU-instanced splat (point cloud) rendering for [Bevy](https://bevyengine.org/) 0.17.

Renders large point sets as camera-facing billboard quads via a single instanced draw call per entity. A shared 4-vertex quad mesh is reused via per-instance vertex buffers — no per-point mesh overhead, no SSBO. Visual settings live in a shared `SplatMaterial` asset that multiple entities can reference.

## Features

- **True GPU instancing** — one shared 4-vertex quad mesh; per-point data lives in a per-instance vertex buffer
- **Asset-based API** — `Splat` / `SplatMaterial` are Bevy `Asset`s with reflection support; mirrors `Mesh` / `StandardMaterial`
- **Soft Gaussian splats** — circle shape uses `exp(-d² × falloff_sharpness)` falloff for smooth additive blooms
- **Blend modes** — additive, alpha, or opaque
- **Point shapes** — circle (Gaussian-soft) or square
- **Size modes** — screen-space pixels or world-unit sizing with perspective projection
- **Real AABB** — frustum culling computed from actual point bounds, not a sentinel
- **Buffer reuse** — instance buffer reused across frames with 25% over-allocation; capacity hint avoids reallocs during animation

## Quick Start

```toml
[dependencies]
bevy_splat = "0.1"
```

```rust
use bevy::prelude::*;
use bevy_splat::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SplatPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut splats: ResMut<Assets<Splat>>) {
    let points = vec![
        SplatPoint::new(Vec3::ZERO, 5.0, Vec4::ONE),
        SplatPoint::new(Vec3::X, 3.0, Vec4::new(1.0, 0.0, 0.0, 0.8)),
        SplatPoint::new(Vec3::Y, 3.0, Vec4::new(0.0, 1.0, 0.0, 0.8)),
    ];

    // Spawning Splat3d alone gets a default SplatMaterial automatically.
    commands.spawn(Splat3d(splats.add(Splat::new(points))));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        bevy::render::view::NoIndirectDrawing,
    ));
}
```

> **`NoIndirectDrawing` is required** on cameras that render splats. Bevy's GPU preprocessing remaps instance indices, which breaks per-instance vertex attributes.

## Configuration

Settings live on `SplatMaterial`. Spawn one explicitly to override defaults:

```rust
fn setup(
    mut commands: Commands,
    mut splats: ResMut<Assets<Splat>>,
    mut materials: ResMut<Assets<SplatMaterial>>,
) {
    let mat = materials.add(SplatMaterial {
        blend: SplatBlend::Alpha,
        shape: SplatShape::Circle,
        size_attenuation: true,
        opacity: 0.8,
        falloff_sharpness: 6.0,
        ..default()
    });

    commands.spawn((
        Splat3d(splats.add(Splat::new(points))),
        SplatMaterial3d(mat.clone()),
        Transform::from_xyz(5.0, 0.0, 0.0),
    ));

    // Sharing the same material across entities is fine — just clone the handle.
    commands.spawn((
        Splat3d(splats.add(Splat::new(more_points))),
        SplatMaterial3d(mat),
    ));
}
```

### Size modes

- `size_attenuation: false` (default) — `SplatPoint::size` is **screen pixels**; constant size regardless of distance.
- `size_attenuation: true` — `SplatPoint::size` is **world units**; shrinks with distance.

### Blend modes

| Mode | Depth write | Use case |
|------|-------------|----------|
| `Additive` (default) | No | Glowing particles, additive bloom |
| `Alpha` | No | Semi-transparent, layered |
| `Opaque` | Yes | Solid points, z-correct occlusion |

### Falloff (circle only)

`falloff_sharpness` controls the Gaussian dropoff: `alpha *= exp(-d² × k)`. Higher = tighter dots, lower = soft halos. Default `4.0`.

### Dynamic updates

Mutate the asset and the GPU buffer is re-uploaded next frame:

```rust
fn animate(
    mut splats: ResMut<Assets<Splat>>,
    handles: Query<&Splat3d>,
    time: Res<Time>,
) {
    for handle in &handles {
        if let Some(splat) = splats.get_mut(&handle.0) {
            for p in &mut splat.points {
                p.position[1] += time.delta_secs() * 0.5;
            }
        }
    }
}
```

For animations where the point count grows over time, pre-size the asset:

```rust
let handle = splats.add(Splat::with_capacity(initial_points, 500_000));
```

## Examples

```bash
cargo run --example basic       # Two spheres: additive vs alpha blend
cargo run --example terrain     # 3D Wave_ref-style terrain (~300K points)
cargo run --example audio       # Live audio spectrum waterfall (needs WAV file)
cargo run --example dashboard   # Composite Ryoji-Ikeda-style data panels
```

## Bevy compatibility

| bevy_splat | Bevy |
|------------|------|
| 0.1        | 0.17 |

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
