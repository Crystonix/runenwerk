use crate::plugins::render::{
    FeatureContributionStatus, FeatureFallbackPolicy, PreparedFeatureContribution,
    PreparedFeatureContributionDiagnostic, PreparedFeaturePayload,
    PreparedRegisteredFeaturePayload, PreparedRegisteredFeaturePayloadInspection,
    PreparedRegisteredFeaturePayloadValue, RenderFeatureContributionCollector,
    RenderFeatureContributionCollectorDescriptor,
    RenderFeatureContributionCollectorRegistryResource, RenderFeatureContributionContext,
    RenderFeatureContributionPayloadKind, RenderFeatureDescriptor,
};
use product::RenderResidencyRequest;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

use super::{
    MATERIAL_RENDER_FEATURE_ID, PARTICLE_VFX_RENDER_FEATURE_ID, PARTICLE_VFX_RENDER_FEATURE_LABEL,
    WORLD_DRAW_RENDER_FEATURE_ID,
};

pub const PARTICLE_VFX_PAYLOAD_KIND: &str = "particle.vfx.prepared";
pub const PARTICLE_VFX_COLLECTOR_ID: &str = "particle.vfx.collector";

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct PreparedParticleVfxFeatureResource {
    pub status: FeatureContributionStatus,
    pub fallback_policy: FeatureFallbackPolicy,
    pub payload: PreparedParticleVfxFeatureContribution,
}

impl Default for PreparedParticleVfxFeatureResource {
    fn default() -> Self {
        Self {
            status: FeatureContributionStatus::Missing,
            fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            payload: PreparedParticleVfxFeatureContribution::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct PreparedParticleVfxFeatureContribution {
    pub batches: Vec<PreparedParticleVfxBatch>,
    pub residency_requests: Vec<RenderResidencyRequest>,
}

impl PreparedParticleVfxFeatureContribution {
    pub fn batch_count(&self) -> usize {
        self.batches.len()
    }

    pub fn instance_count(&self) -> u32 {
        self.batches.iter().map(|batch| batch.instance_count).sum()
    }

    pub fn temporal_input_count(&self) -> usize {
        self.batches
            .iter()
            .map(|batch| batch.temporal_inputs.len())
            .sum()
    }

    pub fn fallback_batch_count(&self) -> usize {
        self.batch_state_count(PreparedParticleVfxBatchState::Fallback)
    }

    pub fn over_budget_batch_count(&self) -> usize {
        self.batch_state_count(PreparedParticleVfxBatchState::OverBudget)
    }

    pub fn unsupported_batch_count(&self) -> usize {
        self.batch_state_count(PreparedParticleVfxBatchState::Unsupported)
    }

    fn batch_state_count(&self, state: PreparedParticleVfxBatchState) -> usize {
        self.batches
            .iter()
            .filter(|batch| batch.state == state)
            .count()
    }

    fn inspection_fields(&self) -> Vec<(String, String)> {
        vec![
            ("batch_count".to_string(), self.batch_count().to_string()),
            (
                "instance_count".to_string(),
                self.instance_count().to_string(),
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
                "sorting_modes".to_string(),
                unique_labels(self.batches.iter().map(|batch| batch.sorting.as_str())),
            ),
            (
                "transparency_modes".to_string(),
                unique_labels(self.batches.iter().map(|batch| batch.transparency.as_str())),
            ),
        ]
    }

    fn summary(&self) -> String {
        format!(
            "particle_vfx batches={} instances={} residency_requests={} temporal_inputs={} fallback={} over_budget={} unsupported={}",
            self.batch_count(),
            self.instance_count(),
            self.residency_requests.len(),
            self.temporal_input_count(),
            self.fallback_batch_count(),
            self.over_budget_batch_count(),
            self.unsupported_batch_count()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreparedParticleVfxBatch {
    pub batch_id: String,
    pub visual_kind: PreparedParticleVfxVisualKind,
    pub source_revision: u64,
    pub instance_count: u32,
    pub sorting: PreparedParticleVfxSortingMode,
    pub transparency: PreparedParticleVfxTransparencyMode,
    pub temporal_inputs: Vec<PreparedParticleVfxTemporalInput>,
    pub state: PreparedParticleVfxBatchState,
}

impl PreparedParticleVfxBatch {
    pub fn new(
        batch_id: impl Into<String>,
        visual_kind: PreparedParticleVfxVisualKind,
        instance_count: u32,
    ) -> Self {
        Self {
            batch_id: batch_id.into(),
            visual_kind,
            source_revision: 0,
            instance_count,
            sorting: PreparedParticleVfxSortingMode::default(),
            transparency: PreparedParticleVfxTransparencyMode::default(),
            temporal_inputs: Vec::new(),
            state: PreparedParticleVfxBatchState::Ready,
        }
    }

    pub fn with_source_revision(mut self, source_revision: u64) -> Self {
        self.source_revision = source_revision;
        self
    }

    pub fn with_sorting(mut self, sorting: PreparedParticleVfxSortingMode) -> Self {
        self.sorting = sorting;
        self
    }

    pub fn with_transparency(mut self, transparency: PreparedParticleVfxTransparencyMode) -> Self {
        self.transparency = transparency;
        self
    }

    pub fn with_temporal_input(mut self, temporal_input: PreparedParticleVfxTemporalInput) -> Self {
        if !self.temporal_inputs.contains(&temporal_input) {
            self.temporal_inputs.push(temporal_input);
        }
        self
    }

    pub fn with_state(mut self, state: PreparedParticleVfxBatchState) -> Self {
        self.state = state;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum PreparedParticleVfxVisualKind {
    #[default]
    Particle,
    Vfx,
    Trail,
    Decal,
}

impl PreparedParticleVfxVisualKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Particle => "particle",
            Self::Vfx => "vfx",
            Self::Trail => "trail",
            Self::Decal => "decal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum PreparedParticleVfxSortingMode {
    #[default]
    None,
    BackToFront,
    FrontToBack,
    StableKey,
}

impl PreparedParticleVfxSortingMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::BackToFront => "back_to_front",
            Self::FrontToBack => "front_to_back",
            Self::StableKey => "stable_key",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum PreparedParticleVfxTransparencyMode {
    Opaque,
    #[default]
    AlphaBlend,
    Additive,
    PremultipliedAlpha,
}

impl PreparedParticleVfxTransparencyMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Opaque => "opaque",
            Self::AlphaBlend => "alpha_blend",
            Self::Additive => "additive",
            Self::PremultipliedAlpha => "premultiplied_alpha",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PreparedParticleVfxTemporalInput {
    MotionVectors,
    ReactiveMask,
    Depth,
    Exposure,
    HistorySignature,
}

impl PreparedParticleVfxTemporalInput {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MotionVectors => "motion_vectors",
            Self::ReactiveMask => "reactive_mask",
            Self::Depth => "depth",
            Self::Exposure => "exposure",
            Self::HistorySignature => "history_signature",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum PreparedParticleVfxBatchState {
    #[default]
    Ready,
    Fallback,
    Unsupported,
    OverBudget,
}

impl PreparedParticleVfxBatchState {
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
pub struct PreparedParticleVfxRegisteredPayload {
    contribution: PreparedParticleVfxFeatureContribution,
    kind: RenderFeatureContributionPayloadKind,
}

impl PreparedParticleVfxRegisteredPayload {
    pub fn new(contribution: PreparedParticleVfxFeatureContribution) -> Self {
        Self {
            contribution,
            kind: RenderFeatureContributionPayloadKind::new(PARTICLE_VFX_PAYLOAD_KIND),
        }
    }
}

impl PreparedRegisteredFeaturePayloadValue for PreparedParticleVfxRegisteredPayload {
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

pub fn particle_vfx_render_feature_descriptor() -> RenderFeatureDescriptor {
    RenderFeatureDescriptor::new(
        PARTICLE_VFX_RENDER_FEATURE_ID,
        PARTICLE_VFX_RENDER_FEATURE_LABEL,
    )
    .depends_on(WORLD_DRAW_RENDER_FEATURE_ID)
    .depends_on(MATERIAL_RENDER_FEATURE_ID)
    .with_order_hint(24)
    .with_fallback_policy(FeatureFallbackPolicy::SkipFeaturePasses)
}

pub fn particle_vfx_feature_collector() -> RenderFeatureContributionCollector {
    RenderFeatureContributionCollector::new(
        RenderFeatureContributionCollectorDescriptor::new(
            PARTICLE_VFX_RENDER_FEATURE_ID,
            PARTICLE_VFX_COLLECTOR_ID,
            PARTICLE_VFX_PAYLOAD_KIND,
        )
        .require_resource::<PreparedParticleVfxFeatureResource>(),
        collect_particle_vfx_feature_contribution,
    )
}

pub fn register_particle_vfx_feature_collector(
    registry: &mut RenderFeatureContributionCollectorRegistryResource,
) -> Result<(), PreparedFeatureContributionDiagnostic> {
    registry.try_register_collector(particle_vfx_feature_collector())
}

fn collect_particle_vfx_feature_contribution(
    context: &RenderFeatureContributionContext<'_>,
) -> Result<PreparedFeatureContribution, PreparedFeatureContributionDiagnostic> {
    let Some(resource) = context.resource::<PreparedParticleVfxFeatureResource>() else {
        return Err(PreparedFeatureContributionDiagnostic::error(
            context.descriptor().feature_id,
            "particle/VFX collector requires PreparedParticleVfxFeatureResource",
        )
        .with_collector_id(context.descriptor().collector_id.clone())
        .with_payload_kind(context.descriptor().payload_kind.clone()));
    };

    Ok(PreparedFeatureContribution {
        status: resource.status,
        fallback_policy: context.fallback_policy(),
        payload: PreparedFeaturePayload::Registered(PreparedRegisteredFeaturePayload::new(
            PreparedParticleVfxRegisteredPayload::new(resource.payload.clone()),
        )),
    })
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
        resource: Option<PreparedParticleVfxFeatureResource>,
        fallback_policy: FeatureFallbackPolicy,
    ) -> Result<PreparedFeatureContribution, PreparedFeatureContributionDiagnostic> {
        let mut world = runen_ecs::World::default();
        if let Some(resource) = resource {
            world.insert_resource(resource);
        }
        let collector = particle_vfx_feature_collector();
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
            panic!("particle/VFX contribution should use registered payload");
        };
        payload.inspect()
    }

    #[test]
    fn render_product_visual_particles_ready_payload_reports_renderer_contract_counts() {
        let contribution = collect(
            Some(PreparedParticleVfxFeatureResource {
                status: FeatureContributionStatus::Ready,
                fallback_policy: FeatureFallbackPolicy::ReuseLastGood,
                payload: PreparedParticleVfxFeatureContribution {
                    batches: vec![
                        PreparedParticleVfxBatch::new(
                            "sparks",
                            PreparedParticleVfxVisualKind::Particle,
                            64,
                        )
                        .with_source_revision(7)
                        .with_sorting(PreparedParticleVfxSortingMode::BackToFront)
                        .with_transparency(PreparedParticleVfxTransparencyMode::Additive)
                        .with_temporal_input(PreparedParticleVfxTemporalInput::MotionVectors)
                        .with_temporal_input(PreparedParticleVfxTemporalInput::ReactiveMask),
                        PreparedParticleVfxBatch::new(
                            "impact_decal",
                            PreparedParticleVfxVisualKind::Decal,
                            4,
                        )
                        .with_sorting(PreparedParticleVfxSortingMode::StableKey)
                        .with_transparency(PreparedParticleVfxTransparencyMode::AlphaBlend)
                        .with_temporal_input(PreparedParticleVfxTemporalInput::Depth),
                    ],
                    residency_requests: vec![RenderResidencyRequest::new(
                        ProductIdentity::new(41),
                        ProductResidency::Resident,
                        12,
                        false,
                    )],
                },
            }),
            FeatureFallbackPolicy::ReuseLastGood,
        )
        .expect("particle/VFX collector should produce a contribution");

        assert_eq!(contribution.status, FeatureContributionStatus::Ready);
        assert_eq!(
            contribution.fallback_policy,
            FeatureFallbackPolicy::ReuseLastGood
        );
        let inspection = inspect(&contribution);
        assert_eq!(inspection.payload_kind, PARTICLE_VFX_PAYLOAD_KIND);
        assert!(inspection.summary.contains("batches=2"));
        assert!(inspection.summary.contains("instances=68"));
        assert!(
            inspection
                .fields
                .contains(&("residency_request_count".to_string(), "1".to_string()))
        );
        assert!(
            inspection
                .fields
                .contains(&("temporal_input_count".to_string(), "3".to_string()))
        );
        assert!(
            inspection
                .fields
                .contains(&("visual_kinds".to_string(), "decal,particle".to_string()))
        );
    }

    #[test]
    fn render_product_visual_particles_missing_resource_is_typed_diagnostic() {
        let diagnostic = collect(None, FeatureFallbackPolicy::SkipFeaturePasses)
            .expect_err("missing prepared resource should fail closed");

        assert_eq!(diagnostic.status, FeatureContributionStatus::Missing);
        assert_eq!(
            diagnostic.collector_id.as_ref().map(|id| id.as_str()),
            Some(PARTICLE_VFX_COLLECTOR_ID)
        );
        assert_eq!(
            diagnostic.payload_kind.as_ref().map(|kind| kind.as_str()),
            Some(PARTICLE_VFX_PAYLOAD_KIND)
        );
        assert!(
            diagnostic
                .message
                .contains("PreparedParticleVfxFeatureResource")
        );
    }

    #[test]
    fn render_product_visual_particles_stale_fallback_and_budget_states_remain_visible() {
        let contribution = collect(
            Some(PreparedParticleVfxFeatureResource {
                status: FeatureContributionStatus::Stale,
                fallback_policy: FeatureFallbackPolicy::ReuseLastGood,
                payload: PreparedParticleVfxFeatureContribution {
                    batches: vec![
                        PreparedParticleVfxBatch::new(
                            "cached_trail",
                            PreparedParticleVfxVisualKind::Trail,
                            9,
                        )
                        .with_state(PreparedParticleVfxBatchState::Fallback),
                        PreparedParticleVfxBatch::new(
                            "heavy_smoke",
                            PreparedParticleVfxVisualKind::Vfx,
                            4096,
                        )
                        .with_state(PreparedParticleVfxBatchState::OverBudget),
                        PreparedParticleVfxBatch::new(
                            "unsupported_decal",
                            PreparedParticleVfxVisualKind::Decal,
                            1,
                        )
                        .with_state(PreparedParticleVfxBatchState::Unsupported),
                    ],
                    residency_requests: Vec::new(),
                },
            }),
            FeatureFallbackPolicy::ReuseLastGood,
        )
        .expect("stale particle/VFX payload should still inspect");

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
