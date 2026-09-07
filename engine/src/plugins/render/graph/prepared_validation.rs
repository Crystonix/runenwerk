use crate::plugins::render::features::FeatureFallbackPolicy;
use crate::plugins::render::graph::{
    CompiledBindingEntry, CompiledBuiltinImport, CompiledDispatchPlan, CompiledPassBindings,
    CompiledPassExecutionPlan, CompiledRasterExecutionPlan, CompiledRenderFlowPlan,
    CompiledResourceRef, CompiledStorageAccess, CompiledTargetAliasRef, CompiledViewMask,
    RenderExecutionGraphDiagnostic, RenderExecutionGraphDiagnosticKind,
    RenderExecutionGraphPreparedError, diagnose_compiled_pass_shapes,
};
use crate::plugins::render::{
    PreparedFlowInvocation, PreparedFlowInvocationId, PreparedRenderFrame, PreparedTargetBinding,
    PreparedViewFrame, RenderDynamicTextureTargetDescriptor, RenderDynamicTextureTargetKey,
    RenderResourceDeclaration, RenderTargetAliasKind, validate_compiled_flow_capabilities,
};
use runen_gpu::{GpuCapabilities, GpuWorkResourceId};
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderExecutionGraphPreparedReport {
    pub diagnostics: Vec<RenderExecutionGraphDiagnostic>,
}

impl RenderExecutionGraphPreparedReport {
    pub fn new(diagnostics: Vec<RenderExecutionGraphDiagnostic>) -> Self {
        Self { diagnostics }
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(RenderExecutionGraphDiagnostic::is_error)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.is_error())
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RenderPreparedFramePreflightMode {
    #[default]
    CachedStrict,
    StrictEveryFrame,
}

impl RenderPreparedFramePreflightMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CachedStrict => "cached_strict",
            Self::StrictEveryFrame => "strict_every_frame",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ecs::Component, ecs::Resource)]
pub struct RenderPreflightValidationConfigResource {
    pub mode: RenderPreparedFramePreflightMode,
}

impl Default for RenderPreflightValidationConfigResource {
    fn default() -> Self {
        Self {
            mode: RenderPreparedFramePreflightMode::CachedStrict,
        }
    }
}

impl RenderPreflightValidationConfigResource {
    pub fn strict_every_frame() -> Self {
        Self {
            mode: RenderPreparedFramePreflightMode::StrictEveryFrame,
        }
    }

    pub fn effective_mode(self) -> RenderPreparedFramePreflightMode {
        self.effective_mode_for_env(std::env::var("RUNENWERK_RENDER_STRICT_PREFLIGHT").ok())
    }

    pub fn effective_mode_for_env(
        self,
        env_value: Option<impl AsRef<str>>,
    ) -> RenderPreparedFramePreflightMode {
        if env_value
            .as_ref()
            .is_some_and(|value| render_strict_preflight_env_value_enabled(value.as_ref()))
        {
            RenderPreparedFramePreflightMode::StrictEveryFrame
        } else {
            self.mode
        }
    }
}

fn render_strict_preflight_env_value_enabled(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RenderPreparedFramePreflightReportSource {
    #[default]
    FullValidation,
    CachedReport,
    RuntimeGuard,
}

impl RenderPreparedFramePreflightReportSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FullValidation => "full_validation",
            Self::CachedReport => "cached_report",
            Self::RuntimeGuard => "runtime_guard",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RenderPreparedFramePreflightCacheStatus {
    #[default]
    ColdMiss,
    KeyMismatch,
    Hit,
    StrictMode,
    GuardRejected,
}

impl RenderPreparedFramePreflightCacheStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ColdMiss => "cold_miss",
            Self::KeyMismatch => "key_mismatch",
            Self::Hit => "hit",
            Self::StrictMode => "strict_mode",
            Self::GuardRejected => "guard_rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderPreparedFramePreflightCacheKey {
    pub normalized_capabilities: GpuCapabilities,
    pub flow_registry_revision: u64,
    pub shader_registry_revision: u64,
    pub prepared_structure_hash: u64,
    pub compiled_flow_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPreparedFramePreflightCacheState {
    pub mode: RenderPreparedFramePreflightMode,
    pub status: RenderPreparedFramePreflightCacheStatus,
    pub report_source: RenderPreparedFramePreflightReportSource,
    pub cache_key: Option<RenderPreparedFramePreflightCacheKey>,
}

impl Default for RenderPreparedFramePreflightCacheState {
    fn default() -> Self {
        Self {
            mode: RenderPreparedFramePreflightMode::CachedStrict,
            status: RenderPreparedFramePreflightCacheStatus::ColdMiss,
            report_source: RenderPreparedFramePreflightReportSource::FullValidation,
            cache_key: None,
        }
    }
}

pub fn prepared_render_frame_preflight_cache_key(
    frame: &PreparedRenderFrame,
    compiled_flows: &[CompiledRenderFlowPlan],
    capabilities: &GpuCapabilities,
) -> RenderPreparedFramePreflightCacheKey {
    RenderPreparedFramePreflightCacheKey {
        normalized_capabilities: capabilities.clone(),
        flow_registry_revision: frame.context.flow_registry_revision,
        shader_registry_revision: frame.context.shader_registry_revision,
        prepared_structure_hash: hash_prepared_frame_structure(frame),
        compiled_flow_hash: hash_compiled_flow_structure(compiled_flows),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AliasUseRole {
    ColorOutput,
    DepthOutput,
    SampledTexture,
    StorageTextureWrite,
    StorageBuffer,
    CopySource,
    CopyDestination,
    PresentSource,
}

#[derive(Debug, Clone)]
struct AliasRequirement {
    alias: CompiledTargetAliasRef,
    role: AliasUseRole,
}

pub fn preflight_prepared_render_frame(
    frame: &PreparedRenderFrame,
    compiled_flows: &[CompiledRenderFlowPlan],
    capabilities: &GpuCapabilities,
) -> Result<RenderExecutionGraphPreparedReport, RenderExecutionGraphPreparedError> {
    let report = validate_prepared_render_frame(frame, compiled_flows, capabilities);
    if report.has_errors() {
        Err(RenderExecutionGraphPreparedError::new(report.diagnostics))
    } else {
        Ok(report)
    }
}

pub fn preflight_prepared_render_frame_runtime_guards(
    frame: &PreparedRenderFrame,
    compiled_flows: &[CompiledRenderFlowPlan],
) -> Result<RenderExecutionGraphPreparedReport, RenderExecutionGraphPreparedError> {
    let report = validate_prepared_render_frame_runtime_guards(frame, compiled_flows);
    if report.has_errors() {
        Err(RenderExecutionGraphPreparedError::new(report.diagnostics))
    } else {
        Ok(report)
    }
}

pub fn validate_prepared_render_frame_runtime_guards(
    frame: &PreparedRenderFrame,
    compiled_flows: &[CompiledRenderFlowPlan],
) -> RenderExecutionGraphPreparedReport {
    let mut diagnostics = Vec::<RenderExecutionGraphDiagnostic>::new();
    let flows_by_id = compiled_flows
        .iter()
        .map(|flow| (flow.flow_id, flow))
        .collect::<BTreeMap<_, _>>();
    let views_by_id = frame
        .views
        .iter()
        .map(|view| (view.view_id.as_str(), view))
        .collect::<BTreeMap<_, _>>();

    diagnose_duplicate_invocation_ids(frame, &mut diagnostics);
    diagnose_dynamic_target_history_conflicts(frame, &views_by_id, &mut diagnostics);

    let mut pass_shape_checked_flows = BTreeSet::<crate::plugins::render::RenderFlowId>::new();
    for invocation in &frame.flow_invocations {
        let Some(flow) = flows_by_id.get(&invocation.flow_id).copied() else {
            diagnostics.push(
                RenderExecutionGraphDiagnostic::error(
                    RenderExecutionGraphDiagnosticKind::FlowValidationIssue,
                    format!(
                        "prepared invocation '{}' references unknown compiled flow '{:?}'",
                        invocation.invocation_id.0, invocation.flow_id
                    ),
                )
                .with_invocation(invocation.invocation_id.clone())
                .with_view(invocation.view_id.clone()),
            );
            continue;
        };
        if pass_shape_checked_flows.insert(flow.flow_id) {
            diagnostics.extend(diagnose_compiled_pass_shapes(flow));
        }
        let Some(view) = views_by_id.get(invocation.view_id.as_str()).copied() else {
            diagnostics.push(
                RenderExecutionGraphDiagnostic::error(
                    RenderExecutionGraphDiagnosticKind::PreparedViewMissing,
                    format!(
                        "prepared invocation '{}' references missing view '{}'",
                        invocation.invocation_id.0, invocation.view_id
                    ),
                )
                .with_flow(flow.flow_id, flow.flow_label.clone())
                .with_invocation(invocation.invocation_id.clone())
                .with_view(invocation.view_id.clone()),
            );
            continue;
        };

        for pass in &flow.execution.passes {
            if !pass_targets_view(pass, view) {
                continue;
            }
            validate_pass_dispatch(flow, pass, invocation, &mut diagnostics);
            validate_pass_uniforms(flow, pass, invocation, &mut diagnostics);
        }
    }

    RenderExecutionGraphPreparedReport::new(diagnostics)
}

pub fn validate_prepared_render_frame(
    frame: &PreparedRenderFrame,
    compiled_flows: &[CompiledRenderFlowPlan],
    capabilities: &GpuCapabilities,
) -> RenderExecutionGraphPreparedReport {
    let mut diagnostics = Vec::<RenderExecutionGraphDiagnostic>::new();
    let flows_by_id = compiled_flows
        .iter()
        .map(|flow| (flow.flow_id, flow))
        .collect::<BTreeMap<_, _>>();
    let views_by_id = frame
        .views
        .iter()
        .map(|view| (view.view_id.as_str(), view))
        .collect::<BTreeMap<_, _>>();
    let dynamic_targets = collect_dynamic_target_descriptors(frame, &mut diagnostics);

    diagnose_duplicate_invocation_ids(frame, &mut diagnostics);
    diagnose_dynamic_target_history_conflicts(frame, &views_by_id, &mut diagnostics);

    for flow in compiled_flows {
        diagnostics.extend(diagnose_compiled_pass_shapes(flow));
        diagnostics.extend(validate_compiled_flow_capabilities(flow, capabilities));
    }

    for invocation in &frame.flow_invocations {
        let Some(flow) = flows_by_id.get(&invocation.flow_id).copied() else {
            diagnostics.push(
                RenderExecutionGraphDiagnostic::error(
                    RenderExecutionGraphDiagnosticKind::FlowValidationIssue,
                    format!(
                        "prepared invocation '{}' references unknown compiled flow '{:?}'",
                        invocation.invocation_id.0, invocation.flow_id
                    ),
                )
                .with_invocation(invocation.invocation_id.clone())
                .with_view(invocation.view_id.clone()),
            );
            continue;
        };
        let Some(view) = views_by_id.get(invocation.view_id.as_str()).copied() else {
            diagnostics.push(
                RenderExecutionGraphDiagnostic::error(
                    RenderExecutionGraphDiagnosticKind::PreparedViewMissing,
                    format!(
                        "prepared invocation '{}' references missing view '{}'",
                        invocation.invocation_id.0, invocation.view_id
                    ),
                )
                .with_flow(flow.flow_id, flow.flow_label.clone())
                .with_invocation(invocation.invocation_id.clone())
                .with_view(invocation.view_id.clone()),
            );
            continue;
        };
        validate_invocation(
            flow,
            invocation,
            view,
            &dynamic_targets,
            frame,
            &mut diagnostics,
        );
    }

    RenderExecutionGraphPreparedReport::new(diagnostics)
}

fn collect_dynamic_target_descriptors<'a>(
    frame: &'a PreparedRenderFrame,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) -> BTreeMap<&'a RenderDynamicTextureTargetKey, &'a RenderDynamicTextureTargetDescriptor> {
    let mut descriptors =
        BTreeMap::<&RenderDynamicTextureTargetKey, &RenderDynamicTextureTargetDescriptor>::new();
    for descriptor in &frame.dynamic_texture_targets {
        if let Err(error) = descriptor.validate() {
            diagnostics.push(
                RenderExecutionGraphDiagnostic::error(
                    RenderExecutionGraphDiagnosticKind::DynamicTargetInvalidDescriptor,
                    format!(
                        "dynamic target '{}' has invalid descriptor: {}",
                        descriptor.key, error
                    ),
                )
                .with_dynamic_target(descriptor.key.clone()),
            );
        }
        if let Some(existing) = descriptors.insert(&descriptor.key, descriptor)
            && existing.signature() != descriptor.signature()
        {
            diagnostics.push(
                RenderExecutionGraphDiagnostic::error(
                    RenderExecutionGraphDiagnosticKind::DynamicTargetInvalidDescriptor,
                    format!(
                        "dynamic target '{}' has conflicting descriptors in one prepared frame",
                        descriptor.key
                    ),
                )
                .with_dynamic_target(descriptor.key.clone()),
            );
        }
    }
    descriptors
}

fn diagnose_duplicate_invocation_ids(
    frame: &PreparedRenderFrame,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    let mut seen = BTreeSet::<&PreparedFlowInvocationId>::new();
    for invocation in &frame.flow_invocations {
        if seen.insert(&invocation.invocation_id) {
            continue;
        }
        diagnostics.push(
            RenderExecutionGraphDiagnostic::error(
                RenderExecutionGraphDiagnosticKind::PreparedInvocationDuplicate,
                format!(
                    "prepared invocation '{}' appears more than once in the prepared frame",
                    invocation.invocation_id.0
                ),
            )
            .with_invocation(invocation.invocation_id.clone())
            .with_view(invocation.view_id.clone()),
        );
    }
}

fn diagnose_dynamic_target_history_conflicts(
    frame: &PreparedRenderFrame,
    views_by_id: &BTreeMap<&str, &PreparedViewFrame>,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    let mut signatures = BTreeMap::<RenderDynamicTextureTargetKey, String>::new();
    for invocation in &frame.flow_invocations {
        let effective_signature = invocation.history_signature.as_ref().or_else(|| {
            views_by_id
                .get(invocation.view_id.as_str())
                .and_then(|view| view.history_signature.as_ref())
        });
        let Some(signature) = effective_signature else {
            continue;
        };
        for binding in invocation.target_alias_bindings.values() {
            let PreparedTargetBinding::DynamicTexture(key) = binding else {
                continue;
            };
            if let Some(existing) = signatures.get(key)
                && existing != signature
            {
                diagnostics.push(
                    RenderExecutionGraphDiagnostic::error(
                        RenderExecutionGraphDiagnosticKind::HistorySignatureConflict,
                        format!(
                            "dynamic target '{}' has incompatible history signatures '{}' and '{}'",
                            key, existing, signature
                        ),
                    )
                    .with_dynamic_target(key.clone())
                    .with_invocation(invocation.invocation_id.clone())
                    .with_view(invocation.view_id.clone())
                    .with_history_signature(signature.clone()),
                );
                continue;
            }
            signatures.insert(key.clone(), signature.clone());
        }
    }
}

fn validate_invocation(
    flow: &CompiledRenderFlowPlan,
    invocation: &PreparedFlowInvocation,
    view: &PreparedViewFrame,
    dynamic_targets: &BTreeMap<
        &RenderDynamicTextureTargetKey,
        &RenderDynamicTextureTargetDescriptor,
    >,
    frame: &PreparedRenderFrame,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    for pass in &flow.execution.passes {
        if !pass_targets_view(pass, view) {
            continue;
        }
        validate_pass_feature_gate(flow, pass, invocation, frame, diagnostics);
        validate_pass_dispatch(flow, pass, invocation, diagnostics);
        validate_pass_uniforms(flow, pass, invocation, diagnostics);
        for requirement in alias_requirements(pass) {
            validate_alias_requirement(
                flow,
                pass,
                invocation,
                &requirement,
                dynamic_targets,
                diagnostics,
            );
        }
    }
}

fn validate_pass_feature_gate(
    flow: &CompiledRenderFlowPlan,
    pass: &CompiledPassExecutionPlan,
    invocation: &PreparedFlowInvocation,
    frame: &PreparedRenderFrame,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    let Some(feature_id) = pass_feature_id(pass) else {
        return;
    };
    let Some(gate) = frame.contributions.feature_gate(&feature_id) else {
        diagnostics.push(
            RenderExecutionGraphDiagnostic::error(
                RenderExecutionGraphDiagnosticKind::FeatureGateMissing,
                format!(
                    "pass '{}' requires feature '{:?}' but the prepared frame has no feature gate",
                    pass_id(pass),
                    feature_id
                ),
            )
            .with_flow(flow.flow_id, flow.flow_label.clone())
            .with_pass(pass_id(pass), pass_id(pass).to_string())
            .with_invocation(invocation.invocation_id.clone()),
        );
        return;
    };

    if gate.fallback_policy == FeatureFallbackPolicy::FailFrame
        && !matches!(
            gate.status,
            crate::plugins::render::FeatureContributionStatus::Ready
        )
    {
        diagnostics.push(
            RenderExecutionGraphDiagnostic::error(
                RenderExecutionGraphDiagnosticKind::FeatureGateMissing,
                format!(
                    "pass '{}' feature '{:?}' is {:?} and fallback policy is fail-frame",
                    pass_id(pass),
                    feature_id,
                    gate.status
                ),
            )
            .with_flow(flow.flow_id, flow.flow_label.clone())
            .with_pass(pass_id(pass), pass_id(pass).to_string())
            .with_invocation(invocation.invocation_id.clone()),
        );
    }
}

fn validate_pass_dispatch(
    flow: &CompiledRenderFlowPlan,
    pass: &CompiledPassExecutionPlan,
    invocation: &PreparedFlowInvocation,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    let CompiledPassExecutionPlan::Compute(value) = pass else {
        return;
    };
    if value.dispatch.is_none() {
        return;
    }
    if invocation
        .inputs
        .projected_dispatch_workgroups
        .contains_key(&value.pass_id)
    {
        return;
    }
    diagnostics.push(
        RenderExecutionGraphDiagnostic::error(
            RenderExecutionGraphDiagnosticKind::DispatchMissing,
            format!(
                "prepared invocation '{}' is missing dispatch workgroups for compute pass '{}'",
                invocation.invocation_id.0, value.pass_id
            ),
        )
        .with_flow(flow.flow_id, flow.flow_label.clone())
        .with_pass(value.pass_id, value.pass_id.to_string())
        .with_invocation(invocation.invocation_id.clone())
        .with_view(invocation.view_id.clone()),
    );
}

fn validate_pass_uniforms(
    flow: &CompiledRenderFlowPlan,
    pass: &CompiledPassExecutionPlan,
    invocation: &PreparedFlowInvocation,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    for uniform_id in pass_uniform_order(pass) {
        match invocation.inputs.projected_uniform_bytes.get(uniform_id) {
            Some(bytes) if !bytes.is_empty() => {}
            Some(_) => diagnostics.push(
                RenderExecutionGraphDiagnostic::error(
                    RenderExecutionGraphDiagnosticKind::UniformOverrideEmpty,
                    format!(
                        "prepared invocation '{}' supplies empty bytes for uniform buffer '{}'",
                        invocation.invocation_id.0, uniform_id
                    ),
                )
                .with_flow(flow.flow_id, flow.flow_label.clone())
                .with_pass(pass_id(pass), pass_id(pass).to_string())
                .with_resource(*uniform_id, flow.resource_label(*uniform_id))
                .with_invocation(invocation.invocation_id.clone()),
            ),
            None => diagnostics.push(
                RenderExecutionGraphDiagnostic::error(
                    RenderExecutionGraphDiagnosticKind::UniformMissing,
                    format!(
                        "prepared invocation '{}' is missing uniform buffer '{}' required by pass '{}'",
                        invocation.invocation_id.0,
                        uniform_id,
                        pass_id(pass)
                    ),
                )
                .with_flow(flow.flow_id, flow.flow_label.clone())
                .with_pass(pass_id(pass), pass_id(pass).to_string())
                .with_resource(*uniform_id, flow.resource_label(*uniform_id))
                .with_invocation(invocation.invocation_id.clone()),
            ),
        }
    }
}

fn validate_alias_requirement(
    flow: &CompiledRenderFlowPlan,
    pass: &CompiledPassExecutionPlan,
    invocation: &PreparedFlowInvocation,
    requirement: &AliasRequirement,
    dynamic_targets: &BTreeMap<
        &RenderDynamicTextureTargetKey,
        &RenderDynamicTextureTargetDescriptor,
    >,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    let Some(binding) = invocation
        .target_alias_bindings
        .get(&requirement.alias.binding_key)
    else {
        diagnostics.push(
            RenderExecutionGraphDiagnostic::error(
                RenderExecutionGraphDiagnosticKind::TargetAliasMissingBinding,
                format!(
                    "prepared invocation '{}' does not bind target alias '{}' required by pass '{}'",
                    invocation.invocation_id.0,
                    requirement.alias.binding_key,
                    pass_id(pass)
                ),
            )
            .with_flow(flow.flow_id, flow.flow_label.clone())
            .with_pass(pass_id(pass), pass_id(pass).to_string())
            .with_alias(requirement.alias.binding_key.clone(), requirement.alias.kind)
            .with_invocation(invocation.invocation_id.clone())
            .with_view(invocation.view_id.clone()),
        );
        return;
    };

    validate_alias_kind_binding(flow, pass, invocation, requirement, binding, diagnostics);
    match binding {
        PreparedTargetBinding::DynamicTexture(key) => {
            let Some(descriptor) = dynamic_targets.get(key) else {
                diagnostics.push(
                    RenderExecutionGraphDiagnostic::error(
                        RenderExecutionGraphDiagnosticKind::DynamicTargetMissingDescriptor,
                        format!(
                            "prepared invocation '{}' binds alias '{}' to dynamic target '{}' but no descriptor was requested this frame",
                            invocation.invocation_id.0,
                            requirement.alias.binding_key,
                            key
                        ),
                    )
                    .with_flow(flow.flow_id, flow.flow_label.clone())
                    .with_pass(pass_id(pass), pass_id(pass).to_string())
                    .with_alias(requirement.alias.binding_key.clone(), requirement.alias.kind)
                    .with_dynamic_target(key.clone())
                    .with_invocation(invocation.invocation_id.clone())
                    .with_view(invocation.view_id.clone()),
                );
                return;
            };
            validate_dynamic_target_usage(
                flow,
                pass,
                invocation,
                requirement,
                descriptor,
                diagnostics,
            );
        }
        PreparedTargetBinding::FlowOwned(resource_id) => validate_flow_owned_alias_binding(
            flow,
            pass,
            invocation,
            requirement,
            *resource_id,
            diagnostics,
        ),
        PreparedTargetBinding::SurfaceDepth => diagnostics.push(
            RenderExecutionGraphDiagnostic::error(
                RenderExecutionGraphDiagnosticKind::UnsupportedImportedResource,
                format!(
                    "prepared invocation '{}' binds alias '{}' to surface depth, but surface-depth imports are not supported by active runtime execution",
                    invocation.invocation_id.0,
                    requirement.alias.binding_key
                ),
            )
            .with_flow(flow.flow_id, flow.flow_label.clone())
            .with_pass(pass_id(pass), pass_id(pass).to_string())
            .with_alias(requirement.alias.binding_key.clone(), requirement.alias.kind)
            .with_invocation(invocation.invocation_id.clone())
            .with_view(invocation.view_id.clone()),
        ),
        PreparedTargetBinding::SurfaceColor => {}
    }
}

fn validate_alias_kind_binding(
    flow: &CompiledRenderFlowPlan,
    pass: &CompiledPassExecutionPlan,
    invocation: &PreparedFlowInvocation,
    requirement: &AliasRequirement,
    binding: &PreparedTargetBinding,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    let compatible = matches!(
        (requirement.alias.kind, requirement.role, binding),
        (
            RenderTargetAliasKind::Color,
            AliasUseRole::ColorOutput
                | AliasUseRole::SampledTexture
                | AliasUseRole::CopySource
                | AliasUseRole::CopyDestination
                | AliasUseRole::PresentSource,
            PreparedTargetBinding::SurfaceColor
                | PreparedTargetBinding::DynamicTexture(_)
                | PreparedTargetBinding::FlowOwned(_),
        ) | (
            RenderTargetAliasKind::Depth,
            AliasUseRole::DepthOutput | AliasUseRole::SampledTexture,
            PreparedTargetBinding::SurfaceDepth
                | PreparedTargetBinding::DynamicTexture(_)
                | PreparedTargetBinding::FlowOwned(_),
        ) | (
            RenderTargetAliasKind::Texture,
            AliasUseRole::SampledTexture
                | AliasUseRole::StorageTextureWrite
                | AliasUseRole::CopySource
                | AliasUseRole::CopyDestination
                | AliasUseRole::PresentSource,
            PreparedTargetBinding::DynamicTexture(_)
                | PreparedTargetBinding::FlowOwned(_)
                | PreparedTargetBinding::SurfaceColor,
        )
    );
    if compatible {
        return;
    }
    diagnostics.push(
        RenderExecutionGraphDiagnostic::error(
            RenderExecutionGraphDiagnosticKind::TargetAliasKindMismatch,
            format!(
                "prepared invocation '{}' binds alias '{}' with incompatible role {:?} and binding {:?}",
                invocation.invocation_id.0,
                requirement.alias.binding_key,
                requirement.role,
                binding
            ),
        )
        .with_flow(flow.flow_id, flow.flow_label.clone())
        .with_pass(pass_id(pass), pass_id(pass).to_string())
        .with_alias(requirement.alias.binding_key.clone(), requirement.alias.kind)
        .with_invocation(invocation.invocation_id.clone())
        .with_view(invocation.view_id.clone()),
    );
}

fn validate_dynamic_target_usage(
    flow: &CompiledRenderFlowPlan,
    pass: &CompiledPassExecutionPlan,
    invocation: &PreparedFlowInvocation,
    requirement: &AliasRequirement,
    descriptor: &RenderDynamicTextureTargetDescriptor,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    let usage_ok = match requirement.role {
        AliasUseRole::ColorOutput => {
            descriptor.usage.color_attachment && !descriptor.format.is_depth()
        }
        AliasUseRole::DepthOutput => {
            descriptor.usage.depth_attachment && descriptor.format.is_depth()
        }
        AliasUseRole::SampledTexture => {
            descriptor.usage.sampled && descriptor.sample_mode.is_sampled()
        }
        AliasUseRole::PresentSource => descriptor.usage.copy_src,
        AliasUseRole::StorageTextureWrite => descriptor.usage.storage,
        AliasUseRole::CopySource => descriptor.usage.copy_src,
        AliasUseRole::CopyDestination => descriptor.usage.copy_dst,
        AliasUseRole::StorageBuffer => false,
    };
    if usage_ok {
        return;
    }
    diagnostics.push(
        RenderExecutionGraphDiagnostic::error(
            RenderExecutionGraphDiagnosticKind::DynamicTargetUsageMismatch,
            format!(
                "dynamic target '{}' is incompatible with alias '{}' role {:?} in pass '{}'",
                descriptor.key,
                requirement.alias.binding_key,
                requirement.role,
                pass_id(pass)
            ),
        )
        .with_flow(flow.flow_id, flow.flow_label.clone())
        .with_pass(pass_id(pass), pass_id(pass).to_string())
        .with_alias(
            requirement.alias.binding_key.clone(),
            requirement.alias.kind,
        )
        .with_dynamic_target(descriptor.key.clone())
        .with_invocation(invocation.invocation_id.clone())
        .with_view(invocation.view_id.clone()),
    );
}

fn validate_flow_owned_alias_binding(
    flow: &CompiledRenderFlowPlan,
    pass: &CompiledPassExecutionPlan,
    invocation: &PreparedFlowInvocation,
    requirement: &AliasRequirement,
    resource_id: GpuWorkResourceId,
    diagnostics: &mut Vec<RenderExecutionGraphDiagnostic>,
) {
    let Some(descriptor) = flow.resource_descriptor(resource_id) else {
        diagnostics.push(
            RenderExecutionGraphDiagnostic::error(
                RenderExecutionGraphDiagnosticKind::InvalidResource,
                format!(
                    "prepared invocation '{}' binds alias '{}' to unknown flow-owned resource '{}'",
                    invocation.invocation_id.0, requirement.alias.binding_key, resource_id
                ),
            )
            .with_flow(flow.flow_id, flow.flow_label.clone())
            .with_pass(pass_id(pass), pass_id(pass).to_string())
            .with_resource(resource_id, Option::<String>::None)
            .with_alias(
                requirement.alias.binding_key.clone(),
                requirement.alias.kind,
            )
            .with_invocation(invocation.invocation_id.clone())
            .with_view(invocation.view_id.clone()),
        );
        return;
    };

    let compatible = match requirement.role {
        AliasUseRole::ColorOutput => matches!(
            descriptor,
            RenderResourceDeclaration::ColorAttachment(_)
                | RenderResourceDeclaration::ImportedTexture(_)
        ),
        AliasUseRole::DepthOutput => {
            matches!(descriptor, RenderResourceDeclaration::DepthAttachment(_))
        }
        AliasUseRole::SampledTexture | AliasUseRole::PresentSource => {
            is_texture_descriptor(descriptor)
        }
        AliasUseRole::StorageTextureWrite => {
            matches!(descriptor, RenderResourceDeclaration::StorageImage(_))
        }
        AliasUseRole::CopySource | AliasUseRole::CopyDestination => true,
        AliasUseRole::StorageBuffer => is_buffer_descriptor(descriptor),
    };
    if compatible {
        return;
    }
    diagnostics.push(
        RenderExecutionGraphDiagnostic::error(
            RenderExecutionGraphDiagnosticKind::TargetAliasKindMismatch,
            format!(
                "prepared invocation '{}' binds alias '{}' to resource '{}' with incompatible role {:?}",
                invocation.invocation_id.0,
                requirement.alias.binding_key,
                resource_id,
                requirement.role
            ),
        )
        .with_flow(flow.flow_id, flow.flow_label.clone())
        .with_pass(pass_id(pass), pass_id(pass).to_string())
        .with_resource(resource_id, flow.resource_label(resource_id))
        .with_alias(
            requirement.alias.binding_key.clone(),
            requirement.alias.kind,
        )
        .with_invocation(invocation.invocation_id.clone())
        .with_view(invocation.view_id.clone()),
    );
}

fn alias_requirements(pass: &CompiledPassExecutionPlan) -> Vec<AliasRequirement> {
    let mut requirements = Vec::<AliasRequirement>::new();
    match pass {
        CompiledPassExecutionPlan::Compute(value) => {
            collect_binding_alias_requirements(
                &value.bindings.bind_group.entries,
                &mut requirements,
            );
        }
        CompiledPassExecutionPlan::Fullscreen(value)
        | CompiledPassExecutionPlan::Graphics(value) => {
            collect_binding_alias_requirements(
                &value.bindings.bind_group.entries,
                &mut requirements,
            );
            for resource in &value.targets.color_outputs {
                collect_alias_requirement(resource, AliasUseRole::ColorOutput, &mut requirements);
            }
            if let Some(resource) = &value.targets.depth_output {
                collect_alias_requirement(resource, AliasUseRole::DepthOutput, &mut requirements);
            }
        }
        CompiledPassExecutionPlan::Copy(value) => {
            if let Some(resource) = &value.source {
                collect_alias_requirement(resource, AliasUseRole::CopySource, &mut requirements);
            }
            if let Some(resource) = &value.destination {
                collect_alias_requirement(
                    resource,
                    AliasUseRole::CopyDestination,
                    &mut requirements,
                );
            }
        }
        CompiledPassExecutionPlan::Present(value) => {
            if let Some(resource) = &value.source {
                collect_alias_requirement(resource, AliasUseRole::PresentSource, &mut requirements);
            }
        }
        CompiledPassExecutionPlan::BuiltinUiComposite(_) => {}
    }
    requirements
}

fn collect_binding_alias_requirements(
    entries: &[CompiledBindingEntry],
    requirements: &mut Vec<AliasRequirement>,
) {
    for entry in entries {
        match entry {
            CompiledBindingEntry::SampledTexture { resource, .. } => {
                collect_alias_requirement(resource, AliasUseRole::SampledTexture, requirements);
            }
            CompiledBindingEntry::StorageTexture { resource, .. } => {
                collect_alias_requirement(
                    resource,
                    AliasUseRole::StorageTextureWrite,
                    requirements,
                );
            }
            CompiledBindingEntry::StorageBuffer {
                resource, access, ..
            } => {
                if matches!(
                    access,
                    CompiledStorageAccess::ReadOnly
                        | CompiledStorageAccess::ReadWrite
                        | CompiledStorageAccess::WriteOnly
                ) {
                    collect_alias_requirement(resource, AliasUseRole::StorageBuffer, requirements);
                }
            }
            CompiledBindingEntry::Sampler { .. } | CompiledBindingEntry::UniformBuffer { .. } => {}
        }
    }
}

fn collect_alias_requirement(
    resource: &CompiledResourceRef,
    role: AliasUseRole,
    requirements: &mut Vec<AliasRequirement>,
) {
    if let CompiledResourceRef::TargetAlias(alias) = resource {
        requirements.push(AliasRequirement {
            alias: alias.clone(),
            role,
        });
    }
}

fn pass_targets_view(pass: &CompiledPassExecutionPlan, view: &PreparedViewFrame) -> bool {
    match pass {
        CompiledPassExecutionPlan::Compute(value) => {
            value.view_mask.includes(view.view_id.as_str(), view.kind)
        }
        CompiledPassExecutionPlan::Fullscreen(value)
        | CompiledPassExecutionPlan::Graphics(value) => {
            value.view_mask.includes(view.view_id.as_str(), view.kind)
        }
        CompiledPassExecutionPlan::Copy(value) => {
            value.view_mask.includes(view.view_id.as_str(), view.kind)
        }
        CompiledPassExecutionPlan::Present(value) => {
            value.view_mask.includes(view.view_id.as_str(), view.kind)
        }
        CompiledPassExecutionPlan::BuiltinUiComposite(value) => {
            value.view_mask.includes(view.view_id.as_str(), view.kind)
        }
    }
}

fn pass_id(pass: &CompiledPassExecutionPlan) -> crate::plugins::render::RenderPassId {
    match pass {
        CompiledPassExecutionPlan::Compute(value) => value.pass_id,
        CompiledPassExecutionPlan::Fullscreen(value)
        | CompiledPassExecutionPlan::Graphics(value) => value.pass_id,
        CompiledPassExecutionPlan::Copy(value) => value.pass_id,
        CompiledPassExecutionPlan::Present(value) => value.pass_id,
        CompiledPassExecutionPlan::BuiltinUiComposite(value) => value.pass_id,
    }
}

fn pass_feature_id(
    pass: &CompiledPassExecutionPlan,
) -> Option<crate::plugins::render::RenderFeatureId> {
    match pass {
        CompiledPassExecutionPlan::Compute(value) => value.feature_id,
        CompiledPassExecutionPlan::Fullscreen(value)
        | CompiledPassExecutionPlan::Graphics(value) => value.feature_id,
        CompiledPassExecutionPlan::Copy(value) => value.feature_id,
        CompiledPassExecutionPlan::Present(value) => value.feature_id,
        CompiledPassExecutionPlan::BuiltinUiComposite(value) => Some(value.feature_id),
    }
}

fn pass_uniform_order(pass: &CompiledPassExecutionPlan) -> &[GpuWorkResourceId] {
    match pass {
        CompiledPassExecutionPlan::Compute(value) => &value.bindings.uniform_order,
        CompiledPassExecutionPlan::Fullscreen(value)
        | CompiledPassExecutionPlan::Graphics(value) => &value.bindings.uniform_order,
        CompiledPassExecutionPlan::Copy(_)
        | CompiledPassExecutionPlan::Present(_)
        | CompiledPassExecutionPlan::BuiltinUiComposite(_) => &[],
    }
}

fn hash_prepared_frame_structure(frame: &PreparedRenderFrame) -> u64 {
    hash_with(|hasher| {
        frame.context.flow_registry_revision.hash(hasher);
        frame.context.shader_registry_revision.hash(hasher);
        frame.shader.registry_revision.hash(hasher);
        frame.surface.render_surface_id.raw().hash(hasher);
        frame
            .surface
            .native_window_id
            .map(|id| id.raw())
            .hash(hasher);
        frame.surface.target_size_px.hash(hasher);

        frame.views.len().hash(hasher);
        for view in &frame.views {
            view.view_id.hash(hasher);
            view_kind_tag(view.kind).hash(hasher);
            view.target_size_px.hash(hasher);
            view.history_signature.hash(hasher);
        }

        frame.flow_invocations.len().hash(hasher);
        for invocation in &frame.flow_invocations {
            invocation.invocation_id.hash(hasher);
            invocation.flow_id.hash(hasher);
            invocation.view_id.hash(hasher);
            invocation.history_signature.hash(hasher);
            invocation.target_alias_bindings.len().hash(hasher);
            for (alias, binding) in &invocation.target_alias_bindings {
                alias.hash(hasher);
                hash_prepared_target_binding(binding, hasher);
            }
            invocation.inputs.projected_uniform_bytes.len().hash(hasher);
            for (resource_id, bytes) in &invocation.inputs.projected_uniform_bytes {
                resource_id.hash(hasher);
                bytes.len().hash(hasher);
            }
            invocation
                .inputs
                .projected_dispatch_workgroups
                .len()
                .hash(hasher);
            for pass_id in invocation.inputs.projected_dispatch_workgroups.keys() {
                pass_id.hash(hasher);
            }
            invocation.inputs.required_state_types.len().hash(hasher);
            for state in &invocation.inputs.required_state_types {
                state.type_name.hash(hasher);
            }
        }

        frame.dynamic_texture_targets.len().hash(hasher);
        for descriptor in &frame.dynamic_texture_targets {
            descriptor.key.hash(hasher);
            descriptor.signature().hash(hasher);
        }

        frame.contributions.by_feature.len().hash(hasher);
        for (feature_id, contribution) in &frame.contributions.by_feature {
            feature_id.hash(hasher);
            contribution.status.hash(hasher);
            contribution.fallback_policy.hash(hasher);
        }
    })
}

fn hash_compiled_flow_structure(compiled_flows: &[CompiledRenderFlowPlan]) -> u64 {
    hash_with(|hasher| {
        compiled_flows.len().hash(hasher);
        for flow in compiled_flows {
            flow.flow_id.hash(hasher);
            flow.flow_label.hash(hasher);
            flow.render_passes.len().hash(hasher);
            for pass in &flow.render_passes {
                pass.pass_id().hash(hasher);
                pass.pass_label().hash(hasher);
                pass.node().kind.hash(hasher);
            }
            flow.resources.resources.len().hash(hasher);
            for descriptor in &flow.resources.resources {
                descriptor.id().hash(hasher);
                hash_resource_descriptor_kind(descriptor, hasher);
            }
            flow.resource_ids_by_label.len().hash(hasher);
            for (label, resource_id) in &flow.resource_ids_by_label {
                label.hash(hasher);
                resource_id.hash(hasher);
            }
            flow.execution.passes.len().hash(hasher);
            for pass in &flow.execution.passes {
                hash_compiled_pass_structure(pass, hasher);
            }
            flow.compiler_diagnostics.len().hash(hasher);
            for diagnostic in &flow.compiler_diagnostics {
                diagnostic.kind.hash(hasher);
                diagnostic.severity.hash(hasher);
            }
        }
    })
}

fn hash_compiled_pass_structure(pass: &CompiledPassExecutionPlan, hasher: &mut impl Hasher) {
    match pass {
        CompiledPassExecutionPlan::Compute(value) => {
            "compute".hash(hasher);
            value.pass_id.hash(hasher);
            value.authoring_index.hash(hasher);
            value.feature_id.hash(hasher);
            hash_view_mask(&value.view_mask, hasher);
            hash_bindings(&value.bindings, hasher);
            hash_dispatch_plan(value.dispatch.as_ref(), hasher);
        }
        CompiledPassExecutionPlan::Fullscreen(value) => {
            "fullscreen".hash(hasher);
            hash_raster_pass_structure(value, hasher);
        }
        CompiledPassExecutionPlan::Graphics(value) => {
            "graphics".hash(hasher);
            hash_raster_pass_structure(value, hasher);
        }
        CompiledPassExecutionPlan::Copy(value) => {
            "copy".hash(hasher);
            value.pass_id.hash(hasher);
            value.authoring_index.hash(hasher);
            value.feature_id.hash(hasher);
            hash_view_mask(&value.view_mask, hasher);
            hash_compiled_resource_ref(value.source.as_ref(), hasher);
            hash_compiled_resource_ref(value.destination.as_ref(), hasher);
        }
        CompiledPassExecutionPlan::Present(value) => {
            "present".hash(hasher);
            value.pass_id.hash(hasher);
            value.authoring_index.hash(hasher);
            value.feature_id.hash(hasher);
            hash_view_mask(&value.view_mask, hasher);
            hash_compiled_resource_ref(value.source.as_ref(), hasher);
        }
        CompiledPassExecutionPlan::BuiltinUiComposite(value) => {
            "builtin_ui".hash(hasher);
            value.pass_id.hash(hasher);
            value.authoring_index.hash(hasher);
            value.feature_id.hash(hasher);
            hash_view_mask(&value.view_mask, hasher);
            hash_compiled_resource_ref(Some(&value.color_output), hasher);
        }
    }
}

fn hash_raster_pass_structure(value: &CompiledRasterExecutionPlan, hasher: &mut impl Hasher) {
    value.pass_id.hash(hasher);
    value.authoring_index.hash(hasher);
    value.feature_id.hash(hasher);
    hash_view_mask(&value.view_mask, hasher);
    hash_bindings(&value.bindings, hasher);
    value.targets.color_outputs.len().hash(hasher);
    for resource in &value.targets.color_outputs {
        hash_compiled_resource_ref(Some(resource), hasher);
    }
    hash_compiled_resource_ref(value.targets.depth_output.as_ref(), hasher);
    value.draw_buffers.vertex_buffers.len().hash(hasher);
    for binding in &value.draw_buffers.vertex_buffers {
        hash_compiled_resource_ref(Some(&binding.resource), hasher);
        binding.layout.hash(hasher);
    }
    value.draw_buffers.instance_buffers.len().hash(hasher);
    for resource in &value.draw_buffers.instance_buffers {
        hash_compiled_resource_ref(Some(resource), hasher);
    }
    value.draw_buffers.index_buffers.len().hash(hasher);
    for resource in &value.draw_buffers.index_buffers {
        hash_compiled_resource_ref(Some(resource), hasher);
    }
    value.draw_buffers.indirect_buffers.len().hash(hasher);
    for resource in &value.draw_buffers.indirect_buffers {
        hash_compiled_resource_ref(Some(resource), hasher);
    }
    if let Some(clear_color) = value.clear_color {
        true.hash(hasher);
        for channel in clear_color {
            channel.to_bits().hash(hasher);
        }
    } else {
        false.hash(hasher);
    }
    value.draw.hash(hasher);
}

fn hash_bindings(bindings: &CompiledPassBindings, hasher: &mut impl Hasher) {
    bindings.uniform_order.hash(hasher);
    bindings.storage_order.len().hash(hasher);
    for binding in &bindings.storage_order {
        hash_compiled_resource_ref(Some(&binding.resource), hasher);
        hash_storage_access(binding.access, hasher);
    }
    bindings.bind_group.entries.len().hash(hasher);
    for entry in &bindings.bind_group.entries {
        entry.key().hash(hasher);
        match entry {
            CompiledBindingEntry::SampledTexture { resource, .. } => {
                "sampled_texture".hash(hasher);
                hash_compiled_resource_ref(Some(resource), hasher);
            }
            CompiledBindingEntry::Sampler { .. } => {
                "sampler".hash(hasher);
            }
            CompiledBindingEntry::StorageTexture {
                resource, access, ..
            } => {
                "storage_texture".hash(hasher);
                hash_compiled_resource_ref(Some(resource), hasher);
                hash_storage_access(*access, hasher);
            }
            CompiledBindingEntry::UniformBuffer { resource, .. } => {
                "uniform_buffer".hash(hasher);
                resource.hash(hasher);
            }
            CompiledBindingEntry::StorageBuffer {
                resource, access, ..
            } => {
                "storage_buffer".hash(hasher);
                hash_compiled_resource_ref(Some(resource), hasher);
                hash_storage_access(*access, hasher);
            }
        }
    }
}

fn hash_compiled_resource_ref(resource: Option<&CompiledResourceRef>, hasher: &mut impl Hasher) {
    match resource {
        Some(CompiledResourceRef::FlowOwned(resource_id)) => {
            "flow_owned".hash(hasher);
            resource_id.hash(hasher);
        }
        Some(CompiledResourceRef::TargetAlias(alias)) => {
            "target_alias".hash(hasher);
            alias.resource_id.hash(hasher);
            alias.binding_key.hash(hasher);
            alias.kind.hash(hasher);
        }
        Some(CompiledResourceRef::ImportedBuiltin(value)) => {
            "imported_builtin".hash(hasher);
            hash_builtin_import(*value, hasher);
        }
        Some(CompiledResourceRef::Imported(resource_id)) => {
            "imported".hash(hasher);
            resource_id.hash(hasher);
        }
        None => {
            "none".hash(hasher);
        }
    }
}

fn hash_view_mask(mask: &CompiledViewMask, hasher: &mut impl Hasher) {
    match mask {
        CompiledViewMask::AllViews => "all_views".hash(hasher),
        CompiledViewMask::MainSurfaceOnly => "main_surface_only".hash(hasher),
        CompiledViewMask::OffscreenProductsOnly => "offscreen_products_only".hash(hasher),
        CompiledViewMask::Explicit(values) => {
            "explicit".hash(hasher);
            values.len().hash(hasher);
            for value in values {
                value.hash(hasher);
            }
        }
    }
}

fn hash_dispatch_plan(dispatch: Option<&CompiledDispatchPlan>, hasher: &mut impl Hasher) {
    match dispatch {
        Some(CompiledDispatchPlan::Fixed(value)) => {
            "fixed".hash(hasher);
            value.hash(hasher);
        }
        Some(CompiledDispatchPlan::FromState {
            state_type_name, ..
        }) => {
            "from_state".hash(hasher);
            state_type_name.hash(hasher);
        }
        None => {
            "none".hash(hasher);
        }
    }
}

fn hash_storage_access(access: CompiledStorageAccess, hasher: &mut impl Hasher) {
    match access {
        CompiledStorageAccess::ReadOnly => "read_only",
        CompiledStorageAccess::WriteOnly => "write_only",
        CompiledStorageAccess::ReadWrite => "read_write",
    }
    .hash(hasher);
}

fn hash_builtin_import(value: CompiledBuiltinImport, hasher: &mut impl Hasher) {
    match value {
        CompiledBuiltinImport::SurfaceColor => "surface_color",
        CompiledBuiltinImport::SurfaceDepth => "surface_depth",
    }
    .hash(hasher);
}

fn hash_prepared_target_binding(binding: &PreparedTargetBinding, hasher: &mut impl Hasher) {
    match binding {
        PreparedTargetBinding::DynamicTexture(key) => {
            "dynamic_texture".hash(hasher);
            key.hash(hasher);
        }
        PreparedTargetBinding::SurfaceColor => "surface_color".hash(hasher),
        PreparedTargetBinding::SurfaceDepth => "surface_depth".hash(hasher),
        PreparedTargetBinding::FlowOwned(resource_id) => {
            "flow_owned".hash(hasher);
            resource_id.hash(hasher);
        }
    }
}

fn hash_resource_descriptor_kind(descriptor: &RenderResourceDeclaration, hasher: &mut impl Hasher) {
    match descriptor {
        RenderResourceDeclaration::Uniform(_) => "uniform_buffer",
        RenderResourceDeclaration::Storage(_) => "storage_buffer",
        RenderResourceDeclaration::Sampled(_) => "sampled_texture",
        RenderResourceDeclaration::StorageImage(_) => "storage_texture",
        RenderResourceDeclaration::ColorAttachment(_) => "color_target",
        RenderResourceDeclaration::DepthAttachment(_) => "depth_target",
        RenderResourceDeclaration::History(_) => "history_texture",
        RenderResourceDeclaration::TargetAlias(_) => "target_alias",
        RenderResourceDeclaration::ImportedTexture(_) => "imported_texture",
        RenderResourceDeclaration::ImportedBuffer(_) => "imported_buffer",
    }
    .hash(hasher);
}

fn view_kind_tag(kind: crate::plugins::render::PreparedViewKind) -> &'static str {
    match kind {
        crate::plugins::render::PreparedViewKind::MainSurface => "main_surface",
        crate::plugins::render::PreparedViewKind::OffscreenProduct => "offscreen_product",
    }
}

fn hash_with(f: impl FnOnce(&mut DefaultHasher)) -> u64 {
    let mut hasher = DefaultHasher::new();
    f(&mut hasher);
    hasher.finish()
}

fn is_texture_descriptor(descriptor: &RenderResourceDeclaration) -> bool {
    matches!(
        descriptor,
        RenderResourceDeclaration::Sampled(_)
            | RenderResourceDeclaration::StorageImage(_)
            | RenderResourceDeclaration::ColorAttachment(_)
            | RenderResourceDeclaration::DepthAttachment(_)
            | RenderResourceDeclaration::History(_)
            | RenderResourceDeclaration::TargetAlias(_)
            | RenderResourceDeclaration::ImportedTexture(_)
    )
}

fn is_buffer_descriptor(descriptor: &RenderResourceDeclaration) -> bool {
    matches!(
        descriptor,
        RenderResourceDeclaration::Uniform(_)
            | RenderResourceDeclaration::Storage(_)
            | RenderResourceDeclaration::ImportedBuffer(_)
    )
}

#[allow(dead_code)]
fn imported_builtin_label(value: CompiledBuiltinImport) -> &'static str {
    match value {
        CompiledBuiltinImport::SurfaceColor => "surface.color",
        CompiledBuiltinImport::SurfaceDepth => "surface.depth",
    }
}
