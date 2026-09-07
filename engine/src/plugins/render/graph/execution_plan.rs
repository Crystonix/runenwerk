use super::{
    CompiledPassDescriptor, RenderPassKind, RenderPassNode, RenderPassViewScope, ResourceGraph,
};
use crate::plugins::render::RenderImportedTextureSemantic;
use crate::plugins::render::api::ids::RenderFeatureId;
use crate::plugins::render::api::{ComputeDispatchDescriptor, RenderShaderBindingResource};
use crate::plugins::render::features::UI_RENDER_FEATURE_ID;
use crate::plugins::render::{
    RenderDrawDescriptor, RenderDrawSource, RenderFixedStepRegionId, RenderIndirectDrawArgsKind,
    RenderPassId, RenderRasterState, RenderResourceDeclaration, RenderShaderConstant,
    RenderShaderReference, RenderTargetAliasKey, RenderTargetAliasKind, RenderVertexAttribute,
    RenderVertexBufferLayout, RenderVertexStepMode,
};
use runen_gpu::{
    GpuBindingKey, GpuStorageBufferAccess, GpuStorageTextureAccess, GpuWorkResourceId,
};
use std::any::TypeId;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default)]
pub struct CompiledFlowExecutionPlan {
    pub required_state_types: Vec<CompiledStateRequirement>,
    pub passes: Vec<CompiledPassExecutionPlan>,
    pub fixed_step_regions: Vec<CompiledFixedStepRegion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledStateRequirement {
    pub type_id: TypeId,
    pub type_name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledFixedStepRegion {
    pub region_id: RenderFixedStepRegionId,
    pub region_label: String,
    pub max_substeps: u32,
    pub iteration_uniform: GpuWorkResourceId,
    pub pass_ids: Vec<RenderPassId>,
}

#[derive(Debug, Clone)]
pub enum CompiledPassExecutionPlan {
    Compute(CompiledComputeExecutionPlan),
    Fullscreen(CompiledRasterExecutionPlan),
    Graphics(CompiledRasterExecutionPlan),
    Copy(CompiledCopyExecutionPlan),
    Present(CompiledPresentExecutionPlan),
    BuiltinUiComposite(CompiledUiCompositeExecutionPlan),
}

#[derive(Debug, Clone)]
pub struct CompiledComputeExecutionPlan {
    pub pass_id: RenderPassId,
    /// Lexical source position retained for provenance, not scheduling.
    pub authoring_index: usize,
    pub feature_id: Option<RenderFeatureId>,
    pub shader: Option<RenderShaderReference>,
    pub shader_constants: Vec<RenderShaderConstant>,
    pub view_mask: CompiledViewMask,
    pub bindings: CompiledPassBindings,
    pub dispatch: Option<CompiledDispatchPlan>,
}

#[derive(Debug, Clone)]
pub struct CompiledRasterExecutionPlan {
    pub pass_id: RenderPassId,
    /// Lexical source position retained for provenance, not scheduling.
    pub authoring_index: usize,
    pub feature_id: Option<RenderFeatureId>,
    pub shader: Option<RenderShaderReference>,
    pub view_mask: CompiledViewMask,
    pub bindings: CompiledPassBindings,
    pub targets: CompiledTargetPlan,
    pub draw_buffers: CompiledDrawBufferPlan,
    pub raster_state: CompiledRasterState,
    pub clear_color: Option<[f32; 4]>,
    pub draw: Option<CompiledDrawPlan>,
}

#[derive(Debug, Clone)]
pub struct CompiledCopyExecutionPlan {
    pub pass_id: RenderPassId,
    /// Lexical source position retained for provenance, not scheduling.
    pub authoring_index: usize,
    pub feature_id: Option<RenderFeatureId>,
    pub view_mask: CompiledViewMask,
    pub source: Option<CompiledResourceRef>,
    pub destination: Option<CompiledResourceRef>,
}

#[derive(Debug, Clone)]
pub struct CompiledPresentExecutionPlan {
    pub pass_id: RenderPassId,
    /// Lexical source position retained for provenance, not scheduling.
    pub authoring_index: usize,
    pub feature_id: Option<RenderFeatureId>,
    pub view_mask: CompiledViewMask,
    pub source: Option<CompiledResourceRef>,
}

#[derive(Debug, Clone)]
pub struct CompiledUiCompositeExecutionPlan {
    pub pass_id: RenderPassId,
    /// Lexical source position retained for provenance, not scheduling.
    pub authoring_index: usize,
    pub feature_id: RenderFeatureId,
    pub view_mask: CompiledViewMask,
    pub color_output: CompiledResourceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CompiledViewMask {
    #[default]
    AllViews,
    MainSurfaceOnly,
    OffscreenProductsOnly,
    Explicit(BTreeSet<String>),
}

impl CompiledViewMask {
    pub fn includes(
        &self,
        view_id: &str,
        view_kind: crate::plugins::render::PreparedViewKind,
    ) -> bool {
        match self {
            Self::AllViews => true,
            Self::MainSurfaceOnly => matches!(
                view_kind,
                crate::plugins::render::PreparedViewKind::MainSurface
            ),
            Self::OffscreenProductsOnly => {
                matches!(
                    view_kind,
                    crate::plugins::render::PreparedViewKind::OffscreenProduct
                )
            }
            Self::Explicit(values) => values.contains(view_id),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompiledPassBindings {
    pub bind_group: CompiledBindGroupPlan,
    pub uniform_order: Vec<GpuWorkResourceId>,
    pub storage_order: Vec<CompiledStorageBinding>,
}

#[derive(Debug, Clone, Default)]
pub struct CompiledBindGroupPlan {
    pub entries: Vec<CompiledBindingEntry>,
}

#[derive(Debug, Clone)]
pub enum CompiledBindingEntry {
    SampledTexture {
        key: GpuBindingKey,
        resource: CompiledResourceRef,
    },
    Sampler {
        key: GpuBindingKey,
    },
    StorageTexture {
        key: GpuBindingKey,
        resource: CompiledResourceRef,
        access: CompiledStorageAccess,
    },
    UniformBuffer {
        key: GpuBindingKey,
        resource: GpuWorkResourceId,
    },
    StorageBuffer {
        key: GpuBindingKey,
        resource: CompiledResourceRef,
        access: CompiledStorageAccess,
    },
}

impl CompiledBindingEntry {
    pub const fn key(&self) -> GpuBindingKey {
        match self {
            Self::SampledTexture { key, .. }
            | Self::Sampler { key }
            | Self::StorageTexture { key, .. }
            | Self::UniformBuffer { key, .. }
            | Self::StorageBuffer { key, .. } => *key,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompiledStorageAccess {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Default)]
pub struct CompiledTargetPlan {
    pub color_outputs: Vec<CompiledResourceRef>,
    pub depth_output: Option<CompiledResourceRef>,
}

#[derive(Debug, Clone, Default)]
pub struct CompiledDrawBufferPlan {
    pub vertex_buffers: Vec<CompiledVertexBufferBinding>,
    pub instance_buffers: Vec<CompiledResourceRef>,
    pub instance_buffer_layouts: Vec<CompiledVertexBufferLayout>,
    pub index_buffers: Vec<CompiledResourceRef>,
    pub indirect_buffers: Vec<CompiledResourceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledVertexBufferBinding {
    pub resource: CompiledResourceRef,
    pub layout: CompiledVertexBufferLayout,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompiledVertexBufferLayout {
    pub slot: u32,
    pub array_stride: u64,
    pub step_mode: RenderVertexStepMode,
    pub attributes: Vec<RenderVertexAttribute>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompiledDrawPlan {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
    pub source: CompiledDrawSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompiledDrawSource {
    Direct,
    Indirect {
        args_buffer: GpuWorkResourceId,
        args_kind: RenderIndirectDrawArgsKind,
        args_element_count: u64,
        args_element_size: u64,
        byte_offset: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompiledRasterState {
    pub state: RenderRasterState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompiledDispatchPlan {
    Fixed([u32; 3]),
    FromState {
        state_type_id: TypeId,
        state_type_name: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledResourceRef {
    FlowOwned(GpuWorkResourceId),
    TargetAlias(CompiledTargetAliasRef),
    ImportedBuiltin(CompiledBuiltinImport),
    Imported(GpuWorkResourceId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledTargetAliasRef {
    pub resource_id: GpuWorkResourceId,
    pub binding_key: RenderTargetAliasKey,
    pub kind: RenderTargetAliasKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompiledBuiltinImport {
    SurfaceColor,
    SurfaceDepth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledStorageBinding {
    pub resource: CompiledResourceRef,
    pub access: CompiledStorageAccess,
}

pub fn compile_execution_plan(
    resources: &ResourceGraph,
    render_passes: &[CompiledPassDescriptor],
) -> CompiledFlowExecutionPlan {
    let mut required_state_types = Vec::<CompiledStateRequirement>::new();
    let mut seen_state_types = BTreeSet::<TypeId>::new();
    for state in &resources.state_resources {
        if seen_state_types.insert(state.type_id) {
            required_state_types.push(CompiledStateRequirement {
                type_id: state.type_id,
                type_name: state.type_name,
            });
        }
    }

    let passes = render_passes
        .iter()
        .map(|pass| compile_pass_execution(pass, resources))
        .collect();
    let fixed_step_regions = compile_fixed_step_regions(render_passes);

    CompiledFlowExecutionPlan {
        required_state_types,
        passes,
        fixed_step_regions,
    }
}

fn compile_fixed_step_regions(
    render_passes: &[CompiledPassDescriptor],
) -> Vec<CompiledFixedStepRegion> {
    let mut regions = BTreeMap::<RenderFixedStepRegionId, CompiledFixedStepRegion>::new();
    for pass in render_passes {
        let Some(region) = pass.node().fixed_step_region.as_ref() else {
            continue;
        };
        let entry = regions
            .entry(region.region_id)
            .or_insert_with(|| CompiledFixedStepRegion {
                region_id: region.region_id,
                region_label: region.region_label.clone(),
                max_substeps: region.max_substeps,
                iteration_uniform: region.iteration_uniform,
                pass_ids: Vec::new(),
            });
        entry.pass_ids.push(pass.pass_id());
    }
    regions.into_values().collect()
}

fn compile_pass_execution(
    pass: &CompiledPassDescriptor,
    resources: &ResourceGraph,
) -> CompiledPassExecutionPlan {
    let node = pass.node();
    let pass_id = node.id;
    let authoring_index = pass.authoring_index();

    let bindings = compile_pass_bindings(node, resources);

    match node.kind {
        RenderPassKind::Compute => {
            CompiledPassExecutionPlan::Compute(CompiledComputeExecutionPlan {
                pass_id,
                authoring_index,
                feature_id: compile_feature_id(node),
                shader: node.shader.clone(),
                shader_constants: node.shader_constants.clone(),
                view_mask: compile_view_mask(node),
                bindings,
                dispatch: compile_dispatch_plan(node),
            })
        }
        RenderPassKind::Fullscreen => {
            CompiledPassExecutionPlan::Fullscreen(CompiledRasterExecutionPlan {
                pass_id,
                authoring_index,
                feature_id: compile_feature_id(node),
                shader: node.shader.clone(),
                view_mask: compile_view_mask(node),
                bindings,
                targets: compile_target_plan(node, resources),
                draw_buffers: CompiledDrawBufferPlan::default(),
                raster_state: compile_raster_state(node.raster_state),
                clear_color: node.clear_color,
                draw: None,
            })
        }
        RenderPassKind::Graphics => {
            CompiledPassExecutionPlan::Graphics(CompiledRasterExecutionPlan {
                pass_id,
                authoring_index,
                feature_id: compile_feature_id(node),
                shader: node.shader.clone(),
                view_mask: compile_view_mask(node),
                bindings,
                targets: compile_target_plan(node, resources),
                draw_buffers: compile_draw_buffer_plan(node, resources),
                raster_state: compile_raster_state(node.raster_state),
                clear_color: node.clear_color,
                draw: node.draw.map(compile_draw_plan),
            })
        }
        RenderPassKind::Copy => CompiledPassExecutionPlan::Copy(CompiledCopyExecutionPlan {
            pass_id,
            authoring_index,
            feature_id: compile_feature_id(node),
            view_mask: compile_view_mask(node),
            source: node
                .copy_source
                .as_ref()
                .map(|resource| compile_resource_ref(resource, resources)),
            destination: node
                .copy_destination
                .as_ref()
                .map(|resource| compile_resource_ref(resource, resources)),
        }),
        RenderPassKind::Present => {
            CompiledPassExecutionPlan::Present(CompiledPresentExecutionPlan {
                pass_id,
                authoring_index,
                feature_id: compile_feature_id(node),
                view_mask: compile_view_mask(node),
                source: node
                    .present_source
                    .as_ref()
                    .map(|resource| compile_resource_ref(resource, resources)),
            })
        }
        RenderPassKind::BuiltinUiComposite => {
            CompiledPassExecutionPlan::BuiltinUiComposite(CompiledUiCompositeExecutionPlan {
                pass_id,
                authoring_index,
                feature_id: UI_RENDER_FEATURE_ID,
                view_mask: compile_view_mask(node),
                color_output: CompiledResourceRef::ImportedBuiltin(
                    CompiledBuiltinImport::SurfaceColor,
                ),
            })
        }
    }
}

fn compile_feature_id(node: &RenderPassNode) -> Option<RenderFeatureId> {
    node.feature_id
}

fn compile_view_mask(node: &RenderPassNode) -> CompiledViewMask {
    match node.view_scope {
        RenderPassViewScope::AllViews => CompiledViewMask::AllViews,
        RenderPassViewScope::MainSurfaceOnly => CompiledViewMask::MainSurfaceOnly,
        RenderPassViewScope::OffscreenProductsOnly => CompiledViewMask::OffscreenProductsOnly,
    }
}

fn compile_dispatch_plan(node: &RenderPassNode) -> Option<CompiledDispatchPlan> {
    match node.compute_dispatch.as_ref() {
        None => None,
        Some(ComputeDispatchDescriptor::Fixed(value)) => Some(CompiledDispatchPlan::Fixed(*value)),
        Some(ComputeDispatchDescriptor::State(binding)) => Some(CompiledDispatchPlan::FromState {
            state_type_id: binding.state_type_id(),
            state_type_name: binding.state_type_name(),
        }),
    }
}

fn compile_pass_bindings(node: &RenderPassNode, resources: &ResourceGraph) -> CompiledPassBindings {
    let mut entries = node
        .shader_bindings
        .iter()
        .map(|binding| compile_shader_binding(binding, resources))
        .collect::<Vec<_>>();
    entries.sort_by_key(CompiledBindingEntry::key);

    let mut uniform_order = Vec::<GpuWorkResourceId>::new();
    let mut seen_uniforms = BTreeSet::<GpuWorkResourceId>::new();
    for binding in &node.uniform_bindings {
        let resource = *binding.uniform_id();
        if seen_uniforms.insert(resource) {
            uniform_order.push(resource);
        }
    }
    for resource in &node.fixed_step_iteration_uniforms {
        if seen_uniforms.insert(*resource) {
            uniform_order.push(*resource);
        }
    }

    let storage_order = collect_storage_usage(node, resources)
        .into_iter()
        .map(|(resource, access)| CompiledStorageBinding {
            resource: compile_resource_ref(&resource, resources),
            access,
        })
        .collect();

    CompiledPassBindings {
        bind_group: CompiledBindGroupPlan { entries },
        uniform_order,
        storage_order,
    }
}

fn compile_shader_binding(
    binding: &crate::plugins::render::api::RenderShaderBinding,
    resources: &ResourceGraph,
) -> CompiledBindingEntry {
    let key = binding.key();
    match binding.resource() {
        RenderShaderBindingResource::SampledTexture(resource) => {
            CompiledBindingEntry::SampledTexture {
                key,
                resource: compile_resource_ref(resource, resources),
            }
        }
        RenderShaderBindingResource::Sampler => CompiledBindingEntry::Sampler { key },
        RenderShaderBindingResource::StorageTexture { resource, access } => {
            CompiledBindingEntry::StorageTexture {
                key,
                resource: compile_resource_ref(resource, resources),
                access: compiled_storage_texture_access(*access),
            }
        }
        RenderShaderBindingResource::UniformBuffer(resource) => {
            CompiledBindingEntry::UniformBuffer {
                key,
                resource: *resource,
            }
        }
        RenderShaderBindingResource::StorageBuffer { resource, access } => {
            CompiledBindingEntry::StorageBuffer {
                key,
                resource: compile_resource_ref(resource, resources),
                access: compiled_storage_buffer_access(*access),
            }
        }
    }
}

const fn compiled_storage_buffer_access(access: GpuStorageBufferAccess) -> CompiledStorageAccess {
    match access {
        GpuStorageBufferAccess::ReadOnly => CompiledStorageAccess::ReadOnly,
        GpuStorageBufferAccess::ReadWrite => CompiledStorageAccess::ReadWrite,
    }
}

const fn compiled_storage_texture_access(access: GpuStorageTextureAccess) -> CompiledStorageAccess {
    match access {
        GpuStorageTextureAccess::ReadOnly => CompiledStorageAccess::ReadOnly,
        GpuStorageTextureAccess::WriteOnly => CompiledStorageAccess::WriteOnly,
        GpuStorageTextureAccess::ReadWrite => CompiledStorageAccess::ReadWrite,
    }
}

fn collect_storage_usage(
    node: &RenderPassNode,
    resources: &ResourceGraph,
) -> Vec<(GpuWorkResourceId, CompiledStorageAccess)> {
    let writable_storage = node
        .storage_writes
        .iter()
        .copied()
        .filter(|resource| is_buffer_like_resource(resources, resource))
        .collect::<BTreeSet<_>>();
    let mut seen_storage = BTreeSet::<GpuWorkResourceId>::new();
    let mut usage = Vec::<(GpuWorkResourceId, CompiledStorageAccess)>::new();
    for resource in node
        .storage_reads
        .iter()
        .chain(node.storage_writes.iter())
        .copied()
    {
        if !is_buffer_like_resource(resources, &resource) {
            continue;
        }
        if !seen_storage.insert(resource) {
            continue;
        }
        let access = if writable_storage.contains(&resource) {
            CompiledStorageAccess::ReadWrite
        } else {
            CompiledStorageAccess::ReadOnly
        };
        usage.push((resource, access));
    }
    usage
}

fn is_buffer_like_resource(resources: &ResourceGraph, resource: &GpuWorkResourceId) -> bool {
    matches!(
        resources
            .resources
            .iter()
            .find(|descriptor| descriptor.id() == resource),
        Some(RenderResourceDeclaration::Storage(_) | RenderResourceDeclaration::ImportedBuffer(_))
    )
}

fn compile_target_plan(node: &RenderPassNode, resources: &ResourceGraph) -> CompiledTargetPlan {
    CompiledTargetPlan {
        color_outputs: node
            .color_outputs
            .iter()
            .map(|resource| compile_resource_ref(resource, resources))
            .collect(),
        depth_output: node
            .depth_target
            .as_ref()
            .map(|resource| compile_resource_ref(resource, resources)),
    }
}

fn compile_draw_buffer_plan(
    node: &RenderPassNode,
    resources: &ResourceGraph,
) -> CompiledDrawBufferPlan {
    CompiledDrawBufferPlan {
        vertex_buffers: node
            .vertex_buffers
            .iter()
            .zip(node.vertex_buffer_layouts.iter())
            .map(|(resource, layout)| CompiledVertexBufferBinding {
                resource: compile_resource_ref(resource, resources),
                layout: compile_vertex_buffer_layout(layout),
            })
            .collect(),
        instance_buffers: node
            .instance_buffers
            .iter()
            .map(|resource| compile_resource_ref(resource, resources))
            .collect(),
        instance_buffer_layouts: node
            .instance_buffer_layouts
            .iter()
            .map(compile_vertex_buffer_layout)
            .collect(),
        index_buffers: node
            .index_buffers
            .iter()
            .map(|resource| compile_resource_ref(resource, resources))
            .collect(),
        indirect_buffers: node
            .indirect_buffers
            .iter()
            .map(|resource| compile_resource_ref(resource, resources))
            .collect(),
    }
}

fn compile_vertex_buffer_layout(layout: &RenderVertexBufferLayout) -> CompiledVertexBufferLayout {
    CompiledVertexBufferLayout {
        slot: layout.slot,
        array_stride: layout.array_stride,
        step_mode: layout.step_mode,
        attributes: layout.attributes.clone(),
    }
}

fn compile_draw_plan(draw: RenderDrawDescriptor) -> CompiledDrawPlan {
    CompiledDrawPlan {
        vertex_count: draw.vertex_count,
        instance_count: draw.instance_count,
        first_vertex: draw.first_vertex,
        first_instance: draw.first_instance,
        source: match draw.source {
            RenderDrawSource::Direct => CompiledDrawSource::Direct,
            RenderDrawSource::Indirect {
                args_buffer,
                args_kind,
                args_element_count,
                args_element_size,
                byte_offset,
            } => CompiledDrawSource::Indirect {
                args_buffer,
                args_kind,
                args_element_count,
                args_element_size,
                byte_offset,
            },
        },
    }
}

fn compile_raster_state(state: RenderRasterState) -> CompiledRasterState {
    CompiledRasterState { state }
}

fn compile_resource_ref(
    resource: &GpuWorkResourceId,
    resources: &ResourceGraph,
) -> CompiledResourceRef {
    match resources
        .resources
        .iter()
        .find(|descriptor| descriptor.id() == resource)
    {
        Some(RenderResourceDeclaration::TargetAlias(value)) => {
            CompiledResourceRef::TargetAlias(CompiledTargetAliasRef {
                resource_id: value.id(),
                binding_key: value.binding_key().clone(),
                kind: value.kind(),
            })
        }
        Some(RenderResourceDeclaration::ImportedTexture(value)) => match value.semantic {
            RenderImportedTextureSemantic::SurfaceColor => {
                CompiledResourceRef::ImportedBuiltin(CompiledBuiltinImport::SurfaceColor)
            }
            RenderImportedTextureSemantic::SurfaceDepth => {
                CompiledResourceRef::ImportedBuiltin(CompiledBuiltinImport::SurfaceDepth)
            }
            RenderImportedTextureSemantic::HistoryTexture
            | RenderImportedTextureSemantic::External => CompiledResourceRef::Imported(*resource),
        },
        Some(RenderResourceDeclaration::ImportedBuffer(_)) => {
            CompiledResourceRef::Imported(*resource)
        }
        Some(_) | None => CompiledResourceRef::FlowOwned(*resource),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::api::{RenderShaderBinding, RenderShaderBindingResource};
    use crate::plugins::render::{RenderPassKind, RenderPassNode};
    use runen_gpu::GpuWorkResourceIdAllocator;

    fn resource(local: u64) -> GpuWorkResourceId {
        let mut allocator = GpuWorkResourceIdAllocator::new();
        (1..=local)
            .map(|_| {
                allocator
                    .allocate()
                    .expect("test allocation should succeed")
            })
            .last()
            .expect("test local value is nonzero")
    }

    fn key(binding: u64) -> GpuBindingKey {
        GpuBindingKey::try_new(0, binding).unwrap()
    }

    fn storage_read_write_pass() -> (RenderPassNode, ResourceGraph, GpuWorkResourceId) {
        let storage_id = resource(7);
        let mut resources = ResourceGraph::default();
        resources.add_resource(RenderResourceDeclaration::declare_imported_external_buffer(
            storage_id,
            "test.storage",
        ));
        let mut pass = RenderPassNode::new(
            RenderPassId::try_from_raw(1).unwrap(),
            "test.pass",
            RenderPassKind::Compute,
        );
        pass.storage_reads.push(storage_id);
        pass.storage_writes.push(storage_id);
        pass.shader_bindings.push(RenderShaderBinding::new(
            key(4),
            RenderShaderBindingResource::StorageBuffer {
                resource: storage_id,
                access: GpuStorageBufferAccess::ReadWrite,
            },
        ));
        (pass, resources, storage_id)
    }

    #[test]
    fn explicit_storage_binding_key_survives_compilation() {
        let (pass, resources, storage_id) = storage_read_write_pass();
        let bindings = compile_pass_bindings(&pass, &resources);

        let storage_bindings = bindings
            .bind_group
            .entries
            .iter()
            .filter_map(|entry| match entry {
                CompiledBindingEntry::StorageBuffer {
                    key,
                    resource,
                    access,
                } => Some((*key, resource.clone(), *access)),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(storage_bindings.len(), 1);
        assert_eq!(
            storage_bindings[0],
            (
                key(4),
                CompiledResourceRef::Imported(storage_id),
                CompiledStorageAccess::ReadWrite
            )
        );
        assert_eq!(bindings.storage_order.len(), 1);
        assert_eq!(
            bindings.storage_order[0],
            CompiledStorageBinding {
                resource: CompiledResourceRef::Imported(storage_id),
                access: CompiledStorageAccess::ReadWrite,
            }
        );
    }

    #[test]
    fn explicit_storage_texture_binding_key_survives_compilation() {
        let texture_id = resource(8);
        let mut resources = ResourceGraph::default();
        resources.add_resource(RenderResourceDeclaration::declare_storage_texture(
            texture_id,
            "test.texture",
        ));
        let mut pass = RenderPassNode::new(
            RenderPassId::try_from_raw(2).unwrap(),
            "test.texture.write",
            RenderPassKind::Compute,
        );
        pass.write_textures.push(texture_id);
        pass.shader_bindings.push(RenderShaderBinding::new(
            key(9),
            RenderShaderBindingResource::StorageTexture {
                resource: texture_id,
                access: GpuStorageTextureAccess::WriteOnly,
            },
        ));

        let bindings = compile_pass_bindings(&pass, &resources);
        let storage_textures = bindings
            .bind_group
            .entries
            .iter()
            .filter_map(|entry| match entry {
                CompiledBindingEntry::StorageTexture {
                    key,
                    resource,
                    access,
                } => Some((*key, resource.clone(), *access)),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            storage_textures,
            vec![(
                key(9),
                CompiledResourceRef::FlowOwned(texture_id),
                CompiledStorageAccess::WriteOnly,
            )]
        );
    }

    #[test]
    fn compiled_bind_group_sorts_by_explicit_key_not_category_or_vector_order() {
        let sampled = resource(1);
        let storage = resource(2);
        let mut resources = ResourceGraph::default();
        resources.add_resource(RenderResourceDeclaration::declare_imported_external_buffer(
            storage,
            "test.storage",
        ));
        resources.add_resource(RenderResourceDeclaration::declare_storage_texture(
            sampled,
            "test.texture",
        ));
        let mut pass = RenderPassNode::new(
            RenderPassId::try_from_raw(11).unwrap(),
            "test.order",
            RenderPassKind::Compute,
        );
        pass.shader_bindings.extend([
            RenderShaderBinding::new(
                key(7),
                RenderShaderBindingResource::StorageBuffer {
                    resource: storage,
                    access: GpuStorageBufferAccess::ReadOnly,
                },
            ),
            RenderShaderBinding::new(
                key(2),
                RenderShaderBindingResource::StorageTexture {
                    resource: sampled,
                    access: GpuStorageTextureAccess::WriteOnly,
                },
            ),
        ]);

        let bindings = compile_pass_bindings(&pass, &resources);
        assert_eq!(
            bindings
                .bind_group
                .entries
                .iter()
                .map(CompiledBindingEntry::key)
                .collect::<Vec<_>>(),
            vec![key(2), key(7)]
        );
    }

    #[test]
    fn collect_storage_usage_is_deduped_and_stable() {
        let first = resource(1);
        let second = resource(2);
        let third = resource(3);

        let mut resources = ResourceGraph::default();
        resources.add_resource(RenderResourceDeclaration::declare_imported_external_buffer(
            first,
            "test.first",
        ));
        resources.add_resource(RenderResourceDeclaration::declare_imported_external_buffer(
            second,
            "test.second",
        ));
        resources.add_resource(RenderResourceDeclaration::declare_imported_external_buffer(
            third,
            "test.third",
        ));

        let mut pass = RenderPassNode::new(
            RenderPassId::try_from_raw(12).unwrap(),
            "test.usage",
            RenderPassKind::Compute,
        );
        pass.storage_reads.extend([first, second, first]);
        pass.storage_writes.extend([second, third, second]);

        assert_eq!(
            collect_storage_usage(&pass, &resources),
            vec![
                (first, CompiledStorageAccess::ReadOnly),
                (second, CompiledStorageAccess::ReadWrite),
                (third, CompiledStorageAccess::ReadWrite),
            ]
        );
    }
}
