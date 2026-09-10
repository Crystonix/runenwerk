//! File: apps/runenwerk_editor/src/runtime/viewport/product_registry.rs
//! Purpose: Session-scoped viewport product registry for editor runtime.

use std::collections::BTreeMap;

use editor_core::RealityVersion;
use editor_viewport::{
    ArtifactObservationFrame, ExpressionChannelLayerSliceMetadata, ExpressionDimensions,
    ExpressionFormat, ExpressionFreshness, ExpressionPresentationHints,
    ExpressionProductDescriptor, ExpressionProductId, ExpressionProductKind,
    ExpressionSourceRealityClass, ViewportId, ViewportPresentationState,
};

use super::producer_field::{
    SCALAR_FIELD_DEBUG_PRODUCER, VECTOR_FIELD_DEBUG_PRODUCER, field_debug_presentation_hints,
    scalar_field_debug_metadata, vector_field_debug_metadata,
};
use super::producer_volume::{
    ATLAS_DEBUG_PRODUCER, BRICKMAP_DEBUG_PRODUCER, HISTORY_COLOR_DEBUG_PRODUCER,
    VOLUME_SLICE_DEBUG_PRODUCER, atlas_debug_metadata, brickmap_debug_metadata,
    history_color_debug_metadata, volume_debug_presentation_hints, volume_slice_debug_metadata,
};

pub const MAIN_VIEWPORT_ID: ViewportId = ViewportId(1);

pub const SCENE_COLOR_PRODUCT_ID: ExpressionProductId = ExpressionProductId(1);
pub const PICKING_IDS_PRODUCT_ID: ExpressionProductId = ExpressionProductId(2);
pub const OVERLAY_PRODUCT_ID: ExpressionProductId = ExpressionProductId(3);
pub const DEPTH_PRODUCT_ID: ExpressionProductId = ExpressionProductId(4);
pub const DIAGNOSTICS_PRODUCT_ID: ExpressionProductId = ExpressionProductId(5);
pub const SCALAR_FIELD_PRODUCT_ID: ExpressionProductId = ExpressionProductId(6);
pub const VECTOR_FIELD_PRODUCT_ID: ExpressionProductId = ExpressionProductId(7);
pub const ATLAS_PRODUCT_ID: ExpressionProductId = ExpressionProductId(8);
pub const VOLUME_SLICE_PRODUCT_ID: ExpressionProductId = ExpressionProductId(9);
pub const BRICKMAP_DEBUG_PRODUCT_ID: ExpressionProductId = ExpressionProductId(10);
pub const HISTORY_COLOR_PRODUCT_ID: ExpressionProductId = ExpressionProductId(11);

pub fn initial_presentation_state(viewport_id: ViewportId) -> ViewportPresentationState {
    ViewportPresentationState::new(viewport_id, SCENE_COLOR_PRODUCT_ID)
}

pub fn initial_product_descriptors(
    dimensions: ExpressionDimensions,
    source_version: RealityVersion,
) -> Vec<ExpressionProductDescriptor> {
    vec![
        ExpressionProductDescriptor::new(
            SCENE_COLOR_PRODUCT_ID,
            ExpressionProductKind::SceneColor2D,
            dimensions,
            ExpressionFormat::Rgba8Unorm,
            "editor.viewport.scene_producer",
            ExpressionSourceRealityClass::ObservedScene,
            source_version,
            ExpressionFreshness::Current,
            ExpressionPresentationHints {
                srgb: true,
                premultiplied_alpha: false,
                y_flipped: false,
            },
            None,
        ),
        ExpressionProductDescriptor::new(
            PICKING_IDS_PRODUCT_ID,
            ExpressionProductKind::PickingIds2D,
            dimensions,
            ExpressionFormat::R32Uint,
            "editor.viewport.picking_producer",
            ExpressionSourceRealityClass::DerivedPicking,
            source_version,
            ExpressionFreshness::Current,
            ExpressionPresentationHints {
                srgb: false,
                premultiplied_alpha: false,
                y_flipped: false,
            },
            None,
        ),
        ExpressionProductDescriptor::new(
            OVERLAY_PRODUCT_ID,
            ExpressionProductKind::Overlay2D,
            dimensions,
            ExpressionFormat::Rgba8Unorm,
            "editor.viewport.overlay_producer",
            ExpressionSourceRealityClass::DerivedOverlay,
            source_version,
            ExpressionFreshness::Current,
            ExpressionPresentationHints {
                srgb: true,
                premultiplied_alpha: true,
                y_flipped: false,
            },
            None,
        ),
        ExpressionProductDescriptor::new(
            DEPTH_PRODUCT_ID,
            ExpressionProductKind::Depth2D,
            dimensions,
            ExpressionFormat::Depth32Float,
            "editor.viewport.depth_producer",
            ExpressionSourceRealityClass::ObservedScene,
            source_version,
            ExpressionFreshness::Current,
            ExpressionPresentationHints::default(),
            Some(ExpressionChannelLayerSliceMetadata {
                channel_label: Some("depth".to_string()),
                layer_label: None,
                slice_label: None,
                slice_count: None,
            }),
        ),
        ExpressionProductDescriptor::new(
            DIAGNOSTICS_PRODUCT_ID,
            ExpressionProductKind::Diagnostics2D,
            dimensions,
            ExpressionFormat::Rgba8Unorm,
            "editor.viewport.diagnostics_producer",
            ExpressionSourceRealityClass::Diagnostics,
            source_version,
            ExpressionFreshness::PotentiallyStale,
            ExpressionPresentationHints {
                srgb: true,
                premultiplied_alpha: false,
                y_flipped: false,
            },
            None,
        ),
        ExpressionProductDescriptor::new(
            SCALAR_FIELD_PRODUCT_ID,
            ExpressionProductKind::ScalarField2D,
            dimensions,
            ExpressionFormat::Rgba8Unorm,
            SCALAR_FIELD_DEBUG_PRODUCER,
            ExpressionSourceRealityClass::DerivedField,
            source_version,
            ExpressionFreshness::PotentiallyStale,
            field_debug_presentation_hints(),
            Some(scalar_field_debug_metadata()),
        ),
        ExpressionProductDescriptor::new(
            VECTOR_FIELD_PRODUCT_ID,
            ExpressionProductKind::VectorField2D,
            dimensions,
            ExpressionFormat::Rgba8Unorm,
            VECTOR_FIELD_DEBUG_PRODUCER,
            ExpressionSourceRealityClass::DerivedField,
            source_version,
            ExpressionFreshness::PotentiallyStale,
            field_debug_presentation_hints(),
            Some(vector_field_debug_metadata()),
        ),
        ExpressionProductDescriptor::new(
            ATLAS_PRODUCT_ID,
            ExpressionProductKind::Atlas2D,
            dimensions,
            ExpressionFormat::Rgba8Unorm,
            ATLAS_DEBUG_PRODUCER,
            ExpressionSourceRealityClass::DerivedAsset,
            source_version,
            ExpressionFreshness::PotentiallyStale,
            volume_debug_presentation_hints(),
            Some(atlas_debug_metadata()),
        ),
        ExpressionProductDescriptor::new(
            VOLUME_SLICE_PRODUCT_ID,
            ExpressionProductKind::VolumeSlice2D,
            dimensions,
            ExpressionFormat::Rgba8Unorm,
            VOLUME_SLICE_DEBUG_PRODUCER,
            ExpressionSourceRealityClass::DerivedVolume,
            source_version,
            ExpressionFreshness::PotentiallyStale,
            volume_debug_presentation_hints(),
            Some(volume_slice_debug_metadata()),
        ),
        ExpressionProductDescriptor::new(
            BRICKMAP_DEBUG_PRODUCT_ID,
            ExpressionProductKind::BrickmapDebug2D,
            dimensions,
            ExpressionFormat::Rgba8Unorm,
            BRICKMAP_DEBUG_PRODUCER,
            ExpressionSourceRealityClass::Diagnostics,
            source_version,
            ExpressionFreshness::PotentiallyStale,
            volume_debug_presentation_hints(),
            Some(brickmap_debug_metadata()),
        ),
        ExpressionProductDescriptor::new(
            HISTORY_COLOR_PRODUCT_ID,
            ExpressionProductKind::HistoryColor2D,
            dimensions,
            ExpressionFormat::Rgba8Unorm,
            HISTORY_COLOR_DEBUG_PRODUCER,
            ExpressionSourceRealityClass::DerivedHistory,
            source_version,
            ExpressionFreshness::PotentiallyStale,
            volume_debug_presentation_hints(),
            Some(history_color_debug_metadata()),
        ),
    ]
}

pub fn material_preview_descriptor(
    product_id: ExpressionProductId,
    dimensions: ExpressionDimensions,
    source_version: RealityVersion,
    specialization_fragment: String,
) -> ExpressionProductDescriptor {
    ExpressionProductDescriptor::new(
        product_id,
        ExpressionProductKind::MaterialPreview2D,
        dimensions,
        ExpressionFormat::Rgba8Unorm,
        format!("editor.material_lab.preview:{specialization_fragment}"),
        ExpressionSourceRealityClass::DerivedMaterial,
        source_version,
        ExpressionFreshness::Current,
        ExpressionPresentationHints {
            srgb: true,
            premultiplied_alpha: false,
            y_flipped: false,
        },
        None,
    )
}

pub fn material_preview_descriptor_with_lineage(
    product_id: ExpressionProductId,
    dimensions: ExpressionDimensions,
    material_generation: RealityVersion,
    specialization_fragment: String,
    material_lineage: impl Into<String>,
) -> ExpressionProductDescriptor {
    ExpressionProductDescriptor::new(
        product_id,
        ExpressionProductKind::MaterialPreview2D,
        dimensions,
        ExpressionFormat::Rgba8Unorm,
        format!(
            "editor.material_lab.preview:{specialization_fragment}:{}",
            material_lineage.into()
        ),
        ExpressionSourceRealityClass::DerivedMaterial,
        material_generation,
        ExpressionFreshness::Current,
        ExpressionPresentationHints {
            srgb: true,
            premultiplied_alpha: false,
            y_flipped: false,
        },
        None,
    )
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource, Default)]
pub struct ViewportProductRegistryResource {
    descriptors_by_viewport: BTreeMap<ViewportId, Vec<ExpressionProductDescriptor>>,
}

impl ViewportProductRegistryResource {
    pub fn update_viewport_descriptors(
        &mut self,
        viewport_id: ViewportId,
        descriptors: Vec<ExpressionProductDescriptor>,
    ) {
        self.descriptors_by_viewport
            .insert(viewport_id, descriptors);
    }

    pub fn descriptors_for(
        &self,
        viewport_id: ViewportId,
    ) -> Option<&[ExpressionProductDescriptor]> {
        self.descriptors_by_viewport
            .get(&viewport_id)
            .map(Vec::as_slice)
    }

    pub fn viewport_ids(&self) -> impl Iterator<Item = ViewportId> + '_ {
        self.descriptors_by_viewport.keys().copied()
    }

    pub fn retain_viewports(&mut self, mut keep: impl FnMut(ViewportId) -> bool) {
        self.descriptors_by_viewport
            .retain(|viewport_id, _| keep(*viewport_id));
    }

    pub fn is_empty(&self) -> bool {
        self.descriptors_by_viewport.is_empty()
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource, Default)]
pub struct ViewportPresentationStateResource {
    states_by_viewport: BTreeMap<ViewportId, ViewportPresentationState>,
}

impl ViewportPresentationStateResource {
    pub fn state_for(&self, viewport_id: ViewportId) -> Option<&ViewportPresentationState> {
        self.states_by_viewport.get(&viewport_id)
    }

    pub fn state_for_mut(
        &mut self,
        viewport_id: ViewportId,
    ) -> Option<&mut ViewportPresentationState> {
        self.states_by_viewport.get_mut(&viewport_id)
    }

    pub fn upsert_state(&mut self, state: ViewportPresentationState) {
        self.states_by_viewport.insert(state.viewport_id, state);
    }

    pub fn states(&self) -> impl Iterator<Item = &ViewportPresentationState> {
        self.states_by_viewport.values()
    }

    pub fn viewport_ids(&self) -> impl Iterator<Item = ViewportId> + '_ {
        self.states_by_viewport.keys().copied()
    }

    pub fn retain_viewports(&mut self, mut keep: impl FnMut(ViewportId) -> bool) {
        self.states_by_viewport
            .retain(|viewport_id, _| keep(*viewport_id));
    }

    pub fn is_empty(&self) -> bool {
        self.states_by_viewport.is_empty()
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource, Default)]
pub struct ViewportArtifactObservationResource {
    frames_by_viewport: BTreeMap<ViewportId, ArtifactObservationFrame>,
    generation: u64,
}

impl ViewportArtifactObservationResource {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn frame_for(&self, viewport_id: ViewportId) -> Option<&ArtifactObservationFrame> {
        self.frames_by_viewport.get(&viewport_id)
    }

    pub fn upsert_frame(&mut self, frame: ArtifactObservationFrame) -> bool {
        if self
            .frames_by_viewport
            .get(&frame.viewport_id)
            .is_some_and(|existing| existing == &frame)
        {
            return false;
        }
        self.frames_by_viewport.insert(frame.viewport_id, frame);
        self.bump_generation();
        true
    }

    pub fn viewport_ids(&self) -> impl Iterator<Item = ViewportId> + '_ {
        self.frames_by_viewport.keys().copied()
    }

    pub fn retain_viewports(&mut self, mut keep: impl FnMut(ViewportId) -> bool) {
        let before_len = self.frames_by_viewport.len();
        self.frames_by_viewport
            .retain(|viewport_id, _| keep(*viewport_id));
        if self.frames_by_viewport.len() != before_len {
            self.bump_generation();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.frames_by_viewport.is_empty()
    }

    fn bump_generation(&mut self) {
        self.generation = self.generation.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_one_uses_stable_single_viewport_session_id() {
        assert_eq!(MAIN_VIEWPORT_ID, ViewportId(1));
    }

    #[test]
    fn initial_registry_contains_locked_product_kind_subset() {
        let products =
            initial_product_descriptors(ExpressionDimensions::new(320, 200), RealityVersion(1));
        let kinds = products
            .iter()
            .map(|descriptor| descriptor.kind)
            .collect::<Vec<_>>();

        assert!(kinds.contains(&ExpressionProductKind::SceneColor2D));
        assert!(kinds.contains(&ExpressionProductKind::PickingIds2D));
        assert!(kinds.contains(&ExpressionProductKind::Overlay2D));
        assert!(kinds.contains(&ExpressionProductKind::ScalarField2D));
        assert!(kinds.contains(&ExpressionProductKind::Atlas2D));
        assert!(kinds.contains(&ExpressionProductKind::VolumeSlice2D));
        assert!(kinds.contains(&ExpressionProductKind::BrickmapDebug2D));
        assert!(kinds.contains(&ExpressionProductKind::HistoryColor2D));
    }

    #[test]
    fn material_preview_descriptor_is_a_selectable_material_product() {
        let descriptor = material_preview_descriptor(
            ExpressionProductId(42),
            ExpressionDimensions::new(320, 200),
            RealityVersion(7),
            "material.first_slice.render_material".to_string(),
        );

        assert_eq!(descriptor.kind, ExpressionProductKind::MaterialPreview2D);
        assert_eq!(
            descriptor.source_reality_class,
            ExpressionSourceRealityClass::DerivedMaterial
        );
        assert_eq!(descriptor.id, ExpressionProductId(42));
        assert!(descriptor.producer_label.contains("material.first_slice"));
    }

    #[test]
    fn runtime_viewport_resources_default_to_empty_before_bootstrap() {
        assert!(ViewportProductRegistryResource::default().is_empty());
        assert!(ViewportPresentationStateResource::default().is_empty());
        assert!(ViewportArtifactObservationResource::default().is_empty());
    }

    #[test]
    fn observation_generation_changes_only_when_frame_content_changes() {
        let mut observations = ViewportArtifactObservationResource::default();
        let frame = ArtifactObservationFrame::new(MAIN_VIEWPORT_ID, RealityVersion(1));

        assert_eq!(observations.generation(), 0);
        assert!(observations.upsert_frame(frame.clone()));
        assert_eq!(observations.generation(), 1);
        assert!(!observations.upsert_frame(frame));
        assert_eq!(observations.generation(), 1);

        observations.retain_viewports(|viewport_id| viewport_id != MAIN_VIEWPORT_ID);
        assert_eq!(observations.generation(), 2);
    }

    #[test]
    fn product_registry_keeps_multiple_viewports_stable() {
        let mut registry = ViewportProductRegistryResource::default();
        let viewport_a = ViewportId(1);
        let viewport_b = ViewportId(2);
        registry.update_viewport_descriptors(
            viewport_a,
            initial_product_descriptors(ExpressionDimensions::new(320, 200), RealityVersion(1)),
        );
        registry.update_viewport_descriptors(
            viewport_b,
            initial_product_descriptors(ExpressionDimensions::new(640, 360), RealityVersion(2)),
        );

        assert_eq!(
            registry
                .descriptors_for(viewport_a)
                .expect("viewport A descriptors should exist")
                .len(),
            initial_product_descriptors(ExpressionDimensions::new(320, 200), RealityVersion(1))
                .len()
        );
        assert_eq!(
            registry
                .descriptors_for(viewport_b)
                .expect("viewport B descriptors should exist")
                .len(),
            initial_product_descriptors(ExpressionDimensions::new(640, 360), RealityVersion(2))
                .len()
        );
    }
}
