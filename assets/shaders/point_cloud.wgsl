// Point cloud billboard shader

#import bevy_pbr::mesh_view_bindings::view

struct PointData {
    position: vec3<f32>,
    size: f32,
    color: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<storage, read> points: array<PointData>;

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
    let point_index = in.vertex_index / 6u;
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

    // CCW winding for front-facing billboards:
    // Triangle 1: BL → BR → TR (CCW from front = +Z)
    // Triangle 2: BL → TR → TL
    var offsets = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0),  // BL
        vec2( 1.0, -1.0),  // BR
        vec2( 1.0,  1.0),  // TR
        vec2(-1.0, -1.0),  // BL
        vec2( 1.0,  1.0),  // TR
        vec2(-1.0,  1.0),  // TL
    );
    let offset = offsets[corner];

    // Project point center to clip space
    let clip_center = view.clip_from_world * vec4(point.position, 1.0);

    // Screen-space pixel offset → clip space
    let resolution = view.viewport.zw;
    let pixel_offset = offset * point.size;
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

    let alpha = smoothstep(1.0, 0.2, dist) * in.color.a;
    return vec4(in.color.rgb * alpha, alpha);
}
