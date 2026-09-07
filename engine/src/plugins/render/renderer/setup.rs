use super::resource_descriptors::{
    buffer_descriptor, linear_sampler_descriptor, texture_descriptor, whole_texture_view_descriptor,
};
use super::*;
use runen_gpu::{
    GpuBindingKey, GpuBindingLayoutRefinement, GpuBlendConstant, GpuBlendMode, GpuBufferRange,
    GpuBufferUsage, GpuColorTargetStateDescriptor, GpuColorWriteMask, GpuDrawIntent, GpuDrawRange,
    GpuEntryPointName, GpuFragmentOutputStateDescriptor, GpuMemoryIntent,
    GpuMultisampleStateDescriptor, GpuPipelineConfiguration, GpuPrimitiveStateDescriptor,
    GpuProgramDescriptor, GpuProgramSourceKey, GpuProgramSourceProvenance,
    GpuRealizedRenderPipeline, GpuRenderDraw, GpuRenderEntryPoints, GpuRenderPipelineDescriptor,
    GpuRenderPipelineStateDescriptor, GpuResourceLifetime, GpuRuntimeBindingResource,
    GpuRuntimeBindingSet, GpuRuntimeBindingValue, GpuRuntimeBufferBinding,
    GpuRuntimeTextureViewBinding, GpuSamplerClass, GpuScissorRect, GpuTextureFormat,
    GpuTextureSampleClass, GpuTextureUsage, GpuTextureViewDimension, GpuVertexAttribute,
    GpuVertexBufferBinding, GpuVertexBufferLayoutDescriptor, GpuVertexFormat,
    GpuVertexInputStateDescriptor, GpuVertexStepMode, GpuViewport,
};
use std::num::NonZeroU64;

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub(super) fn realize_buffer_resource(
        &mut self,
        _context: &GpuContext,
        label: &str,
        size: u64,
        usages: impl IntoIterator<Item = GpuBufferUsage>,
        lifetime: GpuResourceLifetime,
        memory_intent: GpuMemoryIntent,
    ) -> Result<RendererBufferResource> {
        let handle = self.resource_ids.allocate_buffer_handle(buffer_descriptor(
            label,
            size,
            usages,
            lifetime,
            memory_intent,
        )?)?;
        Ok(RendererBufferResource { _handle: handle })
    }

    fn realize_linear_sampler_resource(
        &mut self,
        _context: &GpuContext,
        label: &str,
        lifetime: GpuResourceLifetime,
    ) -> Result<RendererSamplerResource> {
        let handle = self
            .resource_ids
            .allocate_sampler_handle(linear_sampler_descriptor(label, lifetime)?)?;
        Ok(RendererSamplerResource { _handle: handle })
    }

    // Owner: Engine Renderer - UI Pipeline Setup and Encoding
    pub fn new() -> Self {
        Self {
            resource_ids: GpuWorkResourceIdAllocator::new(),
            rect_pass: None,
            rect_pass_format: None,
            rect_pass_shader_revision: 0,
            stroke_pass: None,
            stroke_pass_format: None,
            glyph_pass: None,
            glyph_pass_format: None,
            viewport_embed_pass: None,
            viewport_embed_pass_format: None,
            product_surface_pass: None,
            product_surface_pass_format: None,
            glyph_atlas_gpu: std::collections::BTreeMap::new(),
            dynamic_texture_targets:
                super::dynamic_targets::RendererDynamicTextureTargetCache::default(),
            flow_runtime_cache: std::collections::BTreeMap::new(),
            flow_pipeline_cache: super::pipeline_cache::FlowPipelineArtifactCache::default(),
            preflight_cache: None,
            last_good_ui_prepared: None,
            last_pass_timings: Vec::new(),
            last_gpu_pass_timing_evidence: Vec::new(),
            last_runtime_resources: Vec::new(),
            last_pass_provenance: Vec::new(),
            last_preflight_report:
                crate::plugins::render::graph::RenderExecutionGraphPreparedReport::default(),
            last_preflight_cache_state:
                crate::plugins::render::graph::RenderPreparedFramePreflightCacheState::default(),
            last_capture_plan: crate::plugins::render::inspect::ResolvedRenderCapturePlan::default(
            ),
            last_capture_selector_results: Vec::new(),
            last_captured_textures: Vec::new(),
            gpu_observations: super::render_flow::RendererGpuObservationState::default(),
            pending_gpu_observation_output:
                super::render_flow::RendererGpuObservationOutput::default(),
        }
    }

    pub(super) fn begin_frame_gpu_observation(&mut self, context: &GpuContext) {
        // Gfx owns one nonblocking progress point for its context/device generation. Timing and
        // capture consume the resulting public lifecycle facts; neither feature creates a poll
        // loop or reaches into the backend.
        context.progress();
        let super::render_flow::RendererGpuObservationOutput {
            timing_evidence,
            captured_textures,
            capture_results,
        } = self.gpu_observations.progress(context);
        self.pending_gpu_observation_output
            .timing_evidence
            .extend(timing_evidence);
        self.pending_gpu_observation_output
            .captured_textures
            .extend(captured_textures);
        self.pending_gpu_observation_output
            .capture_results
            .extend(capture_results);
    }

    pub(super) fn publish_progressed_gpu_observations(&mut self) {
        let mut progressed = std::mem::take(&mut self.pending_gpu_observation_output);
        progressed
            .timing_evidence
            .append(&mut self.last_gpu_pass_timing_evidence);
        progressed
            .captured_textures
            .append(&mut self.last_captured_textures);
        progressed
            .capture_results
            .append(&mut self.last_capture_selector_results);
        self.last_gpu_pass_timing_evidence = progressed.timing_evidence;
        self.last_captured_textures = progressed.captured_textures;
        self.last_capture_selector_results = progressed.capture_results;
    }

    pub(in crate::plugins::render) fn clear_published_gpu_observations(&mut self) {
        self.last_gpu_pass_timing_evidence.clear();
        self.last_capture_plan = ResolvedRenderCapturePlan::default();
        self.last_capture_selector_results.clear();
        self.last_captured_textures.clear();
    }

    pub fn last_pass_timings(&self) -> &[crate::plugins::render::inspect::PassTimingSample] {
        &self.last_pass_timings
    }

    pub fn last_gpu_pass_timing_evidence(
        &self,
    ) -> &[crate::plugins::render::inspect::RenderPassTimingEvidence] {
        &self.last_gpu_pass_timing_evidence
    }

    pub fn last_runtime_resources(
        &self,
    ) -> &[crate::plugins::render::inspect::RuntimeResourceInspectionEntry] {
        &self.last_runtime_resources
    }

    pub fn last_pass_provenance(
        &self,
    ) -> &[crate::plugins::render::inspect::RenderPassProvenanceRecord] {
        &self.last_pass_provenance
    }

    pub fn last_preflight_report(
        &self,
    ) -> &crate::plugins::render::graph::RenderExecutionGraphPreparedReport {
        &self.last_preflight_report
    }

    pub fn last_preflight_cache_state(
        &self,
    ) -> &crate::plugins::render::graph::RenderPreparedFramePreflightCacheState {
        &self.last_preflight_cache_state
    }

    pub fn last_capture_plan(&self) -> &crate::plugins::render::inspect::ResolvedRenderCapturePlan {
        &self.last_capture_plan
    }

    pub fn last_capture_selector_results(
        &self,
    ) -> &[crate::plugins::render::inspect::RenderCaptureSelectorResult] {
        &self.last_capture_selector_results
    }

    pub(in crate::plugins::render) fn take_published_capture_selector_results(
        &mut self,
    ) -> Vec<crate::plugins::render::inspect::RenderCaptureSelectorResult> {
        std::mem::take(&mut self.last_capture_selector_results)
    }

    pub fn last_captured_textures(
        &self,
    ) -> &[crate::plugins::render::inspect::RenderCapturedTexture] {
        &self.last_captured_textures
    }

    pub(in crate::plugins::render) fn take_published_captured_textures(
        &mut self,
    ) -> Vec<crate::plugins::render::inspect::RenderCapturedTexture> {
        std::mem::take(&mut self.last_captured_textures)
    }

    pub fn flow_pipeline_cache_stats(&self) -> super::pipeline_cache::RendererPipelineCacheStats {
        self.flow_pipeline_cache.stats()
    }

    #[allow(clippy::too_many_arguments)]
    fn realize_ui_program_binding_artifacts(
        &mut self,
        context: &GpuContext,
        kind: UiPipelineKind,
        format: GpuTextureFormat,
        canonical_wgsl: &str,
        source_key: &'static str,
        source_revision: u64,
        screen_label: &'static str,
        texture_sampler_label: Option<&'static str>,
    ) -> Result<UiProgramBindingArtifacts> {
        // The renderer realizes the full G4C1/G4C2/G4C3 UI dependency chain before G5 begins.
        let screen_buffer = self.realize_buffer_resource(
            context,
            screen_label,
            std::mem::size_of::<ScreenUniformRaw>() as u64,
            [GpuBufferUsage::Uniform, GpuBufferUsage::CopyDestination],
            GpuResourceLifetime::Retained,
            GpuMemoryIntent::Device,
        )?;
        let texture_sampler = texture_sampler_label
            .map(|label| {
                self.realize_linear_sampler_resource(context, label, GpuResourceLifetime::Retained)
            })
            .transpose()?;

        let admitted_source = self.flow_pipeline_cache.admit_program_source(
            GpuProgramSourceKey::new(source_key)?,
            source_revision,
            canonical_wgsl,
            GpuProgramSourceProvenance::new("renderer-ui-pipeline", Some(source_key.to_owned()))?,
        )?;
        let vertex_entry = GpuEntryPointName::new("vs_main")?;
        let fragment_entry = GpuEntryPointName::new("fs_main")?;
        let refinements = if texture_sampler_label.is_some() {
            vec![
                GpuBindingLayoutRefinement::new(GpuBindingKey::try_new(1, 0)?)
                    .with_texture_sample_class(GpuTextureSampleClass::FloatFilterable),
                GpuBindingLayoutRefinement::new(GpuBindingKey::try_new(1, 1)?)
                    .with_sampler_class(GpuSamplerClass::Filtering),
            ]
        } else {
            Vec::new()
        };
        let program_descriptor = GpuProgramDescriptor::new(
            admitted_source,
            [vertex_entry.clone(), fragment_entry.clone()],
            refinements,
        )?;
        let pipeline_descriptor = ui_render_pipeline_descriptor(
            kind,
            format,
            program_descriptor.clone(),
            vertex_entry,
            fragment_entry,
        )?;

        let program = pollster::block_on(context.realize_program(&program_descriptor))?;
        let pipeline_layout =
            pollster::block_on(context.realize_pipeline_layout(pipeline_descriptor.layout()))?;
        let pipeline = pollster::block_on(context.realize_render_pipeline(
            &pipeline_descriptor,
            &program,
            &pipeline_layout,
        ))?;
        let screen_size = NonZeroU64::new(screen_buffer._handle.descriptor().size_bytes())
            .expect("screen uniform buffers are nonempty");
        let screen_binding = GpuRuntimeBindingValue::new(
            GpuBindingKey::try_new(0, 0)?,
            [GpuRuntimeBindingResource::Buffer(
                GpuRuntimeBufferBinding::new(screen_buffer._handle.clone(), 0, screen_size, None),
            )],
        )?;
        Ok(UiProgramBindingArtifacts {
            pipeline,
            screen_buffer,
            screen_binding,
            texture_sampler,
        })
    }

    pub(super) fn ensure_rect_pass(
        &mut self,
        context: &GpuContext,
        format: GpuTextureFormat,
        shader_source: &str,
        shader_revision: u64,
    ) -> Result<()> {
        if self.rect_pass.is_some()
            && self.rect_pass_format == Some(format)
            && self.rect_pass_shader_revision == shader_revision
        {
            return Ok(());
        }
        let artifacts = self.realize_ui_program_binding_artifacts(
            context,
            UiPipelineKind::Rect,
            format,
            shader_source,
            "ui:rect",
            shader_revision,
            "engine_ui_screen_uniform",
            None,
        )?;
        let runtime_bindings =
            ui_runtime_binding_set(&artifacts.pipeline, [artifacts.screen_binding.clone()])?;

        self.rect_pass = Some(RectPass {
            pipeline: artifacts.pipeline,
            screen_buffer: artifacts.screen_buffer,
            runtime_bindings,
        });
        self.rect_pass_format = Some(format);
        self.rect_pass_shader_revision = shader_revision;
        Ok(())
    }

    pub(super) fn ensure_stroke_pass(
        &mut self,
        context: &GpuContext,
        format: GpuTextureFormat,
    ) -> Result<()> {
        if self.stroke_pass.is_some() && self.stroke_pass_format == Some(format) {
            return Ok(());
        }
        let artifacts = self.realize_ui_program_binding_artifacts(
            context,
            UiPipelineKind::Stroke,
            format,
            DEFAULT_UI_STROKE_SHADER,
            "ui:stroke",
            0,
            "engine_ui_stroke_screen_uniform",
            None,
        )?;
        let runtime_bindings =
            ui_runtime_binding_set(&artifacts.pipeline, [artifacts.screen_binding.clone()])?;

        self.stroke_pass = Some(StrokePass {
            pipeline: artifacts.pipeline,
            screen_buffer: artifacts.screen_buffer,
            runtime_bindings,
        });
        self.stroke_pass_format = Some(format);
        Ok(())
    }

    pub(super) fn ensure_glyph_pass(
        &mut self,
        context: &GpuContext,
        format: GpuTextureFormat,
    ) -> Result<()> {
        if self.glyph_pass.is_some() && self.glyph_pass_format == Some(format) {
            return Ok(());
        }
        let artifacts = self.realize_ui_program_binding_artifacts(
            context,
            UiPipelineKind::Glyph,
            format,
            DEFAULT_UI_GLYPH_SHADER,
            "ui:glyph",
            0,
            "engine_ui_glyph_screen_uniform",
            Some("engine_ui_glyph_sampler"),
        )?;

        self.glyph_pass = Some(GlyphPass {
            pipeline: artifacts.pipeline,
            screen_buffer: artifacts.screen_buffer,
            screen_binding: artifacts.screen_binding,
            texture_sampler: artifacts
                .texture_sampler
                .expect("glyph pipeline should realize a texture sampler"),
        });
        self.glyph_pass_format = Some(format);
        self.glyph_atlas_gpu.clear();
        Ok(())
    }

    pub(super) fn ensure_viewport_embed_pass(
        &mut self,
        context: &GpuContext,
        format: GpuTextureFormat,
    ) -> Result<()> {
        if self.viewport_embed_pass.is_some() && self.viewport_embed_pass_format == Some(format) {
            return Ok(());
        }
        let artifacts = self.realize_ui_program_binding_artifacts(
            context,
            UiPipelineKind::ViewportEmbed,
            format,
            DEFAULT_UI_VIEWPORT_EMBED_SHADER,
            "ui:viewport-embed",
            0,
            "engine_ui_viewport_embed_screen_uniform",
            Some("engine_ui_viewport_embed_sampler"),
        )?;

        self.viewport_embed_pass = Some(ViewportEmbedPass {
            pipeline: artifacts.pipeline,
            screen_buffer: artifacts.screen_buffer,
            screen_binding: artifacts.screen_binding,
            texture_sampler: artifacts
                .texture_sampler
                .expect("viewport pipeline should realize a texture sampler"),
        });
        self.viewport_embed_pass_format = Some(format);
        Ok(())
    }

    pub(super) fn ensure_product_surface_pass(
        &mut self,
        context: &GpuContext,
        format: GpuTextureFormat,
    ) -> Result<()> {
        if self.product_surface_pass.is_some() && self.product_surface_pass_format == Some(format) {
            return Ok(());
        }
        let artifacts = self.realize_ui_program_binding_artifacts(
            context,
            UiPipelineKind::ProductSurface,
            format,
            DEFAULT_UI_PRODUCT_SURFACE_SHADER,
            "ui:product-surface",
            0,
            "engine_ui_product_surface_screen_uniform",
            Some("engine_ui_product_surface_sampler"),
        )?;

        self.product_surface_pass = Some(ProductSurfacePass {
            pipeline: artifacts.pipeline,
            screen_buffer: artifacts.screen_buffer,
            screen_binding: artifacts.screen_binding,
            texture_sampler: artifacts
                .texture_sampler
                .expect("product-surface pipeline should realize a texture sampler"),
        });
        self.product_surface_pass_format = Some(format);
        Ok(())
    }

    pub(super) fn ensure_glyph_atlas_gpu(
        &mut self,
        atlas: &crate::plugins::render::features::UiFontAtlasResource,
        texture_id: u64,
        pending_operations: &mut RendererPendingOperations,
    ) -> Result<bool> {
        if self.glyph_atlas_gpu.contains_key(&texture_id) {
            return Ok(true);
        }

        let Some(glyph_pass) = self.glyph_pass.as_ref() else {
            return Ok(false);
        };
        let texture_sampler = glyph_pass.texture_sampler.clone();
        let Some((_, atlas_image)) = atlas.atlas_for_texture_id(texture_id) else {
            return Ok(false);
        };
        let texture_handle = self
            .resource_ids
            .allocate_texture_handle(texture_descriptor(
                "engine_ui_glyph_atlas_texture",
                (atlas_image.width.max(1), atlas_image.height.max(1)),
                GpuTextureFormat::R8Unorm,
                [GpuTextureUsage::Sampled, GpuTextureUsage::CopyDestination],
                GpuResourceLifetime::Retained,
            )?)?;
        let texture = RendererTextureResource {
            _handle: texture_handle,
        };

        let view_handle =
            self.resource_ids
                .allocate_texture_view_handle(whole_texture_view_descriptor(
                    "engine_ui_glyph_atlas_view",
                    &texture._handle,
                )?)?;
        let view = RendererTextureViewResource {
            _handle: view_handle,
        };
        let logical_values =
            ui_texture_bind_group_values(view._handle.clone(), texture_sampler._handle.clone())?;
        let runtime_bindings = ui_runtime_binding_set(
            &glyph_pass.pipeline,
            std::iter::once(glyph_pass.screen_binding.clone())
                .chain(logical_values.iter().cloned()),
        )?;
        // The canonical upload initializes the logical atlas before its first G5 draw.
        pending_operations.queue_texture(&texture, &atlas_image.pixels)?;

        self.glyph_atlas_gpu.insert(
            texture_id,
            UiGlyphAtlasGpu {
                _texture: texture,
                _view: view,
                texture_bindings: UiTextureBindings { runtime_bindings },
            },
        );
        Ok(true)
    }

    pub(super) fn full_scissor(surface_width: u32, surface_height: u32) -> (u32, u32, u32, u32) {
        (0, 0, surface_width.max(1), surface_height.max(1))
    }

    pub(super) fn clip_to_scissor(
        clip: [f32; 4],
        surface_width: u32,
        surface_height: u32,
    ) -> Option<(u32, u32, u32, u32)> {
        let max_x = surface_width.max(1) as i32;
        let max_y = surface_height.max(1) as i32;

        let x0 = (clip[0].floor() as i32).clamp(0, max_x);
        let y0 = (clip[1].floor() as i32).clamp(0, max_y);
        let x1 = ((clip[0] + clip[2]).ceil() as i32).clamp(0, max_x);
        let y1 = ((clip[1] + clip[3]).ceil() as i32).clamp(0, max_y);

        if x1 <= x0 || y1 <= y0 {
            return None;
        }

        Some((x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32))
    }

    /// Lowers the current deletion-bound UI batches into execution-complete generic GPU draws.
    /// Screen-uniform and scissor preparation remain main-view-derived; only the explicit viewport
    /// comes from the exact acquired surface attachment extent.
    pub(super) fn lower_ui_draws(
        &self,
        prepared: &UiPreparedDraws,
        viewport_surface_bindings: &ViewportSurfaceBindingRegistry,
        viewport_bind_groups: &UiViewportBindGroups,
        product_surface_bind_groups: &UiProductSurfaceBindGroups,
        acquired_surface_extent: (u32, u32),
    ) -> Result<Vec<GpuRenderDraw>> {
        let viewport = GpuViewport::new(
            0.0,
            0.0,
            acquired_surface_extent.0 as f32,
            acquired_surface_extent.1 as f32,
            0.0,
            1.0,
        )?;
        let mut draws = Vec::with_capacity(prepared.draw_plan.len());
        for command in &prepared.draw_plan {
            let projected = match *command {
                UiPreparedDrawCommand::Rect(index) => self
                    .rect_pass
                    .as_ref()
                    .zip(prepared.rect_batches.get(index))
                    .map(|(pass, batch)| {
                        (
                            &pass.pipeline,
                            pass.runtime_bindings.clone(),
                            &batch.instance_buffer._handle,
                            batch.instance_count,
                            batch.scissor,
                        )
                    }),
                UiPreparedDrawCommand::Stroke(index) => self
                    .stroke_pass
                    .as_ref()
                    .zip(prepared.stroke_batches.get(index))
                    .map(|(pass, batch)| {
                        (
                            &pass.pipeline,
                            pass.runtime_bindings.clone(),
                            &batch.instance_buffer._handle,
                            batch.instance_count,
                            batch.scissor,
                        )
                    }),
                UiPreparedDrawCommand::Glyph(index) => self
                    .glyph_pass
                    .as_ref()
                    .zip(prepared.glyph_batches.get(index))
                    .and_then(|(pass, batch)| {
                        self.glyph_atlas_gpu.get(&batch.texture_id).map(|atlas| {
                            (
                                &pass.pipeline,
                                atlas.texture_bindings.runtime_bindings.clone(),
                                &batch.instance_buffer._handle,
                                batch.instance_count,
                                batch.scissor,
                            )
                        })
                    }),
                UiPreparedDrawCommand::ViewportEmbed(index) => self
                    .viewport_embed_pass
                    .as_ref()
                    .zip(prepared.viewport_embed_batches.get(index))
                    .and_then(|(pass, batch)| {
                        let binding =
                            viewport_surface_bindings.get(batch.viewport_id, batch.slot)?;
                        let bindings = viewport_bind_groups.get(&binding.source)?;
                        Some((
                            &pass.pipeline,
                            bindings.runtime_bindings.clone(),
                            &batch.instance_buffer._handle,
                            batch.instance_count,
                            batch.scissor,
                        ))
                    }),
                UiPreparedDrawCommand::ProductSurface(index) => self
                    .product_surface_pass
                    .as_ref()
                    .zip(prepared.product_surface_batches.get(index))
                    .and_then(|(pass, batch)| {
                        let bindings = product_surface_bind_groups.get(&batch.source)?;
                        Some((
                            &pass.pipeline,
                            bindings.runtime_bindings.clone(),
                            &batch.instance_buffer._handle,
                            batch.instance_count,
                            batch.scissor,
                        ))
                    }),
            };
            let Some((pipeline, bindings, instance_buffer, instance_count, scissor)) = projected
            else {
                continue;
            };
            if instance_count == 0 {
                continue;
            }
            draws.push(GpuRenderDraw::new(
                pipeline.descriptor().clone(),
                bindings,
                [GpuVertexBufferBinding::new(
                    0,
                    instance_buffer,
                    GpuBufferRange::whole(instance_buffer)?,
                )?],
                None,
                GpuDrawIntent::direct(
                    GpuDrawRange::new(0, 6)?,
                    GpuDrawRange::new(0, instance_count)?,
                ),
                viewport,
                GpuScissorRect::new(scissor.0, scissor.1, scissor.2, scissor.3)?,
                GpuBlendConstant::new(0.0, 0.0, 0.0, 0.0)?,
                0,
            )?);
        }
        Ok(draws)
    }

    pub(super) fn realize_ui_dynamic_bind_groups(
        &self,
        prepared: &UiPreparedDraws,
        viewport_surface_bindings: &ViewportSurfaceBindingRegistry,
    ) -> Result<(UiViewportBindGroups, UiProductSurfaceBindGroups)> {
        let mut viewport_bind_groups = UiViewportBindGroups::new();
        let mut product_surface_bind_groups = UiProductSurfaceBindGroups::new();

        for command in &prepared.draw_plan {
            match *command {
                UiPreparedDrawCommand::ViewportEmbed(index) => {
                    let Some(viewport_embed_pass) = self.viewport_embed_pass.as_ref() else {
                        continue;
                    };
                    let Some(batch) = prepared.viewport_embed_batches.get(index) else {
                        continue;
                    };
                    let Some(binding) =
                        viewport_surface_bindings.get(batch.viewport_id, batch.slot)
                    else {
                        continue;
                    };
                    if viewport_bind_groups.contains_key(&binding.source) {
                        continue;
                    }
                    let ViewportSurfaceBindingSource::DynamicTexture {
                        namespace,
                        target_id,
                    } = &binding.source;
                    let key = crate::plugins::render::RenderDynamicTextureTargetKey::new(
                        namespace.clone(),
                        target_id.clone(),
                    );
                    let Ok(view_handle) = self.dynamic_texture_targets.ui_texture_view_handle(&key)
                    else {
                        continue;
                    };
                    let logical_values = ui_texture_bind_group_values(
                        view_handle,
                        viewport_embed_pass.texture_sampler._handle.clone(),
                    )?;
                    let runtime_bindings = ui_runtime_binding_set(
                        &viewport_embed_pass.pipeline,
                        std::iter::once(viewport_embed_pass.screen_binding.clone())
                            .chain(logical_values.iter().cloned()),
                    )?;
                    viewport_bind_groups.insert(
                        binding.source.clone(),
                        UiTextureBindings { runtime_bindings },
                    );
                }
                UiPreparedDrawCommand::ProductSurface(index) => {
                    let Some(product_surface_pass) = self.product_surface_pass.as_ref() else {
                        continue;
                    };
                    let Some(batch) = prepared.product_surface_batches.get(index) else {
                        continue;
                    };
                    if product_surface_bind_groups.contains_key(&batch.source) {
                        continue;
                    }
                    let ProductSurfaceTextureBindingSource::DynamicTexture {
                        namespace,
                        target_id,
                    } = &batch.source;
                    let key = crate::plugins::render::RenderDynamicTextureTargetKey::new(
                        namespace.clone(),
                        target_id.clone(),
                    );
                    let Ok(view_handle) = self.dynamic_texture_targets.ui_texture_view_handle(&key)
                    else {
                        continue;
                    };
                    let logical_values = ui_texture_bind_group_values(
                        view_handle,
                        product_surface_pass.texture_sampler._handle.clone(),
                    )?;
                    let runtime_bindings = ui_runtime_binding_set(
                        &product_surface_pass.pipeline,
                        std::iter::once(product_surface_pass.screen_binding.clone())
                            .chain(logical_values.iter().cloned()),
                    )?;
                    product_surface_bind_groups
                        .insert(batch.source.clone(), UiTextureBindings { runtime_bindings });
                }
                UiPreparedDrawCommand::Rect(_)
                | UiPreparedDrawCommand::Stroke(_)
                | UiPreparedDrawCommand::Glyph(_) => {}
            }
        }

        Ok((viewport_bind_groups, product_surface_bind_groups))
    }
}

#[derive(Debug)]
struct UiProgramBindingArtifacts {
    pipeline: GpuRealizedRenderPipeline,
    screen_buffer: RendererBufferResource,
    screen_binding: GpuRuntimeBindingValue,
    texture_sampler: Option<RendererSamplerResource>,
}

fn ui_runtime_binding_set(
    pipeline: &GpuRealizedRenderPipeline,
    values: impl IntoIterator<Item = GpuRuntimeBindingValue>,
) -> Result<GpuRuntimeBindingSet> {
    Ok(GpuRuntimeBindingSet::new(
        pipeline.descriptor().layout().clone(),
        values,
    )?)
}

fn ui_render_pipeline_descriptor(
    kind: UiPipelineKind,
    format: GpuTextureFormat,
    program: GpuProgramDescriptor,
    vertex_entry: GpuEntryPointName,
    fragment_entry: GpuEntryPointName,
) -> Result<GpuRenderPipelineDescriptor> {
    let vertex_layout = GpuVertexBufferLayoutDescriptor::new(
        0,
        ui_pipeline_stride(kind),
        GpuVertexStepMode::Instance,
        ui_pipeline_attributes(kind),
    )?;
    let vertex_input = GpuVertexInputStateDescriptor::new([vertex_layout])?;
    let color_target =
        GpuColorTargetStateDescriptor::new(format, GpuBlendMode::Alpha, GpuColorWriteMask::ALL)?;
    let state = GpuRenderPipelineStateDescriptor::new(
        vertex_input,
        Some(GpuFragmentOutputStateDescriptor::new([color_target])),
        GpuPrimitiveStateDescriptor::default(),
        None,
        GpuMultisampleStateDescriptor::default(),
    )?;
    Ok(GpuRenderPipelineDescriptor::new(
        program,
        GpuRenderEntryPoints::new(vertex_entry, Some(fragment_entry)),
        state,
        GpuPipelineConfiguration::default(),
    )?)
}

fn ui_pipeline_stride(kind: UiPipelineKind) -> u64 {
    match kind {
        UiPipelineKind::Rect => std::mem::size_of::<RectInstanceRaw>() as u64,
        UiPipelineKind::Stroke => std::mem::size_of::<StrokeSegmentInstanceRaw>() as u64,
        UiPipelineKind::Glyph => std::mem::size_of::<GlyphInstanceRaw>() as u64,
        UiPipelineKind::ViewportEmbed | UiPipelineKind::ProductSurface => {
            std::mem::size_of::<ViewportEmbedInstanceRaw>() as u64
        }
    }
}

fn ui_pipeline_attributes(kind: UiPipelineKind) -> Vec<GpuVertexAttribute> {
    match kind {
        UiPipelineKind::Rect => vec![
            GpuVertexAttribute::new(0, 0, GpuVertexFormat::Float32x4),
            GpuVertexAttribute::new(1, 16, GpuVertexFormat::Float32x4),
            GpuVertexAttribute::new(2, 32, GpuVertexFormat::Float32),
            GpuVertexAttribute::new(3, 36, GpuVertexFormat::Float32),
        ],
        UiPipelineKind::Stroke => vec![
            GpuVertexAttribute::new(0, 0, GpuVertexFormat::Float32x2),
            GpuVertexAttribute::new(1, 8, GpuVertexFormat::Float32x2),
            GpuVertexAttribute::new(2, 16, GpuVertexFormat::Float32x4),
            GpuVertexAttribute::new(3, 32, GpuVertexFormat::Float32),
        ],
        UiPipelineKind::Glyph | UiPipelineKind::ViewportEmbed | UiPipelineKind::ProductSurface => {
            vec![
                GpuVertexAttribute::new(0, 0, GpuVertexFormat::Float32x4),
                GpuVertexAttribute::new(1, 16, GpuVertexFormat::Float32x4),
                GpuVertexAttribute::new(2, 32, GpuVertexFormat::Float32x4),
            ]
        }
    }
}

fn ui_texture_bind_group_values(
    view: GpuTextureViewHandle,
    sampler: GpuSamplerHandle,
) -> Result<[GpuRuntimeBindingValue; 2]> {
    Ok([
        GpuRuntimeBindingValue::new(
            GpuBindingKey::try_new(1, 0)?,
            [GpuRuntimeBindingResource::TextureView(
                GpuRuntimeTextureViewBinding::new(view, GpuTextureViewDimension::D2),
            )],
        )?,
        GpuRuntimeBindingValue::new(
            GpuBindingKey::try_new(1, 1)?,
            [GpuRuntimeBindingResource::Sampler(sampler)],
        )?,
    ])
}
