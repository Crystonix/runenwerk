use super::super::dynamic_targets::RendererPreparedDynamicTextureUploadBatch;
use super::*;
use super::{
    canonical_work::{
        CanonicalFrameResolution, CanonicalInvocationProjection, CanonicalInvocationResolution,
        CanonicalPassProjection, RealizedLogicalBufferUpload, allocate_aux_occurrence,
        resolve_canonical_frame, resolve_canonical_invocation,
    },
    logical_operations::project_buffer_upload,
    logical_timing::LogicalGpuPassTimingPlan,
    occurrences::expand_render_pass_occurrences_in_frame,
};
use crate::plugins::render::{
    RenderGpuWorkOccurrenceId, RenderPassId, ResolvedRenderGpuWorkNode,
    prepare_render_gpu_frame_work,
};
use runen_gpu::{GpuPresentOperation, GpuResourceLabel, GpuTextureHandle, GpuTextureViewHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeaturePassAction {
    Execute,
    Skip,
}

/// Renderer-local handoff between the G4C1/G4C2/G4C3 realization phase and the G5 operation
/// phase. It holds no raw device or queue reference.
struct RendererRealizationBatch<'a> {
    packet: RendererPreparedPacket,
    dynamic_texture_uploads: RendererPreparedDynamicTextureUploadBatch,
    capture_runtime: FrameCaptureRuntime,
    invocations: Vec<RealizedFlowInvocation<'a>>,
    final_captures: Vec<PreparedCaptureReadback>,
    maximum_occurrence: u64,
}

struct RealizedFlowInvocation<'a> {
    flow: &'a CompiledRenderFlowPlan,
    invocation: &'a crate::plugins::render::PreparedFlowInvocation,
    packet: RendererPreparedPacket,
    scheduled_passes: Vec<RealizedScheduledPass<'a>>,
    timing_frame: Option<GpuPassTimingFrame>,
    /// Owned semantic authority retained through realization and consumed by the frame graph.
    canonical_resolution: Option<CanonicalInvocationResolution>,
}

struct RealizedScheduledPass<'a> {
    occurrence: RenderGpuWorkOccurrenceId,
    control_order_after: Vec<RenderGpuWorkOccurrenceId>,
    fixed_step_upload: Option<RealizedLogicalBufferUpload>,
    execution: RealizedPassExecution<'a>,
}

/// One actual render-domain execution occurrence after view/feature/fixed-step control has been
/// resolved. G5A deliberately resolves those decisions before late logical GPU work is formed, so
/// this value never represents a skipped pass or a ghost fixed-step mutation.
struct RealizedPassExecution<'a> {
    pass: &'a CompiledPassExecutionPlan,
    timestamp_indices: Option<GpuPassTimestampIndices>,
    pipeline: Option<PreparedPipelinePass>,
    before_captures: Vec<PreparedCaptureReadback>,
    after_captures: Vec<PreparedCaptureReadback>,
}

impl Renderer {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_packet(
        &mut self,
        context: &GpuContext,
        surface_texture: &GpuTextureHandle,
        surface_view: &GpuTextureViewHandle,
        acquired_surface_extent: (u32, u32),
        prepared_frame: &PreparedRenderFrame,
        packet: RendererPreparedPacket,
        compiled_flows: &[CompiledRenderFlowPlan],
        shader_registry: &ShaderRegistryResource,
        preflight_config: crate::plugins::render::graph::RenderPreflightValidationConfigResource,
        debug_control: &RenderDebugControlResource,
        debug_config: &RenderDebugConfigResource,
        gpu_timing_capability: RenderGpuTimingCapability,
    ) -> Result<RendererFrameTimings> {
        let mut timings = packet.prepare_timings;
        self.last_pass_timings.clear();
        self.last_runtime_resources.clear();
        self.last_pass_provenance.clear();

        let preflight_start = Instant::now();
        self.last_preflight_report =
            self.preflight_prepared_frame(prepared_frame, compiled_flows, preflight_config)?;
        timings.preflight_ms = preflight_start.elapsed().as_secs_f32() * 1000.0;

        let flow_encode_start = Instant::now();
        // Phase one: all G4C1/G4C2/G4C3 realization plus renderer-owned logical G5 operation
        // formation completes without a raw device/queue loan.
        let mut batch = self.realize_render_batch(
            context,
            surface_texture,
            surface_view,
            acquired_surface_extent,
            prepared_frame,
            packet,
            compiled_flows,
            shader_registry,
            debug_control,
            debug_config,
            gpu_timing_capability,
        )?;

        let canonical_resolutions = batch
            .invocations
            .iter_mut()
            .map(|invocation| {
                let resolution = invocation.canonical_resolution.take().ok_or_else(|| {
                    anyhow::anyhow!(
                        "flow '{}' invocation '{}' lost canonical resolution before frame preparation",
                        invocation.flow.flow_id,
                        invocation.invocation.invocation_id.0
                    )
                })?;
                Ok(resolution)
            })
            .collect::<Result<Vec<_>>>()?;
        let CanonicalFrameResolution::Resolved(mut frame) =
            resolve_canonical_frame(canonical_resolutions)
        else {
            bail!("normal renderer frame retained a residual non-canonical GPU operation");
        };

        let mut nodes = Vec::new();
        for operation in std::mem::take(&mut batch.packet.pending_operations).into_operations() {
            let occurrence = allocate_aux_occurrence(&mut batch.maximum_occurrence)?;
            nodes.push(ResolvedRenderGpuWorkNode::upload(
                occurrence,
                GpuResourceLabel::new(format!("render.frame.pending-upload.{}", occurrence.raw()))?,
                operation,
                [],
            ));
        }
        let (accepted_dynamic_uploads, dynamic_upload_report) = self
            .dynamic_texture_targets
            .validate_prepared_uploads(std::mem::take(&mut batch.dynamic_texture_uploads));
        for diagnostic in &dynamic_upload_report.diagnostics {
            tracing::warn!(
                target = "renderer.dynamic_texture_upload",
                target_key = %diagnostic.target_key,
                message = %diagnostic.message,
                "dynamic texture upload rejected before frame submission"
            );
        }
        for upload in &accepted_dynamic_uploads {
            let occurrence = allocate_aux_occurrence(&mut batch.maximum_occurrence)?;
            nodes.push(ResolvedRenderGpuWorkNode::upload(
                occurrence,
                GpuResourceLabel::new(format!("render.frame.dynamic-upload.{}", occurrence.raw()))?,
                upload.operation().clone(),
                [],
            ));
        }
        nodes.append(&mut frame.nodes);

        let mut terminal_controls =
            resolve_terminal_present_controls(frame.terminal_present_controls)?;
        if !batch.final_captures.is_empty() {
            let mut final_capture_occurrences = Vec::with_capacity(batch.final_captures.len());
            for capture in &batch.final_captures {
                let occurrence = allocate_aux_occurrence(&mut batch.maximum_occurrence)?;
                nodes.push(ResolvedRenderGpuWorkNode::capture_readback(
                    occurrence,
                    GpuResourceLabel::new(format!(
                        "render.frame.final-capture.{}",
                        occurrence.raw()
                    ))?,
                    capture.canonical_operation().clone(),
                    terminal_controls.iter().copied(),
                ));
                final_capture_occurrences.push(occurrence);
            }
            terminal_controls = final_capture_occurrences;
        }
        let present_occurrence = allocate_aux_occurrence(&mut batch.maximum_occurrence)?;
        let present = GpuPresentOperation::new(
            surface_view.clone().into(),
            surface_view.descriptor().subresources(),
        )?;
        nodes.push(ResolvedRenderGpuWorkNode::present(
            present_occurrence,
            GpuResourceLabel::new(format!("render.frame.present.{}", present_occurrence.raw()))?,
            present,
            terminal_controls,
        ));

        let encode_submit_start = Instant::now();
        let _span = tracing::info_span!("renderer.prepare_submit").entered();
        let graph = prepare_render_gpu_frame_work(
            GpuResourceLabel::new(format!(
                "render.frame.{}.surface.{}",
                prepared_frame.context.frame_index,
                prepared_frame.surface.render_surface_id.raw()
            ))?,
            nodes,
        )?;
        let prepared = pollster::block_on(context.prepare_submission(graph))?;
        let submission = context.submit_prepared(prepared).map_err(|rejection| {
            anyhow::anyhow!(
                "GPU frame submission rejected ({:?}): {}",
                rejection.reason().kind(),
                rejection.reason().detail()
            )
        })?;
        // Once G5 accepts the submission, retain every renderer-observed readback before any
        // fallible product evidence work. An accepted lifecycle handle must never be dropped merely
        // because later provenance publication fails for this frame.
        let timing_frames = batch
            .invocations
            .iter_mut()
            .filter_map(|invocation| invocation.timing_frame.take())
            .collect::<Vec<_>>();
        let mut capture_readbacks = batch
            .invocations
            .iter_mut()
            .flat_map(|invocation| {
                invocation.scheduled_passes.iter_mut().flat_map(|pass| {
                    let execution = &mut pass.execution;
                    execution
                        .before_captures
                        .drain(..)
                        .chain(execution.after_captures.drain(..))
                })
            })
            .collect::<Vec<_>>();
        capture_readbacks.append(&mut batch.final_captures);
        let RendererGpuObservationOutput {
            timing_evidence,
            captured_textures,
            capture_results,
        } = self.gpu_observations.accept(
            context,
            submission,
            timing_frames,
            capture_readbacks,
            &mut batch.capture_runtime,
        );
        self.pending_gpu_observation_output
            .timing_evidence
            .extend(timing_evidence);
        self.pending_gpu_observation_output
            .captured_textures
            .extend(captured_textures);
        self.pending_gpu_observation_output
            .capture_results
            .extend(capture_results);

        let accepted_upload_report = self
            .dynamic_texture_targets
            .record_accepted_uploads(&accepted_dynamic_uploads);
        for diagnostic in &accepted_upload_report.diagnostics {
            tracing::warn!(
                target = "renderer.dynamic_texture_upload",
                target_key = %diagnostic.target_key,
                message = %diagnostic.message,
                "accepted dynamic texture upload bookkeeping rejected"
            );
        }

        if debug_control.provenance_enabled {
            let mut runtime_cache = std::mem::take(&mut self.flow_runtime_cache);
            for invocation in &batch.invocations {
                let runtime_resources = runtime_cache
                    .get_mut(&invocation.flow.flow_id)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "flow '{}' lost runtime resources before accepted provenance publication",
                            invocation.flow.flow_id
                        )
                    })?;
                runtime_resources.target_alias_bindings =
                    invocation.invocation.target_alias_bindings.clone();
                runtime_resources.set_active_invocation_uniform_scope(
                    invocation.invocation.invocation_id.0.clone(),
                );
                for scheduled in &invocation.scheduled_passes {
                    let pass = scheduled.execution.pass;
                    let pipeline = scheduled.execution.pipeline.as_ref();
                    let evidence = EncodedPassEvidence {
                        shader_id: pipeline
                            .map(|prepared| prepared.shader_id.clone())
                            .unwrap_or_else(|| {
                                format!("builtin:{}", execution_pass_kind_name(pass))
                            }),
                        shader_revision: pipeline
                            .map(|prepared| prepared.shader_revision)
                            .unwrap_or(0),
                        fallback_used: pipeline
                            .map(|prepared| prepared.fallback_used)
                            .unwrap_or(false),
                        pipeline_key: pipeline
                            .map(|prepared| prepared.bindings.pipeline_key.clone()),
                    };
                    self.last_pass_provenance.push(accepted_pass_provenance(
                        prepared_frame.context.frame_index,
                        invocation.flow,
                        &invocation.packet,
                        pass,
                        runtime_resources,
                        &evidence,
                    ));
                }
                runtime_resources.clear_active_invocation_uniform_scope();
            }
            self.flow_runtime_cache = runtime_cache;
        }

        timings.flow_encode_ms = flow_encode_start.elapsed().as_secs_f32() * 1000.0;
        timings.encode_submit_ms = encode_submit_start.elapsed().as_secs_f32() * 1000.0;
        batch.capture_runtime.finalize_unresolved();
        let (capture_plan, capture_selector_results) =
            batch.capture_runtime.into_plan_and_results();
        self.last_capture_plan = capture_plan;
        self.last_capture_selector_results
            .extend(capture_selector_results);
        Ok(timings)
    }

    #[allow(clippy::too_many_arguments)]
    fn realize_render_batch<'a>(
        &mut self,
        context: &GpuContext,
        surface_texture: &GpuTextureHandle,
        surface_view: &GpuTextureViewHandle,
        acquired_surface_extent: (u32, u32),
        prepared_frame: &'a PreparedRenderFrame,
        mut packet: RendererPreparedPacket,
        compiled_flows: &'a [CompiledRenderFlowPlan],
        shader_registry: &ShaderRegistryResource,
        debug_control: &RenderDebugControlResource,
        debug_config: &RenderDebugConfigResource,
        gpu_timing_capability: RenderGpuTimingCapability,
    ) -> Result<RendererRealizationBatch<'a>> {
        let dynamic_target_history_signatures =
            prepared_frame.dynamic_target_history_signatures()?;
        self.dynamic_texture_targets.realize_for_frame(
            context,
            &prepared_frame.dynamic_texture_targets,
            &dynamic_target_history_signatures,
        )?;
        let dynamic_texture_uploads = self
            .dynamic_texture_targets
            .prepare_uploads(&prepared_frame.dynamic_texture_uploads);
        let (viewport, product_surface) = self.realize_ui_dynamic_bind_groups(
            &packet.prepared_ui,
            &packet.viewport_surface_bindings,
        )?;
        packet.ui_dynamic_bind_groups = UiDynamicBindGroups {
            viewport,
            product_surface,
        };
        let builtin_ui_draws = self.lower_ui_draws(
            &packet.prepared_ui,
            &packet.viewport_surface_bindings,
            &packet.ui_dynamic_bind_groups.viewport,
            &packet.ui_dynamic_bind_groups.product_surface,
            acquired_surface_extent,
        )?;

        let frame_index = prepared_frame.context.frame_index;
        let mut capture_runtime =
            FrameCaptureRuntime::new(frame_index, debug_control, &debug_config.capture_selectors);
        let mut flow_runtime_cache = std::mem::take(&mut self.flow_runtime_cache);
        let realization_result = (|| -> Result<(Vec<RealizedFlowInvocation<'a>>, u64)> {
            let active_flow_ids = compiled_flows
                .iter()
                .map(|flow| flow.flow_id)
                .collect::<Vec<_>>();
            flow_runtime_cache.retain(|flow_id, _| active_flow_ids.contains(flow_id));
            self.flow_pipeline_cache.retain_flows(&active_flow_ids);

            // Reserve every ordinary pass occurrence in one frame-owned identity space before any
            // projected-uniform, fixed-step, or timing-tail auxiliary occurrence is allocated.
            // The resulting schedules are then consumed in the same flow/invocation order while
            // each invocation's mutable runtime-resource scope is active.
            let mut maximum_occurrence = 0_u64;
            let mut scheduled_invocations = std::collections::VecDeque::new();
            for flow in compiled_flows {
                for invocation in prepared_frame.flow_invocations_for_flow(flow.flow_id) {
                    let Some(view) = prepared_frame.view(invocation.view_id.as_str()) else {
                        bail!(
                            "prepared flow invocation '{}' references missing view '{}'",
                            invocation.invocation_id.0,
                            invocation.view_id
                        );
                    };
                    let mut invocation_packet = packet.clone();
                    invocation_packet.pending_operations = RendererPendingOperations::default();
                    invocation_packet.view_id = view.view_id.clone();
                    invocation_packet.surface_size = view.target_size_px;
                    let occurrences = expand_render_pass_occurrences_in_frame(
                        flow,
                        &invocation.inputs,
                        &mut maximum_occurrence,
                        |pass| {
                            if !self.pass_targets_active_view(
                                pass,
                                view.view_id.as_str(),
                                view.kind,
                            ) {
                                return Ok(false);
                            }
                            let pass_id = execution_pass_id(pass);
                            if let Some(feature_id) = execution_pass_feature_id(pass)
                                && self.resolve_feature_pass_action(
                                    feature_id,
                                    pass_id,
                                    &invocation_packet,
                                )? == FeaturePassAction::Skip
                            {
                                return Ok(false);
                            }
                            ensure_compiled_pass_is_supported(pass)?;
                            Ok(true)
                        },
                    )?;
                    scheduled_invocations.push_back((
                        flow.flow_id.to_string(),
                        invocation.invocation_id.0.clone(),
                        invocation_packet,
                        occurrences,
                    ));
                }
            }

            let mut invocations = Vec::new();
            for flow in compiled_flows {
                let runtime_resources = flow_runtime_cache.entry(flow.flow_id).or_default();
                runtime_resources.realize_for_frame(
                    context,
                    flow,
                    packet.surface_size,
                    packet.surface_format,
                )?;
                let invocation_ids = prepared_frame
                    .flow_invocations_for_flow(flow.flow_id)
                    .map(|invocation| invocation.invocation_id.0.as_str())
                    .collect::<Vec<_>>();
                runtime_resources.retain_invocation_uniform_scopes(invocation_ids);

                for invocation in prepared_frame.flow_invocations_for_flow(flow.flow_id) {
                    let Some((
                        scheduled_flow_id,
                        scheduled_invocation_id,
                        invocation_packet,
                        occurrences,
                    )) = scheduled_invocations.pop_front()
                    else {
                        bail!(
                            "frame occurrence reservation is missing flow '{}' invocation '{}'",
                            flow.flow_id,
                            invocation.invocation_id.0
                        );
                    };
                    if scheduled_flow_id != flow.flow_id.to_string()
                        || scheduled_invocation_id.as_str() != invocation.invocation_id.0.as_str()
                    {
                        bail!(
                            "frame occurrence reservation order mismatch: expected flow '{}' invocation '{}', found flow '{}' invocation '{}'",
                            flow.flow_id,
                            invocation.invocation_id.0,
                            scheduled_flow_id,
                            scheduled_invocation_id
                        );
                    }
                    let Some(view) = prepared_frame.view(invocation.view_id.as_str()) else {
                        bail!(
                            "prepared flow invocation '{}' references missing view '{}'",
                            invocation.invocation_id.0,
                            invocation.view_id
                        );
                    };
                    runtime_resources.target_alias_bindings =
                        invocation.target_alias_bindings.clone();
                    runtime_resources
                        .set_active_invocation_uniform_scope(invocation.invocation_id.0.clone());
                    let effective_history_signature = invocation
                        .history_signature
                        .as_deref()
                        .or(view.history_signature.as_deref());

                    let invocation_result = (|| -> Result<RealizedFlowInvocation<'a>> {
                        runtime_resources.realize_invocation_history_textures(
                            invocation.invocation_id.0.as_str(),
                            invocation_packet.surface_size,
                            invocation_packet.surface_format,
                            effective_history_signature,
                        )?;

                        let projected_uploads = self.realize_projected_uniform_uploads(
                            context,
                            flow,
                            invocation.invocation_id.0.as_str(),
                            &invocation.inputs,
                            runtime_resources,
                            &mut maximum_occurrence,
                        )?;

                        let logical_timing_plan =
                            if gpu_timing_capability == RenderGpuTimingCapability::Supported {
                                Some(LogicalGpuPassTimingPlan::new(
                                    occurrences.iter().map(|occurrence| occurrence.pass),
                                )?)
                            } else {
                                None
                            };
                        let mut timing_frame = match logical_timing_plan
                            .as_ref()
                            .and_then(LogicalGpuPassTimingPlan::timing)
                        {
                            Some(timing) => Some(GpuPassTimingFrame::new(
                                context,
                                timing.query_set(),
                                timing.resolve_buffer(),
                                timing.readback_id(),
                                timing.query_capacity(),
                            )?),
                            None => None,
                        };
                        let mut realized_passes = Vec::new();
                        for (ordinal, occurrence) in occurrences.into_iter().enumerate() {
                            let fixed_step_upload = occurrence
                                .fixed_step_iteration
                                .map(|iteration| {
                                    self.realize_fixed_step_iteration_upload(
                                        context,
                                        invocation.invocation_id.0.as_str(),
                                        runtime_resources,
                                        iteration.region,
                                        iteration
                                            .schedule
                                            .with_substep_index(iteration.substep_index),
                                        &mut maximum_occurrence,
                                        occurrence.control_order_after.clone(),
                                    )
                                })
                                .transpose()?;
                            let pass = occurrence.pass;
                            let mut before_captures = Vec::new();
                            if capture_runtime.should_attempt_stage(CaptureStage::Before) {
                                self.prepare_pass_texture_captures(
                                    context,
                                    surface_texture,
                                    acquired_surface_extent,
                                    &invocation_packet,
                                    flow,
                                    pass,
                                    runtime_resources,
                                    CaptureStage::Before,
                                    &mut capture_runtime,
                                    &mut before_captures,
                                )?;
                            }
                            let pipeline = self.realize_compiled_pass(
                                context,
                                &invocation_packet,
                                flow,
                                &invocation.inputs,
                                pass,
                                shader_registry,
                                runtime_resources,
                            )?;
                            let mut after_captures = Vec::new();
                            if capture_runtime.should_attempt_stage(CaptureStage::After) {
                                self.prepare_pass_texture_captures(
                                    context,
                                    surface_texture,
                                    acquired_surface_extent,
                                    &invocation_packet,
                                    flow,
                                    pass,
                                    runtime_resources,
                                    CaptureStage::After,
                                    &mut capture_runtime,
                                    &mut after_captures,
                                )?;
                            }
                            let timestamp_indices = logical_timing_plan
                                .as_ref()
                                .map(|plan| plan.range_for_occurrence(ordinal))
                                .transpose()?
                                .flatten();
                            if let Some(indices) = timestamp_indices {
                                let frame = timing_frame.as_mut().ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "timestampable pass '{}' has no realized timing resources",
                                        execution_pass_id(pass)
                                    )
                                })?;
                                if !frame.register_pass_metadata(
                                    indices,
                                    frame_index,
                                    prepared_frame.surface.render_surface_id.raw(),
                                    flow.flow_id.to_string(),
                                    execution_pass_id(pass).to_string(),
                                    execution_pass_kind_name(pass).to_string(),
                                ) {
                                    bail!(
                                        "renderer timing metadata for flow '{}' pass '{}' disagrees with its admitted query range",
                                        flow.flow_id,
                                        execution_pass_id(pass)
                                    );
                                }
                            }
                            realized_passes.push(RealizedScheduledPass {
                                occurrence: occurrence.occurrence_id,
                                control_order_after: occurrence.control_order_after,
                                fixed_step_upload,
                                execution: RealizedPassExecution {
                                    pass,
                                    timestamp_indices,
                                    pipeline,
                                    before_captures,
                                    after_captures,
                                },
                            });
                        }
                        let canonical_projections = realized_passes
                            .iter()
                            .map(|scheduled| CanonicalPassProjection {
                                occurrence: scheduled.occurrence,
                                control_order_after: &scheduled.control_order_after,
                                pass: scheduled.execution.pass,
                                pipeline: scheduled.execution.pipeline.as_ref(),
                                timestamp_indices: scheduled.execution.timestamp_indices,
                                fixed_step_upload: scheduled.fixed_step_upload.as_ref(),
                                before_captures: &scheduled.execution.before_captures,
                                after_captures: &scheduled.execution.after_captures,
                            })
                            .collect::<Vec<_>>();
                        let canonical_resolution = resolve_canonical_invocation(
                            flow,
                            &invocation.inputs,
                            runtime_resources,
                            Some(&self.dynamic_texture_targets),
                            CanonicalInvocationProjection {
                                projected_uploads: &projected_uploads,
                                passes: &canonical_projections,
                                surface_color_view: Some(surface_view),
                                builtin_ui_draws: Some(&builtin_ui_draws),
                                timing: logical_timing_plan
                                    .as_ref()
                                    .and_then(LogicalGpuPassTimingPlan::timing),
                            },
                            &mut maximum_occurrence,
                        )?;
                        Ok(RealizedFlowInvocation {
                            flow,
                            invocation,
                            packet: invocation_packet,
                            scheduled_passes: realized_passes,
                            timing_frame,
                            canonical_resolution: Some(canonical_resolution),
                        })
                    })();
                    runtime_resources.clear_active_invocation_uniform_scope();
                    invocations.push(invocation_result?);
                }
                self.last_runtime_resources
                    .extend(runtime_resources.inspect_entries(flow.flow_id));
            }
            if !scheduled_invocations.is_empty() {
                bail!(
                    "frame occurrence reservation retained {} unconsumed invocation schedules",
                    scheduled_invocations.len()
                );
            }
            Ok((invocations, maximum_occurrence))
        })();
        self.flow_runtime_cache = flow_runtime_cache;
        let (invocations, maximum_occurrence) = realization_result?;
        let mut final_captures = Vec::new();
        if capture_runtime.should_attempt_stage(CaptureStage::Final) {
            self.prepare_final_surface_capture(
                context,
                surface_texture,
                acquired_surface_extent,
                &packet,
                &mut capture_runtime,
                &mut final_captures,
            )?;
        }
        Ok(RendererRealizationBatch {
            packet,
            dynamic_texture_uploads,
            capture_runtime,
            invocations,
            final_captures,
            maximum_occurrence,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        context: &GpuContext,
        surface_texture: &GpuTextureHandle,
        surface_view: &GpuTextureViewHandle,
        acquired_surface_extent: (u32, u32),
        prepared_frame: &PreparedRenderFrame,
        shader_registry: &mut ShaderRegistryResource,
        compiled_flows: &[CompiledRenderFlowPlan],
        ui_rect_shader: Option<ShaderHandle>,
        ui_font_atlas: &UiFontAtlasResource,
        viewport_surface_bindings: &ViewportSurfaceBindingRegistry,
        surface_format: GpuTextureFormat,
        preflight_config: crate::plugins::render::graph::RenderPreflightValidationConfigResource,
        debug_control: &RenderDebugControlResource,
        debug_config: &RenderDebugConfigResource,
        gpu_timing_capability: RenderGpuTimingCapability,
    ) -> Result<RendererFrameTimings> {
        let packet = self.prepare_packet(
            context,
            prepared_frame,
            shader_registry,
            ui_rect_shader,
            ui_font_atlas,
            viewport_surface_bindings,
            surface_format,
        )?;
        self.render_packet(
            context,
            surface_texture,
            surface_view,
            acquired_surface_extent,
            prepared_frame,
            packet,
            compiled_flows,
            shader_registry,
            preflight_config,
            debug_control,
            debug_config,
            gpu_timing_capability,
        )
    }

    fn realize_projected_uniform_uploads(
        &self,
        _context: &GpuContext,
        flow: &CompiledRenderFlowPlan,
        invocation_id: &str,
        flow_inputs: &PreparedFlowInputs,
        runtime_resources: &mut FlowRuntimeResources,
        maximum_occurrence: &mut u64,
    ) -> Result<Vec<RealizedLogicalBufferUpload>> {
        let mut uploads = Vec::new();
        for (buffer_id, bytes) in &flow_inputs.projected_uniform_bytes {
            if flow
                .execution
                .fixed_step_regions
                .iter()
                .any(|region| region.iteration_uniform == *buffer_id)
            {
                continue;
            }
            let prepared = runtime_resources.prepare_uniform_upload(*buffer_id, bytes)?;
            let runtime_buffer = runtime_resources.realize_invocation_uniform_buffer(
                invocation_id,
                *buffer_id,
                prepared.layout().byte_len(),
            )?;
            if prepared.layout().byte_len() > runtime_buffer.size {
                bail!(
                    "uniform upload for '{}' in invocation '{}' writes {} bytes but runtime buffer size is {}",
                    buffer_id,
                    invocation_id,
                    prepared.layout().byte_len(),
                    runtime_buffer.size
                );
            }
            let operation = project_buffer_upload(&runtime_buffer.handle, prepared.as_bytes())?;
            uploads.push(RealizedLogicalBufferUpload {
                occurrence: allocate_aux_occurrence(maximum_occurrence)?,
                operation,
                control_order_after: Vec::new(),
            });
        }

        Ok(uploads)
    }

    #[allow(clippy::too_many_arguments)]
    fn realize_fixed_step_iteration_upload(
        &self,
        _context: &GpuContext,
        invocation_id: &str,
        runtime_resources: &mut FlowRuntimeResources,
        region: &CompiledFixedStepRegion,
        uniform: RenderFixedStepIterationUniform,
        maximum_occurrence: &mut u64,
        control_order_after: Vec<RenderGpuWorkOccurrenceId>,
    ) -> Result<RealizedLogicalBufferUpload> {
        let bytes = uniform.to_uniform_bytes();
        let prepared =
            runtime_resources.prepare_uniform_upload(region.iteration_uniform, &bytes)?;
        let runtime_buffer = runtime_resources.realize_invocation_uniform_buffer(
            invocation_id,
            region.iteration_uniform,
            prepared.layout().byte_len(),
        )?;
        if prepared.layout().byte_len() > runtime_buffer.size {
            bail!(
                "fixed-step iteration uniform upload for region '{}' in invocation '{}' writes {} bytes but runtime buffer size is {}",
                region.region_label,
                invocation_id,
                prepared.layout().byte_len(),
                runtime_buffer.size
            );
        }
        let operation = project_buffer_upload(&runtime_buffer.handle, prepared.as_bytes())?;
        Ok(RealizedLogicalBufferUpload {
            occurrence: allocate_aux_occurrence(maximum_occurrence)?,
            operation,
            control_order_after,
        })
    }

    pub(super) fn pass_targets_active_view(
        &self,
        pass: &CompiledPassExecutionPlan,
        view_id: &str,
        view_kind: crate::plugins::render::PreparedViewKind,
    ) -> bool {
        let view_mask = match pass {
            CompiledPassExecutionPlan::Compute(value) => &value.view_mask,
            CompiledPassExecutionPlan::Fullscreen(value) => &value.view_mask,
            CompiledPassExecutionPlan::Graphics(value) => &value.view_mask,
            CompiledPassExecutionPlan::Copy(value) => &value.view_mask,
            CompiledPassExecutionPlan::Present(value) => &value.view_mask,
            CompiledPassExecutionPlan::BuiltinUiComposite(value) => &value.view_mask,
        };
        view_mask.includes(view_id, view_kind)
    }

    pub(super) fn resolve_feature_pass_action(
        &self,
        feature_id: RenderFeatureId,
        pass_id: RenderPassId,
        packet: &RendererPreparedPacket,
    ) -> Result<FeaturePassAction> {
        let gate = packet
            .feature_gates
            .get(&feature_id)
            .copied()
            .unwrap_or_default();

        match gate.status {
            FeatureContributionStatus::Ready => Ok(FeaturePassAction::Execute),
            FeatureContributionStatus::Stale => match gate.fallback_policy {
                FeatureFallbackPolicy::FailFrame => {
                    bail!(
                        "feature '{:?}' is stale for pass '{}' and fallback policy is fail-frame",
                        feature_id,
                        pass_id
                    )
                }
                FeatureFallbackPolicy::SkipFeaturePasses => Ok(FeaturePassAction::Skip),
                FeatureFallbackPolicy::ReuseLastGood | FeatureFallbackPolicy::EmptyContribution => {
                    Ok(FeaturePassAction::Execute)
                }
            },
            FeatureContributionStatus::Disabled | FeatureContributionStatus::Missing => {
                match gate.fallback_policy {
                    FeatureFallbackPolicy::FailFrame => {
                        bail!(
                            "feature '{:?}' is {:?} for pass '{}' and fallback policy is fail-frame",
                            feature_id,
                            gate.status,
                            pass_id
                        )
                    }
                    FeatureFallbackPolicy::SkipFeaturePasses => Ok(FeaturePassAction::Skip),
                    FeatureFallbackPolicy::ReuseLastGood
                    | FeatureFallbackPolicy::EmptyContribution => Ok(FeaturePassAction::Execute),
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_pass_texture_captures(
        &mut self,
        context: &GpuContext,
        surface_texture: &GpuTextureHandle,
        acquired_surface_extent: (u32, u32),
        packet: &RendererPreparedPacket,
        flow: &CompiledRenderFlowPlan,
        pass: &CompiledPassExecutionPlan,
        runtime_resources: &FlowRuntimeResources,
        stage: CaptureStage,
        capture_runtime: &mut FrameCaptureRuntime,
        prepared_captures: &mut Vec<PreparedCaptureReadback>,
    ) -> Result<()> {
        let pass_id = execution_pass_id(pass);
        let pass_label = pass_id.to_string();

        for selector_index in 0..capture_runtime.selectors_len() {
            let Some((selector, terminal_is_set, existing_capture_point)) =
                capture_runtime.selector_snapshot(selector_index)
            else {
                continue;
            };
            if terminal_is_set || selector.stage != stage {
                continue;
            }
            let texture_class = runtime_resources
                .capture_texture_class(selector.resource_id.as_str(), selector.texture_class);
            let capture_point = RenderCapturePointIdentity {
                flow_id: flow.flow_id.to_string(),
                pass_id: pass_id.to_string(),
                stage,
                resource_id: selector.resource_id.clone(),
                texture_class,
            };
            if !selector.matches_point(&capture_point) {
                continue;
            }
            if let Some(existing_capture_point) = existing_capture_point
                && existing_capture_point != capture_point
            {
                capture_runtime.set_terminal_with_reason(
                    selector_index,
                    RenderCaptureTerminalCode::Unsupported,
                    "selector_multiple_matches",
                    format!(
                        "selector '{}' matched multiple capture points: '{}' and '{}'",
                        selector.describe(),
                        existing_capture_point.resource_id,
                        capture_point.resource_id,
                    ),
                );
                continue;
            }
            let identity = RenderCaptureIdentity {
                frame_index: capture_runtime.frame_index,
                pass_label: pass_label.clone(),
                capture_point: capture_point.clone(),
            };
            capture_runtime.set_matched_identity(selector_index, capture_point, identity.clone());

            let resolved_key =
                runtime_resources.resolve_resource_key_from_input(selector.resource_id.as_str());
            let resolved_texture = match resolved_key {
                Some(RuntimeResourceKey::SurfaceColor) => Ok(None),
                Some(RuntimeResourceKey::DynamicTexture(key)) => self
                    .dynamic_texture_targets
                    .texture_ref(pass_id, &key)
                    .map(Some),
                None => {
                    if let Some(key) =
                        crate::plugins::render::RenderDynamicTextureTargetKey::from_label(
                            selector.resource_id.as_str(),
                        )
                    {
                        self.dynamic_texture_targets
                            .texture_ref(pass_id, &key)
                            .map(Some)
                    } else if selector.resource_id == SURFACE_COLOR_RESOURCE_LABEL {
                        Ok(None)
                    } else {
                        runtime_resources
                            .resolve_texture_from_label_without_surface(
                                pass_label.as_str(),
                                selector.resource_id.as_str(),
                            )
                            .map(Some)
                    }
                }
                _ => runtime_resources
                    .resolve_texture_from_label_without_surface(
                        pass_label.as_str(),
                        selector.resource_id.as_str(),
                    )
                    .map(Some),
            };
            let resolved_texture = match resolved_texture {
                Ok(value) => value,
                Err(err) => {
                    let terminal = RenderCaptureTerminal::with_reason(
                        RenderCaptureTerminalCode::Skipped,
                        "texture_resolution_failed",
                        err.to_string(),
                    );
                    capture_runtime.set_terminal(selector_index, terminal.clone());
                    self.last_captured_textures.push(RenderCapturedTexture {
                        identity,
                        width: 0,
                        height: 0,
                        format: "unknown".to_string(),
                        bytes_rgba8: None,
                        terminal,
                    });
                    continue;
                }
            };

            let (capture_source, capture_size, capture_format) = match resolved_texture {
                None => (
                    CaptureTextureSource::Surface {
                        handle: surface_texture,
                    },
                    acquired_surface_extent,
                    packet.surface_format,
                ),
                Some(resolved) => {
                    let logical_view = resolved.view_handle.ok_or_else(|| {
                        anyhow::anyhow!(
                            "capture source '{}' has no logical texture view",
                            selector.resource_id
                        )
                    })?;
                    (
                        CaptureTextureSource::Logical {
                            handle: logical_view.descriptor().texture(),
                        },
                        resolved.size,
                        resolved.format,
                    )
                }
            };
            let readback_format = texture_readback_format(capture_format);
            let readback_format = match readback_format {
                Some(value) => value,
                None => {
                    let terminal = RenderCaptureTerminal::with_reason(
                        RenderCaptureTerminalCode::Unsupported,
                        "unsupported_readback_format",
                        format!(
                            "readback for format {:?} is not implemented yet",
                            capture_format
                        ),
                    );
                    capture_runtime.set_terminal(selector_index, terminal.clone());
                    self.last_captured_textures.push(RenderCapturedTexture {
                        identity,
                        width: capture_size.0,
                        height: capture_size.1,
                        format: format!("{:?}", capture_format),
                        bytes_rgba8: None,
                        terminal,
                    });
                    continue;
                }
            };

            match prepare_texture_capture_readback(
                context,
                selector_index,
                selector.clone(),
                identity,
                capture_source,
                capture_size,
                capture_format,
                readback_format,
            ) {
                Ok(prepared) => prepared_captures.push(prepared),
                Err(err) => {
                    let terminal = RenderCaptureTerminal::with_reason(
                        RenderCaptureTerminalCode::ReadbackFailed,
                        "enqueue_capture_copy_failed",
                        err.to_string(),
                    );
                    capture_runtime.set_terminal(selector_index, terminal.clone());
                    self.last_captured_textures.push(RenderCapturedTexture {
                        identity: RenderCaptureIdentity {
                            frame_index: capture_runtime.frame_index,
                            pass_label: pass_label.clone(),
                            capture_point: RenderCapturePointIdentity {
                                flow_id: flow.flow_id.to_string(),
                                pass_id: pass_id.to_string(),
                                stage,
                                resource_id: selector.resource_id.clone(),
                                texture_class,
                            },
                        },
                        width: capture_size.0,
                        height: capture_size.1,
                        format: format!("{:?}", capture_format),
                        bytes_rgba8: None,
                        terminal,
                    });
                }
            }
        }

        Ok(())
    }

    fn prepare_final_surface_capture(
        &mut self,
        context: &GpuContext,
        surface_texture: &GpuTextureHandle,
        acquired_surface_extent: (u32, u32),
        packet: &RendererPreparedPacket,
        capture_runtime: &mut FrameCaptureRuntime,
        prepared_captures: &mut Vec<PreparedCaptureReadback>,
    ) -> Result<()> {
        for selector_index in 0..capture_runtime.selectors_len() {
            let Some((selector, terminal_is_set, existing_capture_point)) =
                capture_runtime.selector_snapshot(selector_index)
            else {
                continue;
            };
            if terminal_is_set || selector.stage != CaptureStage::Final {
                continue;
            }
            let capture_point = RenderCapturePointIdentity {
                flow_id: "frame".to_string(),
                pass_id: "frame.final".to_string(),
                stage: CaptureStage::Final,
                resource_id: selector.resource_id.clone(),
                texture_class: selector.texture_class,
            };
            if !selector.matches_point(&capture_point) {
                continue;
            }
            if let Some(existing_capture_point) = existing_capture_point
                && existing_capture_point != capture_point
            {
                capture_runtime.set_terminal_with_reason(
                    selector_index,
                    RenderCaptureTerminalCode::Unsupported,
                    "selector_multiple_matches",
                    format!(
                        "selector '{}' matched multiple final-stage capture points",
                        selector.describe()
                    ),
                );
                continue;
            }
            let identity = RenderCaptureIdentity {
                frame_index: capture_runtime.frame_index,
                pass_label: "frame.final".to_string(),
                capture_point: capture_point.clone(),
            };
            capture_runtime.set_matched_identity(selector_index, capture_point, identity.clone());
            if selector.resource_id != SURFACE_COLOR_RESOURCE_LABEL {
                let terminal = RenderCaptureTerminal::with_reason(
                    RenderCaptureTerminalCode::Unsupported,
                    "final_stage_resource_unsupported",
                    "final-stage capture currently supports only surface.color".to_string(),
                );
                capture_runtime.set_terminal(selector_index, terminal.clone());
                self.last_captured_textures.push(RenderCapturedTexture {
                    identity,
                    width: acquired_surface_extent.0,
                    height: acquired_surface_extent.1,
                    format: format!("{:?}", packet.surface_format),
                    bytes_rgba8: None,
                    terminal,
                });
                continue;
            }

            let Some(readback_format) = texture_readback_format(packet.surface_format) else {
                let terminal = RenderCaptureTerminal::with_reason(
                    RenderCaptureTerminalCode::Unsupported,
                    "unsupported_final_readback_format",
                    format!(
                        "readback for format {:?} is not implemented yet",
                        packet.surface_format
                    ),
                );
                capture_runtime.set_terminal(selector_index, terminal.clone());
                self.last_captured_textures.push(RenderCapturedTexture {
                    identity,
                    width: acquired_surface_extent.0,
                    height: acquired_surface_extent.1,
                    format: format!("{:?}", packet.surface_format),
                    bytes_rgba8: None,
                    terminal,
                });
                continue;
            };

            match prepare_texture_capture_readback(
                context,
                selector_index,
                selector.clone(),
                identity,
                CaptureTextureSource::Surface {
                    handle: surface_texture,
                },
                acquired_surface_extent,
                packet.surface_format,
                readback_format,
            ) {
                Ok(prepared) => prepared_captures.push(prepared),
                Err(err) => {
                    let terminal = RenderCaptureTerminal::with_reason(
                        RenderCaptureTerminalCode::ReadbackFailed,
                        "enqueue_capture_copy_failed",
                        err.to_string(),
                    );
                    capture_runtime.set_terminal(selector_index, terminal.clone());
                    self.last_captured_textures.push(RenderCapturedTexture {
                        identity: RenderCaptureIdentity {
                            frame_index: capture_runtime.frame_index,
                            pass_label: "frame.final".to_string(),
                            capture_point: RenderCapturePointIdentity {
                                flow_id: "frame".to_string(),
                                pass_id: "frame.final".to_string(),
                                stage: CaptureStage::Final,
                                resource_id: SURFACE_COLOR_RESOURCE_LABEL.to_string(),
                                texture_class: selector.texture_class,
                            },
                        },
                        width: acquired_surface_extent.0,
                        height: acquired_surface_extent.1,
                        format: format!("{:?}", packet.surface_format),
                        bytes_rgba8: None,
                        terminal,
                    });
                }
            }
        }
        Ok(())
    }
}

fn resolve_terminal_present_controls(
    terminal_present_controls: Vec<Vec<RenderGpuWorkOccurrenceId>>,
) -> Result<Vec<RenderGpuWorkOccurrenceId>> {
    if terminal_present_controls.is_empty() {
        bail!("presenting normal frame resolved no compiled Present");
    }

    Ok(terminal_present_controls.into_iter().flatten().fold(
        Vec::new(),
        |mut controls, occurrence| {
            if !controls.contains(&occurrence) {
                controls.push(occurrence);
            }
            controls
        },
    ))
}

fn accepted_pass_provenance(
    frame_index: u64,
    flow: &CompiledRenderFlowPlan,
    packet: &RendererPreparedPacket,
    pass: &CompiledPassExecutionPlan,
    runtime_resources: &FlowRuntimeResources,
    evidence: &EncodedPassEvidence,
) -> RenderPassProvenanceRecord {
    let pass_label = execution_pass_id(pass).to_string();
    let pass_resource_truth = collect_pass_resource_truth(flow.flow_id, pass, runtime_resources);
    let material_binding = collect_pass_material_binding_evidence(packet, pass);
    RenderPassProvenanceRecord {
        frame_index,
        flow_id: flow.flow_id.to_string(),
        pass_id: pass_label.clone(),
        pass_label,
        pass_kind: execution_flow_pass_kind(pass),
        authoring_index: execution_pass_authoring_index(pass),
        feature_id: execution_pass_feature_id(pass).map(|id| id.to_string()),
        shader_id: evidence.shader_id.clone(),
        shader_revision: evidence.shader_revision,
        fallback_used: evidence.fallback_used,
        pipeline_stats_key: evidence
            .pipeline_key
            .as_ref()
            .map(FlowPassPipelineKey::stats_key)
            .unwrap_or_default(),
        bind_group_layout_signature_hash: evidence
            .pipeline_key
            .as_ref()
            .map(FlowPassPipelineKey::primary_bind_group_layout_diagnostic_hash)
            .unwrap_or_default(),
        material_specialization_fragment_hash: material_specialization_fragment_hash(
            packet,
            execution_pass_feature_id(pass),
        ),
        view_signature_hash: hash_view_signature(packet.view_id.as_str(), packet.surface_size),
        feature_runtime_version: feature_runtime_version(packet, execution_pass_feature_id(pass)),
        color_formats: evidence
            .pipeline_key
            .as_ref()
            .and_then(FlowPassPipelineKey::render_pipeline_state)
            .and_then(|state| state.fragment_output())
            .map(|output| {
                output
                    .color_targets()
                    .map(|target| target.format())
                    .collect()
            })
            .unwrap_or_default(),
        depth_format: evidence
            .pipeline_key
            .as_ref()
            .and_then(FlowPassPipelineKey::render_pipeline_state)
            .and_then(|state| state.depth_stencil())
            .map(|depth| depth.format()),
        sample_count: evidence
            .pipeline_key
            .as_ref()
            .and_then(FlowPassPipelineKey::render_pipeline_state)
            .map(|state| state.multisample().sample_count())
            .unwrap_or(1),
        primitive_topology: evidence
            .pipeline_key
            .as_ref()
            .and_then(FlowPassPipelineKey::render_pipeline_state)
            .map(|state| state.primitive().topology()),
        material_binding,
        render_targets: pass_resource_truth.render_targets,
        sampled_textures: pass_resource_truth.sampled_textures,
        storage_textures: pass_resource_truth.storage_textures,
        depth_targets: pass_resource_truth.depth_targets,
        capture_points_available: pass_resource_truth.capture_points_available,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presenting_frame_rejects_when_no_compiled_present_was_resolved() {
        let error = resolve_terminal_present_controls(Vec::new())
            .expect_err("a presenting frame without a compiled Present must be rejected");

        assert_eq!(
            error.to_string(),
            "presenting normal frame resolved no compiled Present"
        );
    }

    #[test]
    fn compiled_present_without_non_data_predecessors_is_valid() {
        let controls = resolve_terminal_present_controls(vec![Vec::new()])
            .expect("an empty inner control set still records a real compiled Present");

        assert!(controls.is_empty());
    }
}
