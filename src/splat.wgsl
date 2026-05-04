// Splat billboard shader — instanced, per-instance vertex buffer.
// Each instance is one point; the shared 4-vertex quad is reused via
// indexed draw. Per-instance attributes carry position+size and color.

#import bevy_pbr::mesh_view_bindings::view

struct SplatMaterial {
    world_from_local: mat4x4<f32>,
    size_attenuation: u32,
    opacity: f32,
    shape: u32,             // 0 = circle, 1 = square
    falloff_sharpness: f32, // Gaussian k for circle: alpha *= exp(-d² × k)
}

@group(1) @binding(0)
var<uniform> material: SplatMaterial;

struct SplatVertex {
    @location(0) corner_offset: vec2<f32>,   // per-vertex: -1..1 quad corner
    @location(1) i_pos_size: vec4<f32>,      // per-instance: xyz = pos, w = size
    @location(2) i_color: vec4<f32>,         // per-instance: rgba
}

struct SplatVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) quad_uv: vec2<f32>,
}

@vertex
fn vertex(in: SplatVertex) -> SplatVertexOutput {
    let world_pos = (material.world_from_local * vec4(in.i_pos_size.xyz, 1.0)).xyz;
    let clip_center = view.clip_from_world * vec4(world_pos, 1.0);

    // size: pixels by default, world units when size_attenuation=1.
    var effective_size = in.i_pos_size.w;
    let resolution = view.viewport.zw;
    if material.size_attenuation == 1u {
        effective_size = in.i_pos_size.w * resolution.y * view.clip_from_world[1][1] / (2.0 * clip_center.w);
    }

    let pixel_offset = in.corner_offset * effective_size;
    let clip_offset = pixel_offset * 2.0 * clip_center.w / resolution;

    var out: SplatVertexOutput;
    out.clip_position = clip_center + vec4(clip_offset, 0.0, 0.0);
    out.color = in.i_color;
    out.quad_uv = in.corner_offset;
    return out;
}

@fragment
fn fragment(in: SplatVertexOutput) -> @location(0) vec4<f32> {
    var alpha = in.color.a;

    if material.shape == 0u {
        // Circle: hard cutoff at radius=1 to keep quad corners cheap, then
        // Gaussian falloff for soft additive bloom.
        let dist_sq = dot(in.quad_uv, in.quad_uv);
        if dist_sq > 1.0 {
            discard;
        }
        alpha *= exp(-dist_sq * material.falloff_sharpness);
    }
    // shape == 1: square — no discard, no falloff.

    alpha *= material.opacity;
    return vec4(in.color.rgb, alpha);
}
