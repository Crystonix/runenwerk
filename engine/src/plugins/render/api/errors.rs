use crate::plugins::render::RenderGpuResourceAdapterError;
use crate::plugins::render::gpu_primitives::GpuPrimitiveValidationError;
use crate::plugins::render::procedural::ProceduralValidationError;
use runen_gpu::{GpuWorkResourceId, GpuWorkResourceIdAllocationError};
use thiserror::Error;

/// Structured failures produced while authoring a render flow.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RenderFlowAuthoringError {
    #[error(transparent)]
    ResourceIdAllocation(#[from] GpuWorkResourceIdAllocationError),

    #[error(transparent)]
    GpuResourceAdapter(#[from] RenderGpuResourceAdapterError),

    #[error(transparent)]
    ProceduralValidation(#[from] ProceduralValidationError),

    #[error(transparent)]
    GpuPrimitiveValidation(#[from] GpuPrimitiveValidationError),

    #[error(
        "resolve logical GPU buffer handle for resource '{resource_id:?}': declaration is not a buffer; retain the matching kind-specific handle"
    )]
    DeclaredBufferHandleMissing { resource_id: GpuWorkResourceId },

    #[error(
        "validate render buffer layout for resource '{resource_id:?}': expected {expected}, found {actual}; retain the declaration-matching GPU buffer handle"
    )]
    BufferLayoutMismatch {
        resource_id: GpuWorkResourceId,
        expected: &'static str,
        actual: &'static str,
    },
}
