use crate::plugins::render::{
    RenderDynamicTextureTargetKey, RenderFrameProducerId, RenderTextureTargetFormat,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderTextureUploadAlphaMode {
    Straight,
    Premultiplied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderDynamicTextureUploadDescriptor {
    pub target_key: RenderDynamicTextureTargetKey,
    pub origin_x: u32,
    pub origin_y: u32,
    pub width: u32,
    pub height: u32,
    pub format: RenderTextureTargetFormat,
    pub alpha_mode: RenderTextureUploadAlphaMode,
    pub product_generation: u64,
    pub rgba8: Vec<u8>,
}

impl RenderDynamicTextureUploadDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub fn rgba8(
        target_key: RenderDynamicTextureTargetKey,
        origin_x: u32,
        origin_y: u32,
        width: u32,
        height: u32,
        alpha_mode: RenderTextureUploadAlphaMode,
        product_generation: u64,
        rgba8: Vec<u8>,
    ) -> Self {
        Self {
            target_key,
            origin_x,
            origin_y,
            width,
            height,
            format: RenderTextureTargetFormat::Rgba8Unorm,
            alpha_mode,
            product_generation,
            rgba8,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rgba8_with_format(
        target_key: RenderDynamicTextureTargetKey,
        origin_x: u32,
        origin_y: u32,
        width: u32,
        height: u32,
        format: RenderTextureTargetFormat,
        alpha_mode: RenderTextureUploadAlphaMode,
        product_generation: u64,
        rgba8: Vec<u8>,
    ) -> Self {
        Self {
            target_key,
            origin_x,
            origin_y,
            width,
            height,
            format,
            alpha_mode,
            product_generation,
            rgba8,
        }
    }

    pub fn expected_byte_len(&self) -> Option<usize> {
        self.width
            .checked_mul(self.height)?
            .checked_mul(4)
            .map(|value| value as usize)
    }

    pub fn validate(&self) -> Result<(), RenderDynamicTextureUploadRegistryError> {
        if self.target_key.namespace.trim().is_empty()
            || self.target_key.target_id.trim().is_empty()
        {
            return Err(RenderDynamicTextureUploadRegistryError::EmptyTargetKey);
        }
        if self.width == 0 || self.height == 0 {
            return Err(RenderDynamicTextureUploadRegistryError::ZeroDimensions {
                width: self.width,
                height: self.height,
            });
        }
        if !matches!(
            self.format,
            RenderTextureTargetFormat::Rgba8Unorm | RenderTextureTargetFormat::Rgba8UnormSrgb
        ) {
            return Err(RenderDynamicTextureUploadRegistryError::UnsupportedFormat {
                format: self.format,
            });
        }
        let expected = self.expected_byte_len().ok_or(
            RenderDynamicTextureUploadRegistryError::ByteLengthOverflow {
                width: self.width,
                height: self.height,
            },
        )?;
        if self.rgba8.len() != expected {
            return Err(RenderDynamicTextureUploadRegistryError::InvalidByteLength {
                expected,
                actual: self.rgba8.len(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderDynamicTextureUploadDiagnostic {
    pub producer_id: RenderFrameProducerId,
    pub target_key: RenderDynamicTextureTargetKey,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderDynamicTextureUploadRegistryError {
    EmptyTargetKey,
    ZeroDimensions {
        width: u32,
        height: u32,
    },
    UnsupportedFormat {
        format: RenderTextureTargetFormat,
    },
    ByteLengthOverflow {
        width: u32,
        height: u32,
    },
    InvalidByteLength {
        expected: usize,
        actual: usize,
    },
    DuplicateTargetWithinProducer {
        producer_id: RenderFrameProducerId,
        target_key: RenderDynamicTextureTargetKey,
    },
}

impl fmt::Display for RenderDynamicTextureUploadRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTargetKey => {
                write!(f, "dynamic texture upload target key must be non-empty")
            }
            Self::ZeroDimensions { width, height } => write!(
                f,
                "dynamic texture upload dimensions must be non-zero, got {width}x{height}"
            ),
            Self::UnsupportedFormat { format } => {
                write!(
                    f,
                    "dynamic texture upload format {format:?} is not supported"
                )
            }
            Self::ByteLengthOverflow { width, height } => write!(
                f,
                "dynamic texture upload byte length overflow for {width}x{height}"
            ),
            Self::InvalidByteLength { expected, actual } => write!(
                f,
                "dynamic texture upload byte length mismatch: expected {expected}, got {actual}"
            ),
            Self::DuplicateTargetWithinProducer {
                producer_id,
                target_key,
            } => write!(
                f,
                "render upload producer '{producer_id:?}' requested duplicate upload for dynamic target '{target_key}'"
            ),
        }
    }
}

impl std::error::Error for RenderDynamicTextureUploadRegistryError {}

#[derive(Debug, Default, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderDynamicTextureUploadRegistryResource {
    contributions: BTreeMap<RenderFrameProducerId, Vec<RenderDynamicTextureUploadDescriptor>>,
    diagnostics: Vec<RenderDynamicTextureUploadDiagnostic>,
}

impl RenderDynamicTextureUploadRegistryResource {
    pub fn clear(&mut self) {
        self.contributions.clear();
        self.diagnostics.clear();
    }

    pub fn replace_contribution(
        &mut self,
        producer_id: impl Into<RenderFrameProducerId>,
        uploads: impl IntoIterator<Item = RenderDynamicTextureUploadDescriptor>,
    ) -> Result<Vec<RenderDynamicTextureUploadDescriptor>, RenderDynamicTextureUploadRegistryError>
    {
        let producer_id = producer_id.into();
        self.diagnostics
            .retain(|diagnostic| diagnostic.producer_id != producer_id);

        let uploads = uploads.into_iter().collect::<Vec<_>>();
        let mut target_keys = BTreeSet::<RenderDynamicTextureTargetKey>::new();
        for upload in &uploads {
            if let Err(err) = upload.validate() {
                self.diagnostics.push(RenderDynamicTextureUploadDiagnostic {
                    producer_id,
                    target_key: upload.target_key.clone(),
                    message: err.to_string(),
                });
                return Err(err);
            }
            if !target_keys.insert(upload.target_key.clone()) {
                let err = RenderDynamicTextureUploadRegistryError::DuplicateTargetWithinProducer {
                    producer_id,
                    target_key: upload.target_key.clone(),
                };
                self.diagnostics.push(RenderDynamicTextureUploadDiagnostic {
                    producer_id,
                    target_key: upload.target_key.clone(),
                    message: err.to_string(),
                });
                return Err(err);
            }
        }

        Ok(self
            .contributions
            .insert(producer_id, uploads)
            .unwrap_or_default())
    }

    pub fn remove_contribution(
        &mut self,
        producer_id: impl Into<RenderFrameProducerId>,
    ) -> Option<Vec<RenderDynamicTextureUploadDescriptor>> {
        let producer_id = producer_id.into();
        self.diagnostics
            .retain(|diagnostic| diagnostic.producer_id != producer_id);
        self.contributions.remove(&producer_id)
    }

    pub fn snapshot(&self) -> Vec<RenderDynamicTextureUploadDescriptor> {
        self.contributions
            .values()
            .flat_map(|uploads| uploads.iter().cloned())
            .collect()
    }

    pub fn diagnostics(&self) -> &[RenderDynamicTextureUploadDiagnostic] {
        &self.diagnostics
    }
}
