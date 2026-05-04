//! Custom render pipeline for instanced splat drawing.
//!
//! Issues one indexed draw call per `Splat3d` entity: a shared 4-vertex /
//! 6-index quad mesh, instanced N times. Per-point data lives in a per-
//! instance vertex buffer (no SSBO). Visual settings come from a shared
//! `SplatMaterial` asset (one uniform + bind group per material).

use bevy::camera::visibility::RenderLayers;
use bevy::mesh::VertexBufferLayout as MeshVertexBufferLayout;
use bevy::{
    core_pipeline::core_3d::Transparent3d,
    ecs::system::{SystemParamItem, lifetimeless::*},
    pbr::{MeshPipeline, MeshPipelineKey, SetMeshViewBindGroup},
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        sync_component::SyncComponentPlugin,
        sync_world::{MainEntity, RenderEntity},
        view::ExtractedView,
    },
};
use bytemuck::{Pod, Zeroable};

use crate::splat::{Splat, Splat3d, SplatPoint};
use crate::splat_material::{SplatBlend, SplatMaterial, SplatMaterial3d};

// ---------------------------------------------------------------------------
// Quad mesh (shared, render-world resource)
// ---------------------------------------------------------------------------

/// One CCW unit quad: 4 vertices, 6 indices. Shared across all splat draws.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct QuadVertex {
    offset: [f32; 2],
    _pad: [f32; 2],
}

const QUAD_VERTICES: [QuadVertex; 4] = [
    QuadVertex { offset: [-1.0, -1.0], _pad: [0.0; 2] },
    QuadVertex { offset: [ 1.0, -1.0], _pad: [0.0; 2] },
    QuadVertex { offset: [ 1.0,  1.0], _pad: [0.0; 2] },
    QuadVertex { offset: [-1.0,  1.0], _pad: [0.0; 2] },
];
const QUAD_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

#[derive(Resource)]
struct SplatQuadMesh {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
}

fn init_quad_mesh(mut commands: Commands, render_device: Res<RenderDevice>) {
    let vertex_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("splat_quad_vertices"),
        contents: bytemuck::cast_slice(&QUAD_VERTICES),
        usage: BufferUsages::VERTEX,
    });
    let index_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("splat_quad_indices"),
        contents: bytemuck::cast_slice(&QUAD_INDICES),
        usage: BufferUsages::INDEX,
    });
    commands.insert_resource(SplatQuadMesh {
        vertex_buffer,
        index_buffer,
        index_count: QUAD_INDICES.len() as u32,
    });
}

// ---------------------------------------------------------------------------
// GPU material uniform
// ---------------------------------------------------------------------------

/// 80-byte uniform: world transform (64) + flags/values (16). Std140-safe.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct GpuSplatMaterial {
    world_from_local: [f32; 16],
    size_attenuation: u32,
    opacity: f32,
    shape: u32,
    falloff_sharpness: f32,
}

impl GpuSplatMaterial {
    fn new(material: &SplatMaterial, transform: &GlobalTransform) -> Self {
        Self {
            world_from_local: transform.to_matrix().to_cols_array(),
            size_attenuation: material.size_attenuation as u32,
            opacity: material.opacity,
            shape: material.shape as u32,
            falloff_sharpness: material.falloff_sharpness,
        }
    }
}

// ---------------------------------------------------------------------------
// Extracted components (live in the render world)
// ---------------------------------------------------------------------------

#[derive(Component, Clone)]
struct ExtractedSplatInstances {
    points: Vec<SplatPoint>,
    initial_capacity: u32,
}

#[derive(Component, Clone)]
struct ExtractedSplatParams {
    uniform: GpuSplatMaterial,
    blend: SplatBlend,
    render_layers: RenderLayers,
}

#[allow(clippy::type_complexity)]
fn extract_splat_instances(
    mut commands: Commands,
    splats: Extract<Res<Assets<Splat>>>,
    mut events: Extract<MessageReader<AssetEvent<Splat>>>,
    mut dirty: Local<bevy::platform::collections::HashSet<AssetId<Splat>>>,
    query: Extract<Query<(RenderEntity, &Splat3d)>>,
) {
    for event in events.read() {
        match event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::LoadedWithDependencies { id } => {
                dirty.insert(*id);
            }
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => {
                dirty.remove(id);
            }
        }
    }
    if dirty.is_empty() {
        return;
    }

    for (render_entity, splat3d) in &query {
        let id = splat3d.0.id();
        if !dirty.contains(&id) {
            continue;
        }
        let Some(splat) = splats.get(id) else {
            continue;
        };
        if splat.points.is_empty() {
            commands
                .entity(render_entity)
                .remove::<ExtractedSplatInstances>();
            continue;
        }
        commands
            .entity(render_entity)
            .insert(ExtractedSplatInstances {
                points: splat.points.clone(),
                initial_capacity: splat.capacity as u32,
            });
    }

    dirty.clear();
}

#[allow(clippy::type_complexity)]
fn extract_splat_params(
    mut commands: Commands,
    materials: Extract<Res<Assets<SplatMaterial>>>,
    query: Extract<
        Query<(
            RenderEntity,
            &Splat3d,
            &SplatMaterial3d,
            &GlobalTransform,
            Option<&RenderLayers>,
        )>,
    >,
) {
    for (render_entity, _splat3d, material3d, transform, render_layers) in &query {
        let Some(material) = materials.get(&material3d.0) else {
            continue;
        };
        commands.entity(render_entity).insert(ExtractedSplatParams {
            uniform: GpuSplatMaterial::new(material, transform),
            blend: material.blend,
            render_layers: render_layers.cloned().unwrap_or_default(),
        });
    }
}

// ---------------------------------------------------------------------------
// GPU-side per-entity buffers
// ---------------------------------------------------------------------------

#[derive(Component)]
pub(crate) struct SplatGpuBuffers {
    instance_buffer: Buffer,
    params_buffer: Buffer,
    bind_group: BindGroup,
    instance_count: u32,
    capacity: u32,
}

#[derive(Resource)]
struct SplatPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    material_layout: BindGroupLayout,
}

fn init_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mesh_pipeline: Res<MeshPipeline>,
    render_device: Res<RenderDevice>,
) {
    let material_layout = render_device.create_bind_group_layout(
        "splat_material_layout",
        &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX_FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    );

    let shader: Handle<Shader> =
        bevy::asset::load_embedded_asset!(asset_server.as_ref(), "splat.wgsl");

    commands.insert_resource(SplatPipeline {
        shader,
        mesh_pipeline: mesh_pipeline.clone(),
        material_layout,
    });
}

// ---------------------------------------------------------------------------
// Pipeline specialization
// ---------------------------------------------------------------------------

impl SpecializedRenderPipeline for SplatPipeline {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let format = if key.contains(MeshPipelineKey::HDR) {
            bevy::render::view::ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };
        let sample_count = key.msaa_samples();

        let quad_layout = MeshVertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: vec![VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            }],
        };
        let instance_layout = MeshVertexBufferLayout {
            array_stride: std::mem::size_of::<SplatPoint>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 1,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 2,
                },
            ],
        };

        let blend_bits = key.intersection(MeshPipelineKey::BLEND_RESERVED_BITS);
        let (blend, depth_write) = if blend_bits == MeshPipelineKey::BLEND_PREMULTIPLIED_ALPHA {
            (
                Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                }),
                false,
            )
        } else if blend_bits == MeshPipelineKey::BLEND_ALPHA {
            (Some(BlendState::ALPHA_BLENDING), false)
        } else {
            (None, true)
        };

        RenderPipelineDescriptor {
            label: Some("splat_pipeline".into()),
            layout: vec![
                self.mesh_pipeline.get_view_layout(key.into()).main_layout.clone(),
                self.material_layout.clone(),
            ],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: Some("vertex".into()),
                buffers: vec![quad_layout, instance_layout],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: bevy::core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: depth_write,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            zero_initialize_workgroup_memory: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Queue
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn queue_splats(
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<SplatPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SplatPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    splats: Query<(Entity, &MainEntity, &ExtractedSplatParams, &SplatGpuBuffers)>,
    mut transparent_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<(&ExtractedView, &Msaa, Option<&RenderLayers>)>,
) {
    let draw_fn = transparent_draw_functions.read().id::<DrawSplat>();

    for (view, msaa, view_layers) in &views {
        let view_mask = view_layers.cloned().unwrap_or_default();
        let Some(phase) = transparent_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);
        let rangefinder = view.rangefinder3d();

        for (entity, main_entity, extracted, _gpu) in &splats {
            if !view_mask.intersects(&extracted.render_layers) {
                continue;
            }

            let blend_key = match extracted.blend {
                SplatBlend::Additive => MeshPipelineKey::BLEND_PREMULTIPLIED_ALPHA,
                SplatBlend::Alpha => MeshPipelineKey::BLEND_ALPHA,
                SplatBlend::Opaque => MeshPipelineKey::NONE,
            };
            let key = view_key | blend_key;
            let pipeline_id = pipelines.specialize(&pipeline_cache, &pipeline, key);

            let translation = Vec3::new(
                extracted.uniform.world_from_local[12],
                extracted.uniform.world_from_local[13],
                extracted.uniform.world_from_local[14],
            );
            phase.add(Transparent3d {
                entity: (entity, *main_entity),
                pipeline: pipeline_id,
                draw_function: draw_fn,
                distance: rangefinder.distance_translation(&translation),
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: true,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Prepare GPU buffers
// ---------------------------------------------------------------------------

fn prepare_splat_buffers(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        Option<&ExtractedSplatInstances>,
        &ExtractedSplatParams,
        Option<&mut SplatGpuBuffers>,
    )>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline: Res<SplatPipeline>,
) {
    for (entity, instances, params, existing) in &mut query {
        let param_bytes: &[u8] = bytemuck::bytes_of(&params.uniform);

        if let Some(instances) = instances {
            let point_bytes: &[u8] = bytemuck::cast_slice(&instances.points);
            let point_count = instances.points.len() as u32;

            if let Some(mut buffers) = existing
                && point_count <= buffers.capacity
            {
                render_queue.write_buffer(&buffers.instance_buffer, 0, point_bytes);
                render_queue.write_buffer(&buffers.params_buffer, 0, param_bytes);
                buffers.instance_count = point_count;
            } else {
                let capacity = point_count.max(instances.initial_capacity).max(64);
                let capacity = capacity + capacity / 4;
                let buffer_size = capacity as u64 * std::mem::size_of::<SplatPoint>() as u64;

                let instance_buffer = render_device.create_buffer(&BufferDescriptor {
                    label: Some("splat_instance_buffer"),
                    size: buffer_size,
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                render_queue.write_buffer(&instance_buffer, 0, point_bytes);

                let params_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("splat_params_uniform"),
                    contents: param_bytes,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });

                let bind_group = render_device.create_bind_group(
                    "splat_material_bind_group",
                    &pipeline.material_layout,
                    &[BindGroupEntry {
                        binding: 0,
                        resource: params_buffer.as_entire_binding(),
                    }],
                );

                commands.entity(entity).insert(SplatGpuBuffers {
                    instance_buffer,
                    params_buffer,
                    bind_group,
                    instance_count: point_count,
                    capacity,
                });
            }

            commands.entity(entity).remove::<ExtractedSplatInstances>();
        } else if let Some(buffers) = existing {
            render_queue.write_buffer(&buffers.params_buffer, 0, param_bytes);
        }
    }
}

// ---------------------------------------------------------------------------
// Draw command
// ---------------------------------------------------------------------------

type DrawSplat = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetSplatBindGroup<1>,
    DrawSplatInstanced,
);

struct SetSplatBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSplatBindGroup<I> {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = Read<SplatGpuBuffers>;

    fn render<'w>(
        _item: &P,
        _view: (),
        gpu: Option<&'w SplatGpuBuffers>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(gpu) = gpu else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, &gpu.bind_group, &[]);
        RenderCommandResult::Success
    }
}

struct DrawSplatInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawSplatInstanced {
    type Param = SRes<SplatQuadMesh>;
    type ViewQuery = ();
    type ItemQuery = Read<SplatGpuBuffers>;

    fn render<'w>(
        _item: &P,
        _view: (),
        gpu: Option<&'w SplatGpuBuffers>,
        quad: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(gpu) = gpu else {
            return RenderCommandResult::Skip;
        };
        let quad = quad.into_inner();
        pass.set_vertex_buffer(0, quad.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, gpu.instance_buffer.slice(..));
        pass.set_index_buffer(quad.index_buffer.slice(..), 0, IndexFormat::Uint32);
        pass.draw_indexed(0..quad.index_count, 0, 0..gpu.instance_count);
        RenderCommandResult::Success
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub(crate) struct SplatRenderPlugin;

impl Plugin for SplatRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SyncComponentPlugin::<Splat3d>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_command::<Transparent3d, DrawSplat>()
            .init_resource::<SpecializedRenderPipelines<SplatPipeline>>()
            .add_systems(
                ExtractSchedule,
                (extract_splat_instances, extract_splat_params),
            )
            .add_systems(RenderStartup, (init_pipeline, init_quad_mesh))
            .add_systems(
                Render,
                (
                    queue_splats.in_set(RenderSystems::QueueMeshes),
                    prepare_splat_buffers.in_set(RenderSystems::PrepareResources),
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_material_packs_to_80_bytes() {
        assert_eq!(std::mem::size_of::<GpuSplatMaterial>(), 80);
    }

    #[test]
    fn gpu_material_from_default() {
        let m = SplatMaterial::default();
        let t = GlobalTransform::IDENTITY;
        let gpu = GpuSplatMaterial::new(&m, &t);
        assert_eq!(gpu.size_attenuation, 0);
        assert_eq!(gpu.opacity, 1.0);
        assert_eq!(gpu.shape, 0);
        assert_eq!(gpu.falloff_sharpness, 4.0);
    }

    #[test]
    fn gpu_material_carries_translation() {
        let t = GlobalTransform::from(Transform::from_xyz(1.0, 2.0, 3.0));
        let gpu = GpuSplatMaterial::new(&SplatMaterial::default(), &t);
        assert_eq!(gpu.world_from_local[12], 1.0);
        assert_eq!(gpu.world_from_local[13], 2.0);
        assert_eq!(gpu.world_from_local[14], 3.0);
    }

    #[test]
    fn quad_geometry_is_two_triangles() {
        assert_eq!(QUAD_VERTICES.len(), 4);
        assert_eq!(QUAD_INDICES.len(), 6);
    }
}
