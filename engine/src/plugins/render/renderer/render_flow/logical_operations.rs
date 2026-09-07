use super::logical_timing::LogicalGpuPassTiming;
use super::*;
use crate::plugins::render::graph::CompiledDrawSource;
use crate::plugins::render::{RenderDepthPolicy, RenderIndirectDrawArgsKind};
use runen_gpu::{
    GpuAttachmentStore, GpuBlendConstant, GpuBufferHandle, GpuBufferRange, GpuBufferRegion,
    GpuColorAttachmentLoad, GpuColorClearValue, GpuComputeOperation, GpuDepthAttachmentLoad,
    GpuDepthClearValue, GpuDepthStencilAccess, GpuDispatchIntent, GpuDispatchSize, GpuDrawIntent,
    GpuDrawRange, GpuIndexBufferBinding, GpuIndexFormat, GpuQueryRange, GpuQueryResolveOperation,
    GpuReadbackId, GpuReadbackOperation, GpuRenderColorAttachment, GpuRenderDepthStencilAttachment,
    GpuRenderDraw, GpuRenderOperation, GpuScissorRect, GpuTextureViewHandle, GpuTimestampWrites,
    GpuUploadOperation, GpuVertexBufferBinding, GpuViewport, GpuWorkOperation, PreparedGpuData,
    TransferData,
};

/// Projects one already-realized compute pass into its execution-complete RunenGPU operation.
///
/// This function does not inspect compiled binding declarations or reconstruct accesses. The
/// exact pipeline descriptor and complete logical runtime binding set are consumed from G4C2/C3
/// realization, and all access/capability meaning is subsequently derived by `GpuWorkOperation`.
pub(super) fn project_compute_operation(
    pass: &CompiledComputeExecutionPlan,
    flow_inputs: &PreparedFlowInputs,
    pipeline: &PreparedPipelinePass,
    timing: Option<(&LogicalGpuPassTiming, GpuPassTimestampIndices)>,
) -> Result<GpuWorkOperation> {
    let PreparedFlowPipeline::Compute(realized_pipeline) = &pipeline.pipeline else {
        bail!(
            "compute pass '{}' realized a non-compute GPU pipeline before G5A operation projection",
            pass.pass_id
        );
    };
    let dispatch = flow_inputs
        .projected_dispatch_workgroups
        .get(&pass.pass_id)
        .copied()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "compute pass '{}' is missing its prepared dispatch projection",
                pass.pass_id
            )
        })?;
    let dispatch =
        GpuDispatchIntent::direct(GpuDispatchSize::new(dispatch[0], dispatch[1], dispatch[2]));
    let mut operation = GpuComputeOperation::new(
        realized_pipeline.descriptor().clone(),
        pipeline.bindings.runtime_bindings.clone(),
        dispatch,
    )?;
    if let Some((timing, indices)) = timing {
        operation = operation.with_timestamp_writes(timestamp_writes(timing, indices)?);
    }
    Ok(GpuWorkOperation::Compute(operation))
}

/// Projects one already-realized raster pass into canonical RunenGPU work.
///
/// The optional surface view is the exact logical view owned by an acquired G7A surface image.
/// When it is absent, `SurfaceColor` remains residual exactly as it did before G7A. Dynamic-target
/// compatibility paths and unsupported surface-depth imports remain residual rather than receiving
/// invented placeholder identities.
pub(super) fn project_render_operation(
    runtime_resources: &FlowRuntimeResources,
    pass: &CompiledPassExecutionPlan,
    pipeline: &PreparedPipelinePass,
    surface_color_view: Option<&GpuTextureViewHandle>,
    timing: Option<(&LogicalGpuPassTiming, GpuPassTimestampIndices)>,
) -> Result<Option<GpuWorkOperation>> {
    let raster = match pass {
        CompiledPassExecutionPlan::Fullscreen(value)
        | CompiledPassExecutionPlan::Graphics(value) => value,
        _ => bail!(
            "pass '{}' is not a raster pass for G5A render-operation projection",
            execution_pass_id(pass)
        ),
    };
    let PreparedFlowPipeline::Render(realized_pipeline) = &pipeline.pipeline else {
        bail!(
            "raster pass '{}' realized a non-render GPU pipeline before G5A operation projection",
            raster.pass_id
        );
    };

    let Some(color_target_ref) = raster.targets.color_outputs.first() else {
        bail!("raster pass '{}' has no color output", raster.pass_id);
    };
    if raster.targets.color_outputs.len() != 1 {
        bail!(
            "raster pass '{}' has {} color outputs; current renderer execution supports exactly one",
            raster.pass_id,
            raster.targets.color_outputs.len()
        );
    }
    let color_key =
        runtime_resources.resolve_resource_key(raster.pass_id, color_target_ref, "color_output")?;
    let Some(color_target) =
        logical_texture_target(runtime_resources, &color_key, surface_color_view)
    else {
        return Ok(None);
    };
    if color_target.is_depth {
        bail!(
            "raster pass '{}' resolved a depth texture as its color output",
            raster.pass_id
        );
    }

    let color_load = match raster.clear_color {
        Some(color) => {
            GpuColorAttachmentLoad::Clear(GpuColorClearValue::from_array(color.map(f64::from))?)
        }
        None => GpuColorAttachmentLoad::Load,
    };
    let color_attachment = GpuRenderColorAttachment::new(
        color_target.view.clone(),
        color_load,
        GpuAttachmentStore::Store,
        None,
    )?;

    let depth_attachment = if matches!(
        raster.raster_state.state.depth_policy,
        RenderDepthPolicy::Disabled
    ) {
        None
    } else if let Some(depth_ref) = raster.targets.depth_output.as_ref() {
        let depth_key =
            runtime_resources.resolve_resource_key(raster.pass_id, depth_ref, "depth_output")?;
        let Some(depth_target) = logical_texture_target(runtime_resources, &depth_key, None) else {
            return Ok(None);
        };
        if !depth_target.is_depth {
            bail!(
                "raster pass '{}' resolved a color texture as its depth output",
                raster.pass_id
            );
        }
        let read_only = matches!(
            raster.raster_state.state.depth_policy,
            RenderDepthPolicy::ReadOnly
        );
        Some(GpuRenderDepthStencilAttachment::new(
            depth_target.view.clone(),
            if read_only {
                GpuDepthStencilAccess::ReadOnly
            } else {
                GpuDepthStencilAccess::ReadWrite
            },
            if read_only {
                GpuDepthAttachmentLoad::Load
            } else {
                GpuDepthAttachmentLoad::Clear(GpuDepthClearValue::new(1.0)?)
            },
            GpuAttachmentStore::Store,
        )?)
    } else {
        None
    };

    let draw = project_render_draw(
        runtime_resources,
        pass,
        realized_pipeline.descriptor().clone(),
        pipeline.bindings.runtime_bindings.clone(),
        color_target.size,
    )?;
    let timestamp_writes = timing
        .map(|(timing, indices)| timestamp_writes(timing, indices))
        .transpose()?;
    Ok(Some(GpuWorkOperation::Render(GpuRenderOperation::new(
        [color_attachment],
        depth_attachment,
        [draw],
        timestamp_writes,
    )?)))
}

fn project_render_draw(
    runtime_resources: &FlowRuntimeResources,
    pass: &CompiledPassExecutionPlan,
    pipeline: runen_gpu::GpuRenderPipelineDescriptor,
    bindings: runen_gpu::GpuRuntimeBindingSet,
    target_size: (u32, u32),
) -> Result<GpuRenderDraw> {
    let pass_id = execution_pass_id(pass);
    let (vertex_buffers, index_buffer, draw) = match pass {
        CompiledPassExecutionPlan::Fullscreen(_) => (
            Vec::new(),
            None,
            GpuDrawIntent::direct(GpuDrawRange::new(0, 3)?, GpuDrawRange::new(0, 1)?),
        ),
        CompiledPassExecutionPlan::Graphics(plan) => {
            let mut vertex_buffers = Vec::new();
            for binding in &plan.draw_buffers.vertex_buffers {
                let buffer =
                    runtime_resources.resolve_storage_buffer_ref(pass_id, &binding.resource)?;
                vertex_buffers.push(GpuVertexBufferBinding::new(
                    binding.layout.slot,
                    buffer.handle,
                    GpuBufferRange::whole(buffer.handle)?,
                )?);
            }
            for (resource, layout) in plan
                .draw_buffers
                .instance_buffers
                .iter()
                .zip(plan.draw_buffers.instance_buffer_layouts.iter())
            {
                let buffer = runtime_resources.resolve_storage_buffer_ref(pass_id, resource)?;
                vertex_buffers.push(GpuVertexBufferBinding::new(
                    layout.slot,
                    buffer.handle,
                    GpuBufferRange::whole(buffer.handle)?,
                )?);
            }
            let index_buffer = match plan.draw_buffers.index_buffers.as_slice() {
                [] => None,
                [only] => {
                    let buffer = runtime_resources.resolve_storage_buffer_ref(pass_id, only)?;
                    Some(GpuIndexBufferBinding::new(
                        buffer.handle,
                        GpuBufferRange::whole(buffer.handle)?,
                        GpuIndexFormat::Uint32,
                    )?)
                }
                _ => bail!(
                    "graphics pass '{}' declares multiple index buffers; current runtime supports exactly one",
                    pass_id
                ),
            };
            let authored_draw = plan.draw.ok_or_else(|| {
                anyhow::anyhow!(
                    "graphics pass '{}' is missing draw parameters in execution plan",
                    pass_id
                )
            })?;
            let instances =
                GpuDrawRange::new(authored_draw.first_instance, authored_draw.instance_count)?;
            let draw = match authored_draw.source {
                CompiledDrawSource::Direct if index_buffer.is_some() => GpuDrawIntent::indexed(
                    GpuDrawRange::new(authored_draw.first_vertex, authored_draw.vertex_count)?,
                    0,
                    instances,
                ),
                CompiledDrawSource::Direct => GpuDrawIntent::direct(
                    GpuDrawRange::new(authored_draw.first_vertex, authored_draw.vertex_count)?,
                    instances,
                ),
                CompiledDrawSource::Indirect {
                    args_buffer,
                    args_kind,
                    byte_offset,
                    ..
                } => {
                    let resource = plan
                        .draw_buffers
                        .indirect_buffers
                        .iter()
                        .find(|resource| compiled_resource_ref_matches_id(resource, args_buffer))
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "graphics pass '{}' indirect draw references an argument buffer absent from its compiled indirect set",
                                pass_id
                            )
                        })?;
                    let buffer = runtime_resources.resolve_storage_buffer_ref(pass_id, resource)?;
                    let indexed = matches!(args_kind, RenderIndirectDrawArgsKind::DrawIndexed);
                    let size = if indexed { 20 } else { 16 };
                    GpuDrawIntent::indirect(
                        buffer.handle,
                        GpuBufferRange::new(buffer.handle, byte_offset, size)?,
                        indexed,
                    )?
                }
            };
            (vertex_buffers, index_buffer, draw)
        }
        _ => bail!("pass '{}' is not a raster draw pass", pass_id),
    };

    let width = target_size.0;
    let height = target_size.1;
    Ok(GpuRenderDraw::new(
        pipeline,
        bindings,
        vertex_buffers,
        index_buffer,
        draw,
        GpuViewport::new(0.0, 0.0, width as f32, height as f32, 0.0, 1.0)?,
        GpuScissorRect::new(0, 0, width, height)?,
        GpuBlendConstant::new(0.0, 0.0, 0.0, 0.0)?,
        0,
    )?)
}

struct LogicalTextureTarget {
    view: GpuTextureViewHandle,
    size: (u32, u32),
    is_depth: bool,
}

fn logical_texture_target(
    runtime_resources: &FlowRuntimeResources,
    key: &RuntimeResourceKey,
    surface_color_view: Option<&GpuTextureViewHandle>,
) -> Option<LogicalTextureTarget> {
    if matches!(key, RuntimeResourceKey::SurfaceColor) {
        let view = surface_color_view?;
        let texture = view.descriptor().texture().descriptor();
        let extent = texture.extent();
        return Some(LogicalTextureTarget {
            view: view.clone(),
            size: (extent.width(), extent.height()),
            is_depth: texture.format().is_depth(),
        });
    }

    let texture = match key {
        RuntimeResourceKey::FlowOwned(resource_id) => runtime_resources.textures.get(resource_id),
        RuntimeResourceKey::InvocationHistory {
            invocation_id,
            resource_id,
        } => runtime_resources
            .invocation_history_textures
            .get(&(invocation_id.clone(), *resource_id)),
        RuntimeResourceKey::InvocationUniform { .. }
        | RuntimeResourceKey::DynamicTexture(_)
        | RuntimeResourceKey::SurfaceColor
        | RuntimeResourceKey::SurfaceDepth => None,
    }?;
    Some(LogicalTextureTarget {
        view: texture.view_handle.clone(),
        size: texture.size,
        is_depth: texture.is_depth,
    })
}

fn compiled_resource_ref_matches_id(
    resource: &CompiledResourceRef,
    expected: runen_gpu::GpuWorkResourceId,
) -> bool {
    match resource {
        CompiledResourceRef::FlowOwned(id) | CompiledResourceRef::Imported(id) => *id == expected,
        CompiledResourceRef::TargetAlias(alias) => alias.resource_id == expected,
        CompiledResourceRef::ImportedBuiltin(_) => false,
    }
}

/// Projects an immutable renderer-prepared byte sequence into exact logical CPU→GPU buffer work.
pub(super) fn project_buffer_upload(
    buffer: &GpuBufferHandle,
    bytes: &[u8],
) -> Result<GpuUploadOperation> {
    let size = u64::try_from(bytes.len())
        .map_err(|_| anyhow::anyhow!("render GPU upload length exceeds u64"))?;
    let range = GpuBufferRange::new(buffer, 0, size)?;
    let region = GpuBufferRegion::new(buffer, range)?;
    let payload = PreparedGpuData::<TransferData>::from_pod_transfer(
        format!(
            "{}.render-upload",
            buffer.descriptor().common().label().as_str()
        ),
        bytes,
        buffer.descriptor().common().provenance().clone(),
    )?;
    Ok(GpuUploadOperation::new(region.into(), payload)?)
}

/// Canonical timestamp tail for one expanded renderer invocation.
///
/// The resolve and GPU-to-CPU readback are both first-class RunenGPU operations. Readback result
/// observation follows the accepted G5 submission lifecycle and does not introduce a second copy
/// destination or readback DAG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectedTimingTail {
    resolve: GpuQueryResolveOperation,
    readback: GpuReadbackOperation,
}

impl ProjectedTimingTail {
    fn new(
        query_set: &runen_gpu::GpuQuerySetHandle,
        query_range: GpuQueryRange,
        resolve_buffer: &GpuBufferHandle,
        readback_id: GpuReadbackId,
    ) -> Result<Self> {
        let resolve = GpuQueryResolveOperation::new(query_set, query_range, resolve_buffer, 0)?;
        let byte_len = u64::from(resolve.source_range().count())
            .checked_mul(8)
            .ok_or_else(|| anyhow::anyhow!("render GPU timestamp byte coverage overflow"))?;
        let readback = GpuReadbackOperation::new(
            GpuBufferRegion::new(
                resolve_buffer,
                GpuBufferRange::new(resolve_buffer, 0, byte_len)?,
            )?
            .into(),
            readback_id,
        )?;
        Ok(Self { resolve, readback })
    }

    pub(super) fn resolve(&self) -> &GpuQueryResolveOperation {
        &self.resolve
    }

    pub(super) fn readback(&self) -> &GpuReadbackOperation {
        &self.readback
    }
}

pub(super) fn project_timing_tail(
    query_set: &runen_gpu::GpuQuerySetHandle,
    query_range: GpuQueryRange,
    resolve_buffer: &GpuBufferHandle,
    readback_id: GpuReadbackId,
) -> Result<ProjectedTimingTail> {
    ProjectedTimingTail::new(query_set, query_range, resolve_buffer, readback_id)
}

pub(super) fn timestamp_writes(
    timing: &LogicalGpuPassTiming,
    indices: GpuPassTimestampIndices,
) -> Result<GpuTimestampWrites> {
    Ok(GpuTimestampWrites::new(
        timing.query_set(),
        Some(indices.begin),
        Some(indices.end),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::renderer::resource_descriptors::{
        texture_descriptor, whole_texture_view_descriptor,
    };
    use runen_gpu::{
        GpuResourceLifetime, GpuTextureFormat, GpuTextureUsage, GpuWorkResourceIdAllocator,
    };

    #[test]
    fn supplied_surface_color_view_is_the_logical_render_target() {
        let mut allocator = GpuWorkResourceIdAllocator::new();
        let texture = allocator
            .allocate_texture_handle(
                texture_descriptor(
                    "canonical surface color texture",
                    (37, 19),
                    GpuTextureFormat::Bgra8UnormSrgb,
                    [GpuTextureUsage::ColorAttachment],
                    GpuResourceLifetime::Transient,
                )
                .expect("surface-like texture descriptor should be valid"),
            )
            .expect("surface-like texture handle should allocate");
        let view = allocator
            .allocate_texture_view_handle(
                whole_texture_view_descriptor("canonical surface color view", &texture)
                    .expect("surface-like view descriptor should be valid"),
            )
            .expect("surface-like view handle should allocate");
        let runtime_resources = FlowRuntimeResources::default();

        let target = logical_texture_target(
            &runtime_resources,
            &RuntimeResourceKey::SurfaceColor,
            Some(&view),
        )
        .expect("supplied surface color view should become a logical render target");

        assert_eq!(target.view.descriptor(), view.descriptor());
        assert_eq!(target.size, (37, 19));
        assert!(!target.is_depth);
        assert!(
            logical_texture_target(&runtime_resources, &RuntimeResourceKey::SurfaceColor, None,)
                .is_none(),
            "surface color remains residual until the acquired G7A view is supplied"
        );
    }
}
