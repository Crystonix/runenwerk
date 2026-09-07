use super::render_flow::{ResolvedTextureRef, RuntimeResourceKey};
use super::resource_descriptors::{texture_descriptor, whole_texture_view_descriptor};
use crate::plugins::render::{
    RenderDynamicTextureRetention, RenderDynamicTextureTargetDescriptor,
    RenderDynamicTextureTargetKey, RenderDynamicTextureTargetSignature,
    RenderDynamicTextureUploadDescriptor, RenderPassId, RenderTextureSampleMode,
    RenderTextureTargetFormat, RenderTextureTargetUsage, RenderTextureUploadAlphaMode,
};
use anyhow::{Result, bail};
use runen_gpu::{
    GpuContext, GpuCopyExtent, GpuResourceLifetime, GpuTextureAspect, GpuTextureCopyRegion,
    GpuTextureFormat, GpuTextureHandle, GpuTextureOrigin, GpuTextureUsage, GpuTextureViewHandle,
    GpuTransferRegion, GpuUploadOperation, GpuWorkResourceIdAllocator, PreparedGpuData,
    TransferData,
};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct RendererDynamicTextureTarget {
    pub _handle: GpuTextureHandle,
    pub view_handle: GpuTextureViewHandle,
    pub format: GpuTextureFormat,
    pub size: (u32, u32),
    pub descriptor: RenderDynamicTextureTargetDescriptor,
    pub signature: RenderDynamicTextureTargetSignature,
    pub history_signature: Option<String>,
    pub generation: u64,
    pub uploaded_product_generation: Option<u64>,
    pub stale: bool,
    pub last_invalidation_reason: Option<String>,
    unrequested_frames: u32,
}

#[derive(Debug, Default)]
pub struct RendererDynamicTextureTargetCache {
    resource_ids: GpuWorkResourceIdAllocator,
    targets: BTreeMap<RenderDynamicTextureTargetKey, RendererDynamicTextureTarget>,
}

#[derive(Debug, Clone)]
struct RendererPreparedDynamicTextureUpload {
    target_key: RenderDynamicTextureTargetKey,
    product_generation: u64,
    operation: GpuUploadOperation,
}

#[derive(Debug, Clone)]
pub(super) struct RendererAcceptedDynamicTextureUpload {
    target_key: RenderDynamicTextureTargetKey,
    product_generation: u64,
    operation: GpuUploadOperation,
}

impl RendererAcceptedDynamicTextureUpload {
    pub(super) fn operation(&self) -> &GpuUploadOperation {
        &self.operation
    }
}

#[derive(Debug, Default)]
pub(super) struct RendererPreparedDynamicTextureUploadBatch {
    uploads: Vec<RendererPreparedDynamicTextureUpload>,
    diagnostics: Vec<RendererDynamicTextureUploadDiagnostic>,
}

impl RendererDynamicTextureTargetCache {
    pub fn realize_for_frame(
        &mut self,
        _context: &GpuContext,
        requests: &[RenderDynamicTextureTargetDescriptor],
        history_signatures: &BTreeMap<RenderDynamicTextureTargetKey, String>,
    ) -> Result<()> {
        let requested_keys = requests
            .iter()
            .map(|descriptor| descriptor.key.clone())
            .collect::<std::collections::BTreeSet<_>>();

        for descriptor in requests {
            descriptor.validate()?;
            let signature = descriptor.signature();
            let history_signature = history_signatures.get(&descriptor.key).cloned();
            let previous_generation = self
                .targets
                .get(&descriptor.key)
                .map(|target| target.generation)
                .unwrap_or(0);
            let invalidation_reason = self.targets.get(&descriptor.key).and_then(|target| {
                if target.signature != signature {
                    Some("descriptor_signature_changed".to_string())
                } else if target.history_signature != history_signature {
                    Some("history_signature_changed".to_string())
                } else {
                    None
                }
            });
            let should_recreate = self
                .targets
                .get(&descriptor.key)
                .map(|_| invalidation_reason.is_some())
                .unwrap_or(true);
            if should_recreate {
                let label = format!("dynamic_render_target_{}", descriptor.key);
                let handle = self
                    .resource_ids
                    .allocate_texture_handle(texture_descriptor(
                        label.clone(),
                        (descriptor.width.max(1), descriptor.height.max(1)),
                        dynamic_format_to_gpu(descriptor.format),
                        dynamic_usage_to_gpu(descriptor.usage),
                        GpuResourceLifetime::Retained,
                    )?)?;
                let view_handle = self.resource_ids.allocate_texture_view_handle(
                    whole_texture_view_descriptor(format!("{label}_view"), &handle)?,
                )?;
                self.targets.insert(
                    descriptor.key.clone(),
                    RendererDynamicTextureTarget {
                        _handle: handle,
                        view_handle,
                        format: dynamic_format_to_gpu(descriptor.format),
                        size: (descriptor.width.max(1), descriptor.height.max(1)),
                        descriptor: descriptor.clone(),
                        signature,
                        history_signature,
                        generation: previous_generation.saturating_add(1),
                        uploaded_product_generation: None,
                        stale: false,
                        last_invalidation_reason: invalidation_reason,
                        unrequested_frames: 0,
                    },
                );
            } else if let Some(existing) = self.targets.get_mut(&descriptor.key) {
                existing.descriptor = descriptor.clone();
                existing.history_signature = history_signature;
                existing.stale = false;
                existing.last_invalidation_reason = None;
                existing.unrequested_frames = 0;
            }
        }

        self.targets.retain(|key, target| {
            if requested_keys.contains(key) {
                return true;
            }
            target.unrequested_frames = target.unrequested_frames.saturating_add(1);
            target.stale = true;
            target.last_invalidation_reason = Some("not_requested_this_frame".to_string());
            match target.descriptor.retention {
                RenderDynamicTextureRetention::RetainWhileRequested => false,
                RenderDynamicTextureRetention::RetainUntilViewportClose => true,
                RenderDynamicTextureRetention::RetainForFrames(frames) => {
                    target.unrequested_frames <= frames
                }
            }
        });

        Ok(())
    }

    pub(super) fn prepare_uploads(
        &self,
        uploads: &[RenderDynamicTextureUploadDescriptor],
    ) -> RendererPreparedDynamicTextureUploadBatch {
        let mut batch = RendererPreparedDynamicTextureUploadBatch::default();
        for upload in uploads {
            match self.prepare_upload(upload) {
                Ok(prepared) => batch.uploads.push(prepared),
                Err(message) => batch
                    .diagnostics
                    .push(RendererDynamicTextureUploadDiagnostic {
                        target_key: upload.target_key.clone(),
                        message,
                    }),
            }
        }
        batch
    }

    pub(super) fn validate_prepared_uploads(
        &self,
        prepared: RendererPreparedDynamicTextureUploadBatch,
    ) -> (
        Vec<RendererAcceptedDynamicTextureUpload>,
        RendererDynamicTextureUploadReport,
    ) {
        let mut report = RendererDynamicTextureUploadReport {
            applied_count: 0,
            rejected_count: prepared.diagnostics.len(),
            diagnostics: prepared.diagnostics,
        };
        let mut accepted = Vec::with_capacity(prepared.uploads.len());
        for upload in prepared.uploads {
            let validation = self
                .targets
                .get(&upload.target_key)
                .ok_or_else(|| {
                    "prepared dynamic texture upload lost its target before physical application"
                        .to_string()
                })
                .and_then(|target| {
                    validate_prepared_dynamic_texture_target(&target._handle, &upload.operation)
                });
            if let Err(message) = validation {
                report.rejected_count = report.rejected_count.saturating_add(1);
                report
                    .diagnostics
                    .push(RendererDynamicTextureUploadDiagnostic {
                        target_key: upload.target_key,
                        message,
                    });
                continue;
            }

            report.applied_count = report.applied_count.saturating_add(1);
            accepted.push(RendererAcceptedDynamicTextureUpload {
                target_key: upload.target_key,
                product_generation: upload.product_generation,
                operation: upload.operation,
            });
        }
        (accepted, report)
    }

    /// Advances product-generation evidence only after RunenGPU has irreversibly accepted the
    /// frame submission that owns these exact canonical Upload operations.
    pub(super) fn record_accepted_uploads(
        &mut self,
        accepted: &[RendererAcceptedDynamicTextureUpload],
    ) -> RendererDynamicTextureUploadReport {
        let mut report = RendererDynamicTextureUploadReport::default();
        for upload in accepted {
            let validation = self
                .targets
                .get(&upload.target_key)
                .ok_or_else(|| {
                    "accepted dynamic texture upload lost its target before bookkeeping".to_string()
                })
                .and_then(|target| {
                    validate_prepared_dynamic_texture_target(&target._handle, &upload.operation)
                });
            if let Err(message) = validation {
                report.rejected_count = report.rejected_count.saturating_add(1);
                report
                    .diagnostics
                    .push(RendererDynamicTextureUploadDiagnostic {
                        target_key: upload.target_key.clone(),
                        message,
                    });
                continue;
            }
            let target = self
                .targets
                .get_mut(&upload.target_key)
                .expect("validated dynamic texture target remains present");
            target.uploaded_product_generation = Some(upload.product_generation);
            report.applied_count = report.applied_count.saturating_add(1);
        }
        report
    }

    fn prepare_upload(
        &self,
        upload: &RenderDynamicTextureUploadDescriptor,
    ) -> std::result::Result<RendererPreparedDynamicTextureUpload, String> {
        upload.validate().map_err(|err| err.to_string())?;
        let target = self.targets.get(&upload.target_key).ok_or_else(|| {
            format!(
                "dynamic texture upload references missing target '{}'",
                upload.target_key
            )
        })?;
        if !target.descriptor.usage.copy_dst {
            return Err(format!(
                "dynamic texture upload target '{}' is not copy-dst compatible",
                upload.target_key
            ));
        }
        if target.descriptor.format != upload.format {
            return Err(format!(
                "dynamic texture upload target '{}' format {:?} does not match upload format {:?}",
                upload.target_key, target.descriptor.format, upload.format
            ));
        }
        let end_x = upload
            .origin_x
            .checked_add(upload.width)
            .ok_or_else(|| "dynamic texture upload x range overflowed".to_string())?;
        let end_y = upload
            .origin_y
            .checked_add(upload.height)
            .ok_or_else(|| "dynamic texture upload y range overflowed".to_string())?;
        if end_x > target.size.0 || end_y > target.size.1 {
            return Err(format!(
                "dynamic texture upload target '{}' region {}x{} at {},{} exceeds target {}x{}",
                upload.target_key,
                upload.width,
                upload.height,
                upload.origin_x,
                upload.origin_y,
                target.size.0,
                target.size.1
            ));
        }

        let bytes = upload_bytes_for_gpu(upload);
        let operation = canonical_dynamic_texture_upload(target, upload, bytes)
            .map_err(|error| error.to_string())?;
        Ok(RendererPreparedDynamicTextureUpload {
            target_key: upload.target_key.clone(),
            product_generation: upload.product_generation,
            operation,
        })
    }

    pub fn texture_ref<'a>(
        &'a self,
        pass_id: RenderPassId,
        key: &RenderDynamicTextureTargetKey,
    ) -> Result<ResolvedTextureRef<'a>> {
        let target = self.targets.get(key).ok_or_else(|| {
            anyhow::anyhow!(
                "pass '{}' references missing dynamic texture target '{}'",
                pass_id,
                key
            )
        })?;
        Ok(ResolvedTextureRef {
            id: RuntimeResourceKey::DynamicTexture(key.clone()),
            view_handle: Some(&target.view_handle),
            format: target.format,
            size: target.size,
            is_depth: target.descriptor.format.is_depth(),
        })
    }

    pub fn color_target_format(
        &self,
        pass_id: RenderPassId,
        key: &RenderDynamicTextureTargetKey,
    ) -> Result<GpuTextureFormat> {
        let target = self.targets.get(key).ok_or_else(|| {
            anyhow::anyhow!(
                "pass '{}' writes missing dynamic color target '{}'",
                pass_id,
                key
            )
        })?;
        if target.descriptor.format.is_depth() || !target.descriptor.usage.color_attachment {
            bail!(
                "pass '{}' writes dynamic target '{}' as color, but descriptor is not color-attachment compatible",
                pass_id,
                key
            );
        }
        Ok(target.format)
    }

    pub fn depth_target_format(
        &self,
        pass_id: RenderPassId,
        key: &RenderDynamicTextureTargetKey,
    ) -> Result<GpuTextureFormat> {
        let target = self.targets.get(key).ok_or_else(|| {
            anyhow::anyhow!(
                "pass '{}' writes missing dynamic depth target '{}'",
                pass_id,
                key
            )
        })?;
        if !target.descriptor.format.is_depth() || !target.descriptor.usage.depth_attachment {
            bail!(
                "pass '{}' writes dynamic target '{}' as depth, but descriptor is not depth-attachment compatible",
                pass_id,
                key
            );
        }
        Ok(target.format)
    }

    pub fn ui_texture_view_handle(
        &self,
        key: &RenderDynamicTextureTargetKey,
    ) -> Result<GpuTextureViewHandle> {
        let target = self.targets.get(key).ok_or_else(|| {
            anyhow::anyhow!("ui viewport embed references missing dynamic texture '{key}'")
        })?;
        if !target.descriptor.usage.sampled
            || target.descriptor.sample_mode == RenderTextureSampleMode::NotSampled
        {
            bail!("ui viewport embed cannot sample non-sampleable dynamic texture '{key}'");
        }
        if !target.descriptor.format.is_displayable() {
            bail!("ui viewport embed cannot sample non-displayable dynamic texture '{key}'");
        }
        Ok(target.view_handle.clone())
    }
}

fn validate_prepared_dynamic_texture_target(
    target: &GpuTextureHandle,
    operation: &GpuUploadOperation,
) -> std::result::Result<(), String> {
    let GpuTransferRegion::Texture(region) = operation.destination() else {
        return Err("prepared dynamic texture upload has a non-texture destination".to_string());
    };
    if target.diagnostic_identity() != region.texture().diagnostic_identity() {
        return Err(
            "prepared dynamic texture upload targets a texture that has since been replaced"
                .to_string(),
        );
    }
    Ok(())
}

fn canonical_dynamic_texture_upload(
    target: &RendererDynamicTextureTarget,
    upload: &RenderDynamicTextureUploadDescriptor,
    bytes: Vec<u8>,
) -> std::result::Result<GpuUploadOperation, Box<dyn std::error::Error>> {
    let region = GpuTextureCopyRegion::new(
        &target._handle,
        0,
        GpuTextureOrigin::new(upload.origin_x, upload.origin_y, 0),
        GpuTextureAspect::Color,
        GpuCopyExtent::new(upload.width, upload.height, 1)?,
    )?;
    let common = target._handle.descriptor().common();
    let payload = PreparedGpuData::<TransferData>::from_pod_transfer(
        format!("dynamic texture upload {}", upload.target_key),
        bytes.as_slice(),
        common.provenance().clone(),
    )?;
    Ok(GpuUploadOperation::new(region.into(), payload)?)
}

fn dynamic_usage_to_gpu(usage: RenderTextureTargetUsage) -> Vec<GpuTextureUsage> {
    let mut out = Vec::new();
    if usage.sampled {
        out.push(GpuTextureUsage::Sampled);
    }
    if usage.storage {
        out.push(GpuTextureUsage::StorageWrite);
    }
    if usage.color_attachment {
        out.push(GpuTextureUsage::ColorAttachment);
    }
    if usage.depth_attachment {
        out.push(GpuTextureUsage::DepthStencilAttachment);
    }
    if usage.copy_src {
        out.push(GpuTextureUsage::CopySource);
    }
    if usage.copy_dst {
        out.push(GpuTextureUsage::CopyDestination);
    }
    out
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RendererDynamicTextureUploadReport {
    pub applied_count: usize,
    pub rejected_count: usize,
    pub diagnostics: Vec<RendererDynamicTextureUploadDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RendererDynamicTextureUploadDiagnostic {
    pub target_key: RenderDynamicTextureTargetKey,
    pub message: String,
}

fn upload_bytes_for_gpu(upload: &RenderDynamicTextureUploadDescriptor) -> Vec<u8> {
    match upload.alpha_mode {
        RenderTextureUploadAlphaMode::Straight => upload.rgba8.clone(),
        RenderTextureUploadAlphaMode::Premultiplied => unpremultiply_rgba8(&upload.rgba8),
    }
}

fn unpremultiply_rgba8(bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    for pixel in out.as_chunks_mut::<4>().0 {
        let alpha = pixel[3];
        if alpha == 0 {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
            continue;
        }
        for channel in &mut pixel[..3] {
            let value = (u16::from(*channel) * 255 + u16::from(alpha) / 2) / u16::from(alpha);
            *channel = value.min(255) as u8;
        }
    }
    out
}

pub fn dynamic_format_to_gpu(format: RenderTextureTargetFormat) -> GpuTextureFormat {
    match format {
        RenderTextureTargetFormat::Rgba8Unorm => GpuTextureFormat::Rgba8Unorm,
        RenderTextureTargetFormat::Rgba8UnormSrgb => GpuTextureFormat::Rgba8UnormSrgb,
        RenderTextureTargetFormat::R32Uint => GpuTextureFormat::R32Uint,
        RenderTextureTargetFormat::Depth32Float => GpuTextureFormat::Depth32Float,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logical_texture(
        resource_ids: &mut GpuWorkResourceIdAllocator,
        label: &str,
    ) -> GpuTextureHandle {
        resource_ids
            .allocate_texture_handle(
                texture_descriptor(
                    label,
                    (1, 1),
                    GpuTextureFormat::Rgba8Unorm,
                    [GpuTextureUsage::CopyDestination],
                    GpuResourceLifetime::Transient,
                )
                .expect("test texture descriptor should be valid"),
            )
            .expect("test texture handle should allocate")
    }

    fn logical_upload(texture: &GpuTextureHandle) -> GpuUploadOperation {
        let region = GpuTextureCopyRegion::new(
            texture,
            0,
            GpuTextureOrigin::new(0, 0, 0),
            GpuTextureAspect::Color,
            GpuCopyExtent::new(1, 1, 1).expect("test copy extent should be valid"),
        )
        .expect("test texture region should be valid");
        let payload = PreparedGpuData::<TransferData>::from_pod_transfer(
            "dynamic target guard test payload",
            &[1_u8, 2, 3, 4],
            texture.descriptor().common().provenance().clone(),
        )
        .expect("test upload payload should be valid");
        GpuUploadOperation::new(region.into(), payload).expect("test upload should be valid")
    }

    #[test]
    fn prepared_dynamic_upload_guard_rejects_replaced_logical_texture() {
        let mut resource_ids = GpuWorkResourceIdAllocator::new();
        let original = logical_texture(&mut resource_ids, "original dynamic target");
        let replacement = logical_texture(&mut resource_ids, "replacement dynamic target");
        let operation = logical_upload(&original);

        assert!(validate_prepared_dynamic_texture_target(&original, &operation).is_ok());
        let error = validate_prepared_dynamic_texture_target(&replacement, &operation).expect_err(
            "replacement target must not accept an upload prepared for the old texture",
        );
        assert!(error.contains("replaced"));
    }
}
