use super::{GpuPrimitiveValidationError, buffer_capacity, validate_capacity};
pub use crate::plugins::render::graph::{
    DrawIndexedIndirectArgs, DrawIndirectArgs, IndirectDrawArgsBuffer,
};
use runen_gpu::GpuBufferHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratedIndirectDrawArgs {
    Draw(DrawIndirectArgs),
    DrawIndexed(DrawIndexedIndirectArgs),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndirectDrawArgsGenerationDescriptor {
    pub label: String,
    pub output: GpuBufferHandle,
    pub output_index: u32,
    pub args: GeneratedIndirectDrawArgs,
}

impl IndirectDrawArgsGenerationDescriptor {
    pub fn draw(
        label: impl Into<String>,
        output: GpuBufferHandle,
        output_index: u32,
        args: DrawIndirectArgs,
    ) -> Result<Self, GpuPrimitiveValidationError> {
        let output_capacity = buffer_capacity(
            &output,
            DrawIndirectArgs::BYTE_SIZE,
            "indirect draw argument output",
        )?;
        Self::new(
            label,
            output,
            output_capacity,
            output_index,
            GeneratedIndirectDrawArgs::Draw(args),
        )
    }

    pub fn draw_indexed(
        label: impl Into<String>,
        output: GpuBufferHandle,
        output_index: u32,
        args: DrawIndexedIndirectArgs,
    ) -> Result<Self, GpuPrimitiveValidationError> {
        let output_capacity = buffer_capacity(
            &output,
            DrawIndexedIndirectArgs::BYTE_SIZE,
            "indexed indirect draw argument output",
        )?;
        Self::new(
            label,
            output,
            output_capacity,
            output_index,
            GeneratedIndirectDrawArgs::DrawIndexed(args),
        )
    }

    fn new(
        label: impl Into<String>,
        output: GpuBufferHandle,
        output_capacity: u64,
        output_index: u32,
        args: GeneratedIndirectDrawArgs,
    ) -> Result<Self, GpuPrimitiveValidationError> {
        let descriptor = Self {
            label: label.into(),
            output,
            output_index,
            args,
        };
        descriptor.validate()?;
        validate_capacity(
            format!("{}.output", descriptor.label),
            output_capacity,
            u64::from(output_index) + 1,
        )?;
        Ok(descriptor)
    }

    pub fn validate(&self) -> Result<(), GpuPrimitiveValidationError> {
        if self.label.trim().is_empty() {
            return Err(GpuPrimitiveValidationError::EmptyLabel {
                primitive: "indirect_draw_args_generation",
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::GpuParams;

    #[test]
    fn indirect_draw_args_sizes_match_wgpu_contracts() {
        assert_eq!(
            std::mem::size_of::<<DrawIndirectArgs as GpuParams>::Raw>() as u64,
            DrawIndirectArgs::BYTE_SIZE
        );
        assert_eq!(
            std::mem::size_of::<<DrawIndexedIndirectArgs as GpuParams>::Raw>() as u64,
            DrawIndexedIndirectArgs::BYTE_SIZE
        );
    }

    #[test]
    fn draw_args_generation_rejects_out_of_range_output_index() {
        let (flow, args) = crate::plugins::render::RenderFlow::new("test.draw.args")
            .storage_array::<DrawIndirectArgs>("draw.args", 1)
            .expect("render flow authoring should succeed");
        let _flow = flow;

        assert!(matches!(
            IndirectDrawArgsGenerationDescriptor::draw(
                "draw.args.gen",
                args,
                1,
                DrawIndirectArgs::new(3, 4, 0, 0),
            ),
            Err(GpuPrimitiveValidationError::InsufficientCapacity { .. })
        ));
    }
}
