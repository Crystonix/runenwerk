use super::resource_descriptors::{
    repeat_linear_sampler_descriptor, texture_descriptor_with_extent, whole_texture_view_descriptor,
};
use super::*;
use crate::plugins::render::features::{
    MATERIAL_RENDER_FEATURE_ID, UI_RENDER_FEATURE_ID, UiFontAtlasResource,
};
use crate::plugins::render::texture_upload::load_material_ktx2_upload;
use crate::plugins::{PreparedUiFrameContribution, RenderFeatureId};
use runen_gpu::{
    GpuBufferRange, GpuBufferRegion, GpuBufferUsage, GpuCopyExtent, GpuMemoryIntent,
    GpuResourceLifetime, GpuTextureAspect, GpuTextureCopyRegion, GpuTextureHandle,
    GpuTextureOrigin, GpuTextureUsage, GpuUploadOperation, PreparedGpuData, TransferData,
};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct UiBatchOrder {
    submission_order: u32,
    surface_order: u32,
    layer_order: u32,
    first_primitive_order: u32,
    last_primitive_order: u32,
}

impl UiBatchOrder {
    fn new(
        submission_order: u32,
        surface_order: u32,
        layer_order: u32,
        primitive_order: u32,
    ) -> Self {
        Self {
            submission_order,
            surface_order,
            layer_order,
            first_primitive_order: primitive_order,
            last_primitive_order: primitive_order,
        }
    }

    fn can_extend(self, submission_order: u32, surface_order: u32, layer_order: u32) -> bool {
        self.submission_order == submission_order
            && self.surface_order == surface_order
            && self.layer_order == layer_order
    }

    fn sort_key(self, family_order: u8) -> (u32, u32, u32, u32, u32, u8) {
        (
            self.submission_order,
            self.surface_order,
            self.layer_order,
            self.first_primitive_order,
            self.last_primitive_order,
            family_order,
        )
    }
}

#[derive(Debug)]
struct ScissoredPrimitiveBatch<T> {
    order: UiBatchOrder,
    scissor: (u32, u32, u32, u32),
    instances: Vec<T>,
}

#[derive(Debug)]
struct ScissoredGlyphBatch {
    order: UiBatchOrder,
    scissor: (u32, u32, u32, u32),
    texture_id: u64,
    instances: Vec<GlyphInstanceRaw>,
}

#[derive(Debug)]
struct ScissoredViewportEmbedBatch {
    order: UiBatchOrder,
    scissor: (u32, u32, u32, u32),
    viewport_id: u64,
    slot: ViewportSurfaceEmbedSlotId,
    instances: Vec<ViewportEmbedInstanceRaw>,
}

#[derive(Debug)]
struct ScissoredProductSurfaceBatch {
    order: UiBatchOrder,
    scissor: (u32, u32, u32, u32),
    source: ProductSurfaceTextureBindingSource,
    instances: Vec<ViewportEmbedInstanceRaw>,
}

impl Renderer {
    fn realize_uploaded_ui_buffer(
        &mut self,
        context: &GpuContext,
        label: &str,
        contents: &[u8],
        pending_operations: &mut RendererPendingOperations,
    ) -> Result<RendererBufferResource> {
        let buffer = self.realize_buffer_resource(
            context,
            label,
            contents.len() as u64,
            [GpuBufferUsage::Vertex, GpuBufferUsage::CopyDestination],
            GpuResourceLifetime::Transient,
            GpuMemoryIntent::Device,
        )?;
        pending_operations.queue_buffer(&buffer, contents)?;
        Ok(buffer)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_ui_draws(
        &mut self,
        context: &GpuContext,
        contribution: &PreparedUiFrameContribution,
        atlas_resource: &UiFontAtlasResource,
        surface_width: f32,
        surface_height: f32,
        pending_operations: &mut RendererPendingOperations,
    ) -> Result<UiPreparedDraws> {
        let surface_width_u32 = surface_width.max(1.0).round() as u32;
        let surface_height_u32 = surface_height.max(1.0).round() as u32;
        let flattened_rect_instances = contribution
            .submissions
            .iter()
            .enumerate()
            .flat_map(|(submission_order, submission)| {
                Self::extract_rect_instances(submission_order as u32, &submission.frame)
            })
            .collect::<Vec<_>>();
        let flattened_stroke_instances = contribution
            .submissions
            .iter()
            .enumerate()
            .flat_map(|(submission_order, submission)| {
                Self::extract_stroke_instances(submission_order as u32, &submission.frame)
            })
            .collect::<Vec<_>>();
        let flattened_glyph_instances = contribution
            .submissions
            .iter()
            .enumerate()
            .flat_map(|(submission_order, submission)| {
                Self::extract_glyph_instances(
                    submission_order as u32,
                    &submission.frame,
                    atlas_resource,
                )
            })
            .collect::<Vec<_>>();
        let flattened_viewport_embed_instances = contribution
            .submissions
            .iter()
            .enumerate()
            .flat_map(|(submission_order, submission)| {
                Self::extract_viewport_embed_instances(submission_order as u32, &submission.frame)
            })
            .collect::<Vec<_>>();
        let flattened_product_surface_instances = contribution
            .submissions
            .iter()
            .enumerate()
            .flat_map(|(submission_order, submission)| {
                Self::extract_product_surface_instances(submission_order as u32, &submission.frame)
            })
            .collect::<Vec<_>>();

        let rect_batches = group_rect_batches_ordered(
            flattened_rect_instances,
            surface_width_u32,
            surface_height_u32,
        )
        .into_iter()
        .map(|batch| -> Result<Option<UiRectBatch>> {
            if batch.instances.is_empty() {
                return Ok(None);
            }
            let instance_buffer = self.realize_uploaded_ui_buffer(
                context,
                "engine_ui_rect_batch_instances",
                bytemuck::cast_slice(&batch.instances),
                pending_operations,
            )?;
            Ok(Some(UiRectBatch {
                submission_order: batch.order.submission_order,
                surface_order: batch.order.surface_order,
                layer_order: batch.order.layer_order,
                first_primitive_order: batch.order.first_primitive_order,
                last_primitive_order: batch.order.last_primitive_order,
                scissor: batch.scissor,
                instance_count: batch.instances.len() as u32,
                instance_buffer,
            }))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        let stroke_batches = group_stroke_batches_ordered(
            flattened_stroke_instances,
            surface_width_u32,
            surface_height_u32,
        )
        .into_iter()
        .map(|batch| -> Result<Option<UiStrokeBatch>> {
            if batch.instances.is_empty() {
                return Ok(None);
            }
            let instance_buffer = self.realize_uploaded_ui_buffer(
                context,
                "engine_ui_stroke_batch_instances",
                bytemuck::cast_slice(&batch.instances),
                pending_operations,
            )?;
            Ok(Some(UiStrokeBatch {
                submission_order: batch.order.submission_order,
                surface_order: batch.order.surface_order,
                layer_order: batch.order.layer_order,
                first_primitive_order: batch.order.first_primitive_order,
                last_primitive_order: batch.order.last_primitive_order,
                scissor: batch.scissor,
                instance_count: batch.instances.len() as u32,
                instance_buffer,
            }))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        let mut glyph_batches_by_scissor = Vec::<ScissoredGlyphBatch>::new();
        for instance in flattened_glyph_instances {
            let scissor = instance
                .clip
                .map(|clip| Self::clip_to_scissor(clip, surface_width_u32, surface_height_u32))
                .unwrap_or_else(|| Some(Self::full_scissor(surface_width_u32, surface_height_u32)));
            let Some(scissor) = scissor else {
                continue;
            };
            if !self.ensure_glyph_atlas_gpu(
                atlas_resource,
                instance.texture_id,
                pending_operations,
            )? {
                continue;
            }
            if let Some(batch) = glyph_batches_by_scissor.last_mut()
                && batch.order.can_extend(
                    instance.submission_order,
                    instance.surface_order,
                    instance.layer_order,
                )
                && batch.scissor == scissor
                && batch.texture_id == instance.texture_id
                && (batch.order.last_primitive_order == instance.primitive_order
                    || batch.order.last_primitive_order.saturating_add(1)
                        == instance.primitive_order)
            {
                batch.instances.push(instance.raw);
                batch.order.last_primitive_order = instance.primitive_order;
            } else {
                glyph_batches_by_scissor.push(ScissoredGlyphBatch {
                    order: UiBatchOrder::new(
                        instance.submission_order,
                        instance.surface_order,
                        instance.layer_order,
                        instance.primitive_order,
                    ),
                    scissor,
                    texture_id: instance.texture_id,
                    instances: vec![instance.raw],
                });
            }
        }
        let glyph_batches = glyph_batches_by_scissor
            .into_iter()
            .map(|batch| -> Result<Option<UiGlyphBatch>> {
                if batch.instances.is_empty() {
                    return Ok(None);
                }
                let instance_buffer = self.realize_uploaded_ui_buffer(
                    context,
                    "engine_ui_glyph_batch_instances",
                    bytemuck::cast_slice(&batch.instances),
                    pending_operations,
                )?;
                Ok(Some(UiGlyphBatch {
                    submission_order: batch.order.submission_order,
                    surface_order: batch.order.surface_order,
                    layer_order: batch.order.layer_order,
                    first_primitive_order: batch.order.first_primitive_order,
                    last_primitive_order: batch.order.last_primitive_order,
                    scissor: batch.scissor,
                    instance_count: batch.instances.len() as u32,
                    instance_buffer,
                    texture_id: batch.texture_id,
                }))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let viewport_embed_batches = group_viewport_embed_batches_ordered(
            flattened_viewport_embed_instances,
            surface_width_u32,
            surface_height_u32,
        )
        .into_iter()
        .map(|batch| -> Result<Option<UiViewportEmbedBatch>> {
            if batch.instances.is_empty() {
                return Ok(None);
            }
            let instance_buffer = self.realize_uploaded_ui_buffer(
                context,
                "engine_ui_viewport_embed_batch_instances",
                bytemuck::cast_slice(&batch.instances),
                pending_operations,
            )?;
            Ok(Some(UiViewportEmbedBatch {
                submission_order: batch.order.submission_order,
                surface_order: batch.order.surface_order,
                layer_order: batch.order.layer_order,
                first_primitive_order: batch.order.first_primitive_order,
                last_primitive_order: batch.order.last_primitive_order,
                scissor: batch.scissor,
                instance_count: batch.instances.len() as u32,
                instance_buffer,
                viewport_id: batch.viewport_id,
                slot: batch.slot,
            }))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        let product_surface_batches = group_product_surface_batches_ordered(
            flattened_product_surface_instances,
            surface_width_u32,
            surface_height_u32,
        )
        .into_iter()
        .map(|batch| -> Result<Option<UiProductSurfaceBatch>> {
            if batch.instances.is_empty() {
                return Ok(None);
            }
            let instance_buffer = self.realize_uploaded_ui_buffer(
                context,
                "engine_ui_product_surface_batch_instances",
                bytemuck::cast_slice(&batch.instances),
                pending_operations,
            )?;
            Ok(Some(UiProductSurfaceBatch {
                submission_order: batch.order.submission_order,
                surface_order: batch.order.surface_order,
                layer_order: batch.order.layer_order,
                first_primitive_order: batch.order.first_primitive_order,
                last_primitive_order: batch.order.last_primitive_order,
                scissor: batch.scissor,
                instance_count: batch.instances.len() as u32,
                instance_buffer,
                source: batch.source,
            }))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        if let Some(rect_pass) = self.rect_pass.as_ref() {
            let screen = ScreenUniformRaw {
                size: [surface_width.max(1.0), surface_height.max(1.0)],
                _pad: [0.0; 2],
            };
            pending_operations
                .queue_buffer(&rect_pass.screen_buffer, bytemuck::bytes_of(&screen))?;
        }
        if let Some(stroke_pass) = self.stroke_pass.as_ref() {
            let screen = ScreenUniformRaw {
                size: [surface_width.max(1.0), surface_height.max(1.0)],
                _pad: [0.0; 2],
            };
            pending_operations
                .queue_buffer(&stroke_pass.screen_buffer, bytemuck::bytes_of(&screen))?;
        }
        if let Some(glyph_pass) = self.glyph_pass.as_ref() {
            let screen = ScreenUniformRaw {
                size: [surface_width.max(1.0), surface_height.max(1.0)],
                _pad: [0.0; 2],
            };
            pending_operations
                .queue_buffer(&glyph_pass.screen_buffer, bytemuck::bytes_of(&screen))?;
        }
        if let Some(viewport_embed_pass) = self.viewport_embed_pass.as_ref() {
            let screen = ScreenUniformRaw {
                size: [surface_width.max(1.0), surface_height.max(1.0)],
                _pad: [0.0; 2],
            };
            pending_operations.queue_buffer(
                &viewport_embed_pass.screen_buffer,
                bytemuck::bytes_of(&screen),
            )?;
        }
        if let Some(product_surface_pass) = self.product_surface_pass.as_ref() {
            let screen = ScreenUniformRaw {
                size: [surface_width.max(1.0), surface_height.max(1.0)],
                _pad: [0.0; 2],
            };
            pending_operations.queue_buffer(
                &product_surface_pass.screen_buffer,
                bytemuck::bytes_of(&screen),
            )?;
        }

        let draw_plan = build_ui_draw_plan(
            &rect_batches,
            &stroke_batches,
            &glyph_batches,
            &viewport_embed_batches,
            &product_surface_batches,
        );

        Ok(UiPreparedDraws {
            rect_batches,
            stroke_batches,
            glyph_batches,
            viewport_embed_batches,
            product_surface_batches,
            draw_plan,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_packet(
        &mut self,
        context: &GpuContext,
        prepared_frame: &PreparedRenderFrame,
        shader_registry: &mut ShaderRegistryResource,
        ui_rect_shader_handle: Option<ShaderHandle>,
        ui_font_atlas: &UiFontAtlasResource,
        viewport_surface_bindings: &ViewportSurfaceBindingRegistry,
        surface_format: GpuTextureFormat,
    ) -> Result<RendererPreparedPacket> {
        let view = prepared_frame.main_view().cloned().unwrap_or_else(|| {
            crate::plugins::render::PreparedViewFrame::main(prepared_frame.surface.target_size_px)
        });
        let (surface_width_u32, surface_height_u32) = view.target_size_px;
        let surface_width = surface_width_u32.max(1) as f32;
        let surface_height = surface_height_u32.max(1) as f32;
        let empty_ui = PreparedUiFrameContribution::default();
        let ui = prepared_frame.ui().unwrap_or(&empty_ui);

        let mut feature_gates = BTreeMap::<RenderFeatureId, FeatureExecutionGate>::new();
        let mut feature_runtime_signatures = BTreeMap::<RenderFeatureId, u64>::new();

        for (feature_id, contribution) in &prepared_frame.contributions.by_feature {
            feature_gates.insert(
                *feature_id,
                FeatureExecutionGate {
                    status: contribution.status,
                    fallback_policy: contribution.fallback_policy,
                },
            );
            feature_runtime_signatures.insert(
                *feature_id,
                hash_prepared_feature_contribution(contribution),
            );
        }

        let ui_gate = feature_gates
            .get(&UI_RENDER_FEATURE_ID)
            .copied()
            .unwrap_or_default();
        let prepared_material = prepared_frame
            .contributions
            .feature(&MATERIAL_RENDER_FEATURE_ID)
            .and_then(|contribution| match &contribution.payload {
                crate::plugins::render::PreparedFeaturePayload::Material(payload) => {
                    Some(payload.clone())
                }
                _ => None,
            });
        let mut pending_operations = RendererPendingOperations::default();
        let prepared_material_gpu_resources = self.prepare_material_gpu_resources(
            context,
            prepared_material.as_ref(),
            &mut pending_operations,
        )?;

        let mut prepare_timings = RendererFrameTimings::default();
        let ui_rect_shader = ui_rect_shader_handle
            .map(|handle| shader_registry.source_or_handle(handle, DEFAULT_UI_RECT_SHADER))
            .unwrap_or(DEFAULT_UI_RECT_SHADER)
            .to_string();
        let ui_rect_revision = ui_rect_shader_handle
            .map(|handle| shader_registry.revision_for_handle(handle))
            .unwrap_or(0);

        self.ensure_rect_pass(context, surface_format, &ui_rect_shader, ui_rect_revision)?;
        self.ensure_stroke_pass(context, surface_format)?;
        self.ensure_glyph_pass(context, surface_format)?;
        self.ensure_viewport_embed_pass(context, surface_format)?;
        self.ensure_product_surface_pass(context, surface_format)?;
        let surface_size = (surface_width_u32.max(1), surface_height_u32.max(1));
        let prepare_ui_start = Instant::now();
        let prepared_ui_current = {
            let _span = tracing::info_span!("renderer.prepare_ui_draws").entered();
            self.prepare_ui_draws(
                context,
                ui,
                ui_font_atlas,
                surface_width,
                surface_height,
                &mut pending_operations,
            )?
        };
        let prepared_ui = self.resolve_ui_prepared_with_gate(prepared_ui_current, ui_gate);
        prepare_timings.prepare_ui_ms = prepare_ui_start.elapsed().as_secs_f32() * 1000.0;
        prepare_timings.prepare_mesh_ms = 0.0;
        prepare_timings.mesh_hot_path = MeshPrepareHotPath::default();

        Ok(RendererPreparedPacket {
            surface_format,
            surface_size,
            view_id: view.view_id,
            feature_gates,
            feature_runtime_signatures,
            prepared_material,
            prepared_material_gpu_resources,
            prepared_ui,
            ui_dynamic_bind_groups: UiDynamicBindGroups::default(),
            pending_operations,
            viewport_surface_bindings: viewport_surface_bindings.clone(),
            prepare_timings,
        })
    }

    fn prepare_material_gpu_resources(
        &mut self,
        _context: &GpuContext,
        material: Option<&crate::plugins::render::PreparedMaterialFeatureContribution>,
        pending_operations: &mut RendererPendingOperations,
    ) -> Result<Option<PreparedMaterialGpuResources>> {
        let Some(material) = material else {
            return Ok(None);
        };
        if material.validate_portable_limits().is_err() {
            return Ok(None);
        }
        let mut bindings = material
            .instances
            .iter()
            .flat_map(|instance| instance.texture_bindings.iter())
            .collect::<Vec<_>>();
        if bindings.is_empty() {
            return Ok(None);
        }
        bindings.sort_by_key(|binding| {
            (
                binding.bind_group,
                binding.texture_binding,
                binding.sampler_binding,
                binding.resource_slot_index,
            )
        });

        let material_group = bindings[0].bind_group;
        if material_group != 1
            || bindings
                .iter()
                .any(|binding| binding.bind_group != material_group)
        {
            tracing::warn!(
                material_group,
                "material texture bindings must occupy the one current material group"
            );
            return Ok(None);
        }

        let mut textures = Vec::with_capacity(bindings.len());
        let mut texture_views = Vec::with_capacity(bindings.len());
        let mut samplers = Vec::with_capacity(bindings.len());
        let mut texture_uploads = Vec::with_capacity(bindings.len());

        for binding in bindings {
            let upload = match load_material_ktx2_upload(binding) {
                Ok(upload) => upload,
                Err(error) => {
                    tracing::warn!(
                        texture_artifact = binding.artifact_id.as_str(),
                        path = binding.artifact_path.as_str(),
                        error = %error,
                        "material texture residency rejected KTX2 artifact"
                    );
                    return Ok(None);
                }
            };
            let dimension = upload.dimension;
            let texture_handle =
                self.resource_ids
                    .allocate_texture_handle(texture_descriptor_with_extent(
                        "engine_material_resident_texture",
                        dimension,
                        (
                            upload.size.width(),
                            upload.size.height(),
                            upload.size.depth_or_layers(),
                        ),
                        upload.format,
                        [GpuTextureUsage::Sampled, GpuTextureUsage::CopyDestination],
                        GpuResourceLifetime::Transient,
                    )?)?;
            let texture = RendererTextureResource {
                _handle: texture_handle,
            };

            let view_handle =
                self.resource_ids
                    .allocate_texture_view_handle(whole_texture_view_descriptor(
                        "engine_material_resident_texture_view",
                        &texture._handle,
                    )?)?;
            let view = RendererTextureViewResource {
                _handle: view_handle,
            };
            let sampler_handle =
                self.resource_ids
                    .allocate_sampler_handle(repeat_linear_sampler_descriptor(
                        "engine_material_resident_sampler",
                        GpuResourceLifetime::Transient,
                    )?)?;
            let sampler = RendererSamplerResource {
                _handle: sampler_handle,
            };
            texture_views.push(view);
            samplers.push(sampler);
            texture_uploads.push(upload);
            textures.push(texture);
        }

        // G5 preparation owns material texture/view/sampler realization from these descriptors.
        for (texture, upload) in textures.iter().zip(texture_uploads) {
            pending_operations.queue_texture(texture, &upload.bytes)?;
        }

        Ok(Some(PreparedMaterialGpuResources {
            _textures: textures,
            _texture_views: texture_views,
            _samplers: samplers,
        }))
    }

    fn resolve_ui_prepared_with_gate(
        &mut self,
        prepared_ui_current: UiPreparedDraws,
        gate: FeatureExecutionGate,
    ) -> UiPreparedDraws {
        match gate.status {
            FeatureContributionStatus::Ready => {
                self.last_good_ui_prepared = Some(prepared_ui_current.clone());
                prepared_ui_current
            }
            FeatureContributionStatus::Stale => {
                if matches!(gate.fallback_policy, FeatureFallbackPolicy::ReuseLastGood)
                    && let Some(cached) = self.last_good_ui_prepared.clone()
                {
                    return cached;
                }
                self.last_good_ui_prepared = Some(prepared_ui_current.clone());
                prepared_ui_current
            }
            FeatureContributionStatus::Disabled | FeatureContributionStatus::Missing => {
                match gate.fallback_policy {
                    FeatureFallbackPolicy::ReuseLastGood => self
                        .last_good_ui_prepared
                        .clone()
                        .unwrap_or(prepared_ui_current),
                    FeatureFallbackPolicy::EmptyContribution
                    | FeatureFallbackPolicy::SkipFeaturePasses
                    | FeatureFallbackPolicy::FailFrame => prepared_ui_current,
                }
            }
        }
    }
}

fn canonical_renderer_buffer_upload(
    buffer: &GpuBufferHandle,
    contents: &[u8],
) -> Result<GpuUploadOperation> {
    let byte_len = u64::try_from(contents.len()).map_err(|_| {
        anyhow::anyhow!(
            "renderer pending buffer upload byte length exceeds the RunenGPU u64 domain"
        )
    })?;
    let label = buffer.descriptor().common().label();
    let payload = PreparedGpuData::<TransferData>::from_pod_transfer(
        format!("renderer pending buffer upload {}", label.as_str()),
        contents,
        buffer.descriptor().common().provenance().clone(),
    )?;
    let range = GpuBufferRange::new(buffer, 0, byte_len)?;
    let region = GpuBufferRegion::new(buffer, range)?;
    Ok(GpuUploadOperation::new(region.into(), payload)?)
}

fn canonical_renderer_texture_upload(
    texture: &GpuTextureHandle,
    contents: &[u8],
) -> Result<GpuUploadOperation> {
    let extent = texture.descriptor().extent();
    let region = GpuTextureCopyRegion::new(
        texture,
        0,
        GpuTextureOrigin::new(0, 0, 0),
        GpuTextureAspect::Color,
        GpuCopyExtent::new(extent.width(), extent.height(), extent.depth_or_layers())?,
    )?;
    let label = texture.descriptor().common().label();
    let payload = PreparedGpuData::<TransferData>::from_pod_transfer(
        format!("renderer pending texture upload {}", label.as_str()),
        contents,
        texture.descriptor().common().provenance().clone(),
    )?;
    Ok(GpuUploadOperation::new(region.into(), payload)?)
}

impl RendererPendingOperations {
    fn queue_buffer(&mut self, buffer: &RendererBufferResource, contents: &[u8]) -> Result<()> {
        self.buffer_uploads
            .push(canonical_renderer_buffer_upload(&buffer._handle, contents)?);
        Ok(())
    }

    pub(super) fn queue_texture(
        &mut self,
        texture: &RendererTextureResource,
        contents: &[u8],
    ) -> Result<()> {
        self.texture_uploads.push(canonical_renderer_texture_upload(
            &texture._handle,
            contents,
        )?);
        Ok(())
    }

    pub(super) fn into_operations(self) -> Vec<GpuUploadOperation> {
        self.buffer_uploads
            .into_iter()
            .chain(self.texture_uploads)
            .collect()
    }
}

fn group_rect_batches_ordered(
    flattened_rect_instances: Vec<FlattenedUiRectInstance>,
    surface_width_u32: u32,
    surface_height_u32: u32,
) -> Vec<ScissoredPrimitiveBatch<RectInstanceRaw>> {
    let mut grouped = Vec::<ScissoredPrimitiveBatch<RectInstanceRaw>>::new();
    for instance in flattened_rect_instances {
        let scissor = instance
            .clip
            .map(|clip| Renderer::clip_to_scissor(clip, surface_width_u32, surface_height_u32))
            .unwrap_or_else(|| {
                Some(Renderer::full_scissor(
                    surface_width_u32,
                    surface_height_u32,
                ))
            });
        let Some(scissor) = scissor else {
            continue;
        };
        if let Some(batch) = grouped.last_mut()
            && batch.order.can_extend(
                instance.submission_order,
                instance.surface_order,
                instance.layer_order,
            )
            && batch.scissor == scissor
            && batch.order.last_primitive_order.saturating_add(1) == instance.primitive_order
        {
            batch.instances.push(instance.raw);
            batch.order.last_primitive_order = instance.primitive_order;
        } else {
            grouped.push(ScissoredPrimitiveBatch {
                order: UiBatchOrder::new(
                    instance.submission_order,
                    instance.surface_order,
                    instance.layer_order,
                    instance.primitive_order,
                ),
                scissor,
                instances: vec![instance.raw],
            });
        }
    }
    grouped
}

fn group_stroke_batches_ordered(
    flattened_instances: Vec<FlattenedUiStrokeSegmentInstance>,
    surface_width_u32: u32,
    surface_height_u32: u32,
) -> Vec<ScissoredPrimitiveBatch<StrokeSegmentInstanceRaw>> {
    let mut grouped = Vec::<ScissoredPrimitiveBatch<StrokeSegmentInstanceRaw>>::new();
    for instance in flattened_instances {
        let scissor = instance
            .clip
            .map(|clip| Renderer::clip_to_scissor(clip, surface_width_u32, surface_height_u32))
            .unwrap_or_else(|| {
                Some(Renderer::full_scissor(
                    surface_width_u32,
                    surface_height_u32,
                ))
            });
        let Some(scissor) = scissor else {
            continue;
        };
        if let Some(batch) = grouped.last_mut()
            && batch.order.can_extend(
                instance.submission_order,
                instance.surface_order,
                instance.layer_order,
            )
            && batch.scissor == scissor
            && (batch.order.last_primitive_order == instance.primitive_order
                || batch.order.last_primitive_order.saturating_add(1) == instance.primitive_order)
        {
            batch.instances.push(instance.raw);
            batch.order.last_primitive_order = instance.primitive_order;
        } else {
            grouped.push(ScissoredPrimitiveBatch {
                order: UiBatchOrder::new(
                    instance.submission_order,
                    instance.surface_order,
                    instance.layer_order,
                    instance.primitive_order,
                ),
                scissor,
                instances: vec![instance.raw],
            });
        }
    }
    grouped
}

fn group_viewport_embed_batches_ordered(
    flattened_instances: Vec<FlattenedUiViewportEmbedInstance>,
    surface_width_u32: u32,
    surface_height_u32: u32,
) -> Vec<ScissoredViewportEmbedBatch> {
    let mut grouped = Vec::<ScissoredViewportEmbedBatch>::new();
    for instance in flattened_instances {
        let scissor = instance
            .clip
            .map(|clip| Renderer::clip_to_scissor(clip, surface_width_u32, surface_height_u32))
            .unwrap_or_else(|| {
                Some(Renderer::full_scissor(
                    surface_width_u32,
                    surface_height_u32,
                ))
            });
        let Some(scissor) = scissor else {
            continue;
        };
        if let Some(batch) = grouped.last_mut()
            && batch.order.can_extend(
                instance.submission_order,
                instance.surface_order,
                instance.layer_order,
            )
            && batch.scissor == scissor
            && batch.viewport_id == instance.viewport_id
            && batch.slot == instance.slot
            && batch.order.last_primitive_order.saturating_add(1) == instance.primitive_order
        {
            batch.instances.push(instance.raw);
            batch.order.last_primitive_order = instance.primitive_order;
        } else {
            grouped.push(ScissoredViewportEmbedBatch {
                order: UiBatchOrder::new(
                    instance.submission_order,
                    instance.surface_order,
                    instance.layer_order,
                    instance.primitive_order,
                ),
                scissor,
                viewport_id: instance.viewport_id,
                slot: instance.slot,
                instances: vec![instance.raw],
            });
        }
    }
    grouped
}

fn group_product_surface_batches_ordered(
    flattened_instances: Vec<FlattenedUiProductSurfaceInstance>,
    surface_width_u32: u32,
    surface_height_u32: u32,
) -> Vec<ScissoredProductSurfaceBatch> {
    let mut grouped = Vec::<ScissoredProductSurfaceBatch>::new();
    for instance in flattened_instances {
        let scissor = instance
            .clip
            .map(|clip| Renderer::clip_to_scissor(clip, surface_width_u32, surface_height_u32))
            .unwrap_or_else(|| {
                Some(Renderer::full_scissor(
                    surface_width_u32,
                    surface_height_u32,
                ))
            });
        let Some(scissor) = scissor else {
            continue;
        };
        if let Some(batch) = grouped.last_mut()
            && batch.order.can_extend(
                instance.submission_order,
                instance.surface_order,
                instance.layer_order,
            )
            && batch.scissor == scissor
            && batch.source == instance.source
            && batch.order.last_primitive_order.saturating_add(1) == instance.primitive_order
        {
            batch.instances.push(instance.raw);
            batch.order.last_primitive_order = instance.primitive_order;
        } else {
            grouped.push(ScissoredProductSurfaceBatch {
                order: UiBatchOrder::new(
                    instance.submission_order,
                    instance.surface_order,
                    instance.layer_order,
                    instance.primitive_order,
                ),
                scissor,
                source: instance.source,
                instances: vec![instance.raw],
            });
        }
    }
    grouped
}

fn build_ui_draw_plan(
    rect_batches: &[UiRectBatch],
    stroke_batches: &[UiStrokeBatch],
    glyph_batches: &[UiGlyphBatch],
    viewport_embed_batches: &[UiViewportEmbedBatch],
    product_surface_batches: &[UiProductSurfaceBatch],
) -> Vec<UiPreparedDrawCommand> {
    let mut commands = Vec::with_capacity(
        rect_batches.len()
            + stroke_batches.len()
            + glyph_batches.len()
            + viewport_embed_batches.len()
            + product_surface_batches.len(),
    );
    commands.extend(
        rect_batches
            .iter()
            .enumerate()
            .map(|(index, _)| UiPreparedDrawCommand::Rect(index)),
    );
    commands.extend(
        stroke_batches
            .iter()
            .enumerate()
            .map(|(index, _)| UiPreparedDrawCommand::Stroke(index)),
    );
    commands.extend(
        viewport_embed_batches
            .iter()
            .enumerate()
            .map(|(index, _)| UiPreparedDrawCommand::ViewportEmbed(index)),
    );
    commands.extend(
        product_surface_batches
            .iter()
            .enumerate()
            .map(|(index, _)| UiPreparedDrawCommand::ProductSurface(index)),
    );
    commands.extend(
        glyph_batches
            .iter()
            .enumerate()
            .map(|(index, _)| UiPreparedDrawCommand::Glyph(index)),
    );
    commands.sort_by(|left, right| {
        let left_key = draw_command_sort_key(
            *left,
            rect_batches,
            stroke_batches,
            glyph_batches,
            viewport_embed_batches,
            product_surface_batches,
        );
        let right_key = draw_command_sort_key(
            *right,
            rect_batches,
            stroke_batches,
            glyph_batches,
            viewport_embed_batches,
            product_surface_batches,
        );
        left_key.cmp(&right_key)
    });
    commands
}

fn draw_command_sort_key(
    command: UiPreparedDrawCommand,
    rect_batches: &[UiRectBatch],
    stroke_batches: &[UiStrokeBatch],
    glyph_batches: &[UiGlyphBatch],
    viewport_embed_batches: &[UiViewportEmbedBatch],
    product_surface_batches: &[UiProductSurfaceBatch],
) -> (u32, u32, u32, u32, u32, u8) {
    match command {
        UiPreparedDrawCommand::Rect(index) => {
            let batch = &rect_batches[index];
            UiBatchOrder {
                submission_order: batch.submission_order,
                surface_order: batch.surface_order,
                layer_order: batch.layer_order,
                first_primitive_order: batch.first_primitive_order,
                last_primitive_order: batch.last_primitive_order,
            }
            .sort_key(0)
        }
        UiPreparedDrawCommand::Stroke(index) => {
            let batch = &stroke_batches[index];
            UiBatchOrder {
                submission_order: batch.submission_order,
                surface_order: batch.surface_order,
                layer_order: batch.layer_order,
                first_primitive_order: batch.first_primitive_order,
                last_primitive_order: batch.last_primitive_order,
            }
            .sort_key(1)
        }
        UiPreparedDrawCommand::ViewportEmbed(index) => {
            let batch = &viewport_embed_batches[index];
            UiBatchOrder {
                submission_order: batch.submission_order,
                surface_order: batch.surface_order,
                layer_order: batch.layer_order,
                first_primitive_order: batch.first_primitive_order,
                last_primitive_order: batch.last_primitive_order,
            }
            .sort_key(2)
        }
        UiPreparedDrawCommand::ProductSurface(index) => {
            let batch = &product_surface_batches[index];
            UiBatchOrder {
                submission_order: batch.submission_order,
                surface_order: batch.surface_order,
                layer_order: batch.layer_order,
                first_primitive_order: batch.first_primitive_order,
                last_primitive_order: batch.last_primitive_order,
            }
            .sort_key(2)
        }
        UiPreparedDrawCommand::Glyph(index) => {
            let batch = &glyph_batches[index];
            UiBatchOrder {
                submission_order: batch.submission_order,
                surface_order: batch.surface_order,
                layer_order: batch.layer_order,
                first_primitive_order: batch.first_primitive_order,
                last_primitive_order: batch.last_primitive_order,
            }
            .sort_key(3)
        }
    }
}

fn hash_prepared_feature_contribution(
    contribution: &crate::plugins::render::PreparedFeatureContribution,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    contribution.status.hash(&mut hasher);
    contribution.fallback_policy.hash(&mut hasher);
    match &contribution.payload {
        crate::plugins::render::PreparedFeaturePayload::Empty => {
            "empty".hash(&mut hasher);
        }
        crate::plugins::render::PreparedFeaturePayload::Ui(value) => {
            "ui".hash(&mut hasher);
            value.submissions.len().hash(&mut hasher);
            for submission in &value.submissions {
                submission.producer_id.hash(&mut hasher);
                submission.route.hash(&mut hasher);
                submission.layer.hash(&mut hasher);
                submission.priority.hash(&mut hasher);
                submission.rect_shader_asset_id.hash(&mut hasher);
                submission.primitive_count_hint().hash(&mut hasher);
            }
        }
        crate::plugins::render::PreparedFeaturePayload::SceneRoute(value) => {
            "scene_route".hash(&mut hasher);
            value.world_scene_label.hash(&mut hasher);
            value.overlay_scene_label.hash(&mut hasher);
        }
        crate::plugins::render::PreparedFeaturePayload::Draw(value) => {
            "draw".hash(&mut hasher);
            value.batches.len().hash(&mut hasher);
            for batch in &value.batches {
                batch.batch_id.hash(&mut hasher);
                batch.mesh_ref.hash(&mut hasher);
                batch.material_ref.hash(&mut hasher);
                batch.instance_count.hash(&mut hasher);
            }
        }
        crate::plugins::render::PreparedFeaturePayload::World(value) => {
            "world".hash(&mut hasher);
            value.visible_chunks.len().hash(&mut hasher);
            value.residency_intents.len().hash(&mut hasher);
            for chunk in &value.visible_chunks {
                chunk.chunk_id.hash(&mut hasher);
                chunk.chunk_revision.hash(&mut hasher);
                chunk.chunk_generation.hash(&mut hasher);
                chunk.draw_batch_ref.hash(&mut hasher);
            }
            for intent in &value.residency_intents {
                intent.chunk_id.hash(&mut hasher);
                intent.priority.hash(&mut hasher);
                intent.hard_pin.hash(&mut hasher);
            }
        }
        crate::plugins::render::PreparedFeaturePayload::Caves(value) => {
            "caves".hash(&mut hasher);
            value.visible_sector_ids.hash(&mut hasher);
            value.scoped_light_volume_count.hash(&mut hasher);
        }
        crate::plugins::render::PreparedFeaturePayload::Detail(value) => {
            "detail".hash(&mut hasher);
            value.cells.len().hash(&mut hasher);
            for cell in &value.cells {
                cell.cell_id.hash(&mut hasher);
                cell.chunk_id.hash(&mut hasher);
                cell.instance_count.hash(&mut hasher);
            }
        }
        crate::plugins::render::PreparedFeaturePayload::ProceduralWorld(value) => {
            "procedural_world".hash(&mut hasher);
            value.overlays.len().hash(&mut hasher);
            for overlay in &value.overlays {
                overlay.overlay_id.hash(&mut hasher);
                overlay.source_revision.hash(&mut hasher);
            }
        }
        crate::plugins::render::PreparedFeaturePayload::WindFields(value) => {
            "wind_fields".hash(&mut hasher);
            value.fields.len().hash(&mut hasher);
            for field in &value.fields {
                field.field_id.hash(&mut hasher);
                field.strength.to_bits().hash(&mut hasher);
            }
        }
        crate::plugins::render::PreparedFeaturePayload::Material(value) => {
            "material".hash(&mut hasher);
            match value.binding_table.backend {
                crate::plugins::render::PreparedMaterialBindingTableBackend::FixedCapacityArray {
                    capacity,
                } => {
                    "fixed_capacity_array".hash(&mut hasher);
                    capacity.hash(&mut hasher);
                }
            }
            value.binding_table.slots.len().hash(&mut hasher);
            for slot in &value.binding_table.slots {
                slot.slot_index.hash(&mut hasher);
                slot.material_instance_id.hash(&mut hasher);
                slot.formed_material_artifact_id.hash(&mut hasher);
                slot.shader_artifact_id.hash(&mut hasher);
                slot.material_cache_key.hash(&mut hasher);
                slot.shader_cache_key.hash(&mut hasher);
                slot.prior_valid.hash(&mut hasher);
            }
            if let Some(scene_bundle) = &value.scene_bundle {
                "scene_bundle".hash(&mut hasher);
                scene_bundle.shader_artifact_id.hash(&mut hasher);
                scene_bundle.shader_cache_key.hash(&mut hasher);
                scene_bundle.shader_path.hash(&mut hasher);
                scene_bundle.shader_identity.hash(&mut hasher);
                scene_bundle.material_table_identity.hash(&mut hasher);
                scene_bundle.resource_layout_identity.hash(&mut hasher);
            }
            value.model_mesh_material_selections.len().hash(&mut hasher);
            for selection in &value.model_mesh_material_selections {
                selection.surface.source.asset_id.hash(&mut hasher);
                selection.surface.source.source_id.hash(&mut hasher);
                selection
                    .surface
                    .source
                    .source_revision_id
                    .hash(&mut hasher);
                selection.surface.source.source_revision.hash(&mut hasher);
                selection.surface.region_key.hash(&mut hasher);
                selection.requested_material_slot_id.hash(&mut hasher);
                selection.resolved_material_slot_id.hash(&mut hasher);
                selection.material_table_index.hash(&mut hasher);
                selection.used_default_fallback.hash(&mut hasher);
            }
            value.instances.len().hash(&mut hasher);
            for instance in &value.instances {
                instance.material_instance_id.hash(&mut hasher);
                instance.specialization_key_fragment.hash(&mut hasher);
                instance.parameter_payload.encode_v1().hash(&mut hasher);
                instance.texture_bindings.len().hash(&mut hasher);
                for binding in &instance.texture_bindings {
                    binding.node_id.hash(&mut hasher);
                    binding.binding_key.hash(&mut hasher);
                    binding.resource_slot_index.hash(&mut hasher);
                    binding.bind_group.hash(&mut hasher);
                    binding.texture_binding.hash(&mut hasher);
                    binding.sampler_binding.hash(&mut hasher);
                    binding.artifact_id.hash(&mut hasher);
                    binding.artifact_path.hash(&mut hasher);
                    binding.texture_kind.hash(&mut hasher);
                    binding.extent_width.hash(&mut hasher);
                    binding.extent_height.hash(&mut hasher);
                    binding.extent_depth.hash(&mut hasher);
                    binding.cache_key.hash(&mut hasher);
                    binding.sampler_policy.hash(&mut hasher);
                    binding.texture_dimension.hash(&mut hasher);
                    binding.residency_identity.hash(&mut hasher);
                    binding.artifact_revision.hash(&mut hasher);
                    binding.descriptor_hash.hash(&mut hasher);
                    binding.pixel_format.hash(&mut hasher);
                    binding.supercompression.hash(&mut hasher);
                    binding.container_byte_length.hash(&mut hasher);
                }
            }
        }
        crate::plugins::render::PreparedFeaturePayload::Deformation(value) => {
            "deformation".hash(&mut hasher);
            value.streams.len().hash(&mut hasher);
            for stream in &value.streams {
                stream.stream_id.hash(&mut hasher);
                stream.input_pose_ref.hash(&mut hasher);
                stream.output_buffer_ref.hash(&mut hasher);
            }
        }
        crate::plugins::render::PreparedFeaturePayload::Registered(value) => {
            "registered".hash(&mut hasher);
            value.kind().hash(&mut hasher);
            value.runtime_signature().hash(&mut hasher);
        }
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::{
        FeatureContributionStatus, FeatureFallbackPolicy, PreparedFeatureContribution,
        PreparedFeaturePayload, PreparedMaterialBindingTable, PreparedMaterialFeatureContribution,
        PreparedMaterialInstanceInput, PreparedMaterialOutputTarget,
        PreparedMaterialParameterInput, PreparedMaterialParameterKind,
        PreparedMaterialParameterPayloadV1, PreparedMaterialParameterProfile,
        PreparedMaterialTextureBinding, PreparedMaterialTextureBindingLocation,
        PreparedMaterialTextureKind,
    };
    use runen_gpu::{GpuTextureDimension, GpuTransferRegion};

    #[test]
    fn pending_buffer_upload_retains_exact_logical_destination_and_payload() {
        let mut resource_ids = GpuWorkResourceIdAllocator::new();
        let buffer = resource_ids
            .allocate_buffer_handle(
                super::super::resource_descriptors::buffer_descriptor(
                    "pending buffer upload test",
                    16,
                    [GpuBufferUsage::CopyDestination],
                    GpuResourceLifetime::Transient,
                    GpuMemoryIntent::Device,
                )
                .expect("test buffer descriptor should be valid"),
            )
            .expect("test buffer handle should allocate");
        let operation = canonical_renderer_buffer_upload(&buffer, &[1_u8, 2, 3, 4])
            .expect("pending buffer upload should become canonical");

        let GpuTransferRegion::Buffer(destination) = operation.destination() else {
            panic!("pending buffer upload should retain a buffer destination");
        };
        assert_eq!(
            destination.buffer().diagnostic_identity(),
            buffer.diagnostic_identity()
        );
        assert_eq!(destination.range().offset(), 0);
        assert_eq!(destination.range().size(), 4);
        assert_eq!(operation.payload().as_bytes(), &[1_u8, 2, 3, 4]);
    }

    #[test]
    fn pending_texture_upload_retains_exact_logical_destination_and_payload() {
        let mut resource_ids = GpuWorkResourceIdAllocator::new();
        let texture = resource_ids
            .allocate_texture_handle(
                texture_descriptor_with_extent(
                    "pending texture upload test",
                    GpuTextureDimension::D2,
                    (2, 1, 1),
                    runen_gpu::GpuTextureFormat::Rgba8Unorm,
                    [GpuTextureUsage::CopyDestination],
                    GpuResourceLifetime::Transient,
                )
                .expect("test texture descriptor should be valid"),
            )
            .expect("test texture handle should allocate");
        let bytes = [1_u8, 2, 3, 4, 5, 6, 7, 8];
        let operation = canonical_renderer_texture_upload(&texture, &bytes)
            .expect("pending texture upload should become canonical");

        let GpuTransferRegion::Texture(destination) = operation.destination() else {
            panic!("pending texture upload should retain a texture destination");
        };
        assert_eq!(
            destination.texture().diagnostic_identity(),
            texture.diagnostic_identity()
        );
        assert_eq!(destination.mip_level(), 0);
        assert_eq!(destination.origin(), GpuTextureOrigin::new(0, 0, 0));
        assert_eq!(destination.extent().width(), 2);
        assert_eq!(destination.extent().height(), 1);
        assert_eq!(destination.extent().depth_or_layers(), 1);
        assert_eq!(operation.payload().as_bytes(), bytes.as_slice());
    }

    #[test]
    fn material_contribution_hash_uses_encoded_typed_parameter_payload() {
        fn contribution_with_parameter(key: &str) -> PreparedFeatureContribution {
            PreparedFeatureContribution {
                status: FeatureContributionStatus::Ready,
                fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
                payload: PreparedFeaturePayload::Material(PreparedMaterialFeatureContribution {
                    instances: vec![PreparedMaterialInstanceInput {
                        material_instance_id: "material.product.1".to_string(),
                        specialization_key_fragment: "material.first_slice".to_string(),
                        parameter_payload: PreparedMaterialParameterPayloadV1::new(
                            PreparedMaterialParameterProfile::RenderMaterial,
                            PreparedMaterialOutputTarget::RenderMaterial,
                            [PreparedMaterialParameterInput::new(
                                key,
                                PreparedMaterialParameterKind::Scalar,
                            )],
                        ),
                        texture_bindings: Vec::new(),
                    }],
                    binding_table: PreparedMaterialBindingTable::default(),
                    scene_bundle: None,
                    model_mesh_material_selections: Vec::new(),
                }),
            }
        }

        let roughness = contribution_with_parameter("roughness");
        let metallic = contribution_with_parameter("metallic");

        assert_ne!(
            hash_prepared_feature_contribution(&roughness),
            hash_prepared_feature_contribution(&metallic)
        );
        let PreparedFeaturePayload::Material(payload) = &roughness.payload else {
            panic!("expected material payload");
        };
        assert!(
            String::from_utf8_lossy(&payload.instances[0].parameter_payload.encode_v1())
                .contains("roughness")
        );
    }

    #[test]
    fn rect_batch_grouping_preserves_non_consecutive_order() {
        let a = FlattenedUiRectInstance {
            raw: RectInstanceRaw {
                rect: [0.0, 0.0, 10.0, 10.0],
                color: [1.0, 1.0, 1.0, 1.0],
                radius: 0.0,
                border_width: 0.0,
                _pad: [0.0; 2],
            },
            clip: Some([0.0, 0.0, 10.0, 10.0]),
            submission_order: 0,
            surface_order: 0,
            layer_order: 0,
            primitive_order: 1,
        };
        let b = FlattenedUiRectInstance {
            raw: RectInstanceRaw {
                rect: [20.0, 0.0, 10.0, 10.0],
                color: [1.0, 1.0, 1.0, 1.0],
                radius: 0.0,
                border_width: 0.0,
                _pad: [0.0; 2],
            },
            clip: Some([20.0, 0.0, 10.0, 10.0]),
            submission_order: 0,
            surface_order: 0,
            layer_order: 0,
            primitive_order: 3,
        };
        let c = FlattenedUiRectInstance {
            raw: RectInstanceRaw {
                rect: [1.0, 1.0, 8.0, 8.0],
                color: [1.0, 1.0, 1.0, 1.0],
                radius: 0.0,
                border_width: 0.0,
                _pad: [0.0; 2],
            },
            clip: Some([0.0, 0.0, 10.0, 10.0]),
            submission_order: 0,
            surface_order: 0,
            layer_order: 0,
            primitive_order: 5,
        };
        let grouped = group_rect_batches_ordered(vec![a, b, c], 100, 100);
        assert_eq!(grouped.len(), 3);
        assert_eq!(grouped[0].instances.len(), 1);
        assert_eq!(grouped[1].instances.len(), 1);
        assert_eq!(grouped[2].instances.len(), 1);
    }

    #[test]
    fn rect_batch_grouping_splits_submission_and_surface_boundaries() {
        let make_instance =
            |submission_order, surface_order, primitive_order| FlattenedUiRectInstance {
                raw: RectInstanceRaw {
                    rect: [primitive_order as f32, 0.0, 10.0, 10.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    radius: 0.0,
                    border_width: 0.0,
                    _pad: [0.0; 2],
                },
                clip: Some([0.0, 0.0, 100.0, 100.0]),
                submission_order,
                surface_order,
                layer_order: 0,
                primitive_order,
            };

        let grouped = group_rect_batches_ordered(
            vec![
                make_instance(0, 0, 0),
                make_instance(0, 0, 1),
                make_instance(0, 1, 2),
                make_instance(1, 1, 3),
            ],
            100,
            100,
        );

        assert_eq!(grouped.len(), 3);
        assert_eq!(grouped[0].order.submission_order, 0);
        assert_eq!(grouped[0].order.surface_order, 0);
        assert_eq!(grouped[0].instances.len(), 2);
        assert_eq!(grouped[1].order.surface_order, 1);
        assert_eq!(grouped[1].instances.len(), 1);
        assert_eq!(grouped[2].order.submission_order, 1);
        assert_eq!(grouped[2].instances.len(), 1);
    }

    #[test]
    fn draw_order_key_prioritizes_submission_surface_and_layer_before_family() {
        let glyph_before_later_surface_rect = UiBatchOrder::new(0, 0, 0, 8).sort_key(3);
        let rect_on_later_surface = UiBatchOrder::new(0, 1, 0, 0).sort_key(0);
        let rect_on_later_submission = UiBatchOrder::new(1, 0, 0, 0).sort_key(0);

        assert!(glyph_before_later_surface_rect < rect_on_later_surface);
        assert!(rect_on_later_surface < rect_on_later_submission);
    }

    #[test]
    fn material_ktx2_upload_reads_exact_base_level_bytes() {
        let bytes = build_rgba8_ktx2(2, 2, 1, [12, 34, 56, 255]);
        let path = std::env::temp_dir().join(format!(
            "runenwerk-material-ktx2-upload-{}.ktx2",
            std::process::id()
        ));
        std::fs::write(&path, &bytes).expect("test ktx2 should write");
        let binding = PreparedMaterialTextureBinding::new(
            7,
            "albedo",
            PreparedMaterialTextureBindingLocation::new(0, 1, 0, 1),
            "artifact.7",
            path.to_string_lossy(),
            PreparedMaterialTextureKind::Texture2D,
            "cache",
        )
        .with_extent(2, 2, 1)
        .with_descriptor_hash("descriptor-hash")
        .with_ktx2_contract("Rgba8Unorm", "None", Some(bytes.len() as u64));

        let upload = load_material_ktx2_upload(&binding).expect("ktx2 upload should load");

        assert_eq!(upload.size.width(), 2);
        assert_eq!(upload.size.height(), 2);
        assert_eq!(upload.size.depth_or_layers(), 1);
        assert_eq!(upload.format, GpuTextureFormat::Rgba8Unorm);
        assert_eq!(&upload.bytes[0..4], &[12, 34, 56, 255]);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn material_ktx2_upload_rejects_byte_length_mismatch() {
        let bytes = build_rgba8_ktx2(1, 1, 1, [1, 2, 3, 255]);
        let path = std::env::temp_dir().join(format!(
            "runenwerk-material-ktx2-mismatch-{}.ktx2",
            std::process::id()
        ));
        std::fs::write(&path, &bytes).expect("test ktx2 should write");
        let binding = PreparedMaterialTextureBinding::new(
            7,
            "albedo",
            PreparedMaterialTextureBindingLocation::new(0, 1, 0, 1),
            "artifact.7",
            path.to_string_lossy(),
            PreparedMaterialTextureKind::Texture2D,
            "cache",
        )
        .with_extent(1, 1, 1)
        .with_descriptor_hash("descriptor-hash")
        .with_ktx2_contract("Rgba8Unorm", "None", Some(bytes.len() as u64 + 1));

        let error = load_material_ktx2_upload(&binding).expect_err("length mismatch should fail");

        assert!(error.to_string().contains("byte length"));
        let _ = std::fs::remove_file(path);
    }

    fn build_rgba8_ktx2(width: u32, height: u32, depth: u32, texel: [u8; 4]) -> Vec<u8> {
        let format = ktx2::Format::R8G8B8A8_UNORM;
        let (basic, type_size) =
            ktx2::dfd::Basic::from_format(format).expect("rgba8 dfd should build");
        let dfd_block = ktx2::dfd::Block::Basic(basic);
        let dfd_block_bytes = dfd_block.to_vec();
        let dfd_total_size = 4 + dfd_block_bytes.len();
        let level_index_offset = ktx2::Header::LENGTH;
        let dfd_offset = level_index_offset + ktx2::LevelIndex::LENGTH;
        let after_dfd = dfd_offset + dfd_total_size;
        let level_data_offset = after_dfd.div_ceil(4) * 4;
        let texel_count = width as usize * height as usize * depth.max(1) as usize;
        let level_data_size = texel_count * 4;
        let mut bytes = vec![0u8; level_data_offset + level_data_size];

        let header = ktx2::Header {
            format: Some(format),
            type_size,
            pixel_width: width,
            pixel_height: height,
            pixel_depth: if depth > 1 { depth } else { 0 },
            layer_count: 0,
            face_count: 1,
            level_count: 1,
            supercompression_scheme: None,
            index: ktx2::Index {
                dfd_byte_offset: dfd_offset as u32,
                dfd_byte_length: dfd_total_size as u32,
                kvd_byte_offset: 0,
                kvd_byte_length: 0,
                sgd_byte_offset: 0,
                sgd_byte_length: 0,
            },
        };
        bytes[..ktx2::Header::LENGTH].copy_from_slice(&header.as_bytes());
        let level_index = ktx2::LevelIndex {
            byte_offset: level_data_offset as u64,
            byte_length: level_data_size as u64,
            uncompressed_byte_length: level_data_size as u64,
        };
        bytes[level_index_offset..level_index_offset + ktx2::LevelIndex::LENGTH]
            .copy_from_slice(&level_index.as_bytes());
        bytes[dfd_offset..dfd_offset + 4].copy_from_slice(&(dfd_total_size as u32).to_le_bytes());
        bytes[dfd_offset + 4..dfd_offset + 4 + dfd_block_bytes.len()]
            .copy_from_slice(&dfd_block_bytes);
        for index in 0..texel_count {
            let start = level_data_offset + index * 4;
            bytes[start..start + 4].copy_from_slice(&texel);
        }
        bytes
    }
}
