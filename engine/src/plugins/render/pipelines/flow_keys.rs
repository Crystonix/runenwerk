use crate::plugins::render::{RenderFeatureId, RenderFlowId, RenderPassId, RenderPassKind};
use runen_gpu::{
    GpuComputePipelineDescriptor, GpuPipelineLayoutDescriptor, GpuProgramDescriptor,
    GpuRenderPipelineDescriptor, GpuRenderPipelineStateDescriptor, GpuSpecializationValueSet,
};
use std::hash::{Hash, Hasher};

/// Renderer-local discrimination over the complete generic G4B pipeline contracts.
///
/// This adds no renderer semantics to RunenGPU. It only lets renderer-local
/// cache partitioning retain one complete generic pipeline descriptor rather
/// than mirroring its source/layout/specialization/state fields.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FlowPassPipelineDescriptor {
    Compute(GpuComputePipelineDescriptor),
    Render(GpuRenderPipelineDescriptor),
}

impl FlowPassPipelineDescriptor {
    pub fn program(&self) -> &GpuProgramDescriptor {
        match self {
            Self::Compute(descriptor) => descriptor.program(),
            Self::Render(descriptor) => descriptor.program(),
        }
    }

    pub fn layout(&self) -> &GpuPipelineLayoutDescriptor {
        match self {
            Self::Compute(descriptor) => descriptor.layout(),
            Self::Render(descriptor) => descriptor.layout(),
        }
    }

    pub fn specialization(&self) -> &GpuSpecializationValueSet {
        match self {
            Self::Compute(descriptor) => descriptor.specialization(),
            Self::Render(descriptor) => descriptor.specialization(),
        }
    }

    pub fn render_state(&self) -> Option<&GpuRenderPipelineStateDescriptor> {
        match self {
            Self::Compute(_) => None,
            Self::Render(descriptor) => Some(descriptor.state()),
        }
    }

    pub fn diagnostic_label(&self) -> String {
        match self {
            Self::Compute(descriptor) => {
                format!("compute:{:016x}", diagnostic_hash(descriptor))
            }
            Self::Render(descriptor) => format!("render:{:016x}", diagnostic_hash(descriptor)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlowPassPipelineKey {
    pub flow_id: RenderFlowId,
    pub pass_id: RenderPassId,
    pub pass_kind: FlowPassKind,
    pub feature_id: Option<RenderFeatureId>,
    pub pipeline_descriptor: FlowPassPipelineDescriptor,
}

impl FlowPassPipelineKey {
    pub fn stats_key(&self) -> String {
        format!(
            "flow:{}:{}:{:?}:{}",
            self.flow_id,
            self.pass_id,
            self.pass_kind,
            self.pipeline_descriptor.diagnostic_label(),
        )
    }

    pub fn pipeline_descriptor_diagnostic_hash(&self) -> u64 {
        diagnostic_hash(&self.pipeline_descriptor)
    }

    pub fn pipeline_layout_diagnostic_hash(&self) -> u64 {
        diagnostic_hash(self.pipeline_descriptor.layout())
    }

    pub fn primary_bind_group_layout_diagnostic_hash(&self) -> u64 {
        diagnostic_hash(&self.pipeline_descriptor.layout().group(0))
    }

    pub fn render_pipeline_state_diagnostic_hash(&self) -> u64 {
        diagnostic_hash(&self.pipeline_descriptor.render_state())
    }

    pub fn render_pipeline_state(&self) -> Option<&GpuRenderPipelineStateDescriptor> {
        self.pipeline_descriptor.render_state()
    }
}

fn diagnostic_hash(value: &impl Hash) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlowPassBindGroupKey {
    pub pipeline: FlowPassPipelineKey,
    pub resource_generation_signature_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlowPassKind {
    Compute,
    Fullscreen,
    Graphics,
    Copy,
    Present,
    BuiltinUiComposite,
}

impl From<RenderPassKind> for FlowPassKind {
    fn from(value: RenderPassKind) -> Self {
        match value {
            RenderPassKind::Compute => Self::Compute,
            RenderPassKind::Fullscreen => Self::Fullscreen,
            RenderPassKind::Graphics => Self::Graphics,
            RenderPassKind::Copy => Self::Copy,
            RenderPassKind::Present => Self::Present,
            RenderPassKind::BuiltinUiComposite => Self::BuiltinUiComposite,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runen_gpu::{
        GpuAdmittedProgramSource, GpuBindingLayoutRefinement, GpuBlendMode,
        GpuCapabilityRequirements, GpuColorTargetStateDescriptor, GpuColorWriteMask,
        GpuEntryPointName, GpuFragmentOutputStateDescriptor, GpuMultisampleStateDescriptor,
        GpuPipelineConfiguration, GpuPrimitiveStateDescriptor, GpuProgramSourceIdentity,
        GpuProgramSourceKey, GpuProgramSourceOwnerId, GpuProgramSourceProvenance,
        GpuProgramSourceRegistry, GpuProgramSourceRevision, GpuRenderEntryPoints,
        GpuSpecializationDeclaration, GpuSpecializationEntry, GpuSpecializationKey,
        GpuSpecializationSchema, GpuSpecializationValue, GpuTextureFormat, GpuVertexAttribute,
        GpuVertexBufferLayoutDescriptor, GpuVertexFormat, GpuVertexInputStateDescriptor,
        GpuVertexStepMode,
    };

    const BASE_RENDER_WGSL: &str = r#"
@vertex
fn vs_main() -> @builtin(position) vec4f {
    return vec4f(0.0, 0.0, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return vec4f(1.0);
}
"#;

    const GROUP0_RENDER_WGSL: &str = r#"
struct Params {
    value: f32,
}

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main() -> @builtin(position) vec4f {
    return vec4f(params.value, 0.0, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return vec4f(1.0);
}
"#;

    const GROUP1_RENDER_WGSL: &str = r#"
struct Params {
    value: f32,
}

@group(1) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main() -> @builtin(position) vec4f {
    return vec4f(params.value, 0.0, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return vec4f(1.0);
}
"#;

    const VERTEX_INPUT_RENDER_WGSL: &str = r#"
struct VertexIn {
    @location(0) position: vec3f,
}

@vertex
fn vs_main(input: VertexIn) -> @builtin(position) vec4f {
    return vec4f(input.position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return vec4f(1.0);
}
"#;

    const COMPUTE_WGSL: &str = "@compute @workgroup_size(1) fn cs_main() {}";

    fn admitted_source(key: &str, canonical_wgsl: &str) -> GpuAdmittedProgramSource {
        let identity = GpuProgramSourceIdentity::new(
            GpuProgramSourceOwnerId::allocate().unwrap(),
            GpuProgramSourceKey::new(key).unwrap(),
            GpuProgramSourceRevision::try_from_raw(1).unwrap(),
        );
        let mut registry = GpuProgramSourceRegistry::new(4, 4096).unwrap();
        registry
            .admit_wgsl(
                identity,
                canonical_wgsl,
                GpuProgramSourceProvenance::new("flow-key-test", None).unwrap(),
            )
            .unwrap()
    }

    fn specialization(name: &str, value: GpuSpecializationValue) -> GpuSpecializationValueSet {
        let key = GpuSpecializationKey::new(name).unwrap();
        let schema = GpuSpecializationSchema::new([GpuSpecializationDeclaration::new(
            key.clone(),
            value.value_type(),
            None,
            GpuCapabilityRequirements::new(),
        )
        .unwrap()])
        .unwrap();
        GpuSpecializationValueSet::new(schema, [GpuSpecializationEntry::new(key, value)]).unwrap()
    }

    fn vertex_input_state() -> GpuVertexInputStateDescriptor {
        GpuVertexInputStateDescriptor::new([GpuVertexBufferLayoutDescriptor::new(
            0,
            12,
            GpuVertexStepMode::Vertex,
            [GpuVertexAttribute::new(0, 0, GpuVertexFormat::Float32x3)],
        )
        .unwrap()])
        .unwrap()
    }

    fn render_pipeline_state(
        vertex_input: GpuVertexInputStateDescriptor,
    ) -> GpuRenderPipelineStateDescriptor {
        let color_target = GpuColorTargetStateDescriptor::new(
            GpuTextureFormat::Rgba8Unorm,
            GpuBlendMode::Alpha,
            GpuColorWriteMask::ALL,
        )
        .unwrap();
        GpuRenderPipelineStateDescriptor::new(
            vertex_input,
            Some(GpuFragmentOutputStateDescriptor::new([color_target])),
            GpuPrimitiveStateDescriptor::default(),
            None,
            GpuMultisampleStateDescriptor::default(),
        )
        .unwrap()
    }

    fn render_key(
        source: GpuAdmittedProgramSource,
        state: GpuRenderPipelineStateDescriptor,
    ) -> FlowPassPipelineKey {
        let vertex = GpuEntryPointName::new("vs_main").unwrap();
        let fragment = GpuEntryPointName::new("fs_main").unwrap();
        let program = GpuProgramDescriptor::new(
            source,
            [vertex.clone(), fragment.clone()],
            std::iter::empty::<GpuBindingLayoutRefinement>(),
        )
        .unwrap();
        let descriptor = GpuRenderPipelineDescriptor::new(
            program,
            GpuRenderEntryPoints::new(vertex, Some(fragment)),
            state,
            GpuPipelineConfiguration::default(),
        )
        .unwrap();
        FlowPassPipelineKey {
            flow_id: RenderFlowId::try_from_raw(1).unwrap(),
            pass_id: RenderPassId::try_from_raw(1).unwrap(),
            pass_kind: FlowPassKind::Fullscreen,
            feature_id: None,
            pipeline_descriptor: FlowPassPipelineDescriptor::Render(descriptor),
        }
    }

    fn compute_key(
        source: GpuAdmittedProgramSource,
        specialization: Option<GpuSpecializationValueSet>,
    ) -> FlowPassPipelineKey {
        let entry_point = GpuEntryPointName::new("cs_main").unwrap();
        let program = GpuProgramDescriptor::new(
            source,
            [entry_point.clone()],
            std::iter::empty::<GpuBindingLayoutRefinement>(),
        )
        .unwrap();
        let descriptor = GpuComputePipelineDescriptor::new(
            program,
            entry_point,
            GpuPipelineConfiguration::new(specialization, None),
        )
        .unwrap();
        FlowPassPipelineKey {
            flow_id: RenderFlowId::try_from_raw(2).unwrap(),
            pass_id: RenderPassId::try_from_raw(2).unwrap(),
            pass_kind: FlowPassKind::Compute,
            feature_id: None,
            pipeline_descriptor: FlowPassPipelineDescriptor::Compute(descriptor),
        }
    }

    #[test]
    fn stats_key_reflects_admitted_program_interface_and_render_state() {
        let base_state = render_pipeline_state(GpuVertexInputStateDescriptor::new([]).unwrap());
        let key = render_key(
            admitted_source("shader", BASE_RENDER_WGSL),
            base_state.clone(),
        );
        let same = key.clone();
        let changed_source = render_key(
            admitted_source("other-shader", BASE_RENDER_WGSL),
            base_state.clone(),
        );
        let changed_group0_interface = render_key(
            admitted_source("shader-group0", GROUP0_RENDER_WGSL),
            base_state.clone(),
        );
        let changed_group1_interface = render_key(
            admitted_source("shader-group1", GROUP1_RENDER_WGSL),
            base_state,
        );
        let changed_render_state = render_key(
            admitted_source("shader-vertex-input", VERTEX_INPUT_RENDER_WGSL),
            render_pipeline_state(vertex_input_state()),
        );

        assert_eq!(key.stats_key(), same.stats_key());
        assert_ne!(key.stats_key(), changed_source.stats_key());
        assert_ne!(key.stats_key(), changed_group0_interface.stats_key());
        assert_ne!(key.stats_key(), changed_group1_interface.stats_key());
        assert_ne!(key.stats_key(), changed_render_state.stats_key());
        assert_ne!(
            key.pipeline_layout_diagnostic_hash(),
            changed_group0_interface.pipeline_layout_diagnostic_hash()
        );
        assert_ne!(
            changed_group0_interface.pipeline_layout_diagnostic_hash(),
            changed_group1_interface.pipeline_layout_diagnostic_hash()
        );
        assert_ne!(
            key.render_pipeline_state_diagnostic_hash(),
            changed_render_state.render_pipeline_state_diagnostic_hash()
        );
    }

    #[test]
    fn compute_descriptor_preserves_typed_specialization_identity() {
        let source = admitted_source("compute-shader", COMPUTE_WGSL);
        let default = compute_key(source.clone(), None);
        let typed = compute_key(
            source.clone(),
            Some(specialization("COUNT", GpuSpecializationValue::U32(4))),
        );
        let signed = compute_key(
            source,
            Some(specialization("COUNT", GpuSpecializationValue::I32(4))),
        );

        assert_ne!(default, typed);
        assert_ne!(typed, signed);
        assert_ne!(default.stats_key(), typed.stats_key());
        assert_ne!(typed.stats_key(), signed.stats_key());
    }
}
