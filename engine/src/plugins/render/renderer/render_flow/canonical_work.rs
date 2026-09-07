use super::super::dynamic_targets::RendererDynamicTextureTargetCache;
use super::logical_copy::{
    ProjectedCopyOperation, project_copy_operation, project_present_copy_operation,
};
use super::logical_operations::{
    project_compute_operation, project_render_operation, project_timing_tail, timestamp_writes,
};
use super::logical_timing::LogicalGpuPassTiming;
use super::*;
use crate::plugins::render::{RenderGpuWorkOccurrenceId, ResolvedRenderGpuWorkNode};
use runen_gpu::{
    GpuAttachmentStore, GpuColorAttachmentLoad, GpuExecutionPreference, GpuRenderColorAttachment,
    GpuRenderDraw, GpuRenderOperation, GpuResourceLabel, GpuTextureViewHandle, GpuUploadOperation,
    GpuWorkOperation,
};
use std::collections::BTreeMap;

/// One execution-complete logical upload in renderer occurrence space.
pub(super) struct RealizedLogicalBufferUpload {
    pub(super) occurrence: RenderGpuWorkOccurrenceId,
    pub(super) operation: GpuUploadOperation,
    pub(super) control_order_after: Vec<RenderGpuWorkOccurrenceId>,
}

pub(super) struct CanonicalPassProjection<'a> {
    pub(super) occurrence: RenderGpuWorkOccurrenceId,
    pub(super) control_order_after: &'a [RenderGpuWorkOccurrenceId],
    pub(super) pass: &'a CompiledPassExecutionPlan,
    pub(super) pipeline: Option<&'a PreparedPipelinePass>,
    pub(super) timestamp_indices: Option<GpuPassTimestampIndices>,
    pub(super) fixed_step_upload: Option<&'a RealizedLogicalBufferUpload>,
    pub(super) before_captures: &'a [PreparedCaptureReadback],
    pub(super) after_captures: &'a [PreparedCaptureReadback],
}

/// One invocation's already-realized inputs for canonical GPU-node resolution.
///
/// This groups the execution-complete projection instead of growing the resolver into a bag of
/// unrelated parameters. G5C1 can therefore reuse the same boundary while moving graph preparation
/// from invocation scope to frame/surface scope. `surface_color_view` is the exact logical view of
/// the currently acquired G7A image when that authority has already moved to the frame caller.
/// `builtin_ui_draws` is only an execution-complete generic lowering of the current transitional
/// Runenwerk UI batches; it is not RunenUI or future RunenRender semantic authority.
#[derive(Clone, Copy)]
pub(super) struct CanonicalInvocationProjection<'a, 'pass> {
    pub(super) projected_uploads: &'a [RealizedLogicalBufferUpload],
    pub(super) passes: &'a [CanonicalPassProjection<'pass>],
    pub(super) surface_color_view: Option<&'a GpuTextureViewHandle>,
    pub(super) builtin_ui_draws: Option<&'a [GpuRenderDraw]>,
    pub(super) timing: Option<&'a LogicalGpuPassTiming>,
}

/// Execution-complete canonical work owned by one resolved renderer invocation.
///
/// This value deliberately contains no borrow of `FlowRuntimeResources`. The live frame caller can
/// therefore resolve an invocation while its mutable runtime-resource scope is active, retain this
/// owned result, and aggregate all invocation results only after that scope has moved on.
pub(super) struct CanonicalResolvedInvocation {
    nodes: Vec<ResolvedRenderGpuWorkNode>,
    /// One entry per terminal compiled Present omitted because its source was already
    /// `SurfaceColor`. Each inner vector is the render-owned non-data predecessor set that the
    /// eventual frame-terminal `GpuPresentOperation` must preserve.
    terminal_present_controls: Vec<Vec<RenderGpuWorkOccurrenceId>>,
}

pub(super) struct CanonicalResolvedFrame {
    pub(super) nodes: Vec<ResolvedRenderGpuWorkNode>,
    /// Aggregated terminal-Present predecessor sets. An empty inner vector still records a real
    /// omitted Present, allowing the transitional per-invocation path to remain residual.
    pub(super) terminal_present_controls: Vec<Vec<RenderGpuWorkOccurrenceId>>,
}

pub(super) enum CanonicalInvocationResolution {
    Resolved(CanonicalResolvedInvocation),
    /// The invocation contains at least one operation whose durable logical identity/semantics is
    /// intentionally deferred to G7A/G5C. No partial canonical node set is retained in this case.
    PreG7Residual,
}

pub(super) enum CanonicalFrameResolution {
    Resolved(CanonicalResolvedFrame),
    /// At least one invocation is not fully canonical yet. G5C1 requires the entire physical
    /// frame/surface submission to remain on the residual path rather than mixing authorities.
    PreG7Residual,
}

/// Aggregates already-resolved renderer invocations into one bounded frame/surface semantic result.
///
/// The caller must reserve ordinary pass occurrence IDs across the complete frame before resolving
/// the first invocation, then pass one shared `maximum_occurrence` through every
/// `resolve_canonical_invocation` call so upload/timing/capture auxiliary identities cannot collide
/// with a later invocation. Each invocation can then be resolved while its mutable
/// `FlowRuntimeResources` scope is active; this aggregator retains only owned generic work and
/// render-owned terminal control metadata. One residual invocation discards the complete canonical
/// frame result so the caller cannot mix legacy and G5 execution authority inside one physical
/// submission.
pub(super) fn resolve_canonical_frame(
    invocations: impl IntoIterator<Item = CanonicalInvocationResolution>,
) -> CanonicalFrameResolution {
    let mut nodes = Vec::<ResolvedRenderGpuWorkNode>::new();
    let mut terminal_present_controls = Vec::<Vec<RenderGpuWorkOccurrenceId>>::new();
    let mut saw_invocation = false;

    for invocation in invocations {
        saw_invocation = true;
        match invocation {
            CanonicalInvocationResolution::Resolved(mut resolved) => {
                nodes.append(&mut resolved.nodes);
                terminal_present_controls.append(&mut resolved.terminal_present_controls);
            }
            CanonicalInvocationResolution::PreG7Residual => {
                return CanonicalFrameResolution::PreG7Residual;
            }
        }
    }

    if !saw_invocation || (nodes.is_empty() && terminal_present_controls.is_empty()) {
        CanonicalFrameResolution::PreG7Residual
    } else {
        CanonicalFrameResolution::Resolved(CanonicalResolvedFrame {
            nodes,
            terminal_present_controls,
        })
    }
}

/// Resolves one renderer invocation into execution-complete canonical GPU occurrences without
/// preparing a G3 graph.
///
/// The caller owns occurrence allocation. For frame-level use it must reserve all ordinary pass
/// occurrence IDs first and then thread the same `maximum_occurrence` through every invocation so
/// auxiliary upload/timing/capture IDs stay frame-unique. Capture stage ordering is render-owned
/// control meaning: Before readbacks become predecessors of the target pass and After readbacks
/// become successors even when G3 derives no data hazard between them. True no-work occurrences are
/// omitted and their effective predecessors are forwarded to later render-owned controls; no fake
/// GPU node is authored only to preserve occurrence identity. If an operation is residual, no
/// partial canonical node set or terminal-Present control metadata from this invocation is retained.
pub(super) fn resolve_canonical_invocation(
    flow: &CompiledRenderFlowPlan,
    flow_inputs: &PreparedFlowInputs,
    runtime_resources: &FlowRuntimeResources,
    dynamic_texture_targets: Option<&RendererDynamicTextureTargetCache>,
    projection: CanonicalInvocationProjection<'_, '_>,
    maximum_occurrence: &mut u64,
) -> Result<CanonicalInvocationResolution> {
    let CanonicalInvocationProjection {
        projected_uploads,
        passes,
        surface_color_view,
        builtin_ui_draws,
        timing,
    } = projection;

    let mut nodes = Vec::<ResolvedRenderGpuWorkNode>::new();
    let mut terminal_present_controls = Vec::<Vec<RenderGpuWorkOccurrenceId>>::new();
    let mut omitted_controls =
        BTreeMap::<RenderGpuWorkOccurrenceId, Vec<RenderGpuWorkOccurrenceId>>::new();
    for upload in projected_uploads {
        *maximum_occurrence = (*maximum_occurrence).max(upload.occurrence.raw());
        nodes.push(ResolvedRenderGpuWorkNode::upload(
            upload.occurrence,
            occurrence_label(flow, "upload", upload.occurrence)?,
            upload.operation.clone(),
            upload.control_order_after.iter().copied(),
        ));
    }

    for projected in passes {
        *maximum_occurrence = (*maximum_occurrence).max(projected.occurrence.raw());
        let mut pass_control =
            effective_controls_after_omissions(projected.control_order_after, &omitted_controls);
        if let Some(upload) = projected.fixed_step_upload {
            *maximum_occurrence = (*maximum_occurrence).max(upload.occurrence.raw());
            nodes.push(ResolvedRenderGpuWorkNode::upload(
                upload.occurrence,
                occurrence_label(flow, "fixed-step-upload", upload.occurrence)?,
                upload.operation.clone(),
                effective_controls_after_omissions(&upload.control_order_after, &omitted_controls),
            ));
            pass_control.clear();
            pass_control.push(upload.occurrence);
        }

        let Some(before_capture_occurrences) = append_capture_readbacks(
            flow,
            projected.before_captures,
            &pass_control,
            maximum_occurrence,
            &mut nodes,
        )?
        else {
            return Ok(CanonicalInvocationResolution::PreG7Residual);
        };
        if !before_capture_occurrences.is_empty() {
            pass_control = before_capture_occurrences;
        }

        let operation = match projected.pass {
            CompiledPassExecutionPlan::Compute(pass) => {
                let pipeline = projected.pipeline.ok_or_else(|| {
                    anyhow::anyhow!(
                        "compute occurrence '{}' has no realized pipeline for canonical G5A work",
                        projected.occurrence
                    )
                })?;
                let timing = timestamp_projection(timing, projected.timestamp_indices)?;
                project_compute_operation(pass, flow_inputs, pipeline, timing)?
            }
            CompiledPassExecutionPlan::Fullscreen(_) | CompiledPassExecutionPlan::Graphics(_) => {
                let pipeline = projected.pipeline.ok_or_else(|| {
                    anyhow::anyhow!(
                        "render occurrence '{}' has no realized pipeline for canonical G5A work",
                        projected.occurrence
                    )
                })?;
                let timing = timestamp_projection(timing, projected.timestamp_indices)?;
                let Some(operation) = project_render_operation(
                    runtime_resources,
                    projected.pass,
                    pipeline,
                    surface_color_view,
                    timing,
                )?
                else {
                    return Ok(CanonicalInvocationResolution::PreG7Residual);
                };
                operation
            }
            CompiledPassExecutionPlan::Copy(pass) => {
                match project_copy_operation(runtime_resources, dynamic_texture_targets, pass)? {
                    ProjectedCopyOperation::Canonical(operation) => *operation,
                    ProjectedCopyOperation::NoWork => {
                        if !forward_no_work_occurrence(
                            flow,
                            projected,
                            &pass_control,
                            maximum_occurrence,
                            &mut nodes,
                            &mut omitted_controls,
                        )? {
                            return Ok(CanonicalInvocationResolution::PreG7Residual);
                        }
                        continue;
                    }
                    ProjectedCopyOperation::PreG7Residual => {
                        return Ok(CanonicalInvocationResolution::PreG7Residual);
                    }
                }
            }
            CompiledPassExecutionPlan::BuiltinUiComposite(_) => {
                let Some(surface_color_view) = surface_color_view else {
                    return Ok(CanonicalInvocationResolution::PreG7Residual);
                };
                let Some(draws) = builtin_ui_draws else {
                    return Ok(CanonicalInvocationResolution::PreG7Residual);
                };
                let timing = timestamp_projection(timing, projected.timestamp_indices)?;
                if draws.is_empty() && timing.is_none() {
                    if !forward_no_work_occurrence(
                        flow,
                        projected,
                        &pass_control,
                        maximum_occurrence,
                        &mut nodes,
                        &mut omitted_controls,
                    )? {
                        return Ok(CanonicalInvocationResolution::PreG7Residual);
                    }
                    continue;
                }
                let color_attachment = GpuRenderColorAttachment::new(
                    surface_color_view.clone(),
                    GpuColorAttachmentLoad::Load,
                    GpuAttachmentStore::Store,
                    None,
                )?;
                let timestamp_writes = timing
                    .map(|(timing, indices)| timestamp_writes(timing, indices))
                    .transpose()?;
                GpuWorkOperation::Render(GpuRenderOperation::new(
                    [color_attachment],
                    None,
                    draws.iter().cloned(),
                    timestamp_writes,
                )?)
            }
            CompiledPassExecutionPlan::Present(pass) => {
                match project_present_copy_operation(
                    runtime_resources,
                    dynamic_texture_targets,
                    pass,
                    surface_color_view,
                )? {
                    ProjectedCopyOperation::Canonical(operation) => *operation,
                    ProjectedCopyOperation::NoWork => {
                        let Some(after_capture_occurrences) = append_capture_readbacks(
                            flow,
                            projected.after_captures,
                            &pass_control,
                            maximum_occurrence,
                            &mut nodes,
                        )?
                        else {
                            return Ok(CanonicalInvocationResolution::PreG7Residual);
                        };
                        terminal_present_controls.push(if after_capture_occurrences.is_empty() {
                            pass_control
                        } else {
                            after_capture_occurrences
                        });
                        continue;
                    }
                    ProjectedCopyOperation::PreG7Residual => {
                        return Ok(CanonicalInvocationResolution::PreG7Residual);
                    }
                }
            }
        };

        let is_present_copy = matches!(projected.pass, CompiledPassExecutionPlan::Present(_));
        nodes.push(ResolvedRenderGpuWorkNode::pass(
            projected.occurrence,
            occurrence_label(flow, "pass", projected.occurrence)?,
            operation,
            execution_preference(projected.pass),
            pass_control,
        ));
        let Some(after_capture_occurrences) = append_capture_readbacks(
            flow,
            projected.after_captures,
            &[projected.occurrence],
            maximum_occurrence,
            &mut nodes,
        )?
        else {
            return Ok(CanonicalInvocationResolution::PreG7Residual);
        };
        if is_present_copy {
            terminal_present_controls.push(if after_capture_occurrences.is_empty() {
                vec![projected.occurrence]
            } else {
                after_capture_occurrences
            });
        }
    }

    if let Some(timing) = timing {
        let tail = project_timing_tail(
            timing.query_set(),
            timing.query_range()?,
            timing.resolve_buffer(),
            timing.readback_id(),
        )?;
        let resolve_occurrence = allocate_aux_occurrence(maximum_occurrence)?;
        nodes.push(ResolvedRenderGpuWorkNode::timing_resolve(
            resolve_occurrence,
            occurrence_label(flow, "timing-resolve", resolve_occurrence)?,
            tail.resolve().clone(),
            [],
        ));
        let readback_occurrence = allocate_aux_occurrence(maximum_occurrence)?;
        nodes.push(ResolvedRenderGpuWorkNode::timing_readback(
            readback_occurrence,
            occurrence_label(flow, "timing-readback", readback_occurrence)?,
            tail.readback().clone(),
            [],
        ));
    }

    Ok(CanonicalInvocationResolution::Resolved(
        CanonicalResolvedInvocation {
            nodes,
            terminal_present_controls,
        },
    ))
}

/// Replaces references to already-omitted no-work occurrences with their effective real
/// predecessors. Values stored in `omitted_controls` are themselves fully contracted when they are
/// recorded, so one deterministic substitution pass is sufficient and can never synthesize a GPU
/// dependency that was not render-owned before omission.
fn effective_controls_after_omissions(
    controls: &[RenderGpuWorkOccurrenceId],
    omitted_controls: &BTreeMap<RenderGpuWorkOccurrenceId, Vec<RenderGpuWorkOccurrenceId>>,
) -> Vec<RenderGpuWorkOccurrenceId> {
    let mut effective = Vec::new();
    for control in controls {
        if let Some(predecessors) = omitted_controls.get(control) {
            for predecessor in predecessors {
                if !effective.contains(predecessor) {
                    effective.push(*predecessor);
                }
            }
        } else if !effective.contains(control) {
            effective.push(*control);
        }
    }
    effective
}

/// Omits one true no-work pass occurrence while preserving all surrounding real work. Before-stage
/// captures and fixed-step uploads have already been folded into `pass_control`; any after-stage
/// captures are appended here and become the effective predecessors forwarded to later controls.
/// A pre-G7 surface capture still makes the invocation residual because its logical readback source
/// does not exist yet.
fn forward_no_work_occurrence(
    flow: &CompiledRenderFlowPlan,
    projected: &CanonicalPassProjection<'_>,
    pass_control: &[RenderGpuWorkOccurrenceId],
    maximum_occurrence: &mut u64,
    nodes: &mut Vec<ResolvedRenderGpuWorkNode>,
    omitted_controls: &mut BTreeMap<RenderGpuWorkOccurrenceId, Vec<RenderGpuWorkOccurrenceId>>,
) -> Result<bool> {
    let Some(after_capture_occurrences) = append_capture_readbacks(
        flow,
        projected.after_captures,
        pass_control,
        maximum_occurrence,
        nodes,
    )?
    else {
        return Ok(false);
    };
    let effective = if after_capture_occurrences.is_empty() {
        pass_control.to_vec()
    } else {
        after_capture_occurrences
    };
    omitted_controls.insert(projected.occurrence, effective);
    Ok(true)
}

fn append_capture_readbacks(
    flow: &CompiledRenderFlowPlan,
    captures: &[PreparedCaptureReadback],
    control_order_after: &[RenderGpuWorkOccurrenceId],
    maximum_occurrence: &mut u64,
    nodes: &mut Vec<ResolvedRenderGpuWorkNode>,
) -> Result<Option<Vec<RenderGpuWorkOccurrenceId>>> {
    let mut occurrences = Vec::with_capacity(captures.len());
    for capture in captures {
        let operation = capture.canonical_operation();
        let occurrence = allocate_aux_occurrence(maximum_occurrence)?;
        nodes.push(ResolvedRenderGpuWorkNode::capture_readback(
            occurrence,
            occurrence_label(flow, "capture-readback", occurrence)?,
            operation.clone(),
            control_order_after.iter().copied(),
        ));
        occurrences.push(occurrence);
    }
    Ok(Some(occurrences))
}

fn timestamp_projection(
    timing: Option<&LogicalGpuPassTiming>,
    indices: Option<GpuPassTimestampIndices>,
) -> Result<Option<(&LogicalGpuPassTiming, GpuPassTimestampIndices)>> {
    match (timing, indices) {
        (Some(timing), Some(indices)) => Ok(Some((timing, indices))),
        (None, None) => Ok(None),
        (Some(_), None) => anyhow::bail!(
            "timestampable canonical pass is missing its occurrence-local timestamp indices"
        ),
        (None, Some(_)) => anyhow::bail!(
            "canonical pass carries timestamp indices without logical timing resources"
        ),
    }
}

const fn execution_preference(pass: &CompiledPassExecutionPlan) -> GpuExecutionPreference {
    match pass {
        CompiledPassExecutionPlan::Compute(_) => GpuExecutionPreference::ComputePreferred,
        CompiledPassExecutionPlan::Fullscreen(_)
        | CompiledPassExecutionPlan::Graphics(_)
        | CompiledPassExecutionPlan::BuiltinUiComposite(_) => {
            GpuExecutionPreference::GraphicsRequired
        }
        CompiledPassExecutionPlan::Copy(_) | CompiledPassExecutionPlan::Present(_) => {
            GpuExecutionPreference::TransferPreferred
        }
    }
}

fn occurrence_label(
    flow: &CompiledRenderFlowPlan,
    kind: &str,
    occurrence: RenderGpuWorkOccurrenceId,
) -> Result<GpuResourceLabel> {
    Ok(GpuResourceLabel::new(format!(
        "{}.{}.{}",
        flow.flow_label,
        kind,
        occurrence.raw()
    ))?)
}

pub(super) fn allocate_aux_occurrence(
    maximum_occurrence: &mut u64,
) -> Result<RenderGpuWorkOccurrenceId> {
    *maximum_occurrence = maximum_occurrence.checked_add(1).ok_or_else(|| {
        anyhow::anyhow!("render GPU execution occurrence identity space is exhausted")
    })?;
    Ok(RenderGpuWorkOccurrenceId::new(*maximum_occurrence))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_occurrence_controls_contract_without_ghost_nodes() {
        let first = RenderGpuWorkOccurrenceId::new(1);
        let omitted_a = RenderGpuWorkOccurrenceId::new(2);
        let omitted_b = RenderGpuWorkOccurrenceId::new(3);
        let independent = RenderGpuWorkOccurrenceId::new(4);
        let mut omitted = BTreeMap::new();

        omitted.insert(omitted_a, vec![first]);
        let omitted_b_controls = effective_controls_after_omissions(&[omitted_a, first], &omitted);
        assert_eq!(omitted_b_controls, vec![first]);
        omitted.insert(omitted_b, omitted_b_controls);

        assert_eq!(
            effective_controls_after_omissions(
                &[omitted_b, independent, omitted_a, first],
                &omitted,
            ),
            vec![first, independent]
        );
    }
}
