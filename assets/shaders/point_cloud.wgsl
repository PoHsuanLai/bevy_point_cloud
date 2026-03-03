// Point cloud billboard shader

#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions::get_world_from_local

struct PointData {
    position: vec3<f32>,
    size: f32,
    color: vec4<f32>,
}

struct PointCloudParams {
    size_attenuation: u32,
    base_scale: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<storage, read> points: array<PointData>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
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
    // Triangle 1: BL → BR → TR
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

    // Apply entity Transform to point position
    let world_pos = (get_world_from_local(in.instance_index) * vec4(point.position, 1.0)).xyz;

    // Project point center to clip space
    let clip_center = view.clip_from_world * vec4(world_pos, 1.0);

    // Compute effective point size
    var effective_size = point.size;
    if params.size_attenuation == 1u {
        // Perspective: size decreases with distance
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

    // Sharp pinprick dot with slight soft edge
    let alpha = smoothstep(1.0, 0.7, dist) * in.color.a;
    return vec4(in.color.rgb * alpha, alpha);
}
