use crate::plugins::render::api::ids::RenderFeatureId;
use crate::plugins::render::frame::*;
use crate::runtime::ResMut;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub mod caves;
pub mod detail;
pub mod editor_picking;
pub mod particle_vfx;
pub mod ui;
pub mod world;

pub use editor_picking::*;
pub use particle_vfx::*;
pub use ui::*;

pub const SCENE_ROUTE_RENDER_FEATURE_LABEL: &str = "scene.route";
pub const WORLD_DRAW_RENDER_FEATURE_LABEL: &str = "world.draw";
pub const CAVE_INTERIOR_RENDER_FEATURE_LABEL: &str = "cave.interior";
pub const PROCEDURAL_WORLD_RENDER_FEATURE_LABEL: &str = "procedural.world";
pub const DETAIL_RENDER_FEATURE_LABEL: &str = "detail";
pub const MATERIAL_RENDER_FEATURE_LABEL: &str = "material";
pub const PARTICLE_VFX_RENDER_FEATURE_LABEL: &str = "particle.vfx";
pub const WORLD_VISUAL_RENDER_FEATURE_LABEL: &str = "world.visual";
pub const DEFORMATION_RENDER_FEATURE_LABEL: &str = "deformation";
pub const WIND_FIELDS_RENDER_FEATURE_LABEL: &str = "wind.fields";
pub const EDITOR_PICKING_RENDER_FEATURE_LABEL: &str = "editor.picking";

pub const SCENE_ROUTE_RENDER_FEATURE_ID: RenderFeatureId = render_feature_id(1);
pub const EDITOR_PICKING_RENDER_FEATURE_ID: RenderFeatureId = render_feature_id(2);
pub const UI_RENDER_FEATURE_ID: RenderFeatureId = render_feature_id(3);
pub const WORLD_DRAW_RENDER_FEATURE_ID: RenderFeatureId = render_feature_id(4);
pub const CAVE_INTERIOR_RENDER_FEATURE_ID: RenderFeatureId = render_feature_id(5);
pub const PROCEDURAL_WORLD_RENDER_FEATURE_ID: RenderFeatureId = render_feature_id(6);
pub const DETAIL_RENDER_FEATURE_ID: RenderFeatureId = render_feature_id(7);
pub const MATERIAL_RENDER_FEATURE_ID: RenderFeatureId = render_feature_id(8);
pub const DEFORMATION_RENDER_FEATURE_ID: RenderFeatureId = render_feature_id(9);
pub const WIND_FIELDS_RENDER_FEATURE_ID: RenderFeatureId = render_feature_id(10);
pub const PARTICLE_VFX_RENDER_FEATURE_ID: RenderFeatureId = render_feature_id(11);
pub const WORLD_VISUAL_RENDER_FEATURE_ID: RenderFeatureId = render_feature_id(12);

const fn render_feature_id(raw: u64) -> RenderFeatureId {
    match RenderFeatureId::try_from_raw(raw) {
        Ok(id) => id,
        Err(_) => panic!("render feature id constants must be non-zero"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FeatureContributionStatus {
    #[default]
    Ready,
    Stale,
    Disabled,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FeatureFallbackPolicy {
    ReuseLastGood,
    EmptyContribution,
    #[default]
    SkipFeaturePasses,
    FailFrame,
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct PreparedDrawFeatureResource {
    pub status: FeatureContributionStatus,
    pub fallback_policy: FeatureFallbackPolicy,
    pub payload: PreparedDrawFeatureContribution,
}

impl Default for PreparedDrawFeatureResource {
    fn default() -> Self {
        Self {
            status: FeatureContributionStatus::Missing,
            fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            payload: PreparedDrawFeatureContribution::default(),
        }
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct PreparedWorldFeatureResource {
    pub status: FeatureContributionStatus,
    pub fallback_policy: FeatureFallbackPolicy,
    pub payload: PreparedWorldFeatureContribution,
}

impl Default for PreparedWorldFeatureResource {
    fn default() -> Self {
        Self {
            status: FeatureContributionStatus::Missing,
            fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            payload: PreparedWorldFeatureContribution::default(),
        }
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct PreparedCaveFeatureResource {
    pub status: FeatureContributionStatus,
    pub fallback_policy: FeatureFallbackPolicy,
    pub payload: PreparedCaveFeatureContribution,
}

impl Default for PreparedCaveFeatureResource {
    fn default() -> Self {
        Self {
            status: FeatureContributionStatus::Missing,
            fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            payload: PreparedCaveFeatureContribution::default(),
        }
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct PreparedDetailFeatureResource {
    pub status: FeatureContributionStatus,
    pub fallback_policy: FeatureFallbackPolicy,
    pub payload: PreparedDetailFeatureContribution,
}

impl Default for PreparedDetailFeatureResource {
    fn default() -> Self {
        Self {
            status: FeatureContributionStatus::Missing,
            fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            payload: PreparedDetailFeatureContribution::default(),
        }
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct PreparedProceduralWorldFeatureResource {
    pub status: FeatureContributionStatus,
    pub fallback_policy: FeatureFallbackPolicy,
    pub payload: PreparedProceduralWorldFeatureContribution,
}

impl Default for PreparedProceduralWorldFeatureResource {
    fn default() -> Self {
        Self {
            status: FeatureContributionStatus::Missing,
            fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            payload: PreparedProceduralWorldFeatureContribution::default(),
        }
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct PreparedWindFieldFeatureResource {
    pub status: FeatureContributionStatus,
    pub fallback_policy: FeatureFallbackPolicy,
    pub payload: PreparedWindFieldFeatureContribution,
}

impl Default for PreparedWindFieldFeatureResource {
    fn default() -> Self {
        Self {
            status: FeatureContributionStatus::Missing,
            fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            payload: PreparedWindFieldFeatureContribution::default(),
        }
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct PreparedMaterialFeatureResource {
    pub status: FeatureContributionStatus,
    pub fallback_policy: FeatureFallbackPolicy,
    pub payload: PreparedMaterialFeatureContribution,
}

impl Default for PreparedMaterialFeatureResource {
    fn default() -> Self {
        Self {
            status: FeatureContributionStatus::Missing,
            fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            payload: PreparedMaterialFeatureContribution::default(),
        }
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct PreparedDeformationFeatureResource {
    pub status: FeatureContributionStatus,
    pub fallback_policy: FeatureFallbackPolicy,
    pub payload: PreparedDeformationFeatureContribution,
}

impl Default for PreparedDeformationFeatureResource {
    fn default() -> Self {
        Self {
            status: FeatureContributionStatus::Missing,
            fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            payload: PreparedDeformationFeatureContribution::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFeatureDescriptor {
    pub id: RenderFeatureId,
    pub label: String,
    pub depends_on: Vec<RenderFeatureId>,
    pub order_hint: i32,
    pub fallback_policy: FeatureFallbackPolicy,
}

impl RenderFeatureDescriptor {
    pub fn new(id: RenderFeatureId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            depends_on: Vec::new(),
            order_hint: 0,
            fallback_policy: FeatureFallbackPolicy::default(),
        }
    }

    pub fn depends_on(mut self, id: RenderFeatureId) -> Self {
        self.depends_on.push(id);
        self
    }

    pub fn with_order_hint(mut self, order_hint: i32) -> Self {
        self.order_hint = order_hint;
        self
    }

    pub fn with_fallback_policy(mut self, fallback_policy: FeatureFallbackPolicy) -> Self {
        self.fallback_policy = fallback_policy;
        self
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderFeatureRegistryResource {
    descriptors: BTreeMap<RenderFeatureId, RenderFeatureDescriptor>,
    resolved_order: Vec<RenderFeatureId>,
    issues: Vec<String>,
    revision: u64,
    applied_revision: u64,
}

impl Default for RenderFeatureRegistryResource {
    fn default() -> Self {
        let mut value = Self {
            descriptors: BTreeMap::new(),
            resolved_order: Vec::new(),
            issues: Vec::new(),
            revision: 0,
            applied_revision: 0,
        };
        value.register_builtin_descriptors();
        value
    }
}

impl RenderFeatureRegistryResource {
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn descriptors(&self) -> &BTreeMap<RenderFeatureId, RenderFeatureDescriptor> {
        &self.descriptors
    }

    pub fn descriptor(&self, id: &RenderFeatureId) -> Option<&RenderFeatureDescriptor> {
        self.descriptors.get(id)
    }

    pub fn descriptor_or_panic(&self, id: &RenderFeatureId) -> &RenderFeatureDescriptor {
        self.descriptors
            .get(id)
            .unwrap_or_else(|| panic!("missing render feature descriptor for id {:?}", id))
    }

    pub fn label_of(&self, id: &RenderFeatureId) -> Option<&str> {
        self.descriptor(id)
            .map(|descriptor| descriptor.label.as_str())
    }

    pub fn resolved_order(&self) -> &[RenderFeatureId] {
        &self.resolved_order
    }

    pub fn ordered_descriptors(&self) -> impl Iterator<Item = &RenderFeatureDescriptor> + '_ {
        self.resolved_order
            .iter()
            .filter_map(|id| self.descriptors.get(id))
    }

    pub fn issues(&self) -> &[String] {
        &self.issues
    }

    pub fn upsert_descriptor(&mut self, descriptor: RenderFeatureDescriptor) {
        self.descriptors.insert(descriptor.id, descriptor);
        self.revision = self.revision.saturating_add(1);
    }

    pub fn register_builtin_descriptors(&mut self) {
        self.upsert_descriptor(
            RenderFeatureDescriptor::new(
                SCENE_ROUTE_RENDER_FEATURE_ID,
                SCENE_ROUTE_RENDER_FEATURE_LABEL,
            )
            .with_order_hint(-100)
            .with_fallback_policy(FeatureFallbackPolicy::EmptyContribution),
        );
        self.upsert_descriptor(
            RenderFeatureDescriptor::new(
                EDITOR_PICKING_RENDER_FEATURE_ID,
                EDITOR_PICKING_RENDER_FEATURE_LABEL,
            )
            .depends_on(SCENE_ROUTE_RENDER_FEATURE_ID)
            .with_order_hint(-90)
            .with_fallback_policy(FeatureFallbackPolicy::ReuseLastGood),
        );
        self.upsert_descriptor(ui_render_feature_descriptor());
        self.upsert_descriptor(
            RenderFeatureDescriptor::new(
                WORLD_DRAW_RENDER_FEATURE_ID,
                WORLD_DRAW_RENDER_FEATURE_LABEL,
            )
            .depends_on(SCENE_ROUTE_RENDER_FEATURE_ID)
            .with_order_hint(10)
            .with_fallback_policy(FeatureFallbackPolicy::SkipFeaturePasses),
        );
        self.upsert_descriptor(
            RenderFeatureDescriptor::new(
                CAVE_INTERIOR_RENDER_FEATURE_ID,
                CAVE_INTERIOR_RENDER_FEATURE_LABEL,
            )
            .depends_on(WORLD_DRAW_RENDER_FEATURE_ID)
            .with_order_hint(12)
            .with_fallback_policy(FeatureFallbackPolicy::SkipFeaturePasses),
        );
        self.upsert_descriptor(
            RenderFeatureDescriptor::new(
                PROCEDURAL_WORLD_RENDER_FEATURE_ID,
                PROCEDURAL_WORLD_RENDER_FEATURE_LABEL,
            )
            .depends_on(WORLD_DRAW_RENDER_FEATURE_ID)
            .with_order_hint(14)
            .with_fallback_policy(FeatureFallbackPolicy::SkipFeaturePasses),
        );
        self.upsert_descriptor(
            RenderFeatureDescriptor::new(DETAIL_RENDER_FEATURE_ID, DETAIL_RENDER_FEATURE_LABEL)
                .depends_on(WORLD_DRAW_RENDER_FEATURE_ID)
                .with_order_hint(16)
                .with_fallback_policy(FeatureFallbackPolicy::SkipFeaturePasses),
        );
        self.upsert_descriptor(
            RenderFeatureDescriptor::new(MATERIAL_RENDER_FEATURE_ID, MATERIAL_RENDER_FEATURE_LABEL)
                .depends_on(WORLD_DRAW_RENDER_FEATURE_ID)
                .depends_on(DETAIL_RENDER_FEATURE_ID)
                .depends_on(CAVE_INTERIOR_RENDER_FEATURE_ID)
                .with_order_hint(20)
                .with_fallback_policy(FeatureFallbackPolicy::SkipFeaturePasses),
        );
        self.upsert_descriptor(particle_vfx_render_feature_descriptor());
        self.upsert_descriptor(world::world_visual_render_feature_descriptor());
        self.upsert_descriptor(
            RenderFeatureDescriptor::new(
                DEFORMATION_RENDER_FEATURE_ID,
                DEFORMATION_RENDER_FEATURE_LABEL,
            )
            .depends_on(MATERIAL_RENDER_FEATURE_ID)
            .with_order_hint(30)
            .with_fallback_policy(FeatureFallbackPolicy::SkipFeaturePasses),
        );
        self.upsert_descriptor(
            RenderFeatureDescriptor::new(
                WIND_FIELDS_RENDER_FEATURE_ID,
                WIND_FIELDS_RENDER_FEATURE_LABEL,
            )
            .depends_on(DEFORMATION_RENDER_FEATURE_ID)
            .with_order_hint(40)
            .with_fallback_policy(FeatureFallbackPolicy::SkipFeaturePasses),
        );
    }

    pub fn sync_order(&mut self) {
        if self.applied_revision == self.revision {
            return;
        }

        let (resolved_order, issues) = resolve_feature_order(&self.descriptors);
        self.resolved_order = resolved_order;
        self.issues = issues;
        self.applied_revision = self.revision;
    }
}

pub(crate) fn sync_render_feature_registry_system(
    mut feature_registry: ResMut<RenderFeatureRegistryResource>,
) {
    feature_registry.sync_order();
}

fn resolve_feature_order(
    descriptors: &BTreeMap<RenderFeatureId, RenderFeatureDescriptor>,
) -> (Vec<RenderFeatureId>, Vec<String>) {
    let mut issues = Vec::<String>::new();
    let mut indegree = BTreeMap::<RenderFeatureId, usize>::new();
    let mut edges = BTreeMap::<RenderFeatureId, BTreeSet<RenderFeatureId>>::new();

    for id in descriptors.keys().copied() {
        indegree.insert(id, 0);
        edges.insert(id, BTreeSet::new());
    }

    for (id, descriptor) in descriptors {
        for dependency in &descriptor.depends_on {
            if !descriptors.contains_key(dependency) {
                issues.push(format!(
                    "render feature '{}' depends on missing feature '{:?}'",
                    descriptor.label, dependency
                ));
                continue;
            }
            let outgoing = edges
                .get_mut(dependency)
                .expect("dependency key should exist in edge graph");
            if outgoing.insert(*id) {
                let value = indegree
                    .get_mut(id)
                    .expect("feature should exist in indegree map");
                *value = value.saturating_add(1);
            }
        }
    }

    let mut queue = VecDeque::<RenderFeatureId>::new();
    let mut ready = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    ready.sort_by_key(|id| {
        let descriptor = descriptors
            .get(id)
            .expect("feature should exist for ordering");
        (descriptor.order_hint, descriptor.label.clone())
    });
    for id in ready {
        queue.push_back(id);
    }

    let mut order = Vec::<RenderFeatureId>::with_capacity(descriptors.len());

    while let Some(next) = queue.pop_front() {
        order.push(next);
        let Some(outgoing) = edges.get(&next) else {
            continue;
        };
        for target in outgoing {
            let degree = indegree
                .get_mut(target)
                .expect("target feature should exist in indegree map");
            *degree = degree.saturating_sub(1);
            if *degree == 0 {
                queue.push_back(*target);
                let mut staged = queue.drain(..).collect::<Vec<_>>();
                staged.sort_by_key(|id| {
                    let descriptor = descriptors
                        .get(id)
                        .expect("feature should exist for ordering");
                    (descriptor.order_hint, descriptor.label.clone())
                });
                for id in staged {
                    queue.push_back(id);
                }
            }
        }
    }

    if order.len() != descriptors.len() {
        let unresolved = indegree
            .into_iter()
            .filter(|(_, degree)| *degree > 0)
            .map(|(id, _)| {
                descriptors
                    .get(&id)
                    .map(|descriptor| descriptor.label.clone())
                    .unwrap_or_else(|| format!("{id:?}"))
            })
            .collect::<Vec<_>>();
        issues.push(format!(
            "render feature dependency cycle detected among: {}",
            unresolved.join(", ")
        ));
    }

    (order, issues)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_resolves_builtin_feature_order() {
        let mut registry = RenderFeatureRegistryResource::default();
        registry.sync_order();
        let ordered = registry
            .ordered_descriptors()
            .map(|descriptor| descriptor.label.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            ordered,
            vec![
                "scene.route".to_string(),
                "editor.picking".to_string(),
                "ui".to_string(),
                "world.draw".to_string(),
                "cave.interior".to_string(),
                "procedural.world".to_string(),
                "detail".to_string(),
                "material".to_string(),
                "particle.vfx".to_string(),
                "world.visual".to_string(),
                "deformation".to_string(),
                "wind.fields".to_string(),
            ]
        );
        assert!(registry.issues().is_empty());
    }

    #[test]
    fn cycle_is_reported_as_issue() {
        const A_ID: RenderFeatureId = render_feature_id(1001);
        const B_ID: RenderFeatureId = render_feature_id(1002);

        let mut registry = RenderFeatureRegistryResource {
            descriptors: BTreeMap::new(),
            resolved_order: Vec::new(),
            issues: Vec::new(),
            revision: 0,
            applied_revision: 0,
        };
        registry.upsert_descriptor(
            RenderFeatureDescriptor::new(A_ID, "a")
                .depends_on(B_ID)
                .with_order_hint(0),
        );
        registry.upsert_descriptor(
            RenderFeatureDescriptor::new(B_ID, "b")
                .depends_on(A_ID)
                .with_order_hint(0),
        );
        registry.sync_order();
        assert!(
            registry
                .issues()
                .iter()
                .any(|issue| issue.contains("dependency cycle")),
            "expected cycle issue, got {:?}",
            registry.issues()
        );
    }
}
