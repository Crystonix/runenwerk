use super::{GpuPrimitiveValidationError, buffer_capacity};
use runen_gpu::GpuBufferHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, crate::plugins::render::GpuStorage)]
pub struct U32Counter {
    pub value: u32,
}

impl U32Counter {
    pub const BYTE_SIZE: u64 = 4;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterResetDescriptor {
    pub label: String,
    pub counters: GpuBufferHandle,
    pub counter_count: u32,
    pub reset_value: u32,
}

impl CounterResetDescriptor {
    pub fn new(
        label: impl Into<String>,
        counters: GpuBufferHandle,
        counter_count: u32,
    ) -> Result<Self, GpuPrimitiveValidationError> {
        Self::with_reset_value(label, counters, counter_count, 0)
    }

    pub fn with_reset_value(
        label: impl Into<String>,
        counters: GpuBufferHandle,
        counter_count: u32,
        reset_value: u32,
    ) -> Result<Self, GpuPrimitiveValidationError> {
        let descriptor = Self {
            label: label.into(),
            counters: counters.clone(),
            counter_count,
            reset_value,
        };
        descriptor.validate()?;
        super::validate_capacity(
            format!("{}.counters", descriptor.label),
            buffer_capacity(
                &counters,
                U32Counter::BYTE_SIZE,
                format!("{}.counters", descriptor.label),
            )?,
            u64::from(counter_count),
        )?;
        Ok(descriptor)
    }

    pub fn validate(&self) -> Result<(), GpuPrimitiveValidationError> {
        if self.label.trim().is_empty() {
            return Err(GpuPrimitiveValidationError::EmptyLabel {
                primitive: "counter_reset",
            });
        }
        if self.counter_count == 0 {
            return Err(GpuPrimitiveValidationError::ZeroElementCount {
                label: self.label.clone(),
            });
        }
        Ok(())
    }
}
