use super::pipeline_cache::FlowPipelineArtifactCache;
use super::*;
use crate::plugins::render::api::{
    RenderFixedStepIterationUniform, SURFACE_COLOR_RESOURCE_LABEL, SURFACE_DEPTH_RESOURCE_LABEL,
};
use crate::plugins::render::backend::ensure_compiled_pass_is_supported;
use crate::plugins::render::frame::{PreparedFlowInputs, PreparedRenderFrame};
use crate::plugins::render::graph::{
    CompiledBindingEntry, CompiledBuiltinImport, CompiledComputeExecutionPlan,
    CompiledCopyExecutionPlan, CompiledFixedStepRegion, CompiledPassBindings,
    CompiledPassExecutionPlan, CompiledPresentExecutionPlan, CompiledRenderFlowPlan,
    CompiledResourceRef, CompiledTargetPlan, RenderShaderReference,
    preflight_prepared_render_frame,
};
use crate::plugins::render::inspect::{
    CaptureStage, CaptureTextureClass, RenderCaptureIdentity, RenderCapturePointIdentity,
    RenderCaptureSelector, RenderCaptureSelectorResult, RenderCaptureTerminal,
    RenderCaptureTerminalCode, RenderDebugConfigResource, RenderDebugControlResource,
    RenderGpuTimingCapability, RenderGpuTimingDiagnostic, RenderPassMaterialBindingEvidence,
    RenderPassModelMeshMaterialSelectionEvidence, RenderPassTimingEvidence,
    RenderSelectorResolution, ResolvedRenderCapturePlan, RuntimeResourceInspectionEntry,
    RuntimeResourceReuse, resource_kind_name,
};
use crate::plugins::render::pipelines::{FlowPassKind, FlowPassPipelineKey};
use crate::plugins::render::{RenderResourceDeclaration, current_runtime_gpu_capabilities};
use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

mod bindings;
mod canonical_work;
mod capture;
mod execute;
mod gpu_timing;
mod logical_copy;
mod logical_operations;
mod logical_timing;
mod observation;
mod occurrences;
mod pipeline_realization;
mod preflight_cache;
mod program_sources;
mod provenance;
mod runtime_resources;

/// Opaque G4C3 pipeline realization retained between the renderer's realization and G5
/// execution phases. No raw backend pipeline reference crosses this boundary.
pub(super) enum PreparedFlowPipeline {
    Compute(runen_gpu::GpuRealizedComputePipeline),
    Render(runen_gpu::GpuRealizedRenderPipeline),
}

/// Complete G4C2/G4C3 shader-pipeline realization carried from the batch's first phase into G5.
/// The renderer retains only opaque RunenGPU handles and does not own a reusable raw pipeline.
pub(super) struct PreparedPipelinePass {
    pub(super) bindings: bindings::RealizedFlowProgramBindings,
    pub(super) pipeline: PreparedFlowPipeline,
    pub(super) shader_id: String,
    pub(super) shader_revision: u64,
    pub(super) fallback_used: bool,
}

pub(super) use capture::{
    CaptureTextureSource, FrameCaptureRuntime, PreparedCaptureReadback, TextureReadbackFormat,
    TextureReadbackMode, prepare_texture_capture_readback, texture_readback_format,
};
#[cfg(test)]
pub(super) use execute::FeaturePassAction;
pub(super) use gpu_timing::{GpuPassTimestampIndices, GpuPassTimingFrame};
pub(super) use observation::{RendererGpuObservationOutput, RendererGpuObservationState};
pub(crate) use preflight_cache::RendererPreparedFramePreflightCacheEntry;
pub(crate) use program_sources::RendererProgramSourceAuthority;
pub(super) use provenance::{
    EncodedPassEvidence, collect_pass_material_binding_evidence, collect_pass_resource_truth,
    execution_flow_pass_kind, execution_pass_authoring_index, execution_pass_feature_id,
    execution_pass_id, execution_pass_kind_name, execution_pass_shader_reference,
    feature_runtime_version, hash_view_signature, material_specialization_fragment_hash,
    pass_consumes_material_resources, resolve_shader_material, resolve_shader_material_for_packet,
};
pub(crate) use runtime_resources::{
    FlowRuntimeResources, ResolvedTextureRef, RuntimeResourceKey, RuntimeResourceKind,
};
