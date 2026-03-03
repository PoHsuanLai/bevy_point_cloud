// Point cloud billboard shader — instanced rendering
// Each instance = one point. vertex_index selects quad corner (0-5).
// instance_index selects point data from the SSBO.

#import bevy_pbr::mesh_view_bindings::view

struct PointData {
    position: vec3<f32>,
    size: f32,
    color: vec4<f32>,
}

struct PointCloudParams {
    world_from_local: mat4x4<f32>,
    size_attenuation: u32,
    base_scale: f32,
    _pad: vec2<f32>,
}

@group(3) @binding(0)
var<storage, read> points: array<PointData>;

@group(3) @binding(1)
var<uniform> params: PointCloudParams;

struct PcVertex {
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct PcVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) quad_uv: vec2<f32>,
}

@vertex
fn vertex(in: PcVertex) -> PcVertexOutput {
    let point_index = in.instance_index;
    let corner = in.vertex_index % 6u;
    let num_points = arrayLength(&points);

    var out: PcVertexOutput;

    if point_index >= num_points {
        out.clip_position = vec4(0.0, 0.0, -2.0, 1.0);
        out.color = vec4(0.0);
        out.quad_uv = vec2(0.0);
        return out;
    }

    let point = points[point_index];

    // Billboard quad corners (2 triangles, CCW winding)
    var offsets = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0),  // BL
        vec2( 1.0, -1.0),  // BR
        vec2( 1.0,  1.0),  // TR
        vec2(-1.0, -1.0),  // BL
        vec2( 1.0,  1.0),  // TR
        vec2(-1.0,  1.0),  // TL
    );
    let offset = offsets[corner];

    // Transform point from local to world space
    let world_pos = (params.world_from_local * vec4(point.position, 1.0)).xyz;

    // Project point center to clip space
    let clip_center = view.clip_from_world * vec4(world_pos, 1.0);

    // Compute effective point size
    var effective_size = point.size;
    if params.size_attenuation == 1u {
        effective_size = point.size * params.base_scale / clip_center.w;
    }

    // Screen-space pixel offset → clip space
    let resolution = view.viewport.zw;
    let pixel_offset = offset * effective_size;
    let clip_offset = pixel_offset * 2.0 * clip_center.w / resolution;

    out.clip_position = clip_center + vec4(clip_offset, 0.0, 0.0);
    out.color = point.color;
    out.quad_uv = offset;
    return out;
}

@fragment
fn fragment(in: PcVertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.quad_uv);
    if dist > 1.0 {
        discard;
    }

    // Soft circular dot
    let alpha = smoothstep(1.0, 0.7, dist) * in.color.a;
    return vec4(in.color.rgb, alpha);
}
