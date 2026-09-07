use super::{GpuPrimitiveValidationError, U32ScanElement, buffer_capacity, validate_capacity};
use runen_gpu::GpuBufferHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct U32ScatterDescriptor {
    pub label: String,
    pub source_indices: GpuBufferHandle,
    pub prefix_offsets: GpuBufferHandle,
    pub output_indices: GpuBufferHandle,
    pub element_count: u32,
    pub output_capacity: u32,
}

impl U32ScatterDescriptor {
    pub fn new(
        label: impl Into<String>,
        source_indices: GpuBufferHandle,
        prefix_offsets: GpuBufferHandle,
        output_indices: GpuBufferHandle,
        element_count: u32,
        output_capacity: u32,
    ) -> Result<Self, GpuPrimitiveValidationError> {
        let descriptor = Self {
            label: label.into(),
            source_indices: source_indices.clone(),
            prefix_offsets: prefix_offsets.clone(),
            output_indices: output_indices.clone(),
            element_count,
            output_capacity,
        };
        descriptor.validate()?;
        validate_capacity(
            format!("{}.source_indices", descriptor.label),
            buffer_capacity(
                &source_indices,
                U32ScanElement::BYTE_SIZE,
                format!("{}.source_indices", descriptor.label),
            )?,
            u64::from(element_count),
        )?;
        validate_capacity(
            format!("{}.prefix_offsets", descriptor.label),
            buffer_capacity(
                &prefix_offsets,
                U32ScanElement::BYTE_SIZE,
                format!("{}.prefix_offsets", descriptor.label),
            )?,
            u64::from(element_count),
        )?;
        validate_capacity(
            format!("{}.output_indices", descriptor.label),
            buffer_capacity(
                &output_indices,
                U32ScanElement::BYTE_SIZE,
                format!("{}.output_indices", descriptor.label),
            )?,
            u64::from(output_capacity),
        )?;
        Ok(descriptor)
    }

    pub fn validate(&self) -> Result<(), GpuPrimitiveValidationError> {
        if self.label.trim().is_empty() {
            return Err(GpuPrimitiveValidationError::EmptyLabel {
                primitive: "u32_scatter",
            });
        }
        if self.element_count == 0 {
            return Err(GpuPrimitiveValidationError::ZeroElementCount {
                label: self.label.clone(),
            });
        }
        if self.source_indices == self.output_indices
            || self.prefix_offsets == self.output_indices
            || self.source_indices == self.prefix_offsets
        {
            return Err(GpuPrimitiveValidationError::AliasedInputOutput {
                label: self.label.clone(),
            });
        }
        validate_capacity(
            self.label.clone(),
            u64::from(self.output_capacity),
            u64::from(self.element_count),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::RenderFlow;

    #[test]
    fn gpu_primitives_scatter_rejects_aliased_buffers() {
        let (flow, indices) = RenderFlow::new("test.primitive.scatter.alias")
            .storage_array::<U32ScanElement>("scatter.indices", 4)
            .expect("render flow authoring should succeed");
        let (flow, offsets) = flow
            .storage_array::<U32ScanElement>("scatter.offsets", 4)
            .expect("render flow authoring should succeed");
        let _flow = flow;

        assert!(matches!(
            U32ScatterDescriptor::new("scatter", indices.clone(), offsets, indices, 4, 4,),
            Err(GpuPrimitiveValidationError::AliasedInputOutput { .. })
        ));
    }

    #[test]
    fn gpu_primitives_scatter_rejects_output_capacity_drift() {
        let (flow, indices) = RenderFlow::new("test.primitive.scatter.capacity")
            .storage_array::<U32ScanElement>("scatter.indices", 4)
            .expect("render flow authoring should succeed");
        let (flow, offsets) = flow
            .storage_array::<U32ScanElement>("scatter.offsets", 4)
            .expect("render flow authoring should succeed");
        let (_flow, output) = flow
            .storage_array::<U32ScanElement>("scatter.output", 3)
            .expect("render flow authoring should succeed");

        assert!(matches!(
            U32ScatterDescriptor::new("scatter", indices, offsets, output, 4, 4),
            Err(GpuPrimitiveValidationError::InsufficientCapacity { .. })
        ));
    }
}
