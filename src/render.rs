//! Custom render pipeline for instanced point cloud drawing.
//!
//! Issues instanced draw calls: 1 quad mesh × N instances. The vertex shader
//! uses `instance_index` to look up per-point data from the SSBO.

use bevy::{
    core_pipeline::core_3d::Transparent3d,
    ecs::system::{SystemParamItem, lifetimeless::*},
    mesh::MeshVertexBufferLayoutRef,
    pbr::{
        MeshPipeline, MeshPipelineKey, RenderMeshInstances, SetMeshBindGroup, SetMeshViewBindGroup,
        SetMeshViewBindingArrayBindGroup,
    },
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
        mesh::{RenderMesh, RenderMeshBufferInfo, allocator::MeshAllocator},
        render_asset::RenderAssets,
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
use bevy::camera::visibility::RenderLayers;
use bytemuck::Pod;

use crate::{
    material::PointCloudBlend,
    point_cloud::{PointCloud, PointCloudSettings, PointData},
};

/// Extracted point data — only inserted when `PointCloud` component changes.
#[derive(Component, Clone)]
struct ExtractedPointCloudData {
    points: Vec<PointData>,
    initial_capacity: u32,
}

/// Extracted rendering params — inserted on any relevant change
/// (transform, settings, or point data).
#[derive(Component, Clone)]
struct ExtractedPointCloudParams {
    params: GpuPointCloudParams,
    blend: PointCloudBlend,
    render_layers: RenderLayers,
}

/// Layout: mat4x4 (64) + size_attenuation (4) + opacity (4) + shape (4) + pad (4) = 80 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, bytemuck::Zeroable)]
pub(crate) struct GpuPointCloudParams {
    world_from_local: [f32; 16],
    size_attenuation: u32,
    opacity: f32,
    shape: u32,
    _pad: u32,
}

impl Default for GpuPointCloudParams {
    fn default() -> Self {
        Self {
            world_from_local: Mat4::IDENTITY.to_cols_array(),
            size_attenuation: 0,
            opacity: 1.0,
            shape: 0,
            _pad: 0,
        }
    }
}

impl GpuPointCloudParams {
    fn from_settings_and_transform(
        settings: Option<&PointCloudSettings>,
        transform: &GlobalTransform,
    ) -> Self {
        let (size_attenuation, opacity, shape) = match settings {
            Some(s) => (s.size_attenuation as u32, s.opacity, s.shape as u32),
            None => (0, 1.0, 0),
        };
        Self {
            world_from_local: transform.to_matrix().to_cols_array(),
            size_attenuation,
            opacity,
            shape,
            _pad: 0,
        }
    }
}

/// Extract point data only when the `PointCloud` component changes.
fn extract_point_cloud_data(
    mut commands: Commands,
    changed: Extract<Query<(RenderEntity, &PointCloud), Changed<PointCloud>>>,
) {
    for (render_entity, cloud) in &changed {
        if cloud.points.is_empty() {
            commands
                .entity(render_entity)
                .remove::<ExtractedPointCloudData>();
            continue;
        }
        commands
            .entity(render_entity)
            .insert(ExtractedPointCloudData {
                points: cloud.points.clone(),
                initial_capacity: cloud.capacity as u32,
            });
    }
}

/// Extract rendering params on any relevant change (transform, settings, points, mesh).
#[allow(clippy::type_complexity)]
fn extract_point_cloud_params(
    mut commands: Commands,
    changed: Extract<
        Query<
            (
                RenderEntity,
                &PointCloud,
                Option<&PointCloudSettings>,
                &GlobalTransform,
                Option<&RenderLayers>,
            ),
            Or<(
                Changed<PointCloud>,
                Changed<PointCloudSettings>,
                Changed<GlobalTransform>,
                Added<Mesh3d>,
            )>,
        >,
    >,
) {
    for (render_entity, cloud, settings, transform, render_layers) in &changed {
        if cloud.points.is_empty() {
            commands
                .entity(render_entity)
                .remove::<ExtractedPointCloudParams>();
            continue;
        }
        commands
            .entity(render_entity)
            .insert(ExtractedPointCloudParams {
                params: GpuPointCloudParams::from_settings_and_transform(settings, transform),
                blend: settings.map(|s| s.blend).unwrap_or_default(),
                render_layers: render_layers.cloned().unwrap_or_default(),
            });
    }
}

#[derive(Component)]
pub(crate) struct PointCloudGpuBuffers {
    ssbo: Buffer,
    params_buffer: Buffer,
    bind_group: BindGroup,
    instance_count: u32,
    /// Only reallocate SSBO when point count exceeds this.
    ssbo_capacity: u32,
}

#[derive(Resource)]
struct PointCloudPipeline {
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
        "point_cloud_material_layout",
        &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    );

    let shader: Handle<Shader> =
        bevy::asset::load_embedded_asset!(asset_server.as_ref(), "point_cloud.wgsl");

    commands.insert_resource(PointCloudPipeline {
        shader,
        mesh_pipeline: mesh_pipeline.clone(),
        material_layout,
    });
}

impl SpecializedMeshPipeline for PointCloudPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;

        descriptor.vertex.shader = self.shader.clone();
        if let Some(ref mut fragment) = descriptor.fragment {
            fragment.shader = self.shader.clone();
        }

        descriptor.primitive.cull_mode = None;

        let blend_bits = key.intersection(MeshPipelineKey::BLEND_RESERVED_BITS);
        if blend_bits == MeshPipelineKey::BLEND_PREMULTIPLIED_ALPHA {
            if let Some(ref mut fragment) = descriptor.fragment
                && let Some(target) = fragment.targets.first_mut().and_then(|t| t.as_mut())
            {
                target.blend = Some(BlendState {
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
                });
            }
            if let Some(ref mut depth) = descriptor.depth_stencil {
                depth.depth_write_enabled = false;
            }
        } else if blend_bits == MeshPipelineKey::BLEND_ALPHA {
            if let Some(ref mut fragment) = descriptor.fragment
                && let Some(target) = fragment.targets.first_mut().and_then(|t| t.as_mut())
            {
                target.blend = Some(BlendState::ALPHA_BLENDING);
            }
            if let Some(ref mut depth) = descriptor.depth_stencil {
                depth.depth_write_enabled = false;
            }
        } else if let Some(ref mut depth) = descriptor.depth_stencil {
            depth.depth_write_enabled = true;
        }

        descriptor.layout.push(self.material_layout.clone());

        Ok(descriptor)
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_point_clouds(
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<PointCloudPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<PointCloudPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    point_clouds: Query<(
        Entity,
        &MainEntity,
        &ExtractedPointCloudParams,
        &PointCloudGpuBuffers,
    )>,
    mut transparent_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<(&ExtractedView, &Msaa, Option<&RenderLayers>)>,
) {
    let draw_fn = transparent_draw_functions.read().id::<DrawPointCloud>();

    for (view, msaa, view_layers) in &views {
        let view_mask = view_layers.cloned().unwrap_or_default();
        let Some(phase) = transparent_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());
        let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);
        let rangefinder = view.rangefinder3d();

        for (entity, main_entity, extracted, _gpu_buffers) in &point_clouds {
            if !view_mask.intersects(&extracted.render_layers) {
                continue;
            }
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*main_entity)
            else {
                continue;
            };
            let Some(mesh) = meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            let blend_key = match extracted.blend {
                PointCloudBlend::Additive => MeshPipelineKey::BLEND_PREMULTIPLIED_ALPHA,
                PointCloudBlend::Alpha => MeshPipelineKey::BLEND_ALPHA,
                PointCloudBlend::Opaque => MeshPipelineKey::NONE,
            };

            let key = view_key
                | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology())
                | blend_key;
            let Ok(pipeline_id) =
                pipelines.specialize(&pipeline_cache, &pipeline, key, &mesh.layout)
            else {
                continue;
            };

            phase.add(Transparent3d {
                entity: (entity, *main_entity),
                pipeline: pipeline_id,
                draw_function: draw_fn,
                distance: rangefinder.distance_translation(&mesh_instance.translation),
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: true,
            });
        }
    }
}

fn prepare_point_cloud_buffers(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        Option<&ExtractedPointCloudData>,
        &ExtractedPointCloudParams,
        Option<&mut PointCloudGpuBuffers>,
    )>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline: Res<PointCloudPipeline>,
) {
    for (entity, data, params, existing) in &mut query {
        let param_bytes: &[u8] = bytemuck::bytes_of(&params.params);

        if let Some(data) = data {
            let point_bytes: &[u8] = bytemuck::cast_slice(&data.points);
            let point_count = data.points.len() as u32;

            if let Some(mut buffers) = existing
                && point_count <= buffers.ssbo_capacity
            {
                // Reuse existing SSBO — mutate in place to avoid change-detection churn.
                render_queue.write_buffer(&buffers.ssbo, 0, point_bytes);
                render_queue.write_buffer(&buffers.params_buffer, 0, param_bytes);
                buffers.instance_count = point_count;
            } else {
                // Allocate new SSBO, respecting user-specified capacity hint.
                let capacity = point_count.max(data.initial_capacity).max(64);
                let capacity = capacity + capacity / 4; // 25% headroom
                let ssbo_size = capacity as u64 * std::mem::size_of::<PointData>() as u64;

                let ssbo = render_device.create_buffer(&BufferDescriptor {
                    label: Some("point_cloud_ssbo"),
                    size: ssbo_size,
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                render_queue.write_buffer(&ssbo, 0, point_bytes);

                let params_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("point_cloud_params"),
                    contents: param_bytes,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });

                let bind_group = render_device.create_bind_group(
                    "point_cloud_bind_group",
                    &pipeline.material_layout,
                    &[
                        BindGroupEntry {
                            binding: 0,
                            resource: ssbo.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: params_buffer.as_entire_binding(),
                        },
                    ],
                );

                commands.entity(entity).insert(PointCloudGpuBuffers {
                    ssbo,
                    params_buffer,
                    bind_group,
                    instance_count: point_count,
                    ssbo_capacity: capacity,
                });
            }

            // Remove extracted data so it's not re-uploaded next frame.
            commands
                .entity(entity)
                .remove::<ExtractedPointCloudData>();
        } else if let Some(buffers) = existing {
            // No new point data — only update the params uniform (80 bytes).
            render_queue.write_buffer(&buffers.params_buffer, 0, param_bytes);
        }
    }
}

type DrawPointCloud = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetMeshBindGroup<2>,
    SetPointCloudBindGroup<3>,
    DrawPointCloudInstanced,
);

struct SetPointCloudBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetPointCloudBindGroup<I> {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = Read<PointCloudGpuBuffers>;

    fn render<'w>(
        _item: &P,
        _view: (),
        gpu_buffers: Option<&'w PointCloudGpuBuffers>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(gpu_buffers) = gpu_buffers else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, &gpu_buffers.bind_group, &[]);
        RenderCommandResult::Success
    }
}

struct DrawPointCloudInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawPointCloudInstanced {
    type Param = (
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderMeshInstances>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = Read<PointCloudGpuBuffers>;

    fn render<'w>(
        item: &P,
        _view: (),
        gpu_buffers: Option<&'w PointCloudGpuBuffers>,
        (meshes, render_mesh_instances, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh_allocator = mesh_allocator.into_inner();

        let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(item.main_entity())
        else {
            return RenderCommandResult::Skip;
        };
        let Some(gpu_mesh) = meshes.into_inner().get(mesh_instance.mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(gpu_buffers) = gpu_buffers else {
            return RenderCommandResult::Skip;
        };
        let Some(vertex_buffer_slice) =
            mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id)
        else {
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));

        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_buffer_slice) =
                    mesh_allocator.mesh_index_slice(&mesh_instance.mesh_asset_id)
                else {
                    return RenderCommandResult::Skip;
                };

                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), 0, *index_format);
                pass.draw_indexed(
                    index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                    vertex_buffer_slice.range.start as i32,
                    0..gpu_buffers.instance_count,
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                pass.draw(
                    vertex_buffer_slice.range.clone(),
                    0..gpu_buffers.instance_count,
                );
            }
        }
        RenderCommandResult::Success
    }
}

pub(crate) struct PointCloudRenderPlugin;

impl Plugin for PointCloudRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SyncComponentPlugin::<PointCloud>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_command::<Transparent3d, DrawPointCloud>()
            .init_resource::<SpecializedMeshPipelines<PointCloudPipeline>>()
            .add_systems(
                ExtractSchedule,
                (extract_point_cloud_data, extract_point_cloud_params),
            )
            .add_systems(RenderStartup, init_pipeline)
            .add_systems(
                Render,
                (
                    queue_point_clouds.in_set(RenderSystems::QueueMeshes),
                    prepare_point_cloud_buffers.in_set(RenderSystems::PrepareResources),
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_params_default() {
        let params = GpuPointCloudParams::default();
        assert_eq!(params.size_attenuation, 0);
        assert_eq!(params.opacity, 1.0);
        assert_eq!(params.shape, 0);
        // Identity matrix
        let identity = Mat4::IDENTITY.to_cols_array();
        assert_eq!(params.world_from_local, identity);
    }

    #[test]
    fn gpu_params_from_default_settings() {
        let transform = GlobalTransform::from(Transform::from_xyz(1.0, 2.0, 3.0));
        let params = GpuPointCloudParams::from_settings_and_transform(None, &transform);
        assert_eq!(params.size_attenuation, 0);
        assert_eq!(params.opacity, 1.0);
        assert_eq!(params.shape, 0);
        assert_eq!(
            params.world_from_local,
            transform.to_matrix().to_cols_array()
        );
    }

    #[test]
    fn gpu_params_from_custom_settings() {
        use crate::material::PointCloudShape;
        let settings = PointCloudSettings {
            blend: PointCloudBlend::Alpha,
            size_attenuation: true,
            opacity: 0.5,
            shape: PointCloudShape::Square,
        };
        let transform = GlobalTransform::IDENTITY;
        let params =
            GpuPointCloudParams::from_settings_and_transform(Some(&settings), &transform);
        assert_eq!(params.size_attenuation, 1);
        assert_eq!(params.opacity, 0.5);
        assert_eq!(params.shape, 1);
    }
}
