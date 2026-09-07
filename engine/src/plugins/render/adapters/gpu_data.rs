//! Current `GpuParams` lowering. G4 deletes this adapter when admitted shader
//! interfaces and purpose-specific encoders replace the transitional traits.

use crate::plugins::render::GpuParams;
use runen_gpu::{
    GpuDataEncoder, GpuDataLayout, GpuDataPreparationError, GpuResourceProvenance, PreparedGpuData,
    StorageData, UniformData, prepare_gpu_data,
};
use std::any::{TypeId, type_name};

#[derive(Debug, Clone, Copy)]
pub struct RenderGpuParamsLayout {
    gpu: GpuDataLayout,
    // Transitional render-side compatibility evidence; never normalized GPU authority.
    params_type_id: TypeId,
    // Diagnostic display only.
    params_type_name: &'static str,
}

impl RenderGpuParamsLayout {
    pub fn uniform<Params: GpuParams + 'static>(
        label: &str,
    ) -> Result<Self, GpuDataPreparationError> {
        Self::for_elements::<Params>(label, 1)
    }

    pub fn storage<Params: GpuParams + 'static>(
        label: &str,
        element_count: u64,
    ) -> Result<Self, GpuDataPreparationError> {
        Self::for_elements::<Params>(label, element_count)
    }

    fn for_elements<Params: GpuParams + 'static>(
        label: &str,
        element_count: u64,
    ) -> Result<Self, GpuDataPreparationError> {
        let alignment = u64::try_from(core::mem::align_of::<Params::Raw>()).map_err(|_| {
            GpuDataPreparationError::Invalid {
                operation: "lower current render parameter layout",
                label: label.to_string(),
                cause: runen_gpu::GpuDataPreparationCause::ArithmeticOverflow,
                correction: "use a parameter alignment representable by the platform",
            }
        })?;
        let raw_size = u64::try_from(core::mem::size_of::<Params::Raw>()).map_err(|_| {
            GpuDataPreparationError::Invalid {
                operation: "lower current render parameter layout",
                label: label.to_string(),
                cause: runen_gpu::GpuDataPreparationCause::ArithmeticOverflow,
                correction: "use a parameter size representable by the platform",
            }
        })?;
        let stride = raw_size.max(alignment);
        let byte_len =
            stride
                .checked_mul(element_count)
                .ok_or_else(|| GpuDataPreparationError::Invalid {
                    operation: "lower current render parameter layout",
                    label: label.to_string(),
                    cause: runen_gpu::GpuDataPreparationCause::ArithmeticOverflow,
                    correction: "reduce the render parameter element count",
                })?;
        Ok(Self {
            gpu: GpuDataLayout::new(label, byte_len, alignment, stride, element_count)?,
            params_type_id: TypeId::of::<Params>(),
            params_type_name: type_name::<Params>(),
        })
    }

    /// Returns the normalized GPU allocation layout.
    pub const fn gpu_layout(&self) -> GpuDataLayout {
        self.gpu
    }

    /// Reports whether two declarations can share the same GPU allocation shape.
    pub fn is_allocation_compatible_with(&self, other: &Self) -> bool {
        self.gpu == other.gpu
    }

    /// Reports whether two declarations name the same transitional render parameter type.
    pub fn declares_same_params_type_as(&self, other: &Self) -> bool {
        self.params_type_id == other.params_type_id
    }

    /// Returns transitional render-side declared-type compatibility evidence.
    pub const fn params_type_id(&self) -> TypeId {
        self.params_type_id
    }

    /// Returns a diagnostic-only type name; it is not semantic authority.
    pub const fn params_type_name(&self) -> &'static str {
        self.params_type_name
    }
}

struct RenderUniformEncoder;

impl<Params: GpuParams + 'static> GpuDataEncoder<UniformData, Params> for RenderUniformEncoder {
    fn encode(
        &self,
        label: &str,
        source: &Params,
    ) -> Result<(Vec<u8>, GpuDataLayout), GpuDataPreparationError> {
        let layout = RenderGpuParamsLayout::uniform::<Params>(label)?.gpu_layout();
        let raw = source.to_gpu();
        Ok((bytemuck::bytes_of(&raw).to_vec(), layout))
    }

    fn diagnostic_type_name(&self) -> Option<&'static str> {
        Some(type_name::<Params>())
    }
}

struct RenderStorageEncoder;

impl<Params: GpuParams + 'static> GpuDataEncoder<StorageData, Params> for RenderStorageEncoder {
    fn encode(
        &self,
        label: &str,
        source: &Params,
    ) -> Result<(Vec<u8>, GpuDataLayout), GpuDataPreparationError> {
        let layout = RenderGpuParamsLayout::storage::<Params>(label, 1)?.gpu_layout();
        let raw = source.to_gpu();
        Ok((bytemuck::bytes_of(&raw).to_vec(), layout))
    }

    fn diagnostic_type_name(&self) -> Option<&'static str> {
        Some(type_name::<Params>())
    }
}

struct ProjectedUniformBytes {
    bytes: Vec<u8>,
    layout: GpuDataLayout,
}

struct ProjectedUniformBytesEncoder {
    diagnostic_type_name: &'static str,
}

impl GpuDataEncoder<UniformData, ProjectedUniformBytes> for ProjectedUniformBytesEncoder {
    fn encode(
        &self,
        _label: &str,
        source: &ProjectedUniformBytes,
    ) -> Result<(Vec<u8>, GpuDataLayout), GpuDataPreparationError> {
        Ok((source.bytes.clone(), source.layout))
    }

    fn diagnostic_type_name(&self) -> Option<&'static str> {
        Some(self.diagnostic_type_name)
    }
}

/// Transitional facade lowering into an explicit uniform-purpose encoder.
///
/// ```
/// use runen_gpu::{
///     GpuResourceLabel, GpuResourceProvenance, PreparedGpuData, StorageData, UniformData,
/// };
/// use engine::plugins::render::{
///     GpuParams, prepare_render_storage, prepare_render_uniform,
/// };
/// struct Params(u32);
/// impl GpuParams for Params {
///     type Raw = u32;
///     fn to_gpu(&self) -> Self::Raw { self.0 }
/// }
/// let label = GpuResourceLabel::new("render params")?;
/// let provenance = GpuResourceProvenance::new(label, None, None);
/// let uniform: PreparedGpuData<UniformData> =
///     prepare_render_uniform("render params", &Params(7), provenance.clone())?;
/// let storage: PreparedGpuData<StorageData> =
///     prepare_render_storage("render params", &Params(7), provenance)?;
/// assert_eq!(uniform.as_bytes(), storage.as_bytes());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn prepare_render_uniform<Params: GpuParams + 'static>(
    label: impl Into<String>,
    params: &Params,
    provenance: GpuResourceProvenance,
) -> Result<PreparedGpuData<UniformData>, GpuDataPreparationError> {
    prepare_gpu_data(label, params, provenance, &RenderUniformEncoder)
}

/// Transitional facade lowering into an explicit storage-purpose encoder.
pub fn prepare_render_storage<Params: GpuParams + 'static>(
    label: impl Into<String>,
    params: &Params,
    provenance: GpuResourceProvenance,
) -> Result<PreparedGpuData<StorageData>, GpuDataPreparationError> {
    prepare_gpu_data(label, params, provenance, &RenderStorageEncoder)
}

pub(crate) fn prepare_projected_uniform_bytes(
    label: impl Into<String>,
    bytes: Vec<u8>,
    layout: RenderGpuParamsLayout,
    provenance: GpuResourceProvenance,
) -> Result<PreparedGpuData<UniformData>, GpuDataPreparationError> {
    let source = ProjectedUniformBytes {
        bytes,
        layout: layout.gpu_layout(),
    };
    prepare_gpu_data(
        label,
        &source,
        provenance,
        &ProjectedUniformBytesEncoder {
            diagnostic_type_name: layout.params_type_name(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FirstParams(u32);

    impl GpuParams for FirstParams {
        type Raw = u32;

        fn to_gpu(&self) -> Self::Raw {
            self.0
        }
    }

    struct SecondParams(u32);

    impl GpuParams for SecondParams {
        type Raw = u32;

        fn to_gpu(&self) -> Self::Raw {
            self.0
        }
    }

    #[test]
    fn allocation_compatibility_is_distinct_from_declared_parameter_type() {
        let first = RenderGpuParamsLayout::uniform::<FirstParams>("first").unwrap();
        let second = RenderGpuParamsLayout::uniform::<SecondParams>("second").unwrap();
        let same_type = RenderGpuParamsLayout::uniform::<FirstParams>("same type").unwrap();

        assert!(first.is_allocation_compatible_with(&second));
        assert!(!first.declares_same_params_type_as(&second));
        assert!(first.declares_same_params_type_as(&same_type));
        assert_ne!(first.params_type_id(), second.params_type_id());
        assert_ne!(first.params_type_name(), second.params_type_name());
    }
}
