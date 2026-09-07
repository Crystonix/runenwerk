use super::super::dynamic_targets::RendererDynamicTextureTargetCache;
use super::*;
use crate::plugins::render::RenderPassId;
use runen_gpu::{
    GpuBufferRange, GpuBufferRegion, GpuCopyExtent, GpuCopyOperation, GpuTextureAspect,
    GpuTextureCopyRegion, GpuTextureHandle, GpuTextureOrigin, GpuTextureViewHandle,
    GpuWorkOperation,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ProjectedCopyOperation {
    /// Source and destination resolve to the same runtime resource, so no GPU operation exists.
    NoWork,
    /// Fully logical transfer semantics owned by RunenGPU.
    Canonical(Box<GpuWorkOperation>),
    /// At least one endpoint still lacks durable logical GPU identity before the live G7A cutover.
    PreG7Residual,
}

/// Projects one resolved renderer copy into canonical RunenGPU work or an explicit pre-G7 residual.
///
/// Buffer/texture class mismatches remain invalid. The current renderer does not implement
/// buffer-texture copies, so G5A does not invent that execution meaning here. Dynamic texture
/// endpoints become canonical only when the frame resolver supplies the existing target cache; the
/// transitional per-invocation path deliberately withholds it. Surface endpoints remain residual
/// until their exact acquired G7A handle is supplied by the frame caller.
pub(super) fn project_copy_operation(
    runtime_resources: &FlowRuntimeResources,
    dynamic_texture_targets: Option<&RendererDynamicTextureTargetCache>,
    pass: &CompiledCopyExecutionPlan,
) -> Result<ProjectedCopyOperation> {
    let source = pass.source.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "copy pass '{}' is missing source resource in execution plan",
            pass.pass_id
        )
    })?;
    let destination = pass.destination.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "copy pass '{}' is missing destination resource in execution plan",
            pass.pass_id
        )
    })?;
    let source_key = runtime_resources.resolve_resource_key(pass.pass_id, source, "copy_source")?;
    let destination_key =
        runtime_resources.resolve_resource_key(pass.pass_id, destination, "copy_destination")?;
    if source_key == destination_key {
        return Ok(ProjectedCopyOperation::NoWork);
    }

    let source_kind = runtime_resources
        .kind_of_resource(source_key.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "copy pass '{}' references unknown source resource '{}'",
                pass.pass_id,
                source_key
            )
        })?;
    let destination_kind = runtime_resources
        .kind_of_resource(destination_key.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "copy pass '{}' references unknown destination resource '{}'",
                pass.pass_id,
                destination_key
            )
        })?;

    let operation = match (source_kind, destination_kind) {
        (RuntimeResourceKind::BufferLike, RuntimeResourceKind::BufferLike) => {
            let source = runtime_resources.resolve_buffer_key(pass.pass_id, source_key)?;
            let destination =
                runtime_resources.resolve_buffer_key(pass.pass_id, destination_key)?;
            let size = source.size.min(destination.size);
            if size == 0 {
                anyhow::bail!(
                    "copy pass '{}' resolved buffer copy extent to zero for '{}' -> '{}'",
                    pass.pass_id,
                    source.id,
                    destination.id
                );
            }
            GpuCopyOperation::buffer_to_buffer(
                GpuBufferRegion::new(source.handle, GpuBufferRange::new(source.handle, 0, size)?)?,
                GpuBufferRegion::new(
                    destination.handle,
                    GpuBufferRange::new(destination.handle, 0, size)?,
                )?,
            )?
        }
        (RuntimeResourceKind::BufferLike, RuntimeResourceKind::TextureLike)
        | (RuntimeResourceKind::TextureLike, RuntimeResourceKind::BufferLike) => {
            anyhow::bail!(
                "copy pass '{}' mixes incompatible resource classes '{}' -> '{}'",
                pass.pass_id,
                source_key,
                destination_key
            );
        }
        (RuntimeResourceKind::TextureLike, RuntimeResourceKind::TextureLike) => {
            let Some(source) = logical_texture(
                runtime_resources,
                dynamic_texture_targets,
                pass.pass_id,
                &source_key,
            )?
            else {
                return Ok(ProjectedCopyOperation::PreG7Residual);
            };
            let Some(destination) = logical_texture(
                runtime_resources,
                dynamic_texture_targets,
                pass.pass_id,
                &destination_key,
            )?
            else {
                return Ok(ProjectedCopyOperation::PreG7Residual);
            };
            project_texture_to_texture(
                pass.pass_id,
                source_key,
                source,
                destination_key,
                destination,
            )?
        }
    };
    Ok(ProjectedCopyOperation::Canonical(Box::new(
        GpuWorkOperation::Copy(operation),
    )))
}

/// Projects the current render-domain Present pass semantics.
///
/// `CompiledPresentExecutionPlan` does not consume the presentation lease. It only ensures that its
/// selected texture is copied into `SurfaceColor`; the one frame-terminal `GpuPresentOperation`
/// remains a separate G5C operation. A source that is already `SurfaceColor` is therefore true
/// no-work. The destination comes only from the exact acquired G7A default view supplied by the
/// frame caller.
pub(super) fn project_present_copy_operation(
    runtime_resources: &FlowRuntimeResources,
    dynamic_texture_targets: Option<&RendererDynamicTextureTargetCache>,
    pass: &CompiledPresentExecutionPlan,
    surface_color_view: Option<&GpuTextureViewHandle>,
) -> Result<ProjectedCopyOperation> {
    let source = pass.source.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "present pass '{}' is missing source resource in execution plan",
            pass.pass_id
        )
    })?;
    let source_key =
        runtime_resources.resolve_resource_key(pass.pass_id, source, "present_source")?;
    if source_key == RuntimeResourceKey::SurfaceColor {
        return Ok(ProjectedCopyOperation::NoWork);
    }

    let source_kind = runtime_resources
        .kind_of_resource(source_key.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "present pass '{}' references unknown source resource '{}'",
                pass.pass_id,
                source_key
            )
        })?;
    if matches!(source_kind, RuntimeResourceKind::BufferLike) {
        anyhow::bail!(
            "present pass '{}' reads buffer-like resource '{}' but present requires a texture-like source",
            pass.pass_id,
            source_key
        );
    }

    let Some(source) = logical_texture(
        runtime_resources,
        dynamic_texture_targets,
        pass.pass_id,
        &source_key,
    )?
    else {
        return Ok(ProjectedCopyOperation::PreG7Residual);
    };
    let Some(surface_color_view) = surface_color_view else {
        return Ok(ProjectedCopyOperation::PreG7Residual);
    };
    let destination_texture = surface_color_view.descriptor().texture();
    let destination_extent = destination_texture.descriptor().extent();
    let destination = LogicalCopyTexture {
        handle: destination_texture,
        size: (destination_extent.width(), destination_extent.height()),
        is_depth: destination_texture.descriptor().format().is_depth(),
    };
    let operation = project_texture_to_texture(
        pass.pass_id,
        source_key,
        source,
        RuntimeResourceKey::SurfaceColor,
        destination,
    )?;
    Ok(ProjectedCopyOperation::Canonical(Box::new(
        GpuWorkOperation::Copy(operation),
    )))
}

fn project_texture_to_texture(
    pass_id: RenderPassId,
    source_key: RuntimeResourceKey,
    source: LogicalCopyTexture<'_>,
    destination_key: RuntimeResourceKey,
    destination: LogicalCopyTexture<'_>,
) -> Result<GpuCopyOperation> {
    if source.is_depth || destination.is_depth {
        anyhow::bail!(
            "copy pass '{}' requested unsupported depth copy '{}' -> '{}'; only color-like texture copies are supported",
            pass_id,
            source_key,
            destination_key
        );
    }
    let width = source.size.0.min(destination.size.0);
    let height = source.size.1.min(destination.size.1);
    if width == 0 || height == 0 {
        anyhow::bail!(
            "copy pass '{}' resolved texture copy extent to zero for '{}' -> '{}'",
            pass_id,
            source_key,
            destination_key
        );
    }
    let extent = GpuCopyExtent::new(width, height, 1)?;
    Ok(GpuCopyOperation::texture_to_texture(
        GpuTextureCopyRegion::new(
            source.handle,
            0,
            GpuTextureOrigin::new(0, 0, 0),
            GpuTextureAspect::Color,
            extent,
        )?,
        GpuTextureCopyRegion::new(
            destination.handle,
            0,
            GpuTextureOrigin::new(0, 0, 0),
            GpuTextureAspect::Color,
            extent,
        )?,
    )?)
}

struct LogicalCopyTexture<'a> {
    handle: &'a GpuTextureHandle,
    size: (u32, u32),
    is_depth: bool,
}

fn logical_texture<'a>(
    runtime_resources: &'a FlowRuntimeResources,
    dynamic_texture_targets: Option<&'a RendererDynamicTextureTargetCache>,
    pass_id: RenderPassId,
    key: &RuntimeResourceKey,
) -> Result<Option<LogicalCopyTexture<'a>>> {
    match key {
        RuntimeResourceKey::FlowOwned(resource_id) => Ok(runtime_resources
            .textures
            .get(resource_id)
            .map(|texture| LogicalCopyTexture {
                handle: &texture.handle,
                size: texture.size,
                is_depth: texture.is_depth,
            })),
        RuntimeResourceKey::InvocationHistory {
            invocation_id,
            resource_id,
        } => Ok(runtime_resources
            .invocation_history_textures
            .get(&(invocation_id.clone(), *resource_id))
            .map(|texture| LogicalCopyTexture {
                handle: &texture.handle,
                size: texture.size,
                is_depth: texture.is_depth,
            })),
        RuntimeResourceKey::DynamicTexture(dynamic_key) => {
            let Some(dynamic_texture_targets) = dynamic_texture_targets else {
                return Ok(None);
            };
            let resolved = dynamic_texture_targets.texture_ref(pass_id, dynamic_key)?;
            let view = resolved.view_handle.ok_or_else(|| {
                anyhow::anyhow!(
                    "dynamic texture target '{}' has no logical RunenGPU view handle",
                    dynamic_key
                )
            })?;
            let texture = view.descriptor().texture();
            let descriptor = texture.descriptor();
            let extent = descriptor.extent();
            Ok(Some(LogicalCopyTexture {
                handle: texture,
                size: (extent.width(), extent.height()),
                is_depth: descriptor.format().is_depth(),
            }))
        }
        RuntimeResourceKey::InvocationUniform { .. }
        | RuntimeResourceKey::SurfaceColor
        | RuntimeResourceKey::SurfaceDepth => Ok(None),
    }
}
