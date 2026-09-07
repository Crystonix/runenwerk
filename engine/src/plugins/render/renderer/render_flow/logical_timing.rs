use super::*;
use crate::plugins::render::renderer::resource_descriptors::{buffer_descriptor, owned_common};
use runen_gpu::{
    GpuBufferHandle, GpuBufferUsage, GpuMemoryIntent, GpuQueryKind, GpuQueryRange,
    GpuQuerySetDescriptor, GpuQuerySetHandle, GpuReadbackId, GpuResourceLifetime,
    GpuWorkResourceIdAllocator,
};

const QUERY_SIZE_BYTES: u64 = 8;

/// Per-occurrence timestamp allocation for one already-expanded renderer invocation.
///
/// Only render/compute operations can carry timestamp writes in the current G5A operation model.
/// Copy and present occurrences therefore retain an explicit `None` slot rather than consuming a
/// query pair that no operation can initialize.
#[derive(Debug, Clone)]
pub(super) struct LogicalGpuPassTimingPlan {
    timing: Option<LogicalGpuPassTiming>,
    occurrence_ranges: Vec<Option<GpuPassTimestampIndices>>,
}

impl LogicalGpuPassTimingPlan {
    pub(super) fn new<'a>(
        passes: impl IntoIterator<Item = &'a CompiledPassExecutionPlan>,
    ) -> Result<Self> {
        let timestampable = passes
            .into_iter()
            .map(pass_supports_timestamp_write)
            .collect::<Vec<_>>();
        let timing = LogicalGpuPassTiming::new(
            timestampable
                .iter()
                .filter(|timestampable| **timestampable)
                .count(),
        )?;
        let mut next_timestamp_ordinal = 0usize;
        let mut occurrence_ranges = Vec::with_capacity(timestampable.len());
        for timestampable in timestampable {
            if !timestampable {
                occurrence_ranges.push(None);
                continue;
            }
            let range = timing
                .as_ref()
                .expect("a timestampable occurrence guarantees logical timing resources")
                .range_for_timestamp_ordinal(next_timestamp_ordinal)?;
            next_timestamp_ordinal = next_timestamp_ordinal
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("render GPU timestamp ordinal overflow"))?;
            occurrence_ranges.push(Some(range));
        }
        Ok(Self {
            timing,
            occurrence_ranges,
        })
    }

    pub(super) fn timing(&self) -> Option<&LogicalGpuPassTiming> {
        self.timing.as_ref()
    }

    pub(super) fn range_for_occurrence(
        &self,
        occurrence_ordinal: usize,
    ) -> Result<Option<GpuPassTimestampIndices>> {
        self.occurrence_ranges
            .get(occurrence_ordinal)
            .copied()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "render GPU timestamp occurrence {occurrence_ordinal} is outside the expanded invocation"
                )
            })
    }
}

/// Logical timestamp resources, result identity, and timestamp-local query ranges prepared before
/// G3 work.
///
/// The readback ID is allocated exactly once here because it identifies the eventual G5 result,
/// while the readback operation itself is projected later from the exact used query range. G5
/// readback targets the resolve buffer directly.
#[derive(Debug, Clone)]
pub(super) struct LogicalGpuPassTiming {
    query_set: GpuQuerySetHandle,
    resolve_buffer: GpuBufferHandle,
    readback_id: GpuReadbackId,
    query_capacity: u32,
}

impl LogicalGpuPassTiming {
    fn new(timestampable_occurrences: usize) -> Result<Option<Self>> {
        if timestampable_occurrences == 0 {
            return Ok(None);
        }
        let query_capacity = timestampable_occurrences
            .checked_mul(2)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| anyhow::anyhow!("render GPU timestamp query capacity exceeds u32"))?;
        let byte_len = u64::from(query_capacity)
            .checked_mul(QUERY_SIZE_BYTES)
            .ok_or_else(|| anyhow::anyhow!("render GPU timestamp buffer size overflow"))?;

        let mut allocator = GpuWorkResourceIdAllocator::new();
        let query_set = allocator.allocate_query_set_handle(GpuQuerySetDescriptor::new(
            owned_common(
                "render.flow.timestamps",
                GpuResourceLifetime::Transient,
                GpuMemoryIntent::Device,
            )?,
            GpuQueryKind::Timestamp,
            query_capacity,
        )?)?;
        let resolve_buffer = allocator.allocate_buffer_handle(buffer_descriptor(
            "render.flow.timestamp_resolve",
            byte_len,
            [GpuBufferUsage::QueryResolve, GpuBufferUsage::CopySource],
            GpuResourceLifetime::Transient,
            GpuMemoryIntent::Device,
        )?)?;
        let readback_id = GpuReadbackId::allocate()?;

        Ok(Some(Self {
            query_set,
            resolve_buffer,
            readback_id,
            query_capacity,
        }))
    }

    pub(super) fn query_set(&self) -> &GpuQuerySetHandle {
        &self.query_set
    }

    pub(super) fn resolve_buffer(&self) -> &GpuBufferHandle {
        &self.resolve_buffer
    }

    pub(super) const fn readback_id(&self) -> GpuReadbackId {
        self.readback_id
    }

    pub(super) const fn query_capacity(&self) -> u32 {
        self.query_capacity
    }

    fn range_for_timestamp_ordinal(
        &self,
        timestamp_ordinal: usize,
    ) -> Result<GpuPassTimestampIndices> {
        let begin = timestamp_ordinal
            .checked_mul(2)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| anyhow::anyhow!("render GPU timestamp occurrence index exceeds u32"))?;
        let end = begin
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("render GPU timestamp query index overflow"))?;
        if end >= self.query_capacity {
            anyhow::bail!(
                "render GPU timestamp occurrence {timestamp_ordinal} exceeds query capacity {}",
                self.query_capacity,
            );
        }
        Ok(GpuPassTimestampIndices { begin, end })
    }

    pub(super) fn query_range(&self) -> Result<GpuQueryRange> {
        Ok(GpuQueryRange::new(
            self.query_set(),
            0,
            self.query_capacity,
        )?)
    }
}

const fn pass_supports_timestamp_write(pass: &CompiledPassExecutionPlan) -> bool {
    matches!(
        pass,
        CompiledPassExecutionPlan::Compute(_)
            | CompiledPassExecutionPlan::Fullscreen(_)
            | CompiledPassExecutionPlan::Graphics(_)
            | CompiledPassExecutionPlan::BuiltinUiComposite(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_timing_uses_two_queries_per_timestampable_occurrence() {
        let timing = LogicalGpuPassTiming::new(3)
            .expect("logical timing should construct")
            .expect("nonzero timestampable occurrence count should allocate timing");

        assert_eq!(timing.query_capacity(), 6);
        assert_eq!(
            timing.range_for_timestamp_ordinal(0).unwrap(),
            GpuPassTimestampIndices { begin: 0, end: 1 }
        );
        assert_eq!(
            timing.range_for_timestamp_ordinal(2).unwrap(),
            GpuPassTimestampIndices { begin: 4, end: 5 }
        );
        assert_eq!(timing.query_range().unwrap().count(), 6);
    }

    #[test]
    fn logical_timing_omits_zero_timestampable_resources() {
        assert!(LogicalGpuPassTiming::new(0).unwrap().is_none());
    }
}
