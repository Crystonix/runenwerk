use crate::plugins::render::inspect::RenderCapturePointIdentity;
use crate::plugins::render::pipelines::FlowPassKind;
use crate::plugins::render::{
    RenderFragmentMergeReport, RenderFragmentProvenanceElementKind, RenderFragmentProvenanceRecord,
};
use runen_gpu::{GpuPrimitiveTopology, GpuTextureFormat};

#[derive(Debug, Clone, Default, ecs::Component, ecs::Resource)]
pub struct RenderPassProvenanceState {
    pub frame_index: u64,
    pub records: Vec<RenderPassProvenanceRecord>,
}

impl RenderPassProvenanceState {
    pub fn observe_frame(&mut self, frame_index: u64, records: &[RenderPassProvenanceRecord]) {
        self.frame_index = frame_index;
        self.records.clear();
        self.records.extend_from_slice(records);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPassProvenanceRecord {
    pub frame_index: u64,
    pub flow_id: String,
    pub pass_id: String,
    pub pass_label: String,
    pub pass_kind: FlowPassKind,
    /// Lexical source position retained as evidence, not execution order.
    pub authoring_index: usize,
    pub feature_id: Option<String>,
    pub shader_id: String,
    pub shader_revision: u64,
    pub fallback_used: bool,
    pub pipeline_stats_key: String,
    pub bind_group_layout_signature_hash: u64,
    pub material_specialization_fragment_hash: u64,
    pub view_signature_hash: u64,
    pub feature_runtime_version: u64,
    pub color_formats: Vec<GpuTextureFormat>,
    pub depth_format: Option<GpuTextureFormat>,
    pub sample_count: u32,
    pub primitive_topology: Option<GpuPrimitiveTopology>,
    pub material_binding: RenderPassMaterialBindingEvidence,
    pub render_targets: Vec<String>,
    pub sampled_textures: Vec<String>,
    pub storage_textures: Vec<String>,
    pub depth_targets: Vec<String>,
    pub capture_points_available: Vec<RenderCapturePointIdentity>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenderPassMaterialBindingEvidence {
    pub consumes_material_resources: bool,
    pub prepared_material_available: bool,
    pub material_table_identity: Option<String>,
    pub scene_shader_identity: Option<String>,
    pub scene_shader_path: Option<String>,
    pub material_instance_count: usize,
    pub material_binding_slot_count: usize,
    pub prepared_model_mesh_material_selection_count: usize,
    pub model_mesh_material_selections_available_to_pass:
        Vec<RenderPassModelMeshMaterialSelectionEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPassModelMeshMaterialSelectionEvidence {
    pub source_asset_id: u64,
    pub source_id: u64,
    pub source_revision_id: Option<u64>,
    pub source_revision: Option<String>,
    pub region_key: String,
    pub requested_material_slot_id: u64,
    pub resolved_material_slot_id: u64,
    pub material_table_index: u32,
    pub used_default_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFragmentPassProvenanceRecord {
    pub package_id: String,
    pub fragment_id: String,
    pub namespace: String,
    pub source_path: String,
    pub source_revision: u64,
    pub source_label: String,
    pub generated_label: String,
}

pub fn inspect_fragment_pass_provenance(
    report: &RenderFragmentMergeReport,
) -> Vec<RenderFragmentPassProvenanceRecord> {
    report
        .provenance
        .iter()
        .filter(|record| record.element_kind == RenderFragmentProvenanceElementKind::Pass)
        .map(fragment_pass_provenance_record)
        .collect()
}

fn fragment_pass_provenance_record(
    record: &RenderFragmentProvenanceRecord,
) -> RenderFragmentPassProvenanceRecord {
    RenderFragmentPassProvenanceRecord {
        package_id: record.package_id.to_string(),
        fragment_id: record.fragment_id.to_string(),
        namespace: record.namespace.to_string(),
        source_path: record.source_path.clone(),
        source_revision: record.source_revision.0,
        source_label: record.source_label.clone(),
        generated_label: record.generated_label.clone(),
    }
}
