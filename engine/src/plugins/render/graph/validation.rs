use crate::plugins::render::api::{SURFACE_COLOR_RESOURCE_LABEL, SURFACE_DEPTH_RESOURCE_LABEL};
use crate::plugins::render::graph::{
    RenderDrawSource, RenderFlowGraph, RenderIndirectDrawArgsKind, RenderPassKind, RenderPassNode,
    validate_builtin_ui_pass_shape,
};
use crate::plugins::render::resource::{RenderTextureSampleMode, RenderTextureTargetFormat};
use crate::plugins::render::{
    RenderImportedBufferSemantic, RenderImportedTextureSemantic, RenderResourceDeclaration,
    RenderTextureDescriptor, RenderTextureFormatPolicy,
};
use crate::plugins::render::{RenderPassId, RenderTargetAliasKind, RenderVertexStepMode};
use runen_gpu::{GpuBindingKey, GpuBufferUsage, GpuWorkResourceId};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowValidationReport {
    /// Lexical authoring identities only. Generic dependency and ordering
    /// authority lives on `GpuPreparedWorkGraph`.
    pub lexical_pass_ids: Vec<RenderPassId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RenderFlowValidationIssue {
    #[error("render flow cannot lower to checked GPU work: {message}")]
    GpuWorkLoweringFailed { message: String },

    #[error("duplicate resource id '{resource_id:?}'")]
    DuplicateResourceId { resource_id: GpuWorkResourceId },

    #[error(
        "storage_buffer '{resource_id:?}' declares zero elements; element_count must be greater than zero"
    )]
    ZeroLengthStorageBuffer { resource_id: GpuWorkResourceId },

    #[error(
        "resource '{resource_id:?}' ({resource_kind}) resolves to format '{format:?}', expected {expected_format_class}"
    )]
    InvalidTextureFormatClass {
        resource_id: GpuWorkResourceId,
        resource_kind: &'static str,
        format: RenderTextureTargetFormat,
        expected_format_class: &'static str,
    },

    #[error(
        "resource '{resource_id:?}' ({resource_kind}) declares invalid format policy: {reason}"
    )]
    InvalidTextureFormatPolicy {
        resource_id: GpuWorkResourceId,
        resource_kind: &'static str,
        reason: &'static str,
    },

    #[error(
        "resource '{resource_id:?}' ({resource_kind}) declares usage that is invalid for format '{format:?}': {reason}"
    )]
    InvalidTextureUsageForFormat {
        resource_id: GpuWorkResourceId,
        resource_kind: &'static str,
        format: RenderTextureTargetFormat,
        reason: &'static str,
    },

    #[error(
        "resource '{resource_id:?}' ({resource_kind}) declares sample mode '{sample_mode:?}' that is invalid for format '{format:?}': {reason}"
    )]
    InvalidTextureSampleModeForFormat {
        resource_id: GpuWorkResourceId,
        resource_kind: &'static str,
        format: RenderTextureTargetFormat,
        sample_mode: RenderTextureSampleMode,
        reason: &'static str,
    },

    #[error("duplicate pass id '{pass_label}' ({pass_id:?})")]
    DuplicatePassId {
        pass_id: RenderPassId,
        pass_label: String,
    },

    #[error(
        "pass '{pass_label}' declares shader binding {key} outside the current logical group-0 contract"
    )]
    ShaderBindingOutsidePrimaryGroup {
        pass_label: String,
        key: GpuBindingKey,
    },

    #[error("pass '{pass_label}' declares duplicate shader binding key {key}")]
    DuplicateShaderBindingKey {
        pass_label: String,
        key: GpuBindingKey,
    },

    #[error("pass '{pass_label}' orders after unknown pass '{ordered_before_id:?}'")]
    UnknownNonDataOrderTarget {
        pass_label: String,
        ordered_before_id: RenderPassId,
    },

    #[error("pass '{pass_label}' references unknown resource '{resource_id:?}'")]
    UnknownResourceReference {
        pass_label: String,
        resource_id: GpuWorkResourceId,
    },

    #[error(
        "pass '{pass_label}' uses uniform projection for state '{state_type_name}' but with_state::<...>() was not declared"
    )]
    MissingProjectedStateDeclaration {
        pass_label: String,
        state_type_name: &'static str,
    },

    #[error("pass '{pass_label}' references missing uniform buffer '{uniform_id:?}'")]
    MissingUniformBuffer {
        pass_label: String,
        uniform_id: GpuWorkResourceId,
    },

    #[error(
        "pass '{pass_label}' projects '{projected_type_name}' into uniform buffer '{uniform_id:?}' declared for '{declared_type_name}'"
    )]
    UniformBufferTypeMismatch {
        pass_label: String,
        uniform_id: GpuWorkResourceId,
        declared_type_name: &'static str,
        projected_type_name: &'static str,
    },

    #[error(
        "pass '{pass_label}' uses dispatch_from_state for resource '{state_type_name}' but with_state::<...>() was not declared"
    )]
    MissingDispatchStateDeclaration {
        pass_label: String,
        state_type_name: &'static str,
    },

    #[error(
        "flow declares {count} present passes ({labels}); exactly zero or one present pass is allowed"
    )]
    MultiplePresentPasses { count: usize, labels: String },

    #[error(
        "present pass '{present_label}' must be terminal but later passes order after it ({ordered_after_labels})"
    )]
    PresentPassNotTerminal {
        present_label: String,
        ordered_after_labels: String,
    },

    #[error("present pass '{present_label}' must be declared last in render authoring order")]
    PresentPassNotLast { present_label: String },

    #[error("fixed-step region '{region_label}' must declare max_substeps greater than zero")]
    FixedStepRegionInvalidMaxSubsteps { region_label: String },

    #[error("fixed-step region '{region_label}' has inconsistent descriptors across member passes")]
    FixedStepRegionInconsistentDescriptor { region_label: String },

    #[error("fixed-step region '{region_label}' cannot include {pass_kind} pass '{pass_label}'")]
    FixedStepRegionUnsupportedPassKind {
        region_label: String,
        pass_label: String,
        pass_kind: &'static str,
    },

    #[error(
        "fixed-step region '{region_label}' pass order is interleaved by non-region pass '{pass_label}'"
    )]
    FixedStepRegionPassesNotContiguous {
        region_label: String,
        pass_label: String,
    },

    #[error(
        "compute pass '{pass_label}' must declare explicit dispatch(...) or dispatch_from_state(...)"
    )]
    ComputePassMissingDispatch { pass_label: String },

    #[error("compute pass '{pass_label}' cannot declare a depth target")]
    ComputePassHasDepthTarget { pass_label: String },

    #[error("compute pass '{pass_label}' cannot declare vertex/index/instance/indirect buffers")]
    ComputePassHasGraphicsBuffers { pass_label: String },

    #[error("compute pass '{pass_label}' cannot declare clear_color")]
    ComputePassHasClearColor { pass_label: String },

    #[error("compute pass '{pass_label}' cannot declare draw parameters")]
    ComputePassHasDraw { pass_label: String },

    #[error("fullscreen pass '{pass_label}' cannot declare workgroup_size")]
    FullscreenPassHasWorkgroupSize { pass_label: String },

    #[error("fullscreen pass '{pass_label}' cannot declare compute dispatch")]
    FullscreenPassHasComputeDispatch { pass_label: String },

    #[error("fullscreen pass '{pass_label}' cannot declare a depth target")]
    FullscreenPassHasDepthTarget { pass_label: String },

    #[error("fullscreen pass '{pass_label}' cannot declare vertex/index/instance/indirect buffers")]
    FullscreenPassHasGraphicsBuffers { pass_label: String },

    #[error("fullscreen pass '{pass_label}' cannot declare vertex buffer layouts")]
    FullscreenPassHasVertexLayouts { pass_label: String },

    #[error("fullscreen pass '{pass_label}' cannot declare draw parameters")]
    FullscreenPassHasDraw { pass_label: String },

    #[error(
        "fullscreen pass '{pass_label}' must declare exactly one color output; found {write_count}"
    )]
    FullscreenPassInvalidColorOutputArity {
        pass_label: String,
        write_count: usize,
    },

    #[error("graphics pass '{pass_label}' cannot declare workgroup_size")]
    GraphicsPassHasWorkgroupSize { pass_label: String },

    #[error("graphics pass '{pass_label}' cannot declare compute dispatch")]
    GraphicsPassHasComputeDispatch { pass_label: String },

    #[error(
        "graphics pass '{pass_label}' must declare exactly one color output; found {write_count}"
    )]
    GraphicsPassInvalidColorOutputArity {
        pass_label: String,
        write_count: usize,
    },

    #[error(
        "{pass_kind} pass '{pass_label}' writes raster color output '{resource_id:?}' but kind '{resource_kind}' is not a runtime-supported color attachment"
    )]
    InvalidRasterColorOutputResource {
        pass_kind: &'static str,
        pass_label: String,
        resource_id: GpuWorkResourceId,
        resource_kind: &'static str,
    },

    #[error("graphics pass '{pass_label}' must declare draw(...) or draw_with_offsets(...)")]
    GraphicsPassMissingDraw { pass_label: String },

    #[error(
        "graphics pass '{pass_label}' declares invalid draw(vertex_count={vertex_count}, instance_count={instance_count})"
    )]
    GraphicsPassInvalidDraw {
        pass_label: String,
        vertex_count: u32,
        instance_count: u32,
    },

    #[error(
        "graphics pass '{pass_label}' declares {buffer_count} {role} buffers but {layout_count} layouts"
    )]
    GraphicsPassBufferLayoutCountMismatch {
        pass_label: String,
        role: &'static str,
        buffer_count: usize,
        layout_count: usize,
    },

    #[error(
        "graphics pass '{pass_label}' declares {role} buffer layout slot {slot} with step mode '{step_mode}', expected '{expected}'"
    )]
    GraphicsPassBufferLayoutStepModeMismatch {
        pass_label: String,
        role: &'static str,
        slot: u32,
        step_mode: &'static str,
        expected: &'static str,
    },

    #[error("graphics pass '{pass_label}' declares duplicate vertex buffer slot {slot}")]
    GraphicsPassDuplicateVertexBufferSlot { pass_label: String, slot: u32 },

    #[error(
        "graphics pass '{pass_label}' declares {count} index buffers; runtime supports at most one"
    )]
    GraphicsPassTooManyIndexBuffers { pass_label: String, count: usize },

    #[error(
        "graphics pass '{pass_label}' declares {count} indirect buffers; runtime supports at most one"
    )]
    GraphicsPassTooManyIndirectBuffers { pass_label: String, count: usize },

    #[error(
        "graphics pass '{pass_label}' indirect draw references args buffer '{resource_id:?}' but that buffer was not declared as an indirect buffer"
    )]
    GraphicsPassIndirectDrawArgsBufferNotDeclared {
        pass_label: String,
        resource_id: GpuWorkResourceId,
    },

    #[error(
        "graphics pass '{pass_label}' declares {count} indirect buffers but uses a direct draw source"
    )]
    GraphicsPassIndirectBufferWithoutIndirectDraw { pass_label: String, count: usize },

    #[error(
        "graphics pass '{pass_label}' indirect draw declares byte offset {byte_offset}, expected a 4-byte aligned offset"
    )]
    GraphicsPassInvalidIndirectDrawOffset {
        pass_label: String,
        byte_offset: u64,
    },

    #[error(
        "graphics pass '{pass_label}' indirect draw uses {args_kind}, expected {expected_args_kind} for the declared index-buffer state"
    )]
    GraphicsPassIndirectDrawArgsKindMismatch {
        pass_label: String,
        args_kind: &'static str,
        expected_args_kind: &'static str,
    },

    #[error(
        "graphics pass '{pass_label}' indirect draw byte offset {byte_offset} with element size {args_element_size} exceeds args buffer size {args_buffer_byte_size} ({args_element_count} elements)"
    )]
    GraphicsPassIndirectDrawOffsetOutOfBounds {
        pass_label: String,
        byte_offset: u64,
        args_element_size: u64,
        args_element_count: u64,
        args_buffer_byte_size: u64,
    },

    #[error(
        "graphics pass '{pass_label}' indirect draw declares CPU-side offsets first_vertex={first_vertex}, first_instance={first_instance}; indirect offsets must be encoded in the args buffer"
    )]
    GraphicsPassIndirectDrawUsesCpuOffsets {
        pass_label: String,
        first_vertex: u32,
        first_instance: u32,
    },

    #[error(
        "graphics pass '{pass_label}' declares vertex buffer slots that must be dense from 0; expected slot {expected}, found {found}"
    )]
    GraphicsPassNonDenseVertexBufferSlots {
        pass_label: String,
        expected: u32,
        found: u32,
    },

    #[error("graphics pass '{pass_label}' declares vertex buffer slot {slot} with zero stride")]
    GraphicsPassInvalidVertexStride { pass_label: String, slot: u32 },

    #[error("graphics pass '{pass_label}' declares vertex buffer slot {slot} with no attributes")]
    GraphicsPassMissingVertexAttributes { pass_label: String, slot: u32 },

    #[error("graphics pass '{pass_label}' declares duplicate shader location {shader_location}")]
    GraphicsPassDuplicateVertexShaderLocation {
        pass_label: String,
        shader_location: u32,
    },

    #[error(
        "graphics pass '{pass_label}' declares vertex attribute at location {shader_location} beyond stride for slot {slot}: offset {offset} + size {size} > stride {stride}"
    )]
    GraphicsPassInvalidVertexAttributeRange {
        pass_label: String,
        slot: u32,
        shader_location: u32,
        offset: u64,
        size: u64,
        stride: u64,
    },

    #[error("copy pass '{pass_label}' must declare one source and one destination")]
    CopyPassMissingEndpoints { pass_label: String },

    #[error("copy pass '{pass_label}' contains roles other than copy endpoints or non-data order")]
    CopyPassHasInvalidRoles { pass_label: String },

    #[error("present pass '{pass_label}' must declare one present source")]
    PresentPassMissingSource { pass_label: String },

    #[error("present pass '{pass_label}' cannot declare resource-write roles")]
    PresentPassHasWriteRoles { pass_label: String },

    #[error(
        "present pass '{pass_label}' contains roles other than present source or non-data order"
    )]
    PresentPassHasInvalidRoles { pass_label: String },

    #[error(
        "pass '{pass_label}' samples resource '{resource_id:?}' which is not texture-like (kind: {resource_kind})"
    )]
    SampledNonTextureResource {
        pass_label: String,
        resource_id: GpuWorkResourceId,
        resource_kind: &'static str,
    },

    #[error(
        "pass '{pass_label}' writes texture resource '{resource_id:?}' via write_texture(...) but kind '{resource_kind}' is not storage/history texture"
    )]
    WriteTextureOnInvalidResource {
        pass_label: String,
        resource_id: GpuWorkResourceId,
        resource_kind: &'static str,
    },

    #[error(
        "graphics pass '{pass_label}' uses depth_target '{resource_id:?}' but kind '{resource_kind}' is not a flow-owned depth target"
    )]
    InvalidDepthTargetResource {
        pass_label: String,
        resource_id: GpuWorkResourceId,
        resource_kind: &'static str,
    },

    #[error(
        "copy pass '{pass_label}' mixes incompatible resource classes: '{read_id:?}' ({read_kind}) -> '{write_id:?}' ({write_kind})"
    )]
    CopyPassMixedResourceClasses {
        pass_label: String,
        read_id: GpuWorkResourceId,
        read_kind: &'static str,
        write_id: GpuWorkResourceId,
        write_kind: &'static str,
    },

    #[error(
        "present pass '{pass_label}' must read a texture-like resource; '{resource_id:?}' is '{resource_kind}'"
    )]
    PresentPassReadsNonTexture {
        pass_label: String,
        resource_id: GpuWorkResourceId,
        resource_kind: &'static str,
    },

    #[error(
        "pass '{pass_label}' writes imported texture '{resource_id:?}' with semantic '{semantic}'; only '{allowed}' is writable in core runtime"
    )]
    InvalidImportedTextureWriteSemantic {
        pass_label: String,
        resource_id: GpuWorkResourceId,
        semantic: &'static str,
        allowed: &'static str,
    },

    #[error(
        "pass '{pass_label}' writes imported texture '{resource_id:?}' but pass kind '{pass_kind:?}' is not supported for imported texture writes"
    )]
    UnsupportedImportedTextureWriteKind {
        pass_label: String,
        resource_id: GpuWorkResourceId,
        pass_kind: RenderPassKind,
    },

    #[error(
        "pass '{pass_label}' uses '{resource_id:?}' in {role}(...) but kind '{resource_kind}' is not buffer-like"
    )]
    InvalidBufferRoleResource {
        pass_label: String,
        resource_id: GpuWorkResourceId,
        role: &'static str,
        resource_kind: &'static str,
    },

    #[error(
        "pass '{pass_label}' uses '{resource_id:?}' in {role}(...) but its normalized buffer descriptor does not declare '{required_usage:?}' usage"
    )]
    MissingBufferRoleUsage {
        pass_label: String,
        resource_id: GpuWorkResourceId,
        role: &'static str,
        required_usage: GpuBufferUsage,
    },

    #[error(
        "pass '{pass_label}' binds uniform buffer '{resource_id:?}' through storage access; bind it through a uniform projection instead"
    )]
    UniformBufferUsedAsStorage {
        pass_label: String,
        resource_id: GpuWorkResourceId,
    },

    #[error(
        "resource '{resource_id:?}' uses external imported texture semantics; external imports are not supported in active runtime flows"
    )]
    UnsupportedExternalImportedTexture { resource_id: GpuWorkResourceId },

    #[error(
        "resource '{resource_id:?}' uses external imported buffer semantics; external imports are not supported in active runtime flows"
    )]
    UnsupportedExternalImportedBuffer { resource_id: GpuWorkResourceId },

    #[error(
        "multiple imported surface-color textures detected; expected zero or one canonical '{canonical_label}' import"
    )]
    MultipleSurfaceColorImports { canonical_label: &'static str },

    #[error(
        "multiple imported surface-depth textures detected; expected zero or one canonical '{canonical_label}' import"
    )]
    MultipleSurfaceDepthImports { canonical_label: &'static str },

    #[error(
        "builtin_ui_composite pass '{pass_label}' cannot declare explicit feature id; feature is fixed to 'ui'"
    )]
    BuiltinUiExplicitFeatureId { pass_label: String },

    #[error("builtin_ui_composite pass '{pass_label}' cannot declare shader")]
    BuiltinUiHasShader { pass_label: String },

    #[error("builtin_ui_composite pass '{pass_label}' cannot declare workgroup_size")]
    BuiltinUiHasWorkgroupSize { pass_label: String },

    #[error("builtin_ui_composite pass '{pass_label}' cannot declare clear_color")]
    BuiltinUiHasClearColor { pass_label: String },

    #[error("builtin_ui_composite pass '{pass_label}' cannot declare compute dispatch")]
    BuiltinUiHasComputeDispatch { pass_label: String },

    #[error("builtin_ui_composite pass '{pass_label}' cannot declare depth target")]
    BuiltinUiHasDepthTarget { pass_label: String },

    #[error("builtin_ui_composite pass '{pass_label}' cannot declare uniform bindings")]
    BuiltinUiHasUniformBindings { pass_label: String },

    #[error("builtin_ui_composite pass '{pass_label}' contains unsupported resource-binding roles")]
    BuiltinUiInvalidResourceBindings { pass_label: String },

    #[error(
        "builtin_ui_composite pass '{pass_label}' must not declare storage, copy, or present roles; UI input comes from PreparedRenderFrame::ui()"
    )]
    BuiltinUiHasInvalidResourceRoles { pass_label: String },

    #[error(
        "builtin_ui_composite pass '{pass_label}' must declare exactly one color output; found {output_count}"
    )]
    BuiltinUiInvalidColorOutputArity {
        pass_label: String,
        output_count: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct RenderFlowValidationError {
    pub issues: Vec<RenderFlowValidationIssue>,
    pub message: String,
}

impl From<Vec<RenderFlowValidationIssue>> for RenderFlowValidationError {
    fn from(issues: Vec<RenderFlowValidationIssue>) -> Self {
        let message = issues
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        Self { issues, message }
    }
}

pub fn validate_flow_graph(
    graph: &RenderFlowGraph,
) -> Result<FlowValidationReport, RenderFlowValidationError> {
    let mut issues = Vec::<RenderFlowValidationIssue>::new();

    let mut resource_ids = BTreeSet::<GpuWorkResourceId>::new();
    let mut resources_by_id = BTreeMap::<GpuWorkResourceId, &RenderResourceDeclaration>::new();
    for resource in &graph.resources.resources {
        let resource_id = *resource.id();
        if !resource_ids.insert(resource_id) {
            issues.push(RenderFlowValidationIssue::DuplicateResourceId { resource_id });
        }
        if let RenderResourceDeclaration::Storage(value) = resource
            && value.element_count() == 0
        {
            issues.push(RenderFlowValidationIssue::ZeroLengthStorageBuffer { resource_id });
        }
        validate_resource_descriptor_shape(resource, &mut issues);
        resources_by_id.insert(resource_id, resource);
    }
    validate_imported_resource_descriptors(&resources_by_id, &mut issues);

    let mut pass_ids = BTreeSet::<RenderPassId>::new();
    for pass in &graph.passes.passes {
        if !pass_ids.insert(pass.id) {
            issues.push(RenderFlowValidationIssue::DuplicatePassId {
                pass_id: pass.id,
                pass_label: pass.label.clone(),
            });
        }
    }

    let pass_lookup: BTreeMap<RenderPassId, &RenderPassNode> = graph
        .passes
        .passes
        .iter()
        .map(|pass| (pass.id, pass))
        .collect();

    for pass in &graph.passes.passes {
        validate_pass_shape(pass, &mut issues);
        validate_pass_shader_binding_identity(pass, &mut issues);

        for ordered_before_id in &pass.non_data_order_after {
            if !pass_lookup.contains_key(ordered_before_id) {
                issues.push(RenderFlowValidationIssue::UnknownNonDataOrderTarget {
                    pass_label: pass.label.clone(),
                    ordered_before_id: *ordered_before_id,
                });
            }
        }

        for resource in pass_resource_refs(pass) {
            if !resource_ids.contains(resource) {
                issues.push(RenderFlowValidationIssue::UnknownResourceReference {
                    pass_label: pass.label.clone(),
                    resource_id: *resource,
                });
            }
        }

        validate_pass_resource_usage(pass, &resources_by_id, &mut issues);

        for binding in &pass.uniform_bindings {
            if !graph.resources.has_state_resource(binding.state_type_id()) {
                issues.push(
                    RenderFlowValidationIssue::MissingProjectedStateDeclaration {
                        pass_label: pass.label.clone(),
                        state_type_name: binding.state_type_name(),
                    },
                );
            }

            match graph.resources.uniform_buffer_params(binding.uniform_id()) {
                None => issues.push(RenderFlowValidationIssue::MissingUniformBuffer {
                    pass_label: pass.label.clone(),
                    uniform_id: *binding.uniform_id(),
                }),
                Some((declared_type_id, declared_type_name))
                    if declared_type_id != binding.params_type_id() =>
                {
                    issues.push(RenderFlowValidationIssue::UniformBufferTypeMismatch {
                        pass_label: pass.label.clone(),
                        uniform_id: *binding.uniform_id(),
                        declared_type_name,
                        projected_type_name: binding.params_type_name(),
                    });
                }
                Some(_) => {}
            }
        }

        for uniform_id in &pass.fixed_step_iteration_uniforms {
            if !graph.resources.has_uniform_buffer(uniform_id) {
                issues.push(RenderFlowValidationIssue::MissingUniformBuffer {
                    pass_label: pass.label.clone(),
                    uniform_id: *uniform_id,
                });
            }
        }

        if let Some(dispatch) = &pass.compute_dispatch
            && let crate::plugins::render::api::ComputeDispatchDescriptor::State(binding) = dispatch
            && !graph.resources.has_state_resource(binding.state_type_id())
        {
            issues.push(RenderFlowValidationIssue::MissingDispatchStateDeclaration {
                pass_label: pass.label.clone(),
                state_type_name: binding.state_type_name(),
            });
        }
    }

    let present_passes = graph
        .passes
        .passes
        .iter()
        .filter(|pass| matches!(pass.kind, RenderPassKind::Present))
        .collect::<Vec<_>>();

    if present_passes.len() > 1 {
        issues.push(RenderFlowValidationIssue::MultiplePresentPasses {
            count: present_passes.len(),
            labels: present_passes
                .iter()
                .map(|pass| pass.label.clone())
                .collect::<Vec<_>>()
                .join(", "),
        });
    }

    // This is lexical render authoring order only. Generic dependency and
    // topological authority belongs exclusively to `GpuPreparedWorkGraph`.
    let lexical_pass_ids = graph
        .passes
        .passes
        .iter()
        .map(|pass| pass.id)
        .collect::<Vec<_>>();
    validate_fixed_step_regions(&graph.passes.passes, &lexical_pass_ids, &mut issues);

    if present_passes.len() == 1 {
        let present_pass = present_passes[0];
        let dependent_passes = graph
            .passes
            .passes
            .iter()
            .filter(|pass| pass.non_data_order_after.contains(&present_pass.id))
            .map(|pass| pass.label.clone())
            .collect::<Vec<_>>();

        if !dependent_passes.is_empty() {
            issues.push(RenderFlowValidationIssue::PresentPassNotTerminal {
                present_label: present_pass.label.clone(),
                ordered_after_labels: dependent_passes.join(", "),
            });
        }

        if lexical_pass_ids
            .last()
            .copied()
            .is_some_and(|id| id != present_pass.id)
        {
            issues.push(RenderFlowValidationIssue::PresentPassNotLast {
                present_label: present_pass.label.clone(),
            });
        }
    }

    if issues.is_empty() {
        Ok(FlowValidationReport { lexical_pass_ids })
    } else {
        Err(RenderFlowValidationError::from(issues))
    }
}

fn validate_pass_shader_binding_identity(
    pass: &RenderPassNode,
    issues: &mut Vec<RenderFlowValidationIssue>,
) {
    let mut keys = BTreeSet::new();
    for binding in &pass.shader_bindings {
        let key = binding.key();
        if key.group() != 0 {
            issues.push(
                RenderFlowValidationIssue::ShaderBindingOutsidePrimaryGroup {
                    pass_label: pass.label.clone(),
                    key,
                },
            );
        }
        if !keys.insert(key) {
            issues.push(RenderFlowValidationIssue::DuplicateShaderBindingKey {
                pass_label: pass.label.clone(),
                key,
            });
        }
    }
}

fn validate_pass_shape(pass: &RenderPassNode, issues: &mut Vec<RenderFlowValidationIssue>) {
    match pass.kind {
        RenderPassKind::Compute => {
            if pass.compute_dispatch.is_none() {
                issues.push(RenderFlowValidationIssue::ComputePassMissingDispatch {
                    pass_label: pass.label.clone(),
                });
            }
            if pass.depth_target.is_some() {
                issues.push(RenderFlowValidationIssue::ComputePassHasDepthTarget {
                    pass_label: pass.label.clone(),
                });
            }
            if !pass.vertex_buffers.is_empty()
                || !pass.index_buffers.is_empty()
                || !pass.instance_buffers.is_empty()
                || !pass.indirect_buffers.is_empty()
                || !pass.vertex_buffer_layouts.is_empty()
                || !pass.instance_buffer_layouts.is_empty()
            {
                issues.push(RenderFlowValidationIssue::ComputePassHasGraphicsBuffers {
                    pass_label: pass.label.clone(),
                });
            }
            if pass.clear_color.is_some() {
                issues.push(RenderFlowValidationIssue::ComputePassHasClearColor {
                    pass_label: pass.label.clone(),
                });
            }
            if pass.draw.is_some() {
                issues.push(RenderFlowValidationIssue::ComputePassHasDraw {
                    pass_label: pass.label.clone(),
                });
            }
        }
        RenderPassKind::Fullscreen => {
            if pass.workgroup_size.is_some() {
                issues.push(RenderFlowValidationIssue::FullscreenPassHasWorkgroupSize {
                    pass_label: pass.label.clone(),
                });
            }
            if pass.compute_dispatch.is_some() {
                issues.push(
                    RenderFlowValidationIssue::FullscreenPassHasComputeDispatch {
                        pass_label: pass.label.clone(),
                    },
                );
            }
            if pass.depth_target.is_some() {
                issues.push(RenderFlowValidationIssue::FullscreenPassHasDepthTarget {
                    pass_label: pass.label.clone(),
                });
            }
            if !pass.vertex_buffers.is_empty()
                || !pass.index_buffers.is_empty()
                || !pass.instance_buffers.is_empty()
                || !pass.indirect_buffers.is_empty()
            {
                issues.push(
                    RenderFlowValidationIssue::FullscreenPassHasGraphicsBuffers {
                        pass_label: pass.label.clone(),
                    },
                );
            }
            if !pass.vertex_buffer_layouts.is_empty() || !pass.instance_buffer_layouts.is_empty() {
                issues.push(RenderFlowValidationIssue::FullscreenPassHasVertexLayouts {
                    pass_label: pass.label.clone(),
                });
            }
            if pass.draw.is_some() {
                issues.push(RenderFlowValidationIssue::FullscreenPassHasDraw {
                    pass_label: pass.label.clone(),
                });
            }
            if pass.color_outputs.len() != 1 {
                issues.push(
                    RenderFlowValidationIssue::FullscreenPassInvalidColorOutputArity {
                        pass_label: pass.label.clone(),
                        write_count: pass.color_outputs.len(),
                    },
                );
            }
        }
        RenderPassKind::BuiltinUiComposite => {
            validate_builtin_ui_pass_shape(pass, issues);
        }
        RenderPassKind::Graphics => {
            if pass.workgroup_size.is_some() {
                issues.push(RenderFlowValidationIssue::GraphicsPassHasWorkgroupSize {
                    pass_label: pass.label.clone(),
                });
            }
            if pass.compute_dispatch.is_some() {
                issues.push(RenderFlowValidationIssue::GraphicsPassHasComputeDispatch {
                    pass_label: pass.label.clone(),
                });
            }
            if pass.color_outputs.len() != 1 {
                issues.push(
                    RenderFlowValidationIssue::GraphicsPassInvalidColorOutputArity {
                        pass_label: pass.label.clone(),
                        write_count: pass.color_outputs.len(),
                    },
                );
            }
            match pass.draw {
                Some(draw) if draw.vertex_count == 0 || draw.instance_count == 0 => {
                    issues.push(RenderFlowValidationIssue::GraphicsPassInvalidDraw {
                        pass_label: pass.label.clone(),
                        vertex_count: draw.vertex_count,
                        instance_count: draw.instance_count,
                    });
                }
                Some(_) => {}
                None => issues.push(RenderFlowValidationIssue::GraphicsPassMissingDraw {
                    pass_label: pass.label.clone(),
                }),
            }
            validate_graphics_vertex_layouts(pass, issues);
            if pass.index_buffers.len() > 1 {
                issues.push(RenderFlowValidationIssue::GraphicsPassTooManyIndexBuffers {
                    pass_label: pass.label.clone(),
                    count: pass.index_buffers.len(),
                });
            }
            if pass.indirect_buffers.len() > 1 {
                issues.push(
                    RenderFlowValidationIssue::GraphicsPassTooManyIndirectBuffers {
                        pass_label: pass.label.clone(),
                        count: pass.indirect_buffers.len(),
                    },
                );
            }
            validate_graphics_draw_source(pass, issues);
        }
        RenderPassKind::Copy => {
            if pass.copy_source.is_none() || pass.copy_destination.is_none() {
                issues.push(RenderFlowValidationIssue::CopyPassMissingEndpoints {
                    pass_label: pass.label.clone(),
                });
            }
            if pass.shader.is_some()
                || pass.workgroup_size.is_some()
                || pass.compute_dispatch.is_some()
                || pass.clear_color.is_some()
                || pass.depth_target.is_some()
                || pass.draw.is_some()
                || !pass.uniform_bindings.is_empty()
                || !pass.sampled_textures.is_empty()
                || !pass.write_textures.is_empty()
                || !pass.vertex_buffers.is_empty()
                || !pass.vertex_buffer_layouts.is_empty()
                || !pass.index_buffers.is_empty()
                || !pass.instance_buffers.is_empty()
                || !pass.instance_buffer_layouts.is_empty()
                || !pass.indirect_buffers.is_empty()
                || !pass.storage_reads.is_empty()
                || !pass.storage_writes.is_empty()
                || !pass.color_outputs.is_empty()
                || pass.present_source.is_some()
            {
                issues.push(RenderFlowValidationIssue::CopyPassHasInvalidRoles {
                    pass_label: pass.label.clone(),
                });
            }
        }
        RenderPassKind::Present => {
            if pass.present_source.is_none() {
                issues.push(RenderFlowValidationIssue::PresentPassMissingSource {
                    pass_label: pass.label.clone(),
                });
            }
            if !pass.storage_writes.is_empty()
                || !pass.write_textures.is_empty()
                || !pass.color_outputs.is_empty()
                || pass.copy_destination.is_some()
            {
                issues.push(RenderFlowValidationIssue::PresentPassHasWriteRoles {
                    pass_label: pass.label.clone(),
                });
            }
            if pass.shader.is_some()
                || pass.workgroup_size.is_some()
                || pass.compute_dispatch.is_some()
                || pass.clear_color.is_some()
                || pass.depth_target.is_some()
                || pass.draw.is_some()
                || !pass.uniform_bindings.is_empty()
                || !pass.sampled_textures.is_empty()
                || !pass.write_textures.is_empty()
                || !pass.vertex_buffers.is_empty()
                || !pass.vertex_buffer_layouts.is_empty()
                || !pass.index_buffers.is_empty()
                || !pass.instance_buffers.is_empty()
                || !pass.instance_buffer_layouts.is_empty()
                || !pass.indirect_buffers.is_empty()
                || !pass.storage_reads.is_empty()
                || pass.copy_source.is_some()
            {
                issues.push(RenderFlowValidationIssue::PresentPassHasInvalidRoles {
                    pass_label: pass.label.clone(),
                });
            }
        }
    }
}

fn validate_resource_descriptor_shape(
    resource: &RenderResourceDeclaration,
    issues: &mut Vec<RenderFlowValidationIssue>,
) {
    match resource {
        RenderResourceDeclaration::Sampled(value) => {
            validate_texture_descriptor_format_usage(
                value.id,
                "sampled_texture",
                &value.texture,
                issues,
            );
        }
        RenderResourceDeclaration::StorageImage(value) => {
            validate_texture_descriptor_format_usage(
                value.id,
                "storage_texture",
                &value.texture,
                issues,
            );
        }
        RenderResourceDeclaration::ColorAttachment(value) => {
            validate_color_target_descriptor_shape(value.id, &value.texture, issues);
        }
        RenderResourceDeclaration::DepthAttachment(value) => {
            validate_depth_target_descriptor_shape(value.id, &value.texture, issues);
        }
        RenderResourceDeclaration::History(value) => {
            validate_texture_descriptor_format_usage(
                value.id,
                "history_texture",
                &value.texture,
                issues,
            );
        }
        RenderResourceDeclaration::Uniform(_)
        | RenderResourceDeclaration::Storage(_)
        | RenderResourceDeclaration::TargetAlias(_)
        | RenderResourceDeclaration::ImportedTexture(_)
        | RenderResourceDeclaration::ImportedBuffer(_) => {}
    }
}

fn validate_color_target_descriptor_shape(
    resource_id: GpuWorkResourceId,
    texture: &RenderTextureDescriptor,
    issues: &mut Vec<RenderFlowValidationIssue>,
) {
    if let RenderTextureFormatPolicy::Exact(format) = texture.format
        && format.is_depth()
    {
        issues.push(RenderFlowValidationIssue::InvalidTextureFormatClass {
            resource_id,
            resource_kind: "color_target",
            format,
            expected_format_class: "a color format",
        });
    }
    validate_texture_descriptor_format_usage(resource_id, "color_target", texture, issues);
}

fn validate_depth_target_descriptor_shape(
    resource_id: GpuWorkResourceId,
    texture: &RenderTextureDescriptor,
    issues: &mut Vec<RenderFlowValidationIssue>,
) {
    match texture.format {
        RenderTextureFormatPolicy::Surface => {
            issues.push(RenderFlowValidationIssue::InvalidTextureFormatPolicy {
                resource_id,
                resource_kind: "depth_target",
                reason: "depth targets must declare an exact depth/stencil format",
            });
        }
        RenderTextureFormatPolicy::Exact(format) if !format.is_depth() => {
            issues.push(RenderFlowValidationIssue::InvalidTextureFormatClass {
                resource_id,
                resource_kind: "depth_target",
                format,
                expected_format_class: "a depth/stencil format",
            });
        }
        RenderTextureFormatPolicy::Exact(_) => {}
    }
    validate_texture_descriptor_format_usage(resource_id, "depth_target", texture, issues);
}

fn validate_texture_descriptor_format_usage(
    resource_id: GpuWorkResourceId,
    resource_kind: &'static str,
    texture: &RenderTextureDescriptor,
    issues: &mut Vec<RenderFlowValidationIssue>,
) {
    let RenderTextureFormatPolicy::Exact(format) = texture.format else {
        return;
    };

    if format.is_depth() {
        if texture.usage.color_attachment || texture.usage.storage {
            issues.push(RenderFlowValidationIssue::InvalidTextureUsageForFormat {
                resource_id,
                resource_kind,
                format,
                reason: "depth/stencil formats cannot be color attachments or storage textures",
            });
        }
        if texture.usage.sampled && texture.sample_mode != RenderTextureSampleMode::Depth {
            issues.push(
                RenderFlowValidationIssue::InvalidTextureSampleModeForFormat {
                    resource_id,
                    resource_kind,
                    format,
                    sample_mode: texture.sample_mode,
                    reason: "sampled depth/stencil formats must use depth sampling mode",
                },
            );
        }
    } else if texture.usage.depth_attachment {
        issues.push(RenderFlowValidationIssue::InvalidTextureUsageForFormat {
            resource_id,
            resource_kind,
            format,
            reason: "color formats cannot be depth/stencil attachments",
        });
    }

    if texture.usage.sampled && !texture.sample_mode.is_sampled() {
        issues.push(
            RenderFlowValidationIssue::InvalidTextureSampleModeForFormat {
                resource_id,
                resource_kind,
                format,
                sample_mode: texture.sample_mode,
                reason: "sampled usage requires a sampled mode",
            },
        );
    }

    if !texture.usage.sampled && texture.sample_mode.is_sampled() {
        issues.push(
            RenderFlowValidationIssue::InvalidTextureSampleModeForFormat {
                resource_id,
                resource_kind,
                format,
                sample_mode: texture.sample_mode,
                reason: "unsampled textures must use NotSampled mode",
            },
        );
    }

    if texture.usage.sampled
        && format.is_uint()
        && texture.sample_mode != RenderTextureSampleMode::Uint
    {
        issues.push(
            RenderFlowValidationIssue::InvalidTextureSampleModeForFormat {
                resource_id,
                resource_kind,
                format,
                sample_mode: texture.sample_mode,
                reason: "integer formats must use integer sampling mode",
            },
        );
    }

    if texture.usage.sampled
        && format.is_displayable()
        && !matches!(
            texture.sample_mode,
            RenderTextureSampleMode::FilterableFloat | RenderTextureSampleMode::NonFilterableFloat
        )
    {
        issues.push(
            RenderFlowValidationIssue::InvalidTextureSampleModeForFormat {
                resource_id,
                resource_kind,
                format,
                sample_mode: texture.sample_mode,
                reason: "displayable color formats must use float sampling mode",
            },
        );
    }
}

fn validate_pass_resource_usage(
    pass: &RenderPassNode,
    resources_by_id: &BTreeMap<GpuWorkResourceId, &RenderResourceDeclaration>,
    issues: &mut Vec<RenderFlowValidationIssue>,
) {
    if matches!(
        pass.kind,
        RenderPassKind::Compute | RenderPassKind::Fullscreen | RenderPassKind::Graphics
    ) {
        let mut seen = BTreeSet::new();
        for resource_id in pass
            .storage_reads
            .iter()
            .chain(&pass.storage_writes)
            .copied()
        {
            if seen.insert(resource_id)
                && matches!(
                    resources_by_id.get(&resource_id),
                    Some(RenderResourceDeclaration::Uniform(_))
                )
            {
                issues.push(RenderFlowValidationIssue::UniformBufferUsedAsStorage {
                    pass_label: pass.label.clone(),
                    resource_id,
                });
            }
        }
    }

    for sampled in &pass.sampled_textures {
        let Some(resource) = resources_by_id.get(sampled) else {
            continue;
        };
        if !matches!(
            resource,
            RenderResourceDeclaration::Sampled(_)
                | RenderResourceDeclaration::StorageImage(_)
                | RenderResourceDeclaration::ColorAttachment(_)
                | RenderResourceDeclaration::DepthAttachment(_)
                | RenderResourceDeclaration::History(_)
                | RenderResourceDeclaration::TargetAlias(_)
                | RenderResourceDeclaration::ImportedTexture(_)
        ) {
            issues.push(RenderFlowValidationIssue::SampledNonTextureResource {
                pass_label: pass.label.clone(),
                resource_id: *sampled,
                resource_kind: resource_kind_name(resource),
            });
        }
    }

    for written in &pass.write_textures {
        let Some(resource) = resources_by_id.get(written) else {
            continue;
        };
        if !matches!(
            resource,
            RenderResourceDeclaration::StorageImage(_) | RenderResourceDeclaration::History(_)
        ) {
            issues.push(RenderFlowValidationIssue::WriteTextureOnInvalidResource {
                pass_label: pass.label.clone(),
                resource_id: *written,
                resource_kind: resource_kind_name(resource),
            });
        }
    }

    if matches!(
        pass.kind,
        RenderPassKind::Fullscreen | RenderPassKind::Graphics
    ) && pass.color_outputs.len() == 1
    {
        let output = pass.color_outputs[0];
        if let Some(resource) = resources_by_id.get(&output)
            && !is_raster_color_output_resource(resource)
        {
            issues.push(
                RenderFlowValidationIssue::InvalidRasterColorOutputResource {
                    pass_kind: render_pass_kind_name(pass.kind),
                    pass_label: pass.label.clone(),
                    resource_id: output,
                    resource_kind: resource_kind_name(resource),
                },
            );
        }
    }

    for id in &pass.vertex_buffers {
        validate_buffer_role_resource(
            pass,
            *id,
            "vertex_buffer",
            GpuBufferUsage::Vertex,
            resources_by_id,
            issues,
        );
    }
    for id in &pass.index_buffers {
        validate_buffer_role_resource(
            pass,
            *id,
            "index_buffer",
            GpuBufferUsage::Index,
            resources_by_id,
            issues,
        );
    }
    for id in &pass.instance_buffers {
        validate_buffer_role_resource(
            pass,
            *id,
            "instance_buffer",
            GpuBufferUsage::Vertex,
            resources_by_id,
            issues,
        );
    }
    for id in &pass.indirect_buffers {
        validate_buffer_role_resource(
            pass,
            *id,
            "indirect_buffer",
            GpuBufferUsage::Indirect,
            resources_by_id,
            issues,
        );
    }

    if let Some(depth_target) = &pass.depth_target
        && let Some(resource) = resources_by_id.get(depth_target)
    {
        let depth_ok = matches!(resource, RenderResourceDeclaration::DepthAttachment(_))
            || matches!(
                resource,
                RenderResourceDeclaration::TargetAlias(value)
                    if value.kind() == RenderTargetAliasKind::Depth
            );
        if !depth_ok {
            issues.push(RenderFlowValidationIssue::InvalidDepthTargetResource {
                pass_label: pass.label.clone(),
                resource_id: *depth_target,
                resource_kind: resource_kind_name(resource),
            });
        }
    }

    if matches!(pass.kind, RenderPassKind::Copy)
        && let (Some(read), Some(write)) = (pass.copy_source, pass.copy_destination)
        && let (Some(read_resource), Some(write_resource)) =
            (resources_by_id.get(&read), resources_by_id.get(&write))
    {
        let read_texture = is_texture_resource(read_resource);
        let write_texture = is_texture_resource(write_resource);
        let read_buffer = is_buffer_resource(read_resource);
        let write_buffer = is_buffer_resource(write_resource);
        if (read_texture && write_buffer) || (read_buffer && write_texture) {
            issues.push(RenderFlowValidationIssue::CopyPassMixedResourceClasses {
                pass_label: pass.label.clone(),
                read_id: read,
                read_kind: resource_kind_name(read_resource),
                write_id: write,
                write_kind: resource_kind_name(write_resource),
            });
        }
    }

    if matches!(pass.kind, RenderPassKind::Present)
        && let Some(read) = pass.present_source
        && let Some(resource) = resources_by_id.get(&read)
        && !is_texture_resource(resource)
    {
        issues.push(RenderFlowValidationIssue::PresentPassReadsNonTexture {
            pass_label: pass.label.clone(),
            resource_id: read,
            resource_kind: resource_kind_name(resource),
        });
    }

    for write in pass
        .storage_writes
        .iter()
        .chain(&pass.write_textures)
        .chain(&pass.color_outputs)
        .chain(pass.copy_destination.iter())
    {
        let Some(resource) = resources_by_id.get(write) else {
            continue;
        };
        if let RenderResourceDeclaration::ImportedTexture(value) = resource {
            if value.semantic != RenderImportedTextureSemantic::SurfaceColor {
                issues.push(
                    RenderFlowValidationIssue::InvalidImportedTextureWriteSemantic {
                        pass_label: pass.label.clone(),
                        resource_id: *write,
                        semantic: value.semantic.as_str(),
                        allowed: RenderImportedTextureSemantic::SurfaceColor.as_str(),
                    },
                );
                continue;
            }
            if !matches!(
                pass.kind,
                RenderPassKind::Fullscreen
                    | RenderPassKind::Graphics
                    | RenderPassKind::BuiltinUiComposite
                    | RenderPassKind::Copy
            ) {
                issues.push(
                    RenderFlowValidationIssue::UnsupportedImportedTextureWriteKind {
                        pass_label: pass.label.clone(),
                        resource_id: *write,
                        pass_kind: pass.kind,
                    },
                );
            }
        }
    }
}

fn validate_graphics_draw_source(
    pass: &RenderPassNode,
    issues: &mut Vec<RenderFlowValidationIssue>,
) {
    let Some(draw) = pass.draw else {
        return;
    };
    if matches!(draw.source, RenderDrawSource::Direct) && !pass.indirect_buffers.is_empty() {
        issues.push(
            RenderFlowValidationIssue::GraphicsPassIndirectBufferWithoutIndirectDraw {
                pass_label: pass.label.clone(),
                count: pass.indirect_buffers.len(),
            },
        );
        return;
    }
    let RenderDrawSource::Indirect {
        args_buffer,
        args_kind,
        args_element_count,
        args_element_size,
        byte_offset,
    } = draw.source
    else {
        return;
    };

    if !pass.indirect_buffers.contains(&args_buffer) {
        issues.push(
            RenderFlowValidationIssue::GraphicsPassIndirectDrawArgsBufferNotDeclared {
                pass_label: pass.label.clone(),
                resource_id: args_buffer,
            },
        );
    }

    let expected_args_kind = if pass.index_buffers.is_empty() {
        RenderIndirectDrawArgsKind::Draw
    } else {
        RenderIndirectDrawArgsKind::DrawIndexed
    };
    if args_kind != expected_args_kind {
        issues.push(
            RenderFlowValidationIssue::GraphicsPassIndirectDrawArgsKindMismatch {
                pass_label: pass.label.clone(),
                args_kind: args_kind.label(),
                expected_args_kind: expected_args_kind.label(),
            },
        );
    }

    if byte_offset % 4 != 0 {
        issues.push(
            RenderFlowValidationIssue::GraphicsPassInvalidIndirectDrawOffset {
                pass_label: pass.label.clone(),
                byte_offset,
            },
        );
    }

    let args_buffer_byte_size = args_element_count.saturating_mul(args_element_size);
    let Some(required_end) = byte_offset.checked_add(args_element_size) else {
        issues.push(
            RenderFlowValidationIssue::GraphicsPassIndirectDrawOffsetOutOfBounds {
                pass_label: pass.label.clone(),
                byte_offset,
                args_element_size,
                args_element_count,
                args_buffer_byte_size,
            },
        );
        return;
    };
    if required_end > args_buffer_byte_size {
        issues.push(
            RenderFlowValidationIssue::GraphicsPassIndirectDrawOffsetOutOfBounds {
                pass_label: pass.label.clone(),
                byte_offset,
                args_element_size,
                args_element_count,
                args_buffer_byte_size,
            },
        );
    }

    if draw.first_vertex != 0 || draw.first_instance != 0 {
        issues.push(
            RenderFlowValidationIssue::GraphicsPassIndirectDrawUsesCpuOffsets {
                pass_label: pass.label.clone(),
                first_vertex: draw.first_vertex,
                first_instance: draw.first_instance,
            },
        );
    }
}

fn validate_fixed_step_regions(
    passes: &[RenderPassNode],
    lexical_pass_ids: &[RenderPassId],
    issues: &mut Vec<RenderFlowValidationIssue>,
) {
    #[derive(Clone)]
    struct RegionShape {
        label: String,
        max_substeps: u32,
        iteration_uniform: GpuWorkResourceId,
        pass_ids: BTreeSet<RenderPassId>,
    }

    let mut regions =
        BTreeMap::<crate::plugins::render::RenderFixedStepRegionId, RegionShape>::new();
    let pass_lookup = passes
        .iter()
        .map(|pass| (pass.id, pass))
        .collect::<BTreeMap<_, _>>();

    for pass in passes {
        let Some(region) = pass.fixed_step_region.as_ref() else {
            continue;
        };
        if region.max_substeps == 0 {
            issues.push(
                RenderFlowValidationIssue::FixedStepRegionInvalidMaxSubsteps {
                    region_label: region.region_label.clone(),
                },
            );
        }
        if matches!(pass.kind, RenderPassKind::Copy | RenderPassKind::Present) {
            issues.push(
                RenderFlowValidationIssue::FixedStepRegionUnsupportedPassKind {
                    region_label: region.region_label.clone(),
                    pass_label: pass.label.clone(),
                    pass_kind: render_pass_kind_name(pass.kind),
                },
            );
        }

        let entry = regions
            .entry(region.region_id)
            .or_insert_with(|| RegionShape {
                label: region.region_label.clone(),
                max_substeps: region.max_substeps,
                iteration_uniform: region.iteration_uniform,
                pass_ids: BTreeSet::new(),
            });
        if entry.label != region.region_label
            || entry.max_substeps != region.max_substeps
            || entry.iteration_uniform != region.iteration_uniform
        {
            issues.push(
                RenderFlowValidationIssue::FixedStepRegionInconsistentDescriptor {
                    region_label: region.region_label.clone(),
                },
            );
        }
        entry.pass_ids.insert(pass.id);
    }

    for region in regions.values() {
        let positions = lexical_pass_ids
            .iter()
            .enumerate()
            .filter_map(|(index, pass_id)| region.pass_ids.contains(pass_id).then_some(index))
            .collect::<Vec<_>>();
        let (Some(first), Some(last)) = (positions.first(), positions.last()) else {
            continue;
        };
        for pass_id in &lexical_pass_ids[*first..=*last] {
            if region.pass_ids.contains(pass_id) {
                continue;
            }
            if let Some(pass) = pass_lookup.get(pass_id) {
                issues.push(
                    RenderFlowValidationIssue::FixedStepRegionPassesNotContiguous {
                        region_label: region.label.clone(),
                        pass_label: pass.label.clone(),
                    },
                );
            }
        }
    }
}

fn validate_graphics_vertex_layouts(
    pass: &RenderPassNode,
    issues: &mut Vec<RenderFlowValidationIssue>,
) {
    validate_graphics_buffer_layout_counts(
        pass,
        "vertex",
        pass.vertex_buffers.len(),
        pass.vertex_buffer_layouts.len(),
        issues,
    );
    validate_graphics_buffer_layout_counts(
        pass,
        "instance",
        pass.instance_buffers.len(),
        pass.instance_buffer_layouts.len(),
        issues,
    );

    let mut slots = BTreeSet::<u32>::new();
    let mut ordered_slots = Vec::<u32>::new();
    let mut shader_locations = BTreeSet::<u32>::new();

    for layout in &pass.vertex_buffer_layouts {
        validate_graphics_layout_step_mode(
            pass,
            "vertex",
            layout.slot,
            layout.step_mode,
            RenderVertexStepMode::Vertex,
            issues,
        );
        validate_graphics_layout_shape(pass, layout, &mut shader_locations, issues);
        if !slots.insert(layout.slot) {
            issues.push(
                RenderFlowValidationIssue::GraphicsPassDuplicateVertexBufferSlot {
                    pass_label: pass.label.clone(),
                    slot: layout.slot,
                },
            );
        }
        ordered_slots.push(layout.slot);
    }

    for layout in &pass.instance_buffer_layouts {
        validate_graphics_layout_step_mode(
            pass,
            "instance",
            layout.slot,
            layout.step_mode,
            RenderVertexStepMode::Instance,
            issues,
        );
        validate_graphics_layout_shape(pass, layout, &mut shader_locations, issues);
        if !slots.insert(layout.slot) {
            issues.push(
                RenderFlowValidationIssue::GraphicsPassDuplicateVertexBufferSlot {
                    pass_label: pass.label.clone(),
                    slot: layout.slot,
                },
            );
        }
        ordered_slots.push(layout.slot);
    }

    ordered_slots.sort_unstable();
    for (expected, found) in ordered_slots.iter().copied().enumerate() {
        if expected as u32 != found {
            issues.push(
                RenderFlowValidationIssue::GraphicsPassNonDenseVertexBufferSlots {
                    pass_label: pass.label.clone(),
                    expected: expected as u32,
                    found,
                },
            );
        }
    }
}

fn validate_graphics_buffer_layout_counts(
    pass: &RenderPassNode,
    role: &'static str,
    buffer_count: usize,
    layout_count: usize,
    issues: &mut Vec<RenderFlowValidationIssue>,
) {
    if buffer_count != layout_count {
        issues.push(
            RenderFlowValidationIssue::GraphicsPassBufferLayoutCountMismatch {
                pass_label: pass.label.clone(),
                role,
                buffer_count,
                layout_count,
            },
        );
    }
}

fn validate_graphics_layout_step_mode(
    pass: &RenderPassNode,
    role: &'static str,
    slot: u32,
    step_mode: RenderVertexStepMode,
    expected: RenderVertexStepMode,
    issues: &mut Vec<RenderFlowValidationIssue>,
) {
    if step_mode != expected {
        issues.push(
            RenderFlowValidationIssue::GraphicsPassBufferLayoutStepModeMismatch {
                pass_label: pass.label.clone(),
                role,
                slot,
                step_mode: vertex_step_mode_name(step_mode),
                expected: vertex_step_mode_name(expected),
            },
        );
    }
}

fn validate_graphics_layout_shape(
    pass: &RenderPassNode,
    layout: &crate::plugins::render::RenderVertexBufferLayout,
    shader_locations: &mut BTreeSet<u32>,
    issues: &mut Vec<RenderFlowValidationIssue>,
) {
    if layout.array_stride == 0 {
        issues.push(RenderFlowValidationIssue::GraphicsPassInvalidVertexStride {
            pass_label: pass.label.clone(),
            slot: layout.slot,
        });
    }

    if layout.attributes.is_empty() {
        issues.push(
            RenderFlowValidationIssue::GraphicsPassMissingVertexAttributes {
                pass_label: pass.label.clone(),
                slot: layout.slot,
            },
        );
    }

    for attribute in &layout.attributes {
        if !shader_locations.insert(attribute.shader_location) {
            issues.push(
                RenderFlowValidationIssue::GraphicsPassDuplicateVertexShaderLocation {
                    pass_label: pass.label.clone(),
                    shader_location: attribute.shader_location,
                },
            );
        }

        let size = attribute.format.size_bytes();
        if attribute.offset.saturating_add(size) > layout.array_stride {
            issues.push(
                RenderFlowValidationIssue::GraphicsPassInvalidVertexAttributeRange {
                    pass_label: pass.label.clone(),
                    slot: layout.slot,
                    shader_location: attribute.shader_location,
                    offset: attribute.offset,
                    size,
                    stride: layout.array_stride,
                },
            );
        }
    }
}

fn vertex_step_mode_name(value: RenderVertexStepMode) -> &'static str {
    match value {
        RenderVertexStepMode::Vertex => "vertex",
        RenderVertexStepMode::Instance => "instance",
    }
}

fn is_raster_color_output_resource(resource: &RenderResourceDeclaration) -> bool {
    match resource {
        RenderResourceDeclaration::ColorAttachment(_) => true,
        RenderResourceDeclaration::TargetAlias(value) => {
            value.kind() == RenderTargetAliasKind::Color
        }
        RenderResourceDeclaration::ImportedTexture(value) => {
            value.semantic == RenderImportedTextureSemantic::SurfaceColor
        }
        _ => false,
    }
}

fn validate_buffer_role_resource(
    pass: &RenderPassNode,
    resource_id: GpuWorkResourceId,
    role: &'static str,
    required_usage: GpuBufferUsage,
    resources_by_id: &BTreeMap<GpuWorkResourceId, &RenderResourceDeclaration>,
    issues: &mut Vec<RenderFlowValidationIssue>,
) {
    let Some(resource) = resources_by_id.get(&resource_id) else {
        return;
    };
    if !is_buffer_resource(resource) {
        issues.push(RenderFlowValidationIssue::InvalidBufferRoleResource {
            pass_label: pass.label.clone(),
            resource_id,
            role,
            resource_kind: resource_kind_name(resource),
        });
        return;
    }

    let descriptor = match resource {
        RenderResourceDeclaration::Uniform(value) => Some(value.handle().descriptor()),
        RenderResourceDeclaration::Storage(value) => Some(value.handle().descriptor()),
        RenderResourceDeclaration::ImportedBuffer(_) => None,
        _ => None,
    };
    if descriptor.is_some_and(|descriptor| !descriptor.usages().contains(required_usage)) {
        issues.push(RenderFlowValidationIssue::MissingBufferRoleUsage {
            pass_label: pass.label.clone(),
            resource_id,
            role,
            required_usage,
        });
    }
}

fn is_texture_resource(resource: &RenderResourceDeclaration) -> bool {
    matches!(
        resource,
        RenderResourceDeclaration::Sampled(_)
            | RenderResourceDeclaration::StorageImage(_)
            | RenderResourceDeclaration::ColorAttachment(_)
            | RenderResourceDeclaration::DepthAttachment(_)
            | RenderResourceDeclaration::History(_)
            | RenderResourceDeclaration::TargetAlias(_)
            | RenderResourceDeclaration::ImportedTexture(_)
    )
}

fn is_buffer_resource(resource: &RenderResourceDeclaration) -> bool {
    matches!(
        resource,
        RenderResourceDeclaration::Uniform(_)
            | RenderResourceDeclaration::Storage(_)
            | RenderResourceDeclaration::ImportedBuffer(_)
    )
}

fn resource_kind_name(resource: &RenderResourceDeclaration) -> &'static str {
    match resource {
        RenderResourceDeclaration::Uniform(_) => "uniform_buffer",
        RenderResourceDeclaration::Storage(_) => "storage_buffer",
        RenderResourceDeclaration::Sampled(_) => "sampled_texture",
        RenderResourceDeclaration::StorageImage(_) => "storage_texture",
        RenderResourceDeclaration::ColorAttachment(_) => "color_target",
        RenderResourceDeclaration::DepthAttachment(_) => "depth_target",
        RenderResourceDeclaration::History(_) => "history_texture",
        RenderResourceDeclaration::TargetAlias(value) => match value.kind() {
            RenderTargetAliasKind::Color => "target_alias(color)",
            RenderTargetAliasKind::Depth => "target_alias(depth)",
            RenderTargetAliasKind::Texture => "target_alias(texture)",
        },
        RenderResourceDeclaration::ImportedTexture(value) => match value.semantic {
            RenderImportedTextureSemantic::SurfaceColor => "imported_texture(surface_color)",
            RenderImportedTextureSemantic::SurfaceDepth => "imported_texture(surface_depth)",
            RenderImportedTextureSemantic::HistoryTexture => "imported_texture(history_texture)",
            RenderImportedTextureSemantic::External => "imported_texture(external)",
        },
        RenderResourceDeclaration::ImportedBuffer(value) => match value.semantic {
            RenderImportedBufferSemantic::HistoryBuffer => "imported_buffer(history_buffer)",
            RenderImportedBufferSemantic::External => "imported_buffer(external)",
        },
    }
}

fn render_pass_kind_name(kind: RenderPassKind) -> &'static str {
    match kind {
        RenderPassKind::Compute => "compute",
        RenderPassKind::Fullscreen => "fullscreen",
        RenderPassKind::BuiltinUiComposite => "builtin_ui_composite",
        RenderPassKind::Graphics => "graphics",
        RenderPassKind::Copy => "copy",
        RenderPassKind::Present => "present",
    }
}

fn validate_imported_resource_descriptors(
    resources_by_id: &BTreeMap<GpuWorkResourceId, &RenderResourceDeclaration>,
    issues: &mut Vec<RenderFlowValidationIssue>,
) {
    let mut surface_color_count = 0usize;
    let mut surface_depth_count = 0usize;

    for (id, descriptor) in resources_by_id {
        match descriptor {
            RenderResourceDeclaration::ImportedTexture(value) => match value.semantic {
                RenderImportedTextureSemantic::SurfaceColor => {
                    surface_color_count = surface_color_count.saturating_add(1);
                }
                RenderImportedTextureSemantic::SurfaceDepth => {
                    surface_depth_count = surface_depth_count.saturating_add(1);
                }
                RenderImportedTextureSemantic::HistoryTexture => {}
                RenderImportedTextureSemantic::External => {
                    issues.push(
                        RenderFlowValidationIssue::UnsupportedExternalImportedTexture {
                            resource_id: *id,
                        },
                    );
                }
            },
            RenderResourceDeclaration::ImportedBuffer(value) => match value.semantic {
                RenderImportedBufferSemantic::HistoryBuffer => {}
                RenderImportedBufferSemantic::External => {
                    issues.push(
                        RenderFlowValidationIssue::UnsupportedExternalImportedBuffer {
                            resource_id: *id,
                        },
                    );
                }
            },
            _ => {}
        }
    }

    if surface_color_count > 1 {
        issues.push(RenderFlowValidationIssue::MultipleSurfaceColorImports {
            canonical_label: SURFACE_COLOR_RESOURCE_LABEL,
        });
    }

    if surface_depth_count > 1 {
        issues.push(RenderFlowValidationIssue::MultipleSurfaceDepthImports {
            canonical_label: SURFACE_DEPTH_RESOURCE_LABEL,
        });
    }
}

fn pass_resource_refs(pass: &RenderPassNode) -> impl Iterator<Item = &GpuWorkResourceId> {
    pass.storage_reads
        .iter()
        .chain(pass.storage_writes.iter())
        .chain(pass.color_outputs.iter())
        .chain(pass.copy_source.iter())
        .chain(pass.copy_destination.iter())
        .chain(pass.present_source.iter())
        .chain(pass.sampled_textures.iter())
        .chain(pass.write_textures.iter())
        .chain(pass.vertex_buffers.iter())
        .chain(pass.index_buffers.iter())
        .chain(pass.instance_buffers.iter())
        .chain(pass.indirect_buffers.iter())
        .chain(pass.depth_target.iter())
        .chain(pass.fixed_step_iteration_uniforms.iter())
}
