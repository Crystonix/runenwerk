use crate::plugins::render::{
    RenderDrawDescriptor, RenderPassKind, RenderPassViewScope, RenderTargetAliasKind,
    RenderTextureTargetFormat,
};
use runen_gpu::GpuBindingKey;
use std::fmt;

pub const SUPPORTED_RENDER_FRAGMENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderFragmentId(String);

impl RenderFragmentId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for RenderFragmentId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RenderFragmentId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for RenderFragmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderFragmentPackageId(String);

impl RenderFragmentPackageId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for RenderFragmentPackageId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RenderFragmentPackageId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for RenderFragmentPackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderFragmentNamespace(String);

impl RenderFragmentNamespace {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn qualify(&self, local_label: &str) -> String {
        format!("{}::{}", self.0, local_label)
    }
}

impl From<&str> for RenderFragmentNamespace {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RenderFragmentNamespace {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for RenderFragmentNamespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderFragmentLabelRef {
    Local(String),
    Absolute(String),
}

impl RenderFragmentLabelRef {
    pub fn local(value: impl Into<String>) -> Self {
        Self::Local(value.into())
    }

    pub fn absolute(value: impl Into<String>) -> Self {
        Self::Absolute(value.into())
    }

    pub fn raw_label(&self) -> &str {
        match self {
            Self::Local(value) | Self::Absolute(value) => value.as_str(),
        }
    }

    pub fn resolve(&self, namespace: &RenderFragmentNamespace) -> String {
        match self {
            Self::Local(value) => namespace.qualify(value),
            Self::Absolute(value) => value.clone(),
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderFragmentRevision(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderFragmentDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderFragmentDiagnosticKind {
    EmptyPackageId,
    EmptyFragmentId,
    EmptyNamespace,
    EmptyLabel,
    UnsupportedSchemaVersion,
    FragmentNamespaceMismatch,
    DuplicateFragmentId,
    DuplicateResourceLabel,
    DuplicatePassLabel,
    MissingResourceReference,
    MissingPassReference,
    InvalidPassShape,
    NamespaceConflict,
    CompileValidationFailed,
    BackendCapabilityMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFragmentDiagnostic {
    pub severity: RenderFragmentDiagnosticSeverity,
    pub kind: RenderFragmentDiagnosticKind,
    pub package_id: Option<RenderFragmentPackageId>,
    pub fragment_id: Option<RenderFragmentId>,
    pub namespace: Option<RenderFragmentNamespace>,
    pub source_path: Option<String>,
    pub revision: Option<RenderFragmentRevision>,
    pub label: Option<String>,
    pub message: String,
}

impl RenderFragmentDiagnostic {
    pub fn new(
        severity: RenderFragmentDiagnosticSeverity,
        kind: RenderFragmentDiagnosticKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            kind,
            package_id: None,
            fragment_id: None,
            namespace: None,
            source_path: None,
            revision: None,
            label: None,
            message: message.into(),
        }
    }

    pub fn error(kind: RenderFragmentDiagnosticKind, message: impl Into<String>) -> Self {
        Self::new(RenderFragmentDiagnosticSeverity::Error, kind, message)
    }

    pub fn with_package(mut self, package: &RenderFragmentPackageDescriptor) -> Self {
        self.package_id = Some(package.package_id.clone());
        self.namespace = Some(package.namespace.clone());
        self.source_path = Some(package.source_path.clone());
        self.revision = Some(RenderFragmentRevision(package.source_revision));
        self
    }

    pub fn with_fragment(mut self, fragment: &RenderFragmentDescriptor) -> Self {
        self.fragment_id = Some(fragment.id.clone());
        self.namespace = Some(fragment.namespace.clone());
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == RenderFragmentDiagnosticSeverity::Error
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderFragmentDiagnosticReport {
    pub diagnostics: Vec<RenderFragmentDiagnostic>,
}

impl RenderFragmentDiagnosticReport {
    pub fn new(diagnostics: Vec<RenderFragmentDiagnostic>) -> Self {
        Self { diagnostics }
    }

    pub fn is_ok(&self) -> bool {
        !self.has_errors()
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(RenderFragmentDiagnostic::is_error)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderFragmentPackageDescriptor {
    pub package_id: RenderFragmentPackageId,
    pub schema_version: u32,
    pub source_path: String,
    pub namespace: RenderFragmentNamespace,
    pub source_revision: u64,
    pub fragments: Vec<RenderFragmentDescriptor>,
}

impl RenderFragmentPackageDescriptor {
    pub fn new(
        package_id: impl Into<RenderFragmentPackageId>,
        namespace: impl Into<RenderFragmentNamespace>,
        source_path: impl Into<String>,
        source_revision: u64,
    ) -> Self {
        Self {
            package_id: package_id.into(),
            schema_version: SUPPORTED_RENDER_FRAGMENT_SCHEMA_VERSION,
            source_path: source_path.into(),
            namespace: namespace.into(),
            source_revision,
            fragments: Vec::new(),
        }
    }

    pub fn with_fragment(mut self, fragment: RenderFragmentDescriptor) -> Self {
        self.fragments.push(fragment);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderFragmentDescriptor {
    pub id: RenderFragmentId,
    pub namespace: RenderFragmentNamespace,
    pub resources: Vec<RenderFragmentResourceDescriptor>,
    pub passes: Vec<RenderFragmentPassDescriptor>,
    pub required_capabilities: Vec<String>,
}

impl RenderFragmentDescriptor {
    pub fn new(
        id: impl Into<RenderFragmentId>,
        namespace: impl Into<RenderFragmentNamespace>,
    ) -> Self {
        Self {
            id: id.into(),
            namespace: namespace.into(),
            resources: Vec::new(),
            passes: Vec::new(),
            required_capabilities: Vec::new(),
        }
    }

    pub fn with_resource(mut self, resource: RenderFragmentResourceDescriptor) -> Self {
        self.resources.push(resource);
        self
    }

    pub fn with_pass(mut self, pass: RenderFragmentPassDescriptor) -> Self {
        self.passes.push(pass);
        self
    }

    pub fn require_capability(mut self, capability: impl Into<String>) -> Self {
        self.required_capabilities.push(capability.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderFragmentResourceKind {
    SurfaceColor,
    SurfaceDepth,
    ColorTarget,
    ColorTargetExact(RenderTextureTargetFormat),
    DepthTarget,
    SampledTexture,
    StorageTexture,
    HistoryTexture,
    TargetAlias(RenderTargetAliasKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFragmentResourceDescriptor {
    pub label: String,
    pub kind: RenderFragmentResourceKind,
}

impl RenderFragmentResourceDescriptor {
    pub fn new(label: impl Into<String>, kind: RenderFragmentResourceKind) -> Self {
        Self {
            label: label.into(),
            kind,
        }
    }

    pub fn surface_color() -> Self {
        Self::new("surface.color", RenderFragmentResourceKind::SurfaceColor)
    }

    pub fn surface_depth() -> Self {
        Self::new("surface.depth", RenderFragmentResourceKind::SurfaceDepth)
    }

    pub fn color_target(label: impl Into<String>) -> Self {
        Self::new(label, RenderFragmentResourceKind::ColorTarget)
    }

    pub fn color_target_exact(label: impl Into<String>, format: RenderTextureTargetFormat) -> Self {
        Self::new(label, RenderFragmentResourceKind::ColorTargetExact(format))
    }

    pub fn depth_target(label: impl Into<String>) -> Self {
        Self::new(label, RenderFragmentResourceKind::DepthTarget)
    }

    pub fn sampled_texture(label: impl Into<String>) -> Self {
        Self::new(label, RenderFragmentResourceKind::SampledTexture)
    }

    pub fn storage_texture(label: impl Into<String>) -> Self {
        Self::new(label, RenderFragmentResourceKind::StorageTexture)
    }

    pub fn history_texture(label: impl Into<String>) -> Self {
        Self::new(label, RenderFragmentResourceKind::HistoryTexture)
    }

    pub fn target_alias(label: impl Into<String>, kind: RenderTargetAliasKind) -> Self {
        Self::new(label, RenderFragmentResourceKind::TargetAlias(kind))
    }

    pub fn generated_label(&self, namespace: &RenderFragmentNamespace) -> String {
        match self.kind {
            RenderFragmentResourceKind::SurfaceColor => "surface.color".to_string(),
            RenderFragmentResourceKind::SurfaceDepth => "surface.depth".to_string(),
            _ => namespace.qualify(self.label.as_str()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderFragmentPassKind {
    Compute,
    Fullscreen,
    BuiltinUiComposite,
    Graphics,
    Copy,
    Present,
}

impl From<RenderFragmentPassKind> for RenderPassKind {
    fn from(value: RenderFragmentPassKind) -> Self {
        match value {
            RenderFragmentPassKind::Compute => Self::Compute,
            RenderFragmentPassKind::Fullscreen => Self::Fullscreen,
            RenderFragmentPassKind::BuiltinUiComposite => Self::BuiltinUiComposite,
            RenderFragmentPassKind::Graphics => Self::Graphics,
            RenderFragmentPassKind::Copy => Self::Copy,
            RenderFragmentPassKind::Present => Self::Present,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderFragmentShaderReference {
    AssetPath(String),
    MaterialSceneBundle { fallback_asset: String },
}

impl RenderFragmentShaderReference {
    pub fn asset(path: impl Into<String>) -> Self {
        Self::AssetPath(path.into())
    }

    pub fn material_scene_bundle(fallback_asset: impl Into<String>) -> Self {
        Self::MaterialSceneBundle {
            fallback_asset: fallback_asset.into(),
        }
    }
}

/// Explicit shader-slot ownership for fragment-authored texture resources.
///
/// Resource-role vectors on the pass remain composition/access semantics only;
/// `GpuBindingKey` values here are the exclusive shader binding identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderFragmentShaderBinding {
    SampledTexture {
        texture_binding: GpuBindingKey,
        sampler_binding: GpuBindingKey,
        resource: RenderFragmentLabelRef,
    },
    StorageTexture {
        binding: GpuBindingKey,
        resource: RenderFragmentLabelRef,
    },
}

impl RenderFragmentShaderBinding {
    pub fn resource(&self) -> &RenderFragmentLabelRef {
        match self {
            Self::SampledTexture { resource, .. } | Self::StorageTexture { resource, .. } => {
                resource
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderFragmentPassDescriptor {
    pub label: String,
    pub kind: RenderFragmentPassKind,
    pub view_scope: RenderPassViewScope,
    pub shader: Option<RenderFragmentShaderReference>,
    pub shader_bindings: Vec<RenderFragmentShaderBinding>,
    /// Resource-role truth for fragment validation/composition. Shader slot
    /// identity lives exclusively in `shader_bindings`.
    pub sample_textures: Vec<RenderFragmentLabelRef>,
    /// Resource-role truth for fragment validation/composition. Shader slot
    /// identity lives exclusively in `shader_bindings`.
    pub write_textures: Vec<RenderFragmentLabelRef>,
    pub color_outputs: Vec<RenderFragmentLabelRef>,
    pub write_surface_color: bool,
    pub depth_target: Option<RenderFragmentLabelRef>,
    /// Genuine render-owned sequencing that is not implied by resource access.
    /// Fragment merge lowers this immediately into G3 explicit non-data order.
    pub non_data_order_after: Vec<RenderFragmentLabelRef>,
    pub clear_color: Option<[f32; 4]>,
    pub compute_dispatch: Option<[u32; 3]>,
    pub copy_source: Option<RenderFragmentLabelRef>,
    pub copy_destination: Option<RenderFragmentLabelRef>,
    pub present_source: Option<RenderFragmentLabelRef>,
    pub draw: Option<RenderDrawDescriptor>,
}

impl RenderFragmentPassDescriptor {
    pub fn new(label: impl Into<String>, kind: RenderFragmentPassKind) -> Self {
        Self {
            label: label.into(),
            kind,
            view_scope: RenderPassViewScope::AllViews,
            shader: None,
            shader_bindings: Vec::new(),
            sample_textures: Vec::new(),
            write_textures: Vec::new(),
            color_outputs: Vec::new(),
            write_surface_color: false,
            depth_target: None,
            non_data_order_after: Vec::new(),
            clear_color: None,
            compute_dispatch: None,
            copy_source: None,
            copy_destination: None,
            present_source: None,
            draw: None,
        }
    }

    pub fn compute(label: impl Into<String>) -> Self {
        Self::new(label, RenderFragmentPassKind::Compute)
    }

    pub fn fullscreen(label: impl Into<String>) -> Self {
        Self::new(label, RenderFragmentPassKind::Fullscreen)
    }

    pub fn graphics(label: impl Into<String>) -> Self {
        Self::new(label, RenderFragmentPassKind::Graphics)
    }

    pub fn copy(label: impl Into<String>) -> Self {
        Self::new(label, RenderFragmentPassKind::Copy)
    }

    pub fn present(label: impl Into<String>) -> Self {
        Self::new(label, RenderFragmentPassKind::Present)
    }

    pub fn builtin_ui_composite(label: impl Into<String>) -> Self {
        Self::new(label, RenderFragmentPassKind::BuiltinUiComposite)
    }

    pub fn shader_asset(mut self, path: impl Into<String>) -> Self {
        self.shader = Some(RenderFragmentShaderReference::asset(path));
        self
    }

    pub fn material_scene_shader_asset(mut self, fallback_asset: impl Into<String>) -> Self {
        self.shader = Some(RenderFragmentShaderReference::material_scene_bundle(
            fallback_asset,
        ));
        self
    }

    pub fn main_surface_only(mut self) -> Self {
        self.view_scope = RenderPassViewScope::MainSurfaceOnly;
        self
    }

    pub fn offscreen_products_only(mut self) -> Self {
        self.view_scope = RenderPassViewScope::OffscreenProductsOnly;
        self
    }

    pub fn sample_local_texture(
        mut self,
        texture_binding: GpuBindingKey,
        sampler_binding: GpuBindingKey,
        label: impl Into<String>,
    ) -> Self {
        let resource = RenderFragmentLabelRef::local(label);
        self.sample_textures.push(resource.clone());
        self.shader_bindings
            .push(RenderFragmentShaderBinding::SampledTexture {
                texture_binding,
                sampler_binding,
                resource,
            });
        self
    }

    pub fn write_local_texture(mut self, binding: GpuBindingKey, label: impl Into<String>) -> Self {
        let resource = RenderFragmentLabelRef::local(label);
        self.write_textures.push(resource.clone());
        self.shader_bindings
            .push(RenderFragmentShaderBinding::StorageTexture { binding, resource });
        self
    }

    pub fn write_local_color_target(mut self, label: impl Into<String>) -> Self {
        self.color_outputs
            .push(RenderFragmentLabelRef::local(label));
        self
    }

    pub fn write_absolute_color_target(mut self, label: impl Into<String>) -> Self {
        self.color_outputs
            .push(RenderFragmentLabelRef::absolute(label));
        self
    }

    pub fn write_surface_color(mut self) -> Self {
        self.write_surface_color = true;
        self
    }

    pub fn depth_target(mut self, label: RenderFragmentLabelRef) -> Self {
        self.depth_target = Some(label);
        self
    }

    pub fn order_after_local(mut self, label: impl Into<String>) -> Self {
        self.non_data_order_after
            .push(RenderFragmentLabelRef::local(label));
        self
    }

    pub fn order_after_absolute(mut self, label: impl Into<String>) -> Self {
        self.non_data_order_after
            .push(RenderFragmentLabelRef::absolute(label));
        self
    }

    pub fn clear_color(mut self, color: [f32; 4]) -> Self {
        self.clear_color = Some(color);
        self
    }

    pub fn dispatch(mut self, dispatch: [u32; 3]) -> Self {
        self.compute_dispatch = Some(dispatch);
        self
    }

    pub fn copy_local(mut self, source: impl Into<String>, destination: impl Into<String>) -> Self {
        self.copy_source = Some(RenderFragmentLabelRef::local(source));
        self.copy_destination = Some(RenderFragmentLabelRef::local(destination));
        self
    }

    pub fn present_source(mut self, source: RenderFragmentLabelRef) -> Self {
        self.present_source = Some(source);
        self
    }

    pub fn draw(mut self, vertex_count: u32, instance_count: u32) -> Self {
        self.draw = Some(RenderDrawDescriptor::new(vertex_count, instance_count));
        self
    }

    pub fn generated_label(&self, namespace: &RenderFragmentNamespace) -> String {
        namespace.qualify(self.label.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderFragmentProvenanceElementKind {
    Resource,
    Pass,
    NonDataOrder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFragmentProvenanceRecord {
    pub package_id: RenderFragmentPackageId,
    pub fragment_id: RenderFragmentId,
    pub namespace: RenderFragmentNamespace,
    pub source_path: String,
    pub source_revision: RenderFragmentRevision,
    pub element_kind: RenderFragmentProvenanceElementKind,
    pub source_label: String,
    pub generated_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderFragmentMergeReport {
    pub package_id: Option<RenderFragmentPackageId>,
    pub source_path: Option<String>,
    pub source_revision: Option<RenderFragmentRevision>,
    pub generated_flow_id: Option<String>,
    pub provenance: Vec<RenderFragmentProvenanceRecord>,
    pub diagnostics: Vec<RenderFragmentDiagnostic>,
}

impl RenderFragmentMergeReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(RenderFragmentDiagnostic::is_error)
    }
}
