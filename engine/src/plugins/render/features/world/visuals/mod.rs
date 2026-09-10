use crate::plugins::render::{
    FeatureContributionStatus, FeatureFallbackPolicy, PreparedFeatureContribution,
    PreparedFeatureContributionDiagnostic, PreparedFeaturePayload,
    PreparedRegisteredFeaturePayload, PreparedRegisteredFeaturePayloadInspection,
    PreparedRegisteredFeaturePayloadValue, RenderFeatureContributionCollector,
    RenderFeatureContributionCollectorDescriptor,
    RenderFeatureContributionCollectorRegistryResource, RenderFeatureContributionContext,
    RenderFeatureContributionPayloadKind, RenderFeatureDescriptor,
};
use product::{ProductScaleBand, RenderResidencyRequest};
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

use crate::plugins::render::features::{
    MATERIAL_RENDER_FEATURE_ID, WORLD_DRAW_RENDER_FEATURE_ID, WORLD_VISUAL_RENDER_FEATURE_ID,
    WORLD_VISUAL_RENDER_FEATURE_LABEL,
};

pub const WORLD_VISUAL_PAYLOAD_KIND: &str = "world.visual.prepared";
pub const WORLD_VISUAL_COLLECTOR_ID: &str = "world.visual.collector";

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct PreparedWorldVisualFeatureResource {
    pub status: FeatureContributionStatus,
    pub fallback_policy: FeatureFallbackPolicy,
    pub payload: PreparedWorldVisualFeatureContribution,
}

impl Default for PreparedWorldVisualFeatureResource {
    fn default() -> Self {
        Self {
            status: FeatureContributionStatus::Missing,
            fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            payload: PreparedWorldVisualFeatureContribution::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct PreparedWorldVisualFeatureContribution {
    pub batches: Vec<PreparedWorldVisualBatch>,
    pub residency_requests: Vec<RenderResidencyRequest>,
}

impl PreparedWorldVisualFeatureContribution {
    pub fn batch_count(&self) -> usize {
        self.batches.len()
    }

    pub fn addressable_count(&self) -> u32 {
        self.batches
            .iter()
            .map(|batch| batch.working_set.addressable_count)
            .sum()
    }

    pub fn resident_count(&self) -> u32 {
        self.batches
            .iter()
            .map(|batch| batch.working_set.resident_count)
            .sum()
    }

    pub fn visible_count(&self) -> u32 {
        self.batches
            .iter()
            .map(|batch| batch.working_set.visible_count)
            .sum()
    }

    pub fn submitted_count(&self) -> u32 {
        self.batches
            .iter()
            .map(|batch| batch.working_set.submitted_count)
            .sum()
    }

    pub fn temporal_input_count(&self) -> usize {
        self.batches
            .iter()
            .map(|batch| batch.temporal_inputs.len())
            .sum()
    }

    pub fn fallback_batch_count(&self) -> usize {
        self.batch_state_count(PreparedWorldVisualBatchState::Fallback)
    }

    pub fn over_budget_batch_count(&self) -> usize {
        self.batch_state_count(PreparedWorldVisualBatchState::OverBudget)
    }

    pub fn unsupported_batch_count(&self) -> usize {
        self.batch_state_count(PreparedWorldVisualBatchState::Unsupported)
    }

    fn batch_state_count(&self, state: PreparedWorldVisualBatchState) -> usize {
        self.batches
            .iter()
            .filter(|batch| batch.state == state)
            .count()
    }

    fn inspection_fields(&self) -> Vec<(String, String)> {
        vec![
            ("batch_count".to_string(), self.batch_count().to_string()),
            (
                "addressable_count".to_string(),
                self.addressable_count().to_string(),
            ),
            (
                "resident_count".to_string(),
                self.resident_count().to_string(),
            ),
            (
                "visible_count".to_string(),
                self.visible_count().to_string(),
            ),
            (
                "submitted_count".to_string(),
                self.submitted_count().to_string(),
            ),
            (
                "residency_request_count".to_string(),
                self.residency_requests.len().to_string(),
            ),
            (
                "temporal_input_count".to_string(),
                self.temporal_input_count().to_string(),
            ),
            (
                "fallback_batch_count".to_string(),
                self.fallback_batch_count().to_string(),
            ),
            (
                "over_budget_batch_count".to_string(),
                self.over_budget_batch_count().to_string(),
            ),
            (
                "unsupported_batch_count".to_string(),
                self.unsupported_batch_count().to_string(),
            ),
            (
                "visual_kinds".to_string(),
                unique_labels(self.batches.iter().map(|batch| batch.visual_kind.as_str())),
            ),
            (
                "scale_bands".to_string(),
                unique_labels(
                    self.batches
                        .iter()
                        .map(|batch| scale_band_label(batch.scale_band)),
                ),
            ),
        ]
    }

    fn summary(&self) -> String {
        format!(
            "world_visual batches={} addressable={} resident={} visible={} submitted={} residency_requests={} temporal_inputs={} fallback={} over_budget={} unsupported={}",
            self.batch_count(),
            self.addressable_count(),
            self.resident_count(),
            self.visible_count(),
            self.submitted_count(),
            self.residency_requests.len(),
            self.temporal_input_count(),
            self.fallback_batch_count(),
            self.over_budget_batch_count(),
            self.unsupported_batch_count()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreparedWorldVisualBatch {
    pub batch_id: String,
    pub visual_kind: PreparedWorldVisualKind,
    pub product_generation: u64,
    pub scale_band: ProductScaleBand,
    pub working_set: PreparedWorldVisualWorkingSet,
    pub temporal_inputs: Vec<PreparedWorldVisualTemporalInput>,
    pub state: PreparedWorldVisualBatchState,
}

impl PreparedWorldVisualBatch {
    pub fn new(
        batch_id: impl Into<String>,
        visual_kind: PreparedWorldVisualKind,
        scale_band: ProductScaleBand,
        working_set: PreparedWorldVisualWorkingSet,
    ) -> Self {
        Self {
            batch_id: batch_id.into(),
            visual_kind,
            product_generation: 0,
            scale_band,
            working_set,
            temporal_inputs: Vec::new(),
            state: PreparedWorldVisualBatchState::Ready,
        }
    }

    pub fn with_product_generation(mut self, product_generation: u64) -> Self {
        self.product_generation = product_generation;
        self
    }

    pub fn with_temporal_input(mut self, temporal_input: PreparedWorldVisualTemporalInput) -> Self {
        if !self.temporal_inputs.contains(&temporal_input) {
            self.temporal_inputs.push(temporal_input);
        }
        self
    }

    pub fn with_state(mut self, state: PreparedWorldVisualBatchState) -> Self {
        self.state = state;
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PreparedWorldVisualWorkingSet {
    pub addressable_count: u32,
    pub resident_count: u32,
    pub visible_count: u32,
    pub submitted_count: u32,
}

impl PreparedWorldVisualWorkingSet {
    pub const fn new(
        addressable_count: u32,
        resident_count: u32,
        visible_count: u32,
        submitted_count: u32,
    ) -> Self {
        Self {
            addressable_count,
            resident_count,
            visible_count,
            submitted_count,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum PreparedWorldVisualKind {
    #[default]
    Vegetation,
    Grass,
    Water,
    Wetness,
    Atmosphere,
    Weather,
    FieldSummary,
}

impl PreparedWorldVisualKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vegetation => "vegetation",
            Self::Grass => "grass",
            Self::Water => "water",
            Self::Wetness => "wetness",
            Self::Atmosphere => "atmosphere",
            Self::Weather => "weather",
            Self::FieldSummary => "field_summary",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PreparedWorldVisualTemporalInput {
    MotionVectors,
    Depth,
    Exposure,
    ReactiveMask,
    WeatherMask,
    HistorySignature,
}

impl PreparedWorldVisualTemporalInput {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MotionVectors => "motion_vectors",
            Self::Depth => "depth",
            Self::Exposure => "exposure",
            Self::ReactiveMask => "reactive_mask",
            Self::WeatherMask => "weather_mask",
            Self::HistorySignature => "history_signature",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum PreparedWorldVisualBatchState {
    #[default]
    Ready,
    Fallback,
    Unsupported,
    OverBudget,
}

impl PreparedWorldVisualBatchState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Fallback => "fallback",
            Self::Unsupported => "unsupported",
            Self::OverBudget => "over_budget",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreparedWorldVisualRegisteredPayload {
    contribution: PreparedWorldVisualFeatureContribution,
    kind: RenderFeatureContributionPayloadKind,
}

impl PreparedWorldVisualRegisteredPayload {
    pub fn new(contribution: PreparedWorldVisualFeatureContribution) -> Self {
        Self {
            contribution,
            kind: RenderFeatureContributionPayloadKind::new(WORLD_VISUAL_PAYLOAD_KIND),
        }
    }
}

impl PreparedRegisteredFeaturePayloadValue for PreparedWorldVisualRegisteredPayload {
    fn kind(&self) -> &RenderFeatureContributionPayloadKind {
        &self.kind
    }

    fn runtime_signature(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.contribution.hash(&mut hasher);
        hasher.finish()
    }

    fn inspect(&self) -> PreparedRegisteredFeaturePayloadInspection {
        PreparedRegisteredFeaturePayloadInspection {
            payload_kind: self.kind.to_string(),
            summary: self.contribution.summary(),
            fields: self.contribution.inspection_fields(),
        }
    }
}

pub fn world_visual_render_feature_descriptor() -> RenderFeatureDescriptor {
    RenderFeatureDescriptor::new(
        WORLD_VISUAL_RENDER_FEATURE_ID,
        WORLD_VISUAL_RENDER_FEATURE_LABEL,
    )
    .depends_on(WORLD_DRAW_RENDER_FEATURE_ID)
    .depends_on(MATERIAL_RENDER_FEATURE_ID)
    .with_order_hint(26)
    .with_fallback_policy(FeatureFallbackPolicy::SkipFeaturePasses)
}

pub fn world_visual_feature_collector() -> RenderFeatureContributionCollector {
    RenderFeatureContributionCollector::new(
        RenderFeatureContributionCollectorDescriptor::new(
            WORLD_VISUAL_RENDER_FEATURE_ID,
            WORLD_VISUAL_COLLECTOR_ID,
            WORLD_VISUAL_PAYLOAD_KIND,
        )
        .require_resource::<PreparedWorldVisualFeatureResource>(),
        collect_world_visual_feature_contribution,
    )
}

pub fn register_world_visual_feature_collector(
    registry: &mut RenderFeatureContributionCollectorRegistryResource,
) -> Result<(), PreparedFeatureContributionDiagnostic> {
    registry.try_register_collector(world_visual_feature_collector())
}

fn collect_world_visual_feature_contribution(
    context: &RenderFeatureContributionContext<'_>,
) -> Result<PreparedFeatureContribution, PreparedFeatureContributionDiagnostic> {
    let Some(resource) = context.resource::<PreparedWorldVisualFeatureResource>() else {
        return Err(PreparedFeatureContributionDiagnostic::error(
            context.descriptor().feature_id,
            "world visual collector requires PreparedWorldVisualFeatureResource",
        )
        .with_collector_id(context.descriptor().collector_id.clone())
        .with_payload_kind(context.descriptor().payload_kind.clone()));
    };

    Ok(PreparedFeatureContribution {
        status: resource.status,
        fallback_policy: context.fallback_policy(),
        payload: PreparedFeaturePayload::Registered(PreparedRegisteredFeaturePayload::new(
            PreparedWorldVisualRegisteredPayload::new(resource.payload.clone()),
        )),
    })
}

fn scale_band_label(scale_band: ProductScaleBand) -> &'static str {
    match scale_band {
        ProductScaleBand::Near => "near",
        ProductScaleBand::Mid => "mid",
        ProductScaleBand::Far => "far",
        ProductScaleBand::Summary => "summary",
        ProductScaleBand::Preview => "preview",
        ProductScaleBand::Final => "final",
        ProductScaleBand::CollisionStrictQuery => "collision_strict_query",
        ProductScaleBand::Offline => "offline",
        ProductScaleBand::FamilySpecific => "family_specific",
    }
}

fn unique_labels<'a>(values: impl Iterator<Item = &'a str>) -> String {
    values
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use product::{ProductIdentity, ProductResidency};

    fn collect(
        resource: Option<PreparedWorldVisualFeatureResource>,
        fallback_policy: FeatureFallbackPolicy,
    ) -> Result<PreparedFeatureContribution, PreparedFeatureContributionDiagnostic> {
        let mut world = runen_ecs::World::default();
        if let Some(resource) = resource {
            world.insert_resource(resource);
        }
        let collector = world_visual_feature_collector();
        let context = RenderFeatureContributionContext::new(
            &world,
            &collector.descriptor,
            fallback_policy,
            None,
        );
        (collector.collect)(&context)
    }

    fn inspect(
        contribution: &PreparedFeatureContribution,
    ) -> PreparedRegisteredFeaturePayloadInspection {
        let PreparedFeaturePayload::Registered(payload) = &contribution.payload else {
            panic!("world visual contribution should use registered payload");
        };
        payload.inspect()
    }

    #[test]
    fn render_product_visual_world_ready_payload_reports_scale_residency_and_temporal_counts() {
        let contribution = collect(
            Some(PreparedWorldVisualFeatureResource {
                status: FeatureContributionStatus::Ready,
                fallback_policy: FeatureFallbackPolicy::ReuseLastGood,
                payload: PreparedWorldVisualFeatureContribution {
                    batches: vec![
                        PreparedWorldVisualBatch::new(
                            "forest_near",
                            PreparedWorldVisualKind::Vegetation,
                            ProductScaleBand::Near,
                            PreparedWorldVisualWorkingSet::new(50_000, 2048, 512, 64),
                        )
                        .with_product_generation(7)
                        .with_temporal_input(PreparedWorldVisualTemporalInput::MotionVectors)
                        .with_temporal_input(PreparedWorldVisualTemporalInput::ReactiveMask),
                        PreparedWorldVisualBatch::new(
                            "storm_front",
                            PreparedWorldVisualKind::Weather,
                            ProductScaleBand::Summary,
                            PreparedWorldVisualWorkingSet::new(12, 4, 3, 1),
                        )
                        .with_product_generation(3)
                        .with_temporal_input(PreparedWorldVisualTemporalInput::Exposure)
                        .with_temporal_input(PreparedWorldVisualTemporalInput::WeatherMask),
                    ],
                    residency_requests: vec![RenderResidencyRequest::new(
                        ProductIdentity::new(77),
                        ProductResidency::Resident,
                        18,
                        false,
                    )],
                },
            }),
            FeatureFallbackPolicy::ReuseLastGood,
        )
        .expect("world visual collector should produce a contribution");

        assert_eq!(contribution.status, FeatureContributionStatus::Ready);
        assert_eq!(
            contribution.fallback_policy,
            FeatureFallbackPolicy::ReuseLastGood
        );
        let inspection = inspect(&contribution);
        assert_eq!(inspection.payload_kind, WORLD_VISUAL_PAYLOAD_KIND);
        assert!(inspection.summary.contains("batches=2"));
        assert!(inspection.summary.contains("addressable=50012"));
        assert!(inspection.summary.contains("submitted=65"));
        assert!(
            inspection
                .fields
                .contains(&("residency_request_count".to_string(), "1".to_string()))
        );
        assert!(
            inspection
                .fields
                .contains(&("temporal_input_count".to_string(), "4".to_string()))
        );
        assert!(
            inspection
                .fields
                .contains(&("scale_bands".to_string(), "near,summary".to_string()))
        );
        assert!(
            inspection
                .fields
                .contains(&("visual_kinds".to_string(), "vegetation,weather".to_string()))
        );
    }

    #[test]
    fn render_product_visual_world_missing_resource_is_typed_diagnostic() {
        let diagnostic = collect(None, FeatureFallbackPolicy::SkipFeaturePasses)
            .expect_err("missing prepared resource should fail closed");

        assert_eq!(diagnostic.status, FeatureContributionStatus::Missing);
        assert_eq!(
            diagnostic.collector_id.as_ref().map(|id| id.as_str()),
            Some(WORLD_VISUAL_COLLECTOR_ID)
        );
        assert_eq!(
            diagnostic.payload_kind.as_ref().map(|kind| kind.as_str()),
            Some(WORLD_VISUAL_PAYLOAD_KIND)
        );
        assert!(
            diagnostic
                .message
                .contains("PreparedWorldVisualFeatureResource")
        );
    }

    #[test]
    fn render_product_visual_world_fallback_budget_and_unsupported_states_remain_visible() {
        let contribution = collect(
            Some(PreparedWorldVisualFeatureResource {
                status: FeatureContributionStatus::Stale,
                fallback_policy: FeatureFallbackPolicy::ReuseLastGood,
                payload: PreparedWorldVisualFeatureContribution {
                    batches: vec![
                        PreparedWorldVisualBatch::new(
                            "cached_grass",
                            PreparedWorldVisualKind::Grass,
                            ProductScaleBand::Far,
                            PreparedWorldVisualWorkingSet::new(20_000, 1000, 150, 12),
                        )
                        .with_state(PreparedWorldVisualBatchState::Fallback),
                        PreparedWorldVisualBatch::new(
                            "river_surface",
                            PreparedWorldVisualKind::Water,
                            ProductScaleBand::Mid,
                            PreparedWorldVisualWorkingSet::new(4096, 4096, 2048, 512),
                        )
                        .with_state(PreparedWorldVisualBatchState::OverBudget),
                        PreparedWorldVisualBatch::new(
                            "unsupported_atmosphere",
                            PreparedWorldVisualKind::Atmosphere,
                            ProductScaleBand::Summary,
                            PreparedWorldVisualWorkingSet::new(1, 1, 1, 0),
                        )
                        .with_state(PreparedWorldVisualBatchState::Unsupported),
                    ],
                    residency_requests: Vec::new(),
                },
            }),
            FeatureFallbackPolicy::ReuseLastGood,
        )
        .expect("stale world visual payload should still inspect");

        assert_eq!(contribution.status, FeatureContributionStatus::Stale);
        let inspection = inspect(&contribution);
        assert!(
            inspection
                .fields
                .contains(&("fallback_batch_count".to_string(), "1".to_string()))
        );
        assert!(
            inspection
                .fields
                .contains(&("over_budget_batch_count".to_string(), "1".to_string()))
        );
        assert!(
            inspection
                .fields
                .contains(&("unsupported_batch_count".to_string(), "1".to_string()))
        );
    }
}
