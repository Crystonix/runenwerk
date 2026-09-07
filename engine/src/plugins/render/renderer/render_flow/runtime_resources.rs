use super::*;
use crate::plugins::render::{
    PreparedTargetBinding, RenderDynamicTextureTargetKey, RenderFlowId,
    RenderGpuResourceAdapterError, RenderImportedBufferIntent, RenderImportedTextureIntent,
    RenderPassId, RenderTargetAliasDeclaration, RenderTargetAliasKey,
    prepare_projected_uniform_bytes,
};
use runen_gpu::{
    GpuBufferDescriptor, GpuBufferHandle, GpuBufferUsages, GpuTextureDescriptor,
    GpuTextureDimension, GpuTextureFormat, GpuTextureHandle, GpuTextureUsages,
    GpuTextureViewHandle, GpuWorkResourceId, GpuWorkResourceIdAllocator, PreparedGpuData,
    UniformData,
};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeResourceKind {
    TextureLike,
    BufferLike,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeBufferKind {
    Uniform,
    Storage,
}

#[derive(Debug)]
pub struct RuntimeTextureResource {
    pub handle: GpuTextureHandle,
    pub view_handle: GpuTextureViewHandle,
    pub format: GpuTextureFormat,
    pub size: (u32, u32),
    pub usage: GpuTextureUsages,
    pub is_depth: bool,
    pub history_signature: Option<String>,
    pub generation: u64,
    pub reused_last_frame: bool,
}

#[derive(Debug)]
pub struct RuntimeBufferResource {
    pub handle: GpuBufferHandle,
    pub size: u64,
    pub kind: RuntimeBufferKind,
    pub generation: u64,
    pub reused_last_frame: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureAllocationSpec {
    pub descriptor: GpuTextureDescriptor,
    pub size: (u32, u32),
    pub format: GpuTextureFormat,
    pub usage: GpuTextureUsages,
    pub is_depth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferAllocationSpec {
    pub descriptor: GpuBufferDescriptor,
    pub size: u64,
    pub usage: GpuBufferUsages,
    pub kind: RuntimeBufferKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CurrentRuntimeResourceDisposition {
    Buffer(BufferAllocationSpec),
    FlowTexture(TextureAllocationSpec),
    InvocationHistoryTexture(TextureAllocationSpec),
    ImportedTexture(RenderImportedTextureIntent),
    ImportedBuffer(RenderImportedBufferIntent),
    TargetAlias(RenderTargetAliasDeclaration),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
enum CurrentRuntimeResourceRealizationError {
    #[error(transparent)]
    Adapter(#[from] RenderGpuResourceAdapterError),
    #[error(
        "current render runtime cannot realize normalized GPU {kind} resource '{resource_id}'; the owning later phase must add an explicit realization path"
    )]
    UnsupportedNormalizedKind {
        resource_id: GpuWorkResourceId,
        kind: &'static str,
    },
    #[error(
        "current render runtime cannot realize normalized texture resource '{resource_id}' with shape {dimension:?} and depth/layers {depth_or_layers}; only 2D single-layer textures are supported"
    )]
    UnsupportedTextureShape {
        resource_id: GpuWorkResourceId,
        dimension: GpuTextureDimension,
        depth_or_layers: u32,
    },
    #[error(
        "current render declaration '{resource_id}' produced normalized GPU {normalized_kind} facts that do not match its render-owned declaration kind"
    )]
    NormalizedDeclarationMismatch {
        resource_id: GpuWorkResourceId,
        normalized_kind: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeResourceKey {
    FlowOwned(GpuWorkResourceId),
    InvocationUniform {
        invocation_id: String,
        resource_id: GpuWorkResourceId,
    },
    InvocationHistory {
        invocation_id: String,
        resource_id: GpuWorkResourceId,
    },
    DynamicTexture(RenderDynamicTextureTargetKey),
    SurfaceColor,
    SurfaceDepth,
}

impl fmt::Display for RuntimeResourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FlowOwned(id) => write!(f, "{}", id),
            Self::InvocationUniform {
                invocation_id,
                resource_id,
            } => write!(f, "{}@{}", resource_id, invocation_id),
            Self::InvocationHistory {
                invocation_id,
                resource_id,
            } => write!(f, "{}@history:{}", resource_id, invocation_id),
            Self::DynamicTexture(key) => write!(f, "{}", key),
            Self::SurfaceColor => f.write_str(SURFACE_COLOR_RESOURCE_LABEL),
            Self::SurfaceDepth => f.write_str(SURFACE_DEPTH_RESOURCE_LABEL),
        }
    }
}

#[derive(Debug, Default)]
pub struct FlowRuntimeResources {
    resource_ids: GpuWorkResourceIdAllocator,
    pub textures: BTreeMap<GpuWorkResourceId, RuntimeTextureResource>,
    pub buffers: BTreeMap<GpuWorkResourceId, RuntimeBufferResource>,
    pub invocation_uniform_buffers: BTreeMap<(String, GpuWorkResourceId), RuntimeBufferResource>,
    pub invocation_history_textures: BTreeMap<(String, GpuWorkResourceId), RuntimeTextureResource>,
    pub active_invocation_uniform_scope: Option<String>,
    pub kinds: BTreeMap<GpuWorkResourceId, RuntimeResourceKind>,
    pub descriptors: BTreeMap<GpuWorkResourceId, RenderResourceDeclaration>,
    pub resource_ids_by_label: BTreeMap<String, GpuWorkResourceId>,
    pub target_alias_bindings: BTreeMap<RenderTargetAliasKey, PreparedTargetBinding>,
}

impl FlowRuntimeResources {
    pub(super) fn prepare_uniform_upload(
        &self,
        resource_id: GpuWorkResourceId,
        bytes: &[u8],
    ) -> anyhow::Result<PreparedGpuData<UniformData>> {
        let declaration = self.descriptors.get(&resource_id).ok_or_else(|| {
            anyhow::anyhow!(
                "uniform upload references undeclared logical resource '{}'",
                resource_id
            )
        })?;
        let RenderResourceDeclaration::Uniform(uniform) = declaration else {
            anyhow::bail!(
                "uniform upload references non-uniform logical resource '{}'",
                resource_id
            );
        };
        let common = uniform.handle().descriptor().common();
        Ok(prepare_projected_uniform_bytes(
            common.label().as_str(),
            bytes.to_vec(),
            uniform.layout(),
            common.provenance().clone(),
        )?)
    }
}

#[derive(Debug)]
pub struct ResolvedTextureRef<'a> {
    pub id: RuntimeResourceKey,
    pub view_handle: Option<&'a GpuTextureViewHandle>,
    pub format: GpuTextureFormat,
    pub size: (u32, u32),
    pub is_depth: bool,
}

#[derive(Debug)]
pub struct ResolvedBufferRef<'a> {
    pub id: RuntimeResourceKey,
    pub handle: &'a GpuBufferHandle,
    pub size: u64,
    pub kind: RuntimeBufferKind,
}

mod inspect;
mod realize;
mod resolve;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::{
        CompiledTargetAliasRef, GpuParams, RenderGpuResourceLowering, RenderImportedBufferSemantic,
        RenderImportedTextureSemantic, RenderTargetAliasKind, RenderTextureIntent,
    };
    use runen_gpu::{
        GpuAddressMode, GpuBufferUsage, GpuFilterMode, GpuMemoryIntent, GpuQueryKind,
        GpuQuerySetDescriptor, GpuReconstruction, GpuResourceCommon, GpuResourceDescriptor,
        GpuResourceLabel, GpuResourceLifetime, GpuResourceProvenance, GpuSamplerDescriptor,
        GpuTextureAspect, GpuTextureDescriptor, GpuTextureExtent, GpuTextureFormat,
        GpuTextureInitialization, GpuTextureSubresourceRange, GpuTextureUsage, GpuTextureUsages,
        GpuTextureViewDescriptor, GpuWorkResourceIdAllocator,
    };

    struct RuntimeTestUniform(u32);

    impl GpuParams for RuntimeTestUniform {
        type Raw = u32;

        fn to_gpu(&self) -> Self::Raw {
            self.0
        }
    }

    fn resource(local: u64) -> GpuWorkResourceId {
        let mut allocator = GpuWorkResourceIdAllocator::new();
        (1..=local)
            .map(|_| {
                allocator
                    .allocate()
                    .expect("test allocation should succeed")
            })
            .last()
            .expect("test local value is nonzero")
    }

    fn gpu_label(value: &str) -> GpuResourceLabel {
        GpuResourceLabel::new(value).unwrap()
    }

    fn gpu_common(value: &str) -> GpuResourceCommon {
        let label = gpu_label(value);
        GpuResourceCommon::owned(
            label.clone(),
            GpuResourceLifetime::Retained,
            GpuMemoryIntent::Device,
            GpuReconstruction::SourceBacked,
            GpuResourceProvenance::new(label, None, None),
        )
        .unwrap()
    }

    fn normalized_texture_view() -> GpuTextureViewDescriptor {
        let parent_label = gpu_label("runtime test parent texture");
        let parent = GpuTextureDescriptor::new(
            gpu_common("runtime test parent texture"),
            GpuTextureDimension::D2,
            GpuTextureExtent::new(&parent_label, GpuTextureDimension::D2, 4, 4, 1).unwrap(),
            1,
            1,
            GpuTextureFormat::Rgba8Unorm,
            GpuTextureUsages::new(&parent_label, [GpuTextureUsage::Sampled]).unwrap(),
            GpuTextureInitialization::Uninitialized,
        )
        .unwrap();
        let mut allocator = GpuWorkResourceIdAllocator::new();
        let handle = allocator.allocate_texture_handle(parent).unwrap();
        let view_label = gpu_label("runtime test texture view");
        let subresources =
            GpuTextureSubresourceRange::new(&view_label, 0, 1, 0, 1, GpuTextureAspect::Color)
                .unwrap();
        GpuTextureViewDescriptor::new(
            gpu_common("runtime test texture view"),
            &handle,
            None,
            GpuTextureDimension::D2,
            subresources,
        )
        .unwrap()
    }

    #[test]
    fn kind_of_resolves_label_alias_to_runtime_id() {
        let mut resources = FlowRuntimeResources::default();
        let id = resource(42);
        resources.kinds.insert(id, RuntimeResourceKind::TextureLike);
        resources
            .resource_ids_by_label
            .insert("editor.viewport.v1.scene_color".to_string(), id);

        assert_eq!(
            resources.kind_of("editor.viewport.v1.scene_color"),
            Some(RuntimeResourceKind::TextureLike),
        );
    }

    #[test]
    fn capture_texture_class_resolves_label_alias_to_descriptor() {
        let mut resources = FlowRuntimeResources::default();
        let id = resource(7);
        resources.descriptors.insert(
            id,
            RenderResourceDeclaration::declare_color_attachment(id, "overlay"),
        );
        resources
            .resource_ids_by_label
            .insert("editor.viewport.v1.overlay".to_string(), id);

        assert_eq!(
            resources.capture_texture_class(
                "editor.viewport.v1.overlay",
                CaptureTextureClass::DepthTarget,
            ),
            CaptureTextureClass::ColorTarget,
        );
    }

    #[test]
    fn runtime_target_alias_lookup_distinguishes_binding_keys() {
        let mut resources = FlowRuntimeResources::default();
        let color_key = RenderTargetAliasKey::new("viewport.color").unwrap();
        let depth_key = RenderTargetAliasKey::new("viewport.depth").unwrap();
        resources
            .target_alias_bindings
            .insert(color_key.clone(), PreparedTargetBinding::SurfaceColor);
        resources
            .target_alias_bindings
            .insert(depth_key.clone(), PreparedTargetBinding::SurfaceDepth);
        let pass_id = RenderPassId::try_from_raw(1).unwrap();

        let color = resources
            .resolve_resource_key(
                pass_id,
                &CompiledResourceRef::TargetAlias(CompiledTargetAliasRef {
                    resource_id: resource(8),
                    binding_key: color_key,
                    kind: RenderTargetAliasKind::Color,
                }),
                "color_output",
            )
            .unwrap();
        let depth = resources
            .resolve_resource_key(
                pass_id,
                &CompiledResourceRef::TargetAlias(CompiledTargetAliasRef {
                    resource_id: resource(9),
                    binding_key: depth_key,
                    kind: RenderTargetAliasKind::Depth,
                }),
                "depth_output",
            )
            .unwrap();

        assert_eq!(color, RuntimeResourceKey::SurfaceColor);
        assert_eq!(depth, RuntimeResourceKey::SurfaceDepth);
    }

    #[test]
    fn flow_owned_texture_allocation_honors_fixed_size_exact_format_and_usage() {
        let id = resource(11);
        let descriptor = RenderResourceDeclaration::StorageImage(RenderTextureIntent {
            id,
            label: "fixed storage".to_string(),
            lifetime: GpuResourceLifetime::Retained,
            texture: crate::plugins::render::RenderTextureDescriptor {
                size: crate::plugins::render::RenderTextureSizePolicy::Fixed {
                    width: 320,
                    height: 180,
                },
                format: crate::plugins::render::RenderTextureFormatPolicy::Exact(
                    crate::plugins::render::RenderTextureTargetFormat::R32Uint,
                ),
                usage: crate::plugins::render::RenderTextureTargetUsage {
                    color_attachment: false,
                    depth_attachment: false,
                    sampled: true,
                    storage: true,
                    copy_src: false,
                    copy_dst: true,
                },
                sample_mode: crate::plugins::render::RenderTextureSampleMode::Uint,
            },
        });

        let disposition = FlowRuntimeResources::current_runtime_resource_disposition(
            &descriptor,
            (1920, 1080),
            GpuTextureFormat::Bgra8UnormSrgb,
        )
        .expect("normalized descriptor lowering should succeed");
        let CurrentRuntimeResourceDisposition::FlowTexture(spec) = disposition else {
            panic!("storage texture should have flow-owned allocation facts");
        };

        assert_eq!(spec.size, (320, 180));
        assert_eq!(spec.format, GpuTextureFormat::R32Uint);
        assert!(spec.usage.contains(GpuTextureUsage::Sampled));
        assert!(spec.usage.contains(GpuTextureUsage::StorageWrite));
        assert!(spec.usage.contains(GpuTextureUsage::CopyDestination));
        assert!(!spec.usage.contains(GpuTextureUsage::ColorAttachment));
        assert!(!spec.usage.contains(GpuTextureUsage::CopySource));
    }

    #[test]
    fn flow_owned_texture_allocation_resolves_surface_policy_from_frame() {
        let id = resource(12);
        let descriptor = RenderResourceDeclaration::declare_color_attachment(id, "color");

        let disposition = FlowRuntimeResources::current_runtime_resource_disposition(
            &descriptor,
            (1280, 720),
            GpuTextureFormat::Rgba8UnormSrgb,
        )
        .expect("normalized descriptor lowering should succeed");
        let CurrentRuntimeResourceDisposition::FlowTexture(spec) = disposition else {
            panic!("color target should have flow-owned allocation facts");
        };

        assert_eq!(spec.size, (1280, 720));
        assert_eq!(spec.format, GpuTextureFormat::Rgba8UnormSrgb);
        assert!(spec.usage.contains(GpuTextureUsage::ColorAttachment));
        assert!(spec.usage.contains(GpuTextureUsage::Sampled));
    }

    #[test]
    fn exact_color_target_allocation_ignores_surface_format_policy() {
        let id = resource(13);
        let descriptor = RenderResourceDeclaration::declare_color_attachment_exact(
            id,
            "exact color",
            crate::plugins::render::RenderTextureTargetFormat::Rgba8Unorm,
        );

        let disposition = FlowRuntimeResources::current_runtime_resource_disposition(
            &descriptor,
            (1280, 720),
            GpuTextureFormat::Rgba8UnormSrgb,
        )
        .expect("normalized descriptor lowering should succeed");
        let CurrentRuntimeResourceDisposition::FlowTexture(spec) = disposition else {
            panic!("exact color target should have flow-owned allocation facts");
        };

        assert_eq!(spec.size, (1280, 720));
        assert_eq!(spec.format, GpuTextureFormat::Rgba8Unorm);
        assert!(spec.usage.contains(GpuTextureUsage::ColorAttachment));
        assert!(spec.usage.contains(GpuTextureUsage::Sampled));
    }

    #[test]
    fn normalized_buffer_produces_current_buffer_allocation_facts() {
        let mut allocator = GpuWorkResourceIdAllocator::new();
        let descriptor = RenderResourceDeclaration::declare_uniform::<RuntimeTestUniform>(
            &mut allocator,
            "runtime uniform",
        )
        .unwrap();

        let disposition = FlowRuntimeResources::current_runtime_resource_disposition(
            &descriptor,
            (1, 1),
            GpuTextureFormat::Rgba8Unorm,
        )
        .unwrap();
        let CurrentRuntimeResourceDisposition::Buffer(spec) = disposition else {
            panic!("uniform should have current buffer allocation facts");
        };

        assert_eq!(spec.kind, RuntimeBufferKind::Uniform);
        assert!(spec.size > 0);
        assert!(spec.usage.contains(GpuBufferUsage::Uniform));
        assert!(spec.usage.contains(GpuBufferUsage::CopyDestination));
        assert!(matches!(
            descriptor,
            RenderResourceDeclaration::Uniform(ref value)
                if value.handle().descriptor().usages().contains(GpuBufferUsage::Uniform)
        ));
    }

    #[test]
    fn imported_and_alias_declarations_are_explicit_non_allocation_dispositions() {
        let ids = [resource(15), resource(16), resource(17)];
        let imported_texture = RenderResourceDeclaration::declare_imported_external_texture(
            ids[0],
            "external texture",
        );
        let imported_buffer =
            RenderResourceDeclaration::declare_imported_external_buffer(ids[1], "external buffer");
        let alias = RenderResourceDeclaration::declare_target_alias(
            ids[2],
            "color alias",
            RenderTargetAliasKind::Color,
        )
        .expect("target alias declaration should be valid");

        assert!(matches!(
            FlowRuntimeResources::current_runtime_resource_disposition(
                &imported_texture,
                (64, 64),
                GpuTextureFormat::Rgba8Unorm,
            )
            .unwrap(),
            CurrentRuntimeResourceDisposition::ImportedTexture(intent)
                if intent.id == ids[0]
                    && intent.label == "external texture"
                    && intent.semantic == RenderImportedTextureSemantic::External
        ));
        assert!(matches!(
            FlowRuntimeResources::current_runtime_resource_disposition(
                &imported_buffer,
                (64, 64),
                GpuTextureFormat::Rgba8Unorm,
            )
            .unwrap(),
            CurrentRuntimeResourceDisposition::ImportedBuffer(intent)
                if intent.id == ids[1]
                    && intent.label == "external buffer"
                    && intent.semantic == RenderImportedBufferSemantic::External
        ));
        assert!(matches!(
            FlowRuntimeResources::current_runtime_resource_disposition(
                &alias,
                (64, 64),
                GpuTextureFormat::Rgba8Unorm,
            )
            .unwrap(),
            CurrentRuntimeResourceDisposition::TargetAlias(value)
                if value.id() == ids[2]
                    && value.binding_key().as_str() == "color alias"
                    && value.kind() == RenderTargetAliasKind::Color
        ));
    }

    #[test]
    fn history_texture_remains_invocation_scoped_with_surface_policy() {
        let descriptor =
            RenderResourceDeclaration::declare_history_texture(resource(18), "history texture");

        let disposition = FlowRuntimeResources::current_runtime_resource_disposition(
            &descriptor,
            (1024, 576),
            GpuTextureFormat::Bgra8UnormSrgb,
        )
        .unwrap();
        let CurrentRuntimeResourceDisposition::InvocationHistoryTexture(spec) = disposition else {
            panic!("history textures must remain invocation-scoped");
        };

        assert_eq!(spec.size, (1024, 576));
        assert_eq!(spec.format, GpuTextureFormat::Bgra8UnormSrgb);
        assert!(spec.usage.contains(GpuTextureUsage::Sampled));
        assert!(spec.usage.contains(GpuTextureUsage::ColorAttachment));
    }

    #[test]
    fn unsupported_normalized_kinds_return_structured_current_runtime_errors() {
        let declaration = RenderResourceDeclaration::declare_color_attachment(
            resource(19),
            "unsupported normalized kind",
        );
        let unsupported = [
            (
                GpuResourceDescriptor::TextureView(normalized_texture_view()),
                "texture-view",
            ),
            (
                GpuResourceDescriptor::Sampler(
                    GpuSamplerDescriptor::new(
                        gpu_common("runtime test sampler"),
                        GpuAddressMode::ClampToEdge,
                        GpuAddressMode::ClampToEdge,
                        GpuAddressMode::ClampToEdge,
                        GpuFilterMode::Nearest,
                        GpuFilterMode::Nearest,
                        GpuFilterMode::Nearest,
                        0.0,
                        1.0,
                        None,
                    )
                    .unwrap(),
                ),
                "sampler",
            ),
            (
                GpuResourceDescriptor::QuerySet(
                    GpuQuerySetDescriptor::new(
                        gpu_common("runtime test query set"),
                        GpuQueryKind::Timestamp,
                        2,
                    )
                    .unwrap(),
                ),
                "query-set",
            ),
        ];

        for (normalized, expected_kind) in unsupported {
            let error = FlowRuntimeResources::current_runtime_resource_disposition_from_lowering(
                &declaration,
                RenderGpuResourceLowering::Normalized(Box::new(normalized)),
                GpuTextureFormat::Rgba8Unorm,
            )
            .unwrap_err();
            assert!(matches!(
                error,
                CurrentRuntimeResourceRealizationError::UnsupportedNormalizedKind { kind, .. }
                    if kind == expected_kind
            ));
            assert!(error.to_string().contains("current render runtime"));
        }
    }

    #[test]
    fn concurrently_live_invocation_uniforms_have_distinct_runengpu_handles() {
        let mut declaration_ids = GpuWorkResourceIdAllocator::new();
        let declaration = RenderResourceDeclaration::declare_uniform::<RuntimeTestUniform>(
            &mut declaration_ids,
            "invocation uniform",
        )
        .expect("test invocation uniform should be valid");
        let resource_id = *declaration.id();
        let mut resources = FlowRuntimeResources::default();
        resources.descriptors.insert(resource_id, declaration);

        let first = resources
            .realize_invocation_uniform_buffer("viewport.a", resource_id, 4)
            .expect("first invocation uniform should realize");
        let first_handle_identity = first.handle.diagnostic_identity();

        let second = resources
            .realize_invocation_uniform_buffer("viewport.b", resource_id, 4)
            .expect("second invocation uniform should realize");
        let second_handle_identity = second.handle.diagnostic_identity();

        assert_ne!(first_handle_identity, second_handle_identity);
        assert_eq!(resources.invocation_uniform_buffers.len(), 2);
        assert!(
            resources
                .invocation_uniform_buffers
                .contains_key(&("viewport.a".to_string(), resource_id))
        );
        assert!(
            resources
                .invocation_uniform_buffers
                .contains_key(&("viewport.b".to_string(), resource_id))
        );
    }
}
