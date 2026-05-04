use bevy::asset::Asset;
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum SplatBlend {
    #[default]
    Additive,
    Alpha,
    Opaque,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum SplatShape {
    #[default]
    Circle,
    Square,
}

/// Visual settings asset shared across splat entities.
///
/// All fields except `blend` and `shape` map directly to a GPU uniform
/// (`GpuSplatMaterial`). Multiple `Splat3d` entities can reference the same
/// `SplatMaterial` handle to share settings and avoid duplicate uniforms.
#[derive(Asset, Clone, Debug, Reflect)]
#[reflect(Default)]
pub struct SplatMaterial {
    pub blend: SplatBlend,
    pub shape: SplatShape,
    /// When true, point sizes are interpreted as world units (perspective
    /// scaling). When false, sizes are screen-space pixels.
    pub size_attenuation: bool,
    /// Global opacity multiplier (0.0–1.0), applied on top of per-point alpha.
    pub opacity: f32,
    /// Gaussian falloff exponent for `Circle` shape: alpha *= exp(-d² × k).
    /// Higher values make tighter dots; lower values make softer halos.
    /// Default 4.0 gives a Ryoji-Ikeda-ish soft additive bloom.
    pub falloff_sharpness: f32,
}

impl Default for SplatMaterial {
    fn default() -> Self {
        Self {
            blend: SplatBlend::default(),
            shape: SplatShape::default(),
            size_attenuation: false,
            opacity: 1.0,
            falloff_sharpness: 4.0,
        }
    }
}

/// Component referencing a `SplatMaterial` asset, mirroring `MeshMaterial3d`.
#[derive(Component, Clone, Debug, Default, Reflect, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct SplatMaterial3d(pub Handle<SplatMaterial>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splat_material_defaults() {
        let m = SplatMaterial::default();
        assert_eq!(m.blend, SplatBlend::Additive);
        assert_eq!(m.shape, SplatShape::Circle);
        assert!(!m.size_attenuation);
        assert_eq!(m.opacity, 1.0);
        assert_eq!(m.falloff_sharpness, 4.0);
    }
}
