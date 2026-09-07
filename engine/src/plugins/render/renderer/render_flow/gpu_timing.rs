use super::*;
use runen_gpu::{
    GpuBufferHandle, GpuContext, GpuQuerySetHandle, GpuReadbackBytes, GpuReadbackId,
    GpuSubmissionFailure,
};

const TIMESTAMP_SIZE_BYTES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::plugins::render::renderer) struct GpuPassTimestampIndices {
    pub begin: u32,
    pub end: u32,
}

#[derive(Debug, Clone)]
struct GpuPassTimingEntry {
    frame_index: u64,
    render_surface_id: u64,
    flow_id: String,
    pass_id: String,
    pass_kind: String,
    indices: GpuPassTimestampIndices,
}

/// Renderer-owned observation metadata for timestamp operations submitted by G5.
///
/// Query/resolve/readback resources are materialized only by RunenGPU preparation. The renderer
/// retains semantic identities so it can publish truthful pending evidence at acceptance.
#[derive(Debug)]
pub(in crate::plugins::render::renderer) struct GpuPassTimingFrame {
    readback_id: GpuReadbackId,
    timestamp_period_ns: f32,
    query_capacity: u32,
    entries: Vec<GpuPassTimingEntry>,
}

impl GpuPassTimingFrame {
    pub fn new(
        context: &GpuContext,
        query_set_handle: &GpuQuerySetHandle,
        resolve_buffer_handle: &GpuBufferHandle,
        readback_id: GpuReadbackId,
        query_capacity: u32,
    ) -> Result<Self> {
        if query_capacity == 0 {
            anyhow::bail!(
                "GPU timing frame requires nonzero query capacity after logical timing admission"
            );
        }
        let readback_size = u64::from(query_capacity) * TIMESTAMP_SIZE_BYTES as u64;
        if query_set_handle.descriptor().count() != query_capacity
            || resolve_buffer_handle.descriptor().size_bytes() < readback_size
        {
            anyhow::bail!(
                "prepared timing handles do not cover the declared query capacity {query_capacity}"
            );
        }
        let timestamp_period_ns = context.timestamp_period_ns().ok_or_else(|| {
            anyhow::anyhow!(
                "GPU timing was admitted without a finite backend-neutral timestamp period"
            )
        })?;
        Ok(Self {
            readback_id,
            timestamp_period_ns,
            query_capacity,
            entries: Vec::new(),
        })
    }

    pub fn register_pass_metadata(
        &mut self,
        indices: GpuPassTimestampIndices,
        frame_index: u64,
        render_surface_id: u64,
        flow_id: impl Into<String>,
        pass_id: impl Into<String>,
        pass_kind: impl Into<String>,
    ) -> bool {
        if indices.begin >= indices.end
            || indices.end >= self.query_capacity
            || self.entries.iter().any(|entry| entry.indices == indices)
        {
            return false;
        }
        self.entries.push(GpuPassTimingEntry {
            frame_index,
            render_surface_id,
            flow_id: flow_id.into(),
            pass_id: pass_id.into(),
            pass_kind: pass_kind.into(),
            indices,
        });
        true
    }

    pub const fn readback_id(&self) -> GpuReadbackId {
        self.readback_id
    }

    pub fn pending_evidence(&self) -> Vec<RenderPassTimingEvidence> {
        self.entries
            .iter()
            .map(|entry| {
                RenderPassTimingEvidence::gpu_diagnostic(
                    Some(entry.frame_index),
                    Some(entry.render_surface_id),
                    entry.flow_id.clone(),
                    entry.pass_id.clone(),
                    entry.pass_kind.clone(),
                    RenderGpuTimingDiagnostic::readback_pending(
                        "GPU timestamp submission was accepted; readback is pending",
                    ),
                )
            })
            .collect()
    }

    pub fn ready_evidence(&self, readback: &GpuReadbackBytes) -> Vec<RenderPassTimingEvidence> {
        if readback.texture_format().is_some() {
            return self.diagnostic_evidence(
                "GPU timestamp readback unexpectedly carried texture-format metadata",
            );
        }
        let expected_len =
            usize::try_from(u64::from(self.query_capacity) * TIMESTAMP_SIZE_BYTES as u64)
                .expect("u32 timestamp query capacity times eight fits usize on supported targets");
        let bytes = readback.as_bytes();
        if bytes.len() != expected_len {
            return self.diagnostic_evidence(format!(
                "GPU timestamp readback byte length mismatch: expected {expected_len}, got {}",
                bytes.len()
            ));
        }

        self.entries
            .iter()
            .map(|entry| {
                let begin = decode_timestamp(bytes, entry.indices.begin);
                let end = decode_timestamp(bytes, entry.indices.end);
                let Some((begin, end)) = begin.zip(end) else {
                    return timing_diagnostic(
                        entry,
                        "GPU timestamp readback did not contain the admitted begin/end pair",
                    );
                };
                let Some(delta_ticks) = end.checked_sub(begin) else {
                    return timing_diagnostic(
                        entry,
                        format!(
                            "GPU timestamp end value {end} precedes begin value {begin}; no sample was published"
                        ),
                    );
                };
                let millis = (delta_ticks as f64) * f64::from(self.timestamp_period_ns) / 1_000_000.0;
                if !millis.is_finite() || millis > f64::from(f32::MAX) {
                    return timing_diagnostic(
                        entry,
                        "GPU timestamp conversion exceeded the renderer timing domain",
                    );
                }
                RenderPassTimingEvidence::gpu_sample(
                    Some(entry.frame_index),
                    Some(entry.render_surface_id),
                    entry.flow_id.clone(),
                    entry.pass_id.clone(),
                    entry.pass_kind.clone(),
                    millis as f32,
                )
            })
            .collect()
    }

    pub fn failed_evidence(&self, failure: &GpuSubmissionFailure) -> Vec<RenderPassTimingEvidence> {
        self.diagnostic_evidence(format!(
            "GPU timestamp readback failed ({:?}): {}",
            failure.kind(),
            failure.detail()
        ))
    }

    pub fn diagnostic_evidence(&self, message: impl Into<String>) -> Vec<RenderPassTimingEvidence> {
        let message = message.into();
        self.entries
            .iter()
            .map(|entry| timing_diagnostic(entry, message.clone()))
            .collect()
    }
}

fn decode_timestamp(bytes: &[u8], index: u32) -> Option<u64> {
    let start = usize::try_from(index)
        .ok()?
        .checked_mul(TIMESTAMP_SIZE_BYTES)?;
    let end = start.checked_add(TIMESTAMP_SIZE_BYTES)?;
    let encoded: [u8; TIMESTAMP_SIZE_BYTES] = bytes.get(start..end)?.try_into().ok()?;
    Some(u64::from_le_bytes(encoded))
}

fn timing_diagnostic(
    entry: &GpuPassTimingEntry,
    message: impl Into<String>,
) -> RenderPassTimingEvidence {
    RenderPassTimingEvidence::gpu_diagnostic(
        Some(entry.frame_index),
        Some(entry.render_surface_id),
        entry.flow_id.clone(),
        entry.pass_id.clone(),
        entry.pass_kind.clone(),
        RenderGpuTimingDiagnostic::unavailable_this_frame(message),
    )
}
