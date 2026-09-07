use crate::plugins::render::GpuStorage;
use runen_gpu::GpuBufferHandle;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, GpuStorage)]
pub struct U32ScanElement {
    pub value: u32,
}

impl U32ScanElement {
    pub const BYTE_SIZE: u64 = 4;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixScanMode {
    Exclusive,
    Inclusive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct U32PrefixScanDescriptor {
    pub label: String,
    pub input: GpuBufferHandle,
    pub output: GpuBufferHandle,
    pub total_count: u32,
    pub mode: PrefixScanMode,
}

impl U32PrefixScanDescriptor {
    pub fn new(
        label: impl Into<String>,
        input: GpuBufferHandle,
        output: GpuBufferHandle,
        total_count: u32,
        mode: PrefixScanMode,
    ) -> Result<Self, GpuPrimitiveValidationError> {
        let descriptor = Self {
            label: label.into(),
            input: input.clone(),
            output: output.clone(),
            total_count,
            mode,
        };
        descriptor.validate()?;
        validate_capacity(
            format!("{}.input", descriptor.label),
            buffer_capacity(
                &input,
                U32ScanElement::BYTE_SIZE,
                format!("{}.input", descriptor.label),
            )?,
            u64::from(total_count),
        )?;
        validate_capacity(
            format!("{}.output", descriptor.label),
            buffer_capacity(
                &output,
                U32ScanElement::BYTE_SIZE,
                format!("{}.output", descriptor.label),
            )?,
            u64::from(total_count),
        )?;
        Ok(descriptor)
    }

    pub fn validate(&self) -> Result<(), GpuPrimitiveValidationError> {
        if self.label.trim().is_empty() {
            return Err(GpuPrimitiveValidationError::EmptyLabel {
                primitive: "u32_prefix_scan",
            });
        }
        if self.total_count == 0 {
            return Err(GpuPrimitiveValidationError::ZeroElementCount {
                label: self.label.clone(),
            });
        }
        if self.input == self.output {
            return Err(GpuPrimitiveValidationError::AliasedInputOutput {
                label: self.label.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GpuPrimitiveValidationError {
    #[error("{primitive} primitive label must not be empty")]
    EmptyLabel { primitive: &'static str },

    #[error("gpu primitive '{label}' must process at least one element")]
    ZeroElementCount { label: String },

    #[error("gpu primitive '{label}' requires distinct input and output buffers")]
    AliasedInputOutput { label: String },

    #[error(
        "gpu primitive '{label}' declares capacity {capacity}, but required count is {required_count}"
    )]
    InsufficientCapacity {
        label: String,
        capacity: u64,
        required_count: u64,
    },

    #[error("gpu primitive plan '{label}' must contain at least one step")]
    EmptyExecutionPlan { label: String },

    #[error(
        "gpu primitive buffer '{label}' has {size_bytes} bytes, which is not a multiple of the required {element_size}-byte element layout"
    )]
    InvalidBufferElementLayout {
        label: String,
        size_bytes: u64,
        element_size: u64,
    },
}

pub(crate) fn buffer_capacity(
    handle: &GpuBufferHandle,
    element_size: u64,
    label: impl Into<String>,
) -> Result<u64, GpuPrimitiveValidationError> {
    let label = label.into();
    let size_bytes = handle.descriptor().size_bytes();
    if element_size == 0 || !size_bytes.is_multiple_of(element_size) {
        return Err(GpuPrimitiveValidationError::InvalidBufferElementLayout {
            label,
            size_bytes,
            element_size,
        });
    }
    Ok(size_bytes / element_size)
}

pub fn validate_capacity(
    label: impl Into<String>,
    capacity: u64,
    required_count: u64,
) -> Result<(), GpuPrimitiveValidationError> {
    let label = label.into();
    if capacity < required_count {
        return Err(GpuPrimitiveValidationError::InsufficientCapacity {
            label,
            capacity,
            required_count,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::RenderFlow;

    #[test]
    fn gpu_primitives_prefix_scan_uses_real_storage_lengths() {
        let (flow, input) = RenderFlow::new("test.primitive.scan")
            .storage_array::<U32ScanElement>("scan.input", 4)
            .expect("render flow authoring should succeed");
        let (_flow, output) = flow
            .storage_array::<U32ScanElement>("scan.output", 3)
            .expect("render flow authoring should succeed");

        assert!(matches!(
            U32PrefixScanDescriptor::new("scan", input, output, 4, PrefixScanMode::Exclusive,),
            Err(GpuPrimitiveValidationError::InsufficientCapacity { .. })
        ));
    }

    #[test]
    fn gpu_primitives_prefix_scan_rejects_aliased_output() {
        let (flow, input) = RenderFlow::new("test.primitive.scan.alias")
            .storage_array::<U32ScanElement>("scan.input", 4)
            .expect("render flow authoring should succeed");
        let _flow = flow;

        assert!(matches!(
            U32PrefixScanDescriptor::new(
                "scan",
                input.clone(),
                input,
                4,
                PrefixScanMode::Exclusive,
            ),
            Err(GpuPrimitiveValidationError::AliasedInputOutput { .. })
        ));
    }
}
