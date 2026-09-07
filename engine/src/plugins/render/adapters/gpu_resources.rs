//! Current render-resource lowering. G4-G7 delete this adapter as resource
//! realization, history, and surface acquisition move to their owning phases.

use super::{RenderGpuParamsLayout, normalized_render_format};
use crate::plugins::render::{
    GpuParams, RenderTextureSampleMode, RenderTextureTargetFormat, RenderTextureTargetUsage,
};
use runen_gpu::{
    GpuBufferDescriptor, GpuBufferHandle, GpuBufferInitialization, GpuBufferUsage, GpuBufferUsages,
    GpuDataPreparationError, GpuMemoryIntent, GpuReconstruction, GpuResourceCommon,
    GpuResourceDescriptor, GpuResourceDescriptorError, GpuResourceLabel, GpuResourceLifetime,
    GpuResourceProvenance, GpuTextureDescriptor, GpuTextureDimension, GpuTextureExtent,
    GpuTextureFormat, GpuTextureInitialization, GpuTextureUsage, GpuTextureUsages,
    GpuWorkResourceId, GpuWorkResourceIdAllocationError, GpuWorkResourceIdAllocator,
};
use std::any::TypeId;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RenderGpuResourceAdapterError {
    #[error(transparent)]
    Data(#[from] GpuDataPreparationError),
    #[error(transparent)]
    Descriptor(#[from] GpuResourceDescriptorError),
    #[error(transparent)]
    Identity(#[from] GpuWorkResourceIdAllocationError),
    #[error(
        "cannot declare render target alias binding key {value:?}: the key is empty after trimming; correction: {correction}"
    )]
    InvalidTargetAliasBindingKey {
        value: String,
        correction: &'static str,
    },
}

#[derive(Debug, Clone)]
pub struct RenderUniformDeclaration {
    handle: GpuBufferHandle,
    layout: RenderGpuParamsLayout,
}

impl RenderUniformDeclaration {
    pub fn handle(&self) -> &GpuBufferHandle {
        &self.handle
    }

    pub const fn layout(&self) -> RenderGpuParamsLayout {
        self.layout
    }

    pub fn id(&self) -> &GpuWorkResourceId {
        self.handle.diagnostic_identity_ref()
    }

    pub fn size_bytes(&self) -> u64 {
        self.handle.descriptor().size_bytes()
    }

    pub const fn params_type_id(&self) -> TypeId {
        self.layout.params_type_id()
    }

    pub const fn params_type_name(&self) -> &'static str {
        self.layout.params_type_name()
    }
}

#[derive(Debug, Clone)]
pub struct RenderStorageDeclaration {
    handle: GpuBufferHandle,
    layout: RenderGpuParamsLayout,
}

impl RenderStorageDeclaration {
    pub fn handle(&self) -> &GpuBufferHandle {
        &self.handle
    }

    pub const fn layout(&self) -> RenderGpuParamsLayout {
        self.layout
    }

    pub fn id(&self) -> &GpuWorkResourceId {
        self.handle.diagnostic_identity_ref()
    }

    pub fn size_bytes(&self) -> u64 {
        self.handle.descriptor().size_bytes()
    }

    pub const fn element_count(&self) -> u64 {
        self.layout.gpu_layout().element_count()
    }

    pub const fn params_type_id(&self) -> TypeId {
        self.layout.params_type_id()
    }

    pub const fn params_type_name(&self) -> &'static str {
        self.layout.params_type_name()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTextureSizePolicy {
    Surface,
    Fixed { width: u32, height: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTextureFormatPolicy {
    Surface,
    Exact(RenderTextureTargetFormat),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderTextureDescriptor {
    pub size: RenderTextureSizePolicy,
    pub format: RenderTextureFormatPolicy,
    pub usage: RenderTextureTargetUsage,
    pub sample_mode: RenderTextureSampleMode,
}

impl RenderTextureDescriptor {
    pub const fn surface_color() -> Self {
        Self {
            size: RenderTextureSizePolicy::Surface,
            format: RenderTextureFormatPolicy::Surface,
            usage: RenderTextureTargetUsage::color_sampled(),
            sample_mode: RenderTextureSampleMode::FilterableFloat,
        }
    }

    pub const fn surface_color_exact(format: RenderTextureTargetFormat) -> Self {
        Self {
            size: RenderTextureSizePolicy::Surface,
            format: RenderTextureFormatPolicy::Exact(format),
            usage: RenderTextureTargetUsage::color_sampled(),
            sample_mode: RenderTextureSampleMode::FilterableFloat,
        }
    }

    pub const fn surface_sampled() -> Self {
        Self {
            size: RenderTextureSizePolicy::Surface,
            format: RenderTextureFormatPolicy::Surface,
            usage: RenderTextureTargetUsage::sampled_only(),
            sample_mode: RenderTextureSampleMode::FilterableFloat,
        }
    }

    pub const fn storage_rgba8() -> Self {
        Self {
            size: RenderTextureSizePolicy::Surface,
            format: RenderTextureFormatPolicy::Exact(RenderTextureTargetFormat::Rgba8Unorm),
            usage: RenderTextureTargetUsage::storage_sampled(),
            sample_mode: RenderTextureSampleMode::NonFilterableFloat,
        }
    }

    pub const fn surface_depth() -> Self {
        Self {
            size: RenderTextureSizePolicy::Surface,
            format: RenderTextureFormatPolicy::Exact(RenderTextureTargetFormat::Depth32Float),
            usage: RenderTextureTargetUsage::depth_sampled(),
            sample_mode: RenderTextureSampleMode::Depth,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderTextureIntent {
    pub id: GpuWorkResourceId,
    pub label: String,
    pub lifetime: GpuResourceLifetime,
    pub texture: RenderTextureDescriptor,
}

impl PartialEq for RenderTextureIntent {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.lifetime == other.lifetime && self.texture == other.texture
    }
}

impl Eq for RenderTextureIntent {}

impl RenderTextureIntent {
    pub const fn id(&self) -> GpuWorkResourceId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn lifetime(&self) -> GpuResourceLifetime {
        self.lifetime
    }

    pub const fn texture(&self) -> RenderTextureDescriptor {
        self.texture
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderTargetAliasKind {
    Color,
    Depth,
    Texture,
}

/// Validated render-owned semantic key for resolving one target-alias binding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderTargetAliasKey(String);

impl RenderTargetAliasKey {
    pub fn new(value: impl Into<String>) -> Result<Self, RenderGpuResourceAdapterError> {
        let value = value.into();
        let normalized = value.trim();
        if normalized.is_empty() {
            return Err(
                RenderGpuResourceAdapterError::InvalidTargetAliasBindingKey {
                    value,
                    correction: "provide at least one non-whitespace character",
                },
            );
        }
        Ok(Self(normalized.to_string()))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for RenderTargetAliasKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderTargetAliasDeclaration {
    id: GpuWorkResourceId,
    binding_key: RenderTargetAliasKey,
    kind: RenderTargetAliasKind,
}

impl RenderTargetAliasDeclaration {
    pub fn new(
        id: GpuWorkResourceId,
        binding_key: impl Into<String>,
        kind: RenderTargetAliasKind,
    ) -> Result<Self, RenderGpuResourceAdapterError> {
        Ok(Self {
            id,
            binding_key: RenderTargetAliasKey::new(binding_key)?,
            kind,
        })
    }

    pub const fn id(&self) -> GpuWorkResourceId {
        self.id
    }

    pub fn binding_key(&self) -> &RenderTargetAliasKey {
        &self.binding_key
    }

    pub const fn kind(&self) -> RenderTargetAliasKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderImportedTextureSemantic {
    SurfaceColor,
    SurfaceDepth,
    HistoryTexture,
    External,
}

impl RenderImportedTextureSemantic {
    pub fn is_external(self) -> bool {
        matches!(self, Self::External)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SurfaceColor => "surface_color",
            Self::SurfaceDepth => "surface_depth",
            Self::HistoryTexture => "history_texture",
            Self::External => "external",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderImportedBufferSemantic {
    HistoryBuffer,
    External,
}

impl RenderImportedBufferSemantic {
    pub fn is_external(self) -> bool {
        matches!(self, Self::External)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HistoryBuffer => "history_buffer",
            Self::External => "external",
        }
    }
}

/// Render-owned imported-texture facts awaiting complete admission facts.
///
/// G4 resolves ordinary imported textures. G7 resolves surface acquisition and
/// presentation-owned facts. Render history remains render-owned policy while
/// its generic imported-resource realization moves to the owning phase.
#[derive(Debug, Clone)]
pub struct RenderImportedTextureIntent {
    pub id: GpuWorkResourceId,
    pub label: String,
    pub semantic: RenderImportedTextureSemantic,
}

impl PartialEq for RenderImportedTextureIntent {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.semantic == other.semantic
    }
}

impl Eq for RenderImportedTextureIntent {}

/// Render-owned imported-buffer facts awaiting G4 admission and realization.
#[derive(Debug, Clone)]
pub struct RenderImportedBufferIntent {
    pub id: GpuWorkResourceId,
    pub label: String,
    pub semantic: RenderImportedBufferSemantic,
}

impl PartialEq for RenderImportedBufferIntent {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.semantic == other.semantic
    }
}

impl Eq for RenderImportedBufferIntent {}

/// Explicit result of lowering one current render resource declaration.
///
/// Only `Normalized` contains complete checked G2 descriptor facts. Imports
/// retain their render-owned admission intent until G4 or G7 supplies the
/// missing facts, and target aliases remain render-graph relationships.
/// Equality is intentionally retained because every variant has an explicit
/// semantic contract: normalized descriptor facts, import ID plus semantic,
/// or target-alias ID plus binding key plus kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderGpuResourceLowering {
    Normalized(Box<GpuResourceDescriptor>),
    ImportedTexture(RenderImportedTextureIntent),
    ImportedBuffer(RenderImportedBufferIntent),
    TargetAlias(RenderTargetAliasDeclaration),
}

#[derive(Debug, Clone)]
pub enum RenderResourceDeclaration {
    Uniform(RenderUniformDeclaration),
    Storage(RenderStorageDeclaration),
    Sampled(RenderTextureIntent),
    StorageImage(RenderTextureIntent),
    ColorAttachment(RenderTextureIntent),
    DepthAttachment(RenderTextureIntent),
    History(RenderTextureIntent),
    TargetAlias(RenderTargetAliasDeclaration),
    ImportedTexture(RenderImportedTextureIntent),
    ImportedBuffer(RenderImportedBufferIntent),
}

impl RenderResourceDeclaration {
    pub fn declare_uniform<Params: GpuParams + 'static>(
        allocator: &mut GpuWorkResourceIdAllocator,
        label: impl Into<String>,
    ) -> Result<Self, RenderGpuResourceAdapterError> {
        Self::declare_uniform_with_lifetime::<Params>(
            allocator,
            label,
            GpuResourceLifetime::Retained,
        )
    }

    pub fn declare_uniform_with_lifetime<Params: GpuParams + 'static>(
        allocator: &mut GpuWorkResourceIdAllocator,
        label: impl Into<String>,
        lifetime: GpuResourceLifetime,
    ) -> Result<Self, RenderGpuResourceAdapterError> {
        let label = label.into();
        let layout = RenderGpuParamsLayout::uniform::<Params>(&label)?;
        let common = owned_common(&label, lifetime)?;
        let usages = GpuBufferUsages::new(
            common.label(),
            [
                GpuBufferUsage::Uniform,
                GpuBufferUsage::CopySource,
                GpuBufferUsage::CopyDestination,
            ],
        )?;
        let descriptor = GpuBufferDescriptor::new(
            common,
            layout.gpu_layout().byte_len(),
            usages,
            GpuBufferInitialization::Uninitialized,
        )?;
        let handle = allocator.allocate_buffer_handle(descriptor)?;
        Ok(Self::Uniform(RenderUniformDeclaration { handle, layout }))
    }

    pub fn declare_storage<Params: GpuParams + 'static>(
        allocator: &mut GpuWorkResourceIdAllocator,
        label: impl Into<String>,
    ) -> Result<Self, RenderGpuResourceAdapterError> {
        Self::declare_storage_array::<Params>(allocator, label, 1)
    }

    pub fn declare_storage_array<Params: GpuParams + 'static>(
        allocator: &mut GpuWorkResourceIdAllocator,
        label: impl Into<String>,
        element_count: u64,
    ) -> Result<Self, RenderGpuResourceAdapterError> {
        Self::declare_storage_array_with_lifetime::<Params>(
            allocator,
            label,
            element_count,
            GpuResourceLifetime::Retained,
        )
    }

    pub fn declare_storage_array_with_lifetime<Params: GpuParams + 'static>(
        allocator: &mut GpuWorkResourceIdAllocator,
        label: impl Into<String>,
        element_count: u64,
        lifetime: GpuResourceLifetime,
    ) -> Result<Self, RenderGpuResourceAdapterError> {
        Self::declare_storage_array_with_lifetime_and_initialization::<Params>(
            allocator,
            label,
            element_count,
            lifetime,
            GpuBufferInitialization::Uninitialized,
        )
    }

    pub(crate) fn declare_storage_array_with_lifetime_and_initialization<
        Params: GpuParams + 'static,
    >(
        allocator: &mut GpuWorkResourceIdAllocator,
        label: impl Into<String>,
        element_count: u64,
        lifetime: GpuResourceLifetime,
        initialization: GpuBufferInitialization,
    ) -> Result<Self, RenderGpuResourceAdapterError> {
        let label = label.into();
        let layout = RenderGpuParamsLayout::storage::<Params>(&label, element_count)?;
        let common = owned_common(&label, lifetime)?;
        let usages = GpuBufferUsages::new(
            common.label(),
            [
                GpuBufferUsage::Storage,
                GpuBufferUsage::Vertex,
                GpuBufferUsage::Index,
                GpuBufferUsage::Indirect,
                GpuBufferUsage::CopySource,
                GpuBufferUsage::CopyDestination,
            ],
        )?;
        let descriptor = GpuBufferDescriptor::new(
            common,
            layout.gpu_layout().byte_len(),
            usages,
            initialization,
        )?;
        let handle = allocator.allocate_buffer_handle(descriptor)?;
        Ok(Self::Storage(RenderStorageDeclaration { handle, layout }))
    }

    pub fn declare_sampled_texture(id: GpuWorkResourceId, label: impl Into<String>) -> Self {
        Self::declare_sampled_texture_with_lifetime(id, label, GpuResourceLifetime::Retained)
    }

    pub fn declare_sampled_texture_with_lifetime(
        id: GpuWorkResourceId,
        label: impl Into<String>,
        lifetime: GpuResourceLifetime,
    ) -> Self {
        Self::Sampled(texture_intent(
            id,
            label,
            lifetime,
            RenderTextureDescriptor::surface_sampled(),
        ))
    }

    pub fn declare_storage_texture(id: GpuWorkResourceId, label: impl Into<String>) -> Self {
        Self::declare_storage_texture_with_lifetime(id, label, GpuResourceLifetime::Retained)
    }

    pub fn declare_storage_texture_with_lifetime(
        id: GpuWorkResourceId,
        label: impl Into<String>,
        lifetime: GpuResourceLifetime,
    ) -> Self {
        Self::StorageImage(texture_intent(
            id,
            label,
            lifetime,
            RenderTextureDescriptor::storage_rgba8(),
        ))
    }

    pub fn declare_color_attachment(id: GpuWorkResourceId, label: impl Into<String>) -> Self {
        Self::declare_color_attachment_with_lifetime(id, label, GpuResourceLifetime::Retained)
    }

    pub fn declare_color_attachment_exact(
        id: GpuWorkResourceId,
        label: impl Into<String>,
        format: RenderTextureTargetFormat,
    ) -> Self {
        Self::declare_color_attachment_exact_with_lifetime(
            id,
            label,
            format,
            GpuResourceLifetime::Retained,
        )
    }

    pub fn declare_color_attachment_with_lifetime(
        id: GpuWorkResourceId,
        label: impl Into<String>,
        lifetime: GpuResourceLifetime,
    ) -> Self {
        Self::ColorAttachment(texture_intent(
            id,
            label,
            lifetime,
            RenderTextureDescriptor::surface_color(),
        ))
    }

    pub fn declare_color_attachment_exact_with_lifetime(
        id: GpuWorkResourceId,
        label: impl Into<String>,
        format: RenderTextureTargetFormat,
        lifetime: GpuResourceLifetime,
    ) -> Self {
        Self::ColorAttachment(texture_intent(
            id,
            label,
            lifetime,
            RenderTextureDescriptor::surface_color_exact(format),
        ))
    }

    pub fn declare_depth_attachment(id: GpuWorkResourceId, label: impl Into<String>) -> Self {
        Self::declare_depth_attachment_with_lifetime(id, label, GpuResourceLifetime::Retained)
    }

    pub fn declare_depth_attachment_with_lifetime(
        id: GpuWorkResourceId,
        label: impl Into<String>,
        lifetime: GpuResourceLifetime,
    ) -> Self {
        Self::DepthAttachment(texture_intent(
            id,
            label,
            lifetime,
            RenderTextureDescriptor::surface_depth(),
        ))
    }

    pub fn declare_history_texture(id: GpuWorkResourceId, label: impl Into<String>) -> Self {
        Self::History(texture_intent(
            id,
            label,
            GpuResourceLifetime::Retained,
            RenderTextureDescriptor::surface_color(),
        ))
    }

    pub fn declare_target_alias(
        id: GpuWorkResourceId,
        binding_key: impl Into<String>,
        kind: RenderTargetAliasKind,
    ) -> Result<Self, RenderGpuResourceAdapterError> {
        let binding_key = RenderTargetAliasKey::new(binding_key)?;
        Ok(Self::declare_target_alias_with_key(id, binding_key, kind))
    }

    pub(crate) fn declare_target_alias_with_key(
        id: GpuWorkResourceId,
        binding_key: RenderTargetAliasKey,
        kind: RenderTargetAliasKind,
    ) -> Self {
        Self::TargetAlias(RenderTargetAliasDeclaration {
            id,
            binding_key,
            kind,
        })
    }

    pub fn declare_imported_surface_color(id: GpuWorkResourceId, label: impl Into<String>) -> Self {
        Self::imported_texture(id, label, RenderImportedTextureSemantic::SurfaceColor)
    }

    pub fn declare_imported_surface_depth(id: GpuWorkResourceId, label: impl Into<String>) -> Self {
        Self::imported_texture(id, label, RenderImportedTextureSemantic::SurfaceDepth)
    }

    pub fn declare_imported_history_texture(
        id: GpuWorkResourceId,
        label: impl Into<String>,
    ) -> Self {
        Self::imported_texture(id, label, RenderImportedTextureSemantic::HistoryTexture)
    }

    pub fn declare_imported_external_texture(
        id: GpuWorkResourceId,
        label: impl Into<String>,
    ) -> Self {
        Self::imported_texture(id, label, RenderImportedTextureSemantic::External)
    }

    fn imported_texture(
        id: GpuWorkResourceId,
        label: impl Into<String>,
        semantic: RenderImportedTextureSemantic,
    ) -> Self {
        Self::ImportedTexture(RenderImportedTextureIntent {
            id,
            label: label.into(),
            semantic,
        })
    }

    pub fn declare_imported_history_buffer(
        id: GpuWorkResourceId,
        label: impl Into<String>,
    ) -> Self {
        Self::imported_buffer(id, label, RenderImportedBufferSemantic::HistoryBuffer)
    }

    pub fn declare_imported_external_buffer(
        id: GpuWorkResourceId,
        label: impl Into<String>,
    ) -> Self {
        Self::imported_buffer(id, label, RenderImportedBufferSemantic::External)
    }

    fn imported_buffer(
        id: GpuWorkResourceId,
        label: impl Into<String>,
        semantic: RenderImportedBufferSemantic,
    ) -> Self {
        Self::ImportedBuffer(RenderImportedBufferIntent {
            id,
            label: label.into(),
            semantic,
        })
    }

    pub fn id(&self) -> &GpuWorkResourceId {
        match self {
            Self::Uniform(value) => value.id(),
            Self::Storage(value) => value.id(),
            Self::Sampled(value)
            | Self::StorageImage(value)
            | Self::ColorAttachment(value)
            | Self::DepthAttachment(value)
            | Self::History(value) => &value.id,
            Self::TargetAlias(value) => &value.id,
            Self::ImportedTexture(value) => &value.id,
            Self::ImportedBuffer(value) => &value.id,
        }
    }

    pub fn lifetime(&self) -> GpuResourceLifetime {
        match self {
            Self::Uniform(value) => value.handle.descriptor().common().lifetime(),
            Self::Storage(value) => value.handle.descriptor().common().lifetime(),
            Self::Sampled(value)
            | Self::StorageImage(value)
            | Self::ColorAttachment(value)
            | Self::DepthAttachment(value)
            | Self::History(value) => value.lifetime,
            Self::TargetAlias(_) | Self::ImportedTexture(_) | Self::ImportedBuffer(_) => {
                GpuResourceLifetime::Retained
            }
        }
    }

    pub fn is_imported(&self) -> bool {
        matches!(self, Self::ImportedTexture(_) | Self::ImportedBuffer(_))
    }

    pub fn imported_texture_semantic(&self) -> Option<RenderImportedTextureSemantic> {
        match self {
            Self::ImportedTexture(value) => Some(value.semantic),
            _ => None,
        }
    }

    pub fn imported_buffer_semantic(&self) -> Option<RenderImportedBufferSemantic> {
        match self {
            Self::ImportedBuffer(value) => Some(value.semantic),
            _ => None,
        }
    }

    pub fn buffer_handle(&self) -> Option<&GpuBufferHandle> {
        match self {
            Self::Uniform(value) => Some(&value.handle),
            Self::Storage(value) => Some(&value.handle),
            _ => None,
        }
    }

    pub fn params_layout(&self) -> Option<RenderGpuParamsLayout> {
        match self {
            Self::Uniform(value) => Some(value.layout),
            Self::Storage(value) => Some(value.layout),
            _ => None,
        }
    }

    pub fn params_type_id(&self) -> Option<TypeId> {
        self.params_layout().map(|layout| layout.params_type_id())
    }

    pub fn texture_intent(&self) -> Option<&RenderTextureIntent> {
        match self {
            Self::Sampled(value)
            | Self::StorageImage(value)
            | Self::ColorAttachment(value)
            | Self::DepthAttachment(value)
            | Self::History(value) => Some(value),
            _ => None,
        }
    }

    pub fn lower_gpu_resource(
        &self,
        resolved_size: (u32, u32),
        resolved_surface_format: GpuTextureFormat,
    ) -> Result<RenderGpuResourceLowering, RenderGpuResourceAdapterError> {
        match self {
            Self::Uniform(value) => Ok(RenderGpuResourceLowering::Normalized(Box::new(
                GpuResourceDescriptor::Buffer(value.handle.descriptor().clone()),
            ))),
            Self::Storage(value) => Ok(RenderGpuResourceLowering::Normalized(Box::new(
                GpuResourceDescriptor::Buffer(value.handle.descriptor().clone()),
            ))),
            Self::Sampled(value)
            | Self::StorageImage(value)
            | Self::ColorAttachment(value)
            | Self::DepthAttachment(value)
            | Self::History(value) => Ok(RenderGpuResourceLowering::Normalized(Box::new(
                GpuResourceDescriptor::Texture(lower_texture_intent(
                    value,
                    resolved_size,
                    resolved_surface_format,
                )?),
            ))),
            Self::ImportedTexture(value) => {
                validate_unresolved_intent_label(&value.label)?;
                Ok(RenderGpuResourceLowering::ImportedTexture(value.clone()))
            }
            Self::ImportedBuffer(value) => {
                validate_unresolved_intent_label(&value.label)?;
                Ok(RenderGpuResourceLowering::ImportedBuffer(value.clone()))
            }
            Self::TargetAlias(value) => Ok(RenderGpuResourceLowering::TargetAlias(value.clone())),
        }
    }
}

fn validate_unresolved_intent_label(label: &str) -> Result<(), RenderGpuResourceAdapterError> {
    GpuResourceLabel::new(label).map(|_| ()).map_err(Into::into)
}

fn texture_intent(
    id: GpuWorkResourceId,
    label: impl Into<String>,
    lifetime: GpuResourceLifetime,
    texture: RenderTextureDescriptor,
) -> RenderTextureIntent {
    RenderTextureIntent {
        id,
        label: label.into(),
        lifetime,
        texture,
    }
}

fn owned_common(
    label: &str,
    lifetime: GpuResourceLifetime,
) -> Result<GpuResourceCommon, GpuResourceDescriptorError> {
    let label = GpuResourceLabel::new(label)?;
    let provenance = GpuResourceProvenance::new(label.clone(), None, None);
    GpuResourceCommon::owned(
        label,
        lifetime,
        GpuMemoryIntent::Device,
        GpuReconstruction::SourceBacked,
        provenance,
    )
}

fn lower_texture_intent(
    intent: &RenderTextureIntent,
    resolved_size: (u32, u32),
    resolved_surface_format: GpuTextureFormat,
) -> Result<GpuTextureDescriptor, RenderGpuResourceAdapterError> {
    let common = owned_common(&intent.label, intent.lifetime)?;
    let size = match intent.texture.size {
        RenderTextureSizePolicy::Surface => resolved_size,
        RenderTextureSizePolicy::Fixed { width, height } => (width, height),
    };
    let format = match intent.texture.format {
        RenderTextureFormatPolicy::Surface => resolved_surface_format,
        RenderTextureFormatPolicy::Exact(format) => normalized_render_format(format),
    };
    let extent = GpuTextureExtent::new(common.label(), GpuTextureDimension::D2, size.0, size.1, 1)?;
    let usages = GpuTextureUsages::new(common.label(), normalized_texture_usages(intent.texture))?;
    Ok(GpuTextureDescriptor::new(
        common,
        GpuTextureDimension::D2,
        extent,
        1,
        1,
        format,
        usages,
        GpuTextureInitialization::Uninitialized,
    )?)
}

fn normalized_texture_usages(texture: RenderTextureDescriptor) -> BTreeSet<GpuTextureUsage> {
    let mut usages = BTreeSet::new();
    if texture.usage.sampled {
        usages.insert(GpuTextureUsage::Sampled);
    }
    if texture.usage.storage {
        usages.insert(GpuTextureUsage::StorageWrite);
    }
    if texture.usage.color_attachment {
        usages.insert(GpuTextureUsage::ColorAttachment);
    }
    if texture.usage.depth_attachment {
        usages.insert(GpuTextureUsage::DepthStencilAttachment);
    }
    if texture.usage.copy_src {
        usages.insert(GpuTextureUsage::CopySource);
    }
    if texture.usage.copy_dst {
        usages.insert(GpuTextureUsage::CopyDestination);
    }
    usages
}

pub fn detect_duplicate_resource_ids(
    declarations: &[RenderResourceDeclaration],
) -> Vec<GpuWorkResourceId> {
    let mut seen = BTreeSet::<GpuWorkResourceId>::new();
    let mut duplicates = BTreeSet::<GpuWorkResourceId>::new();
    for declaration in declarations {
        let id = *declaration.id();
        if !seen.insert(id) {
            duplicates.insert(id);
        }
    }
    duplicates.into_iter().collect()
}
