//! File: apps/runenwerk_editor/src/runtime/viewport/product_targets.rs
//! Purpose: App-owned viewport product target registry and dynamic target requests.

use std::collections::{BTreeMap, BTreeSet};

use editor_core::RealityVersion;
use editor_shell::viewport_embed_slot_for;
use editor_viewport::{
    ArtifactObservationFrame, ExpressionDimensions, ExpressionFormat, ExpressionProductDescriptor,
    ExpressionProductId, ExpressionProductKind, ProducerHealth, ProductAvailabilityState,
    ViewportId, ViewportPresentationState, ViewportSurfacePresentationSlot,
};
use engine::plugins::render::{
    RenderDynamicTextureRetention, RenderDynamicTextureTargetDescriptor,
    RenderDynamicTextureTargetKey, RenderDynamicTextureTargetRequestRegistryResource,
    RenderProductSurfaceManifest, RenderProductSurfaceStatusKind, RenderTextureSampleMode,
    RenderTextureTargetFormat, RenderTextureTargetUsage, ViewportSurfaceBindingRegistryResource,
};
use engine::runtime::{Res, ResMut};
use ui_render_data::ViewportSurfaceBindingSource;

use crate::runtime::resources::EditorHostResource;
use crate::runtime::viewport::{
    EDITOR_VIEWPORT_RENDER_PRODUCT_PRODUCER_ID, MAIN_VIEWPORT_ID,
    ToolSurfaceRuntimeBindingRegistryResource, ViewportArtifactObservationResource,
    ViewportInstanceRegistryResource, ViewportPresentationStateResource,
    ViewportProductRegistryResource, ViewportRenderStateResource, ViewportSurfaceHandle,
    ViewportSurfaceSetResource, ViewportSurfaceSlot, build_surface_binding_registry,
    ensure_editor_main_surface_set, expression_dimensions_for_bounds, initial_presentation_state,
    initial_product_descriptors,
};

pub const VIEWPORT_DYNAMIC_TARGET_NAMESPACE: &str = "runenwerk.editor.viewport";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportProductTargetKey {
    pub viewport_id: ViewportId,
    pub presentation_slot: ViewportSurfacePresentationSlot,
    pub product_id: ExpressionProductId,
}

impl ViewportProductTargetKey {
    pub const fn new(
        viewport_id: ViewportId,
        presentation_slot: ViewportSurfacePresentationSlot,
        product_id: ExpressionProductId,
    ) -> Self {
        Self {
            viewport_id,
            presentation_slot,
            product_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportProductTargetStatus {
    Requested,
    Unavailable,
}

#[derive(Debug, Clone)]
pub struct ViewportProductTargetRecord {
    pub key: ViewportProductTargetKey,
    pub surface_slot: ViewportSurfaceSlot,
    pub namespace: String,
    pub target_id: String,
    pub width: u32,
    pub height: u32,
    pub format: ExpressionFormat,
    pub target_format: Option<RenderTextureTargetFormat>,
    pub usage: RenderTextureTargetUsage,
    pub sample_mode: RenderTextureSampleMode,
    pub ui_sampleable: bool,
    pub status: ViewportProductTargetStatus,
    pub generation: u64,
}

impl ViewportProductTargetRecord {
    pub fn dynamic_key(&self) -> RenderDynamicTextureTargetKey {
        RenderDynamicTextureTargetKey {
            namespace: self.namespace.clone(),
            target_id: self.target_id.clone(),
        }
    }

    pub fn dynamic_descriptor(&self) -> Option<RenderDynamicTextureTargetDescriptor> {
        if self.status != ViewportProductTargetStatus::Requested {
            return None;
        }
        let target_format = self.target_format?;
        Some(RenderDynamicTextureTargetDescriptor {
            key: self.dynamic_key(),
            width: self.width,
            height: self.height,
            format: target_format,
            usage: self.usage,
            sample_mode: self.sample_mode,
            retention: RenderDynamicTextureRetention::RetainWhileRequested,
        })
    }

    pub fn surface_handle(&self) -> Option<ViewportSurfaceHandle> {
        if self.status != ViewportProductTargetStatus::Requested {
            return None;
        }
        Some(ViewportSurfaceHandle::dynamic_texture(
            self.namespace.clone(),
            self.target_id.clone(),
            self.ui_sampleable,
        ))
    }
}

#[derive(Debug, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct ViewportProductTargetRegistryResource {
    records: BTreeMap<ViewportProductTargetKey, ViewportProductTargetRecord>,
    generation: u64,
}

impl ViewportProductTargetRegistryResource {
    pub fn from_descriptors_for_viewport(
        viewport_id: ViewportId,
        descriptors: &[ExpressionProductDescriptor],
    ) -> Self {
        let mut registry = Self::default();
        registry.replace_records(
            descriptors
                .iter()
                .filter_map(|descriptor| {
                    product_target_record_for_descriptor(viewport_id, descriptor)
                })
                .collect(),
        );
        registry
    }

    pub fn from_descriptors_for_viewports(
        viewport_ids: impl IntoIterator<Item = ViewportId>,
        descriptors: &[ExpressionProductDescriptor],
    ) -> Self {
        let viewport_ids = viewport_ids.into_iter().collect::<Vec<_>>();
        let mut registry = Self::default();
        registry.replace_records(
            descriptors
                .iter()
                .flat_map(|descriptor| {
                    viewport_ids.iter().copied().filter_map(move |viewport_id| {
                        product_target_record_for_descriptor(viewport_id, descriptor)
                    })
                })
                .collect(),
        );
        registry
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn record(&self, key: ViewportProductTargetKey) -> Option<&ViewportProductTargetRecord> {
        self.records.get(&key)
    }

    pub fn record_for_product(
        &self,
        viewport_id: ViewportId,
        presentation_slot: ViewportSurfacePresentationSlot,
        product_id: ExpressionProductId,
    ) -> Option<&ViewportProductTargetRecord> {
        self.record(ViewportProductTargetKey::new(
            viewport_id,
            presentation_slot,
            product_id,
        ))
    }

    pub fn records(&self) -> impl Iterator<Item = &ViewportProductTargetRecord> {
        self.records.values()
    }

    pub fn viewport_ids(&self) -> impl Iterator<Item = ViewportId> + '_ {
        self.records
            .keys()
            .map(|key| key.viewport_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
    }

    pub fn requested_dynamic_descriptors(&self) -> Vec<RenderDynamicTextureTargetDescriptor> {
        self.records
            .values()
            .filter_map(ViewportProductTargetRecord::dynamic_descriptor)
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn replace_records(&mut self, records: Vec<ViewportProductTargetRecord>) {
        let previous = std::mem::take(&mut self.records);
        self.generation = self.generation.saturating_add(1);
        let generation = self.generation;

        for mut record in records {
            if let Some(previous_record) = previous.get(&record.key)
                && target_signature_matches(previous_record, &record)
            {
                record.generation = previous_record.generation;
            } else {
                record.generation = generation;
            }
            self.records.insert(record.key, record);
        }
    }
}

pub fn sync_viewport_product_targets_system(
    viewport_products: Res<ViewportProductRegistryResource>,
    viewport_presentations: Res<ViewportPresentationStateResource>,
    mut viewport_product_targets: ResMut<ViewportProductTargetRegistryResource>,
    mut dynamic_target_requests: ResMut<RenderDynamicTextureTargetRequestRegistryResource>,
    mut viewport_surface_sets: ResMut<ViewportSurfaceSetResource>,
    mut viewport_surface_bindings: ResMut<ViewportSurfaceBindingRegistryResource>,
) {
    let mut records = Vec::new();
    for viewport_id in viewport_products.viewport_ids() {
        let Some(descriptors) = viewport_products.descriptors_for(viewport_id) else {
            continue;
        };
        records.extend(descriptors.iter().filter_map(|descriptor| {
            product_target_record_for_descriptor(viewport_id, descriptor)
        }));
    }

    viewport_product_targets.replace_records(records);
    let manifest =
        viewport_product_surface_manifest(&viewport_product_targets, &viewport_presentations);
    let (dynamic_target_descriptors, _, _, _) = manifest.into_render_parts();
    dynamic_target_requests
        .replace_contribution(
            EDITOR_VIEWPORT_RENDER_PRODUCT_PRODUCER_ID,
            dynamic_target_descriptors,
        )
        .expect("editor viewport dynamic target contribution must be valid and uniquely owned");
    sync_surface_sets_from_product_targets(
        &viewport_product_targets,
        &viewport_presentations,
        &mut viewport_surface_sets,
    );
    viewport_surface_bindings.replace_registry(build_surface_binding_registry(
        &viewport_surface_sets,
        &viewport_presentations,
        &viewport_product_targets,
    ));
}

#[allow(clippy::too_many_arguments)]
pub fn sync_viewport_presentation_products_system(
    host: Res<EditorHostResource>,
    viewport_instances: Res<ViewportInstanceRegistryResource>,
    viewport_render_states: Res<ViewportRenderStateResource>,
    mut viewport_surface_sets: ResMut<ViewportSurfaceSetResource>,
    tool_surface_bindings: Res<ToolSurfaceRuntimeBindingRegistryResource>,
    mut viewport_products_registry: ResMut<ViewportProductRegistryResource>,
    mut viewport_presentations: ResMut<ViewportPresentationStateResource>,
    mut viewport_observations: ResMut<ViewportArtifactObservationResource>,
) {
    let canonical_viewport_ids = canonical_viewport_ids_for_sync(
        &viewport_instances,
        &viewport_surface_sets,
        &tool_surface_bindings,
    );
    for viewport_id in &canonical_viewport_ids {
        ensure_editor_main_surface_set(&mut viewport_surface_sets, *viewport_id);
    }

    let source_version = host.app.runtime().current_scene_reality_version();
    for viewport_id in &canonical_viewport_ids {
        let product_dimensions =
            product_dimensions_for_viewport(*viewport_id, &viewport_render_states);
        let mut descriptors = initial_product_descriptors(product_dimensions, source_version);
        if let Some(material_preview) = host.app.material_lab_runtime().active_preview() {
            descriptors.push(super::product_registry::material_preview_descriptor_with_lineage(
                material_preview.viewport_product_id,
                product_dimensions,
                editor_core::RealityVersion(material_preview.artifact_id.raw()),
                material_preview.product.specialization_fragment.0.clone(),
                format!(
                    "material_artifact={}:shader_artifact={}:material_cache={}:shader_cache={}:scene_shader_cache={}",
                    material_preview.artifact_id.raw(),
                    material_preview.shader_artifact_id.raw(),
                    material_preview.artifact_cache_key.as_str(),
                    material_preview.shader_cache_key.as_str(),
                    material_preview.scene_shader_cache_key.as_str()
                ),
            ));
        }
        viewport_products_registry.update_viewport_descriptors(*viewport_id, descriptors.clone());

        let mut presentation_state = viewport_presentations
            .state_for(*viewport_id)
            .cloned()
            .unwrap_or_else(|| initial_presentation_state(*viewport_id));
        if !descriptors
            .iter()
            .any(|descriptor| descriptor.id == presentation_state.selected_primary_product_id)
        {
            presentation_state.select_primary_product(
                initial_presentation_state(*viewport_id).selected_primary_product_id,
            );
        }
        viewport_presentations.upsert_state(presentation_state.clone());
        viewport_observations.upsert_frame(build_artifact_observation_frame(
            &descriptors,
            &presentation_state,
            source_version,
        ));
    }

    let viewport_id_set = canonical_viewport_ids
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    viewport_products_registry.retain_viewports(|viewport_id| {
        viewport_id == MAIN_VIEWPORT_ID || viewport_id_set.contains(&viewport_id)
    });
    viewport_presentations.retain_viewports(|viewport_id| {
        viewport_id == MAIN_VIEWPORT_ID || viewport_id_set.contains(&viewport_id)
    });
    viewport_observations.retain_viewports(|viewport_id| {
        viewport_id == MAIN_VIEWPORT_ID || viewport_id_set.contains(&viewport_id)
    });
    viewport_surface_sets.retain_viewports(|viewport_id| {
        viewport_id == MAIN_VIEWPORT_ID || viewport_id_set.contains(&viewport_id)
    });
}

pub(crate) fn viewport_product_surface_manifest(
    viewport_product_targets: &ViewportProductTargetRegistryResource,
    viewport_presentations: &ViewportPresentationStateResource,
) -> RenderProductSurfaceManifest {
    let mut manifest = RenderProductSurfaceManifest::new(
        EDITOR_VIEWPORT_RENDER_PRODUCT_PRODUCER_ID,
        "editor.viewport.products",
    )
    .with_dynamic_targets(viewport_product_targets.requested_dynamic_descriptors());

    for record in viewport_product_targets.records() {
        if record.status != ViewportProductTargetStatus::Requested {
            manifest = manifest.with_status(
                viewport_product_surface_key(record),
                RenderProductSurfaceStatusKind::Unavailable,
                format!(
                    "viewport product {} target is unavailable for slot {}",
                    record.key.product_id.0,
                    presentation_slot_label(record.key.presentation_slot)
                ),
            );
            continue;
        }

        if !target_record_is_selected_or_support(record, viewport_presentations)
            || !record.ui_sampleable
        {
            continue;
        }

        manifest = manifest.with_viewport_surface_binding(
            record.key.viewport_id.0,
            viewport_embed_slot_for(record.key.presentation_slot),
            viewport_product_surface_key(record),
            ViewportSurfaceBindingSource::dynamic_texture(
                record.namespace.clone(),
                record.target_id.clone(),
            ),
        );
    }

    manifest
}

fn canonical_viewport_ids_for_sync(
    viewport_instances: &ViewportInstanceRegistryResource,
    viewport_surface_sets: &ViewportSurfaceSetResource,
    tool_surface_bindings: &ToolSurfaceRuntimeBindingRegistryResource,
) -> Vec<ViewportId> {
    let mut viewport_ids = viewport_instances
        .records()
        .map(|record| record.viewport_id)
        .collect::<std::collections::BTreeSet<_>>();
    viewport_ids.extend(
        tool_surface_bindings
            .bindings()
            .map(|binding| binding.viewport_id),
    );
    if viewport_ids.is_empty() {
        viewport_ids.extend(viewport_surface_sets.viewport_ids());
    }
    viewport_ids.into_iter().collect()
}

fn product_dimensions_for_viewport(
    viewport_id: ViewportId,
    viewport_render_states: &ViewportRenderStateResource,
) -> ExpressionDimensions {
    viewport_render_states
        .state_for(viewport_id)
        .map(|state| expression_dimensions_for_bounds(state.bounds))
        .unwrap_or_else(|| ExpressionDimensions::new(1, 1))
}

fn build_artifact_observation_frame(
    descriptors: &[ExpressionProductDescriptor],
    presentation_state: &ViewportPresentationState,
    source_version: RealityVersion,
) -> ArtifactObservationFrame {
    let mut frame = ArtifactObservationFrame::new(presentation_state.viewport_id, source_version);
    frame.available_products = descriptors.to_vec();
    frame.selected_primary_product_id = Some(presentation_state.selected_primary_product_id);
    frame.selected_overlay_product_ids = presentation_state.selected_overlay_product_ids.clone();
    frame.field_visualizer_settings = presentation_state.field_visualizer_settings;

    for descriptor in descriptors {
        let target_record =
            product_target_record_for_descriptor(presentation_state.viewport_id, descriptor);
        let available = target_record
            .as_ref()
            .is_some_and(|record| record.status == ViewportProductTargetStatus::Requested);
        frame.availability_by_product.insert(
            descriptor.id,
            if available {
                ProductAvailabilityState::Available
            } else {
                ProductAvailabilityState::Unavailable
            },
        );
        frame.producer_health_by_product.insert(
            descriptor.id,
            if available {
                ProducerHealth::Healthy
            } else {
                ProducerHealth::Unavailable
            },
        );
    }

    if let std::collections::btree_map::Entry::Vacant(e) = frame
        .availability_by_product
        .entry(presentation_state.selected_primary_product_id)
    {
        e.insert(ProductAvailabilityState::Unavailable);
        frame.producer_health_by_product.insert(
            presentation_state.selected_primary_product_id,
            ProducerHealth::Unavailable,
        );
    }

    frame
}

fn sync_surface_sets_from_product_targets(
    viewport_product_targets: &ViewportProductTargetRegistryResource,
    viewport_presentations: &ViewportPresentationStateResource,
    viewport_surface_sets: &mut ViewportSurfaceSetResource,
) {
    let active_viewports = viewport_product_targets
        .viewport_ids()
        .collect::<BTreeSet<_>>();
    viewport_surface_sets.retain_viewports(|viewport_id| active_viewports.contains(&viewport_id));
    for viewport_id in &active_viewports {
        viewport_surface_sets.clear_viewport_surfaces(*viewport_id);
    }

    for record in viewport_product_targets.records() {
        if !target_record_is_selected_or_support(record, viewport_presentations) {
            continue;
        }
        let Some(handle) = record.surface_handle() else {
            continue;
        };
        viewport_surface_sets.set_surface(record.key.viewport_id, record.surface_slot, handle);
    }
}

fn target_record_is_selected_or_support(
    record: &ViewportProductTargetRecord,
    viewport_presentations: &ViewportPresentationStateResource,
) -> bool {
    match record.key.presentation_slot {
        ViewportSurfacePresentationSlot::Primary => viewport_presentations
            .state_for(record.key.viewport_id)
            .is_some_and(|state| state.selected_primary_product_id == record.key.product_id),
        ViewportSurfacePresentationSlot::Picking | ViewportSurfacePresentationSlot::Overlay => true,
    }
}

pub(crate) fn product_target_record_for_descriptor(
    viewport_id: ViewportId,
    descriptor: &ExpressionProductDescriptor,
) -> Option<ViewportProductTargetRecord> {
    let (presentation_slot, surface_slot) = product_target_slots(descriptor.kind)?;
    let key = ViewportProductTargetKey::new(viewport_id, presentation_slot, descriptor.id);
    let target_format = target_format_for_descriptor(descriptor);
    let status = if descriptor.dimensions.width == 0
        || descriptor.dimensions.height == 0
        || target_format.is_none()
    {
        ViewportProductTargetStatus::Unavailable
    } else {
        ViewportProductTargetStatus::Requested
    };
    let sample_mode = sample_mode_for_descriptor(descriptor);
    let ui_sampleable = matches!(
        sample_mode,
        RenderTextureSampleMode::FilterableFloat | RenderTextureSampleMode::NonFilterableFloat
    );

    Some(ViewportProductTargetRecord {
        key,
        surface_slot,
        namespace: VIEWPORT_DYNAMIC_TARGET_NAMESPACE.to_string(),
        target_id: dynamic_target_id_for(key),
        width: descriptor.dimensions.width,
        height: descriptor.dimensions.height,
        format: descriptor.format.clone(),
        target_format,
        usage: usage_for_descriptor(descriptor),
        sample_mode,
        ui_sampleable,
        status,
        generation: 0,
    })
}

fn product_target_slots(
    kind: ExpressionProductKind,
) -> Option<(ViewportSurfacePresentationSlot, ViewportSurfaceSlot)> {
    match kind {
        ExpressionProductKind::SceneColor2D => Some((
            ViewportSurfacePresentationSlot::Primary,
            ViewportSurfaceSlot::PrimaryColor,
        )),
        ExpressionProductKind::PickingIds2D => Some((
            ViewportSurfacePresentationSlot::Picking,
            ViewportSurfaceSlot::PickingIds,
        )),
        ExpressionProductKind::Overlay2D => Some((
            ViewportSurfacePresentationSlot::Overlay,
            ViewportSurfaceSlot::Overlay,
        )),
        ExpressionProductKind::ScalarField2D => Some((
            ViewportSurfacePresentationSlot::Primary,
            ViewportSurfaceSlot::ScalarField,
        )),
        ExpressionProductKind::VectorField2D => Some((
            ViewportSurfacePresentationSlot::Primary,
            ViewportSurfaceSlot::VectorField,
        )),
        ExpressionProductKind::Atlas2D => Some((
            ViewportSurfacePresentationSlot::Primary,
            ViewportSurfaceSlot::Atlas,
        )),
        ExpressionProductKind::VolumeSlice2D => Some((
            ViewportSurfacePresentationSlot::Primary,
            ViewportSurfaceSlot::VolumeSlice,
        )),
        ExpressionProductKind::BrickmapDebug2D => Some((
            ViewportSurfacePresentationSlot::Primary,
            ViewportSurfaceSlot::BrickmapDebug,
        )),
        ExpressionProductKind::HistoryColor2D => Some((
            ViewportSurfacePresentationSlot::Primary,
            ViewportSurfaceSlot::HistoryColor,
        )),
        ExpressionProductKind::MaterialPreview2D => Some((
            ViewportSurfacePresentationSlot::Primary,
            ViewportSurfaceSlot::PrimaryColor,
        )),
        ExpressionProductKind::Depth2D | ExpressionProductKind::Diagnostics2D => None,
    }
}

pub fn dynamic_target_id_for(key: ViewportProductTargetKey) -> String {
    format!(
        "editor.viewport.{}.{}.{}",
        key.viewport_id.0,
        key.product_id.0,
        presentation_slot_label(key.presentation_slot)
    )
}

fn viewport_product_surface_key(record: &ViewportProductTargetRecord) -> String {
    record.dynamic_key().to_string()
}

fn presentation_slot_label(slot: ViewportSurfacePresentationSlot) -> &'static str {
    match slot {
        ViewportSurfacePresentationSlot::Primary => "primary",
        ViewportSurfacePresentationSlot::Picking => "picking",
        ViewportSurfacePresentationSlot::Overlay => "overlay",
    }
}

fn target_format_for_descriptor(
    descriptor: &ExpressionProductDescriptor,
) -> Option<RenderTextureTargetFormat> {
    match &descriptor.format {
        ExpressionFormat::Rgba8Unorm if descriptor.presentation_hints.srgb => {
            Some(RenderTextureTargetFormat::Rgba8UnormSrgb)
        }
        ExpressionFormat::Rgba8Unorm => Some(RenderTextureTargetFormat::Rgba8Unorm),
        ExpressionFormat::R32Uint => Some(RenderTextureTargetFormat::R32Uint),
        ExpressionFormat::Depth32Float => Some(RenderTextureTargetFormat::Depth32Float),
        ExpressionFormat::Other(_) => None,
    }
}

fn usage_for_descriptor(descriptor: &ExpressionProductDescriptor) -> RenderTextureTargetUsage {
    match descriptor.kind {
        ExpressionProductKind::SceneColor2D | ExpressionProductKind::Overlay2D => {
            RenderTextureTargetUsage {
                color_attachment: true,
                depth_attachment: false,
                sampled: true,
                storage: false,
                copy_src: true,
                copy_dst: false,
            }
        }
        ExpressionProductKind::PickingIds2D => RenderTextureTargetUsage {
            color_attachment: true,
            depth_attachment: false,
            sampled: false,
            storage: false,
            copy_src: true,
            copy_dst: false,
        },
        ExpressionProductKind::Depth2D => RenderTextureTargetUsage {
            color_attachment: false,
            depth_attachment: true,
            sampled: false,
            storage: false,
            copy_src: true,
            copy_dst: false,
        },
        ExpressionProductKind::Diagnostics2D => RenderTextureTargetUsage {
            color_attachment: true,
            depth_attachment: false,
            sampled: true,
            storage: false,
            copy_src: true,
            copy_dst: false,
        },
        ExpressionProductKind::ScalarField2D
        | ExpressionProductKind::VectorField2D
        | ExpressionProductKind::Atlas2D
        | ExpressionProductKind::VolumeSlice2D
        | ExpressionProductKind::BrickmapDebug2D
        | ExpressionProductKind::HistoryColor2D => RenderTextureTargetUsage {
            color_attachment: false,
            depth_attachment: false,
            sampled: true,
            storage: false,
            copy_src: true,
            copy_dst: true,
        },
        ExpressionProductKind::MaterialPreview2D => RenderTextureTargetUsage {
            color_attachment: true,
            depth_attachment: false,
            sampled: true,
            storage: false,
            copy_src: true,
            copy_dst: false,
        },
    }
}

fn sample_mode_for_descriptor(descriptor: &ExpressionProductDescriptor) -> RenderTextureSampleMode {
    match &descriptor.format {
        ExpressionFormat::Rgba8Unorm => RenderTextureSampleMode::FilterableFloat,
        ExpressionFormat::R32Uint => RenderTextureSampleMode::NotSampled,
        ExpressionFormat::Depth32Float => RenderTextureSampleMode::NotSampled,
        ExpressionFormat::Other(_) => RenderTextureSampleMode::NotSampled,
    }
}

fn target_signature_matches(
    previous: &ViewportProductTargetRecord,
    next: &ViewportProductTargetRecord,
) -> bool {
    previous.namespace == next.namespace
        && previous.target_id == next.target_id
        && previous.width == next.width
        && previous.height == next.height
        && previous.format == next.format
        && previous.ui_sampleable == next.ui_sampleable
        && previous.status == next.status
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::viewport::{
        HISTORY_COLOR_PRODUCT_ID, OVERLAY_PRODUCT_ID, PICKING_IDS_PRODUCT_ID,
        SCALAR_FIELD_PRODUCT_ID, SCENE_COLOR_PRODUCT_ID, ToolSurfaceRuntimeBindingRecord,
    };
    use editor_core::RealityVersion;
    use editor_shell::{PanelInstanceId, TabStackId, ToolSurfaceInstanceId, WidgetId};
    use editor_viewport::{
        ExpressionDimensions, ExpressionFreshness, ExpressionPresentationHints,
        ExpressionSourceRealityClass,
    };
    use engine::plugins::render::{
        RenderProductSurfaceDiagnosticKind, RenderProductSurfaceRequestKind,
        RenderProductSurfaceStatusKind,
    };
    use ui_math::UiRect;

    fn binding(
        surface: u64,
        panel: u64,
        stack: u64,
        viewport_id: ViewportId,
        bounds: UiRect,
    ) -> ToolSurfaceRuntimeBindingRecord {
        ToolSurfaceRuntimeBindingRecord {
            tool_surface_id: ToolSurfaceInstanceId::try_from_raw(surface).unwrap(),
            panel_instance_id: PanelInstanceId::try_from_raw(panel).unwrap(),
            tab_stack_id: TabStackId::try_from_raw(stack).unwrap(),
            viewport_id,
            host_widget_id: WidgetId(10_000 + surface),
            bounds,
            generation: 1,
        }
    }

    fn descriptor(
        id: ExpressionProductId,
        kind: ExpressionProductKind,
        format: ExpressionFormat,
    ) -> ExpressionProductDescriptor {
        ExpressionProductDescriptor::new(
            id,
            kind,
            ExpressionDimensions::new(320, 200),
            format,
            "test.producer",
            ExpressionSourceRealityClass::ObservedScene,
            RealityVersion(1),
            ExpressionFreshness::Current,
            ExpressionPresentationHints::default(),
            None,
        )
    }

    #[test]
    fn dynamic_target_ids_are_stable_per_viewport_product_slot() {
        let key = ViewportProductTargetKey::new(
            ViewportId(42),
            ViewportSurfacePresentationSlot::Primary,
            SCENE_COLOR_PRODUCT_ID,
        );

        assert_eq!(dynamic_target_id_for(key), "editor.viewport.42.1.primary");
    }

    #[test]
    fn scene_targets_are_sampleable_and_picking_targets_are_not() {
        let scene = product_target_record_for_descriptor(
            ViewportId(1),
            &descriptor(
                SCENE_COLOR_PRODUCT_ID,
                ExpressionProductKind::SceneColor2D,
                ExpressionFormat::Rgba8Unorm,
            ),
        )
        .expect("scene color should map to a product target");
        let picking = product_target_record_for_descriptor(
            ViewportId(1),
            &descriptor(
                PICKING_IDS_PRODUCT_ID,
                ExpressionProductKind::PickingIds2D,
                ExpressionFormat::R32Uint,
            ),
        )
        .expect("picking ids should map to a product target");

        assert!(scene.ui_sampleable);
        assert!(!picking.ui_sampleable);
    }

    #[test]
    fn overlay_targets_use_the_overlay_presentation_slot() {
        let overlay = product_target_record_for_descriptor(
            ViewportId(7),
            &descriptor(
                OVERLAY_PRODUCT_ID,
                ExpressionProductKind::Overlay2D,
                ExpressionFormat::Rgba8Unorm,
            ),
        )
        .expect("overlay should map to a product target");

        assert_eq!(
            overlay.key.presentation_slot,
            ViewportSurfacePresentationSlot::Overlay
        );
        assert_eq!(overlay.surface_slot, ViewportSurfaceSlot::Overlay);
    }

    #[test]
    fn field_and_volume_debug_products_are_displayable_rgba_targets() {
        let descriptors =
            initial_product_descriptors(ExpressionDimensions::new(320, 200), RealityVersion(1));
        let presentation_state = initial_presentation_state(ViewportId(1));

        let frame =
            build_artifact_observation_frame(&descriptors, &presentation_state, RealityVersion(1));

        assert_eq!(
            frame
                .availability_by_product
                .get(&SCENE_COLOR_PRODUCT_ID)
                .copied(),
            Some(ProductAvailabilityState::Available)
        );
        assert_eq!(
            frame
                .availability_by_product
                .get(&SCALAR_FIELD_PRODUCT_ID)
                .copied(),
            Some(ProductAvailabilityState::Available)
        );
        assert_eq!(
            frame
                .producer_health_by_product
                .get(&HISTORY_COLOR_PRODUCT_ID)
                .copied(),
            Some(ProducerHealth::Healthy)
        );
    }

    #[test]
    fn material_preview_products_are_selectable_primary_targets() {
        let descriptor = descriptor(
            ExpressionProductId(12),
            ExpressionProductKind::MaterialPreview2D,
            ExpressionFormat::Rgba8Unorm,
        );

        let target = product_target_record_for_descriptor(ViewportId(1), &descriptor)
            .expect("material preview should map to a primary target");

        assert_eq!(
            target.key.presentation_slot,
            ViewportSurfacePresentationSlot::Primary
        );
        assert_eq!(target.surface_slot, ViewportSurfaceSlot::PrimaryColor);
        assert!(target.ui_sampleable);
        assert!(
            target.usage.color_attachment,
            "material preview targets are produced by a material preview render pass"
        );
        assert!(
            !target.usage.copy_dst,
            "material preview targets must not rely on CPU dynamic uploads"
        );
    }

    #[test]
    fn material_viewport_preview_selection_publishes_through_presentation_state() {
        let material_product_id = ExpressionProductId(12);
        let material_descriptor =
            crate::runtime::viewport::product_registry::material_preview_descriptor(
                material_product_id,
                ExpressionDimensions::new(320, 200),
                RealityVersion(2),
                "material.first_slice.render_material".to_string(),
            );
        let mut presentation_state = initial_presentation_state(ViewportId(1));
        presentation_state.select_primary_product(material_product_id);

        let frame = build_artifact_observation_frame(
            &[material_descriptor],
            &presentation_state,
            RealityVersion(2),
        );

        assert_eq!(frame.selected_primary_product_id, Some(material_product_id));
        assert_eq!(
            frame
                .availability_by_product
                .get(&material_product_id)
                .copied(),
            Some(ProductAvailabilityState::Available)
        );
    }

    #[test]
    fn surface_sync_keeps_scene_primary_when_material_preview_is_unselected() {
        let viewport_id = ViewportId(1);
        let material_product_id = ExpressionProductId(12);
        let scene_descriptor = descriptor(
            SCENE_COLOR_PRODUCT_ID,
            ExpressionProductKind::SceneColor2D,
            ExpressionFormat::Rgba8Unorm,
        );
        let material_descriptor = descriptor(
            material_product_id,
            ExpressionProductKind::MaterialPreview2D,
            ExpressionFormat::Rgba8Unorm,
        );
        let mut targets = ViewportProductTargetRegistryResource::default();
        targets.replace_records(
            [&scene_descriptor, &material_descriptor]
                .into_iter()
                .filter_map(|descriptor| {
                    product_target_record_for_descriptor(viewport_id, descriptor)
                })
                .collect(),
        );
        let mut presentations = ViewportPresentationStateResource::default();
        presentations.upsert_state(initial_presentation_state(viewport_id));
        let mut surface_sets = ViewportSurfaceSetResource::default();

        sync_surface_sets_from_product_targets(&targets, &presentations, &mut surface_sets);

        let primary = surface_sets
            .surface(viewport_id, ViewportSurfaceSlot::PrimaryColor)
            .expect("scene primary target should bind");
        assert_eq!(
            primary.target_id,
            dynamic_target_id_for(ViewportProductTargetKey::new(
                viewport_id,
                ViewportSurfacePresentationSlot::Primary,
                scene_descriptor.id,
            ))
        );
    }

    #[test]
    fn surface_sync_binds_material_preview_only_when_selected_primary() {
        let viewport_id = ViewportId(1);
        let material_product_id = ExpressionProductId(12);
        let scene_descriptor = descriptor(
            SCENE_COLOR_PRODUCT_ID,
            ExpressionProductKind::SceneColor2D,
            ExpressionFormat::Rgba8Unorm,
        );
        let material_descriptor = descriptor(
            material_product_id,
            ExpressionProductKind::MaterialPreview2D,
            ExpressionFormat::Rgba8Unorm,
        );
        let mut targets = ViewportProductTargetRegistryResource::default();
        targets.replace_records(
            [&scene_descriptor, &material_descriptor]
                .into_iter()
                .filter_map(|descriptor| {
                    product_target_record_for_descriptor(viewport_id, descriptor)
                })
                .collect(),
        );
        let mut presentation = initial_presentation_state(viewport_id);
        presentation.select_primary_product(material_product_id);
        let mut presentations = ViewportPresentationStateResource::default();
        presentations.upsert_state(presentation);
        let mut surface_sets = ViewportSurfaceSetResource::default();

        sync_surface_sets_from_product_targets(&targets, &presentations, &mut surface_sets);

        let primary = surface_sets
            .surface(viewport_id, ViewportSurfaceSlot::PrimaryColor)
            .expect("material preview target should bind as selected primary");
        assert_eq!(
            primary.target_id,
            dynamic_target_id_for(ViewportProductTargetKey::new(
                viewport_id,
                ViewportSurfacePresentationSlot::Primary,
                material_descriptor.id,
            ))
        );
    }

    #[test]
    fn field_visualizer_settings_do_not_parameterize_dynamic_target_identity() {
        let descriptor = descriptor(
            SCALAR_FIELD_PRODUCT_ID,
            ExpressionProductKind::ScalarField2D,
            ExpressionFormat::Rgba8Unorm,
        );
        let before =
            product_target_record_for_descriptor(ViewportId(1), &descriptor).expect("target");
        let mut presentation_state = initial_presentation_state(ViewportId(1));
        presentation_state.set_field_visualizer_settings(
            editor_viewport::ViewportFieldVisualizerSettings::default()
                .with_component(editor_viewport::ViewportFieldVisualizerComponent::Magnitude)
                .with_slice_index(9)
                .with_color_ramp(editor_viewport::ViewportFieldVisualizerColorRamp::Heat)
                .with_debug_mode(editor_viewport::ViewportFieldVisualizerDebugMode::Freshness),
        );
        let frame =
            build_artifact_observation_frame(&[descriptor], &presentation_state, RealityVersion(1));
        let after = product_target_record_for_descriptor(
            ViewportId(1),
            frame.available_products.first().expect("descriptor"),
        )
        .expect("target");

        assert_eq!(before.key, after.key);
        assert_eq!(before.target_id, after.target_id);
        assert_eq!(
            frame.field_visualizer_settings,
            presentation_state.field_visualizer_settings
        );
    }

    #[test]
    fn viewport_product_surface_manifest_traces_selected_field_surface_binding() {
        let viewport_id = ViewportId(1);
        let descriptor = descriptor(
            SCALAR_FIELD_PRODUCT_ID,
            ExpressionProductKind::ScalarField2D,
            ExpressionFormat::Rgba8Unorm,
        );
        let mut targets = ViewportProductTargetRegistryResource::default();
        targets.replace_records(vec![
            product_target_record_for_descriptor(viewport_id, &descriptor).expect("target"),
        ]);
        let mut presentation = initial_presentation_state(viewport_id);
        presentation.select_primary_product(SCALAR_FIELD_PRODUCT_ID);
        let mut presentations = ViewportPresentationStateResource::default();
        presentations.upsert_state(presentation);

        let manifest = viewport_product_surface_manifest(&targets, &presentations);

        assert_eq!(manifest.product_family(), "editor.viewport.products");
        assert_eq!(manifest.dynamic_targets().len(), 1);
        assert_eq!(manifest.viewport_bindings().len(), 1);
        assert_eq!(manifest.viewport_bindings()[0].viewport_id, viewport_id.0);
        assert_eq!(
            manifest.viewport_bindings()[0].surface_key,
            manifest.dynamic_targets()[0].key.to_string()
        );
        assert!(
            manifest.diagnostics().is_empty(),
            "selected field product should bind through a declared sampleable target"
        );
    }

    #[test]
    fn viewport_product_surface_manifest_traces_unavailable_field_status() {
        let viewport_id = ViewportId(1);
        let descriptor = ExpressionProductDescriptor::new(
            SCALAR_FIELD_PRODUCT_ID,
            ExpressionProductKind::ScalarField2D,
            ExpressionDimensions::new(0, 64),
            ExpressionFormat::Rgba8Unorm,
            "test.field.producer",
            ExpressionSourceRealityClass::ObservedScene,
            RealityVersion(1),
            ExpressionFreshness::Current,
            ExpressionPresentationHints::default(),
            None,
        );
        let mut targets = ViewportProductTargetRegistryResource::default();
        targets.replace_records(vec![
            product_target_record_for_descriptor(viewport_id, &descriptor).expect("target"),
        ]);
        let presentations = ViewportPresentationStateResource::default();

        let manifest = viewport_product_surface_manifest(&targets, &presentations);
        let diagnostics = manifest.diagnostics();

        assert!(manifest.dynamic_targets().is_empty());
        assert_eq!(manifest.statuses().len(), 1);
        assert_eq!(
            manifest.statuses()[0].status,
            RenderProductSurfaceStatusKind::Unavailable
        );
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.diagnostic_kind == RenderProductSurfaceDiagnosticKind::ProducerStatus
                && diagnostic.request_kind == RenderProductSurfaceRequestKind::Status
                && diagnostic.status == Some(RenderProductSurfaceStatusKind::Unavailable)
        }));
    }

    #[test]
    fn presentation_sync_prefers_runtime_bindings_over_bootstrap_surface_set() {
        let mut surface_sets = ViewportSurfaceSetResource::default();
        ensure_editor_main_surface_set(&mut surface_sets, MAIN_VIEWPORT_ID);
        let mut bindings = ToolSurfaceRuntimeBindingRegistryResource::default();
        let first = ViewportId(2);
        let second = ViewportId(3);
        bindings.upsert_binding(binding(1, 1, 1, first, UiRect::new(0.0, 0.0, 320.0, 240.0)));
        bindings.upsert_binding(binding(
            2,
            2,
            2,
            second,
            UiRect::new(320.0, 0.0, 480.0, 240.0),
        ));

        assert_eq!(
            canonical_viewport_ids_for_sync(
                &ViewportInstanceRegistryResource::default(),
                &surface_sets,
                &bindings
            ),
            vec![first, second],
        );
    }
}
