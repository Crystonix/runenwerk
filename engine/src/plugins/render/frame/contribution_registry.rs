use crate::plugins::render::api::ids::RenderFeatureId;
use crate::plugins::render::features::{
    FeatureContributionStatus, FeatureFallbackPolicy, SCENE_ROUTE_RENDER_FEATURE_ID,
};
use crate::plugins::render::{
    PreparedFeatureContribution, PreparedFeatureContributionDiagnostic, PreparedFeaturePayload,
    PreparedSceneRouteContribution,
};
use std::any::{TypeId, type_name};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderFeatureContributionCollectorId(String);

impl RenderFeatureContributionCollectorId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for RenderFeatureContributionCollectorId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderFeatureContributionPayloadKind(String);

impl RenderFeatureContributionPayloadKind {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for RenderFeatureContributionPayloadKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone)]
pub struct RenderFeatureContributionResourceRequirement {
    pub type_id: TypeId,
    pub type_name: &'static str,
}

impl RenderFeatureContributionResourceRequirement {
    pub fn resource<R: runen_ecs::Resource>() -> Self {
        Self {
            type_id: TypeId::of::<R>(),
            type_name: type_name::<R>(),
        }
    }
}

impl PartialEq for RenderFeatureContributionResourceRequirement {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

impl Eq for RenderFeatureContributionResourceRequirement {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFeatureContributionCollectorDescriptor {
    pub feature_id: RenderFeatureId,
    pub collector_id: RenderFeatureContributionCollectorId,
    pub payload_kind: RenderFeatureContributionPayloadKind,
    pub required_resources: Vec<RenderFeatureContributionResourceRequirement>,
    pub fallback_policy: FeatureFallbackPolicy,
}

impl RenderFeatureContributionCollectorDescriptor {
    pub fn new(
        feature_id: RenderFeatureId,
        collector_id: impl Into<String>,
        payload_kind: impl Into<String>,
    ) -> Self {
        Self {
            feature_id,
            collector_id: RenderFeatureContributionCollectorId::new(collector_id),
            payload_kind: RenderFeatureContributionPayloadKind::new(payload_kind),
            required_resources: Vec::new(),
            fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
        }
    }

    pub fn require_resource<R: runen_ecs::Resource>(mut self) -> Self {
        self.required_resources
            .push(RenderFeatureContributionResourceRequirement::resource::<R>());
        self
    }

    pub fn with_fallback_policy(mut self, fallback_policy: FeatureFallbackPolicy) -> Self {
        self.fallback_policy = fallback_policy;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRegisteredFeaturePayloadInspection {
    pub payload_kind: String,
    pub summary: String,
    pub fields: Vec<(String, String)>,
}

pub trait PreparedRegisteredFeaturePayloadValue: fmt::Debug + Send + Sync {
    fn kind(&self) -> &RenderFeatureContributionPayloadKind;
    fn runtime_signature(&self) -> u64;
    fn inspect(&self) -> PreparedRegisteredFeaturePayloadInspection;
}

#[derive(Debug, Clone)]
pub struct PreparedRegisteredFeaturePayload {
    value: std::sync::Arc<dyn PreparedRegisteredFeaturePayloadValue>,
}

impl PreparedRegisteredFeaturePayload {
    pub fn new(value: impl PreparedRegisteredFeaturePayloadValue + 'static) -> Self {
        Self {
            value: std::sync::Arc::new(value),
        }
    }

    pub fn kind(&self) -> &RenderFeatureContributionPayloadKind {
        self.value.kind()
    }

    pub fn runtime_signature(&self) -> u64 {
        self.value.runtime_signature()
    }

    pub fn inspect(&self) -> PreparedRegisteredFeaturePayloadInspection {
        self.value.inspect()
    }
}

#[derive(Debug, Clone)]
pub struct StaticRegisteredFeaturePayload {
    kind: RenderFeatureContributionPayloadKind,
    summary: String,
    fields: Vec<(String, String)>,
}

impl StaticRegisteredFeaturePayload {
    pub fn new(kind: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            kind: RenderFeatureContributionPayloadKind::new(kind),
            summary: summary.into(),
            fields: Vec::new(),
        }
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push((key.into(), value.into()));
        self
    }
}

impl PreparedRegisteredFeaturePayloadValue for StaticRegisteredFeaturePayload {
    fn kind(&self) -> &RenderFeatureContributionPayloadKind {
        &self.kind
    }

    fn runtime_signature(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.kind.hash(&mut hasher);
        self.summary.hash(&mut hasher);
        self.fields.hash(&mut hasher);
        hasher.finish()
    }

    fn inspect(&self) -> PreparedRegisteredFeaturePayloadInspection {
        PreparedRegisteredFeaturePayloadInspection {
            payload_kind: self.kind.to_string(),
            summary: self.summary.clone(),
            fields: self.fields.clone(),
        }
    }
}

pub struct RenderFeatureContributionContext<'a> {
    world: &'a runen_ecs::World,
    descriptor: &'a RenderFeatureContributionCollectorDescriptor,
    fallback_policy: FeatureFallbackPolicy,
    scene_route: Option<&'a PreparedSceneRouteContribution>,
}

impl<'a> RenderFeatureContributionContext<'a> {
    pub fn new(
        world: &'a runen_ecs::World,
        descriptor: &'a RenderFeatureContributionCollectorDescriptor,
        fallback_policy: FeatureFallbackPolicy,
        scene_route: Option<&'a PreparedSceneRouteContribution>,
    ) -> Self {
        Self {
            world,
            descriptor,
            fallback_policy,
            scene_route,
        }
    }

    pub fn descriptor(&self) -> &RenderFeatureContributionCollectorDescriptor {
        self.descriptor
    }

    pub fn fallback_policy(&self) -> FeatureFallbackPolicy {
        self.fallback_policy
    }

    pub fn scene_route(&self) -> Option<&PreparedSceneRouteContribution> {
        self.scene_route
    }

    pub fn resource<R: runen_ecs::Resource>(&self) -> Option<&R> {
        let required_type_id = TypeId::of::<R>();
        if self
            .descriptor
            .required_resources
            .iter()
            .all(|requirement| requirement.type_id != required_type_id)
        {
            return None;
        }
        self.world.resource::<R>().ok()
    }
}

pub type RenderFeatureContributionCollectorFn =
    fn(
        &RenderFeatureContributionContext<'_>,
    ) -> Result<PreparedFeatureContribution, PreparedFeatureContributionDiagnostic>;

#[derive(Clone)]
pub struct RenderFeatureContributionCollector {
    pub descriptor: RenderFeatureContributionCollectorDescriptor,
    pub collect: RenderFeatureContributionCollectorFn,
}

impl fmt::Debug for RenderFeatureContributionCollector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RenderFeatureContributionCollector")
            .field("descriptor", &self.descriptor)
            .finish_non_exhaustive()
    }
}

impl RenderFeatureContributionCollector {
    pub fn new(
        descriptor: RenderFeatureContributionCollectorDescriptor,
        collect: RenderFeatureContributionCollectorFn,
    ) -> Self {
        Self {
            descriptor,
            collect,
        }
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderFeatureContributionCollectorRegistryResource {
    collectors: BTreeMap<RenderFeatureContributionCollectorId, RenderFeatureContributionCollector>,
    diagnostics: Vec<PreparedFeatureContributionDiagnostic>,
}

impl Default for RenderFeatureContributionCollectorRegistryResource {
    fn default() -> Self {
        let mut registry = Self {
            collectors: BTreeMap::new(),
            diagnostics: Vec::new(),
        };
        registry
            .try_register_collector(scene_route_collector())
            .expect("builtin scene route collector should register");
        registry
    }
}

impl RenderFeatureContributionCollectorRegistryResource {
    pub fn try_register_collector(
        &mut self,
        collector: RenderFeatureContributionCollector,
    ) -> Result<(), PreparedFeatureContributionDiagnostic> {
        let descriptor = &collector.descriptor;
        if self.collectors.contains_key(&descriptor.collector_id) {
            let diagnostic = PreparedFeatureContributionDiagnostic::error(
                descriptor.feature_id,
                format!(
                    "duplicate render feature contribution collector '{}'",
                    descriptor.collector_id
                ),
            )
            .with_collector_id(descriptor.collector_id.clone())
            .with_payload_kind(descriptor.payload_kind.clone());
            self.diagnostics.push(diagnostic.clone());
            return Err(diagnostic);
        }

        if self.collectors.values().any(|existing| {
            existing.descriptor.feature_id == descriptor.feature_id
                && existing.descriptor.payload_kind == descriptor.payload_kind
        }) {
            let diagnostic = PreparedFeatureContributionDiagnostic::error(
                descriptor.feature_id,
                format!(
                    "duplicate render feature contribution payload kind '{}' for feature {:?}",
                    descriptor.payload_kind, descriptor.feature_id
                ),
            )
            .with_collector_id(descriptor.collector_id.clone())
            .with_payload_kind(descriptor.payload_kind.clone());
            self.diagnostics.push(diagnostic.clone());
            return Err(diagnostic);
        }

        self.collectors
            .insert(descriptor.collector_id.clone(), collector);
        Ok(())
    }

    pub fn collectors(&self) -> impl Iterator<Item = &RenderFeatureContributionCollector> {
        self.collectors.values()
    }

    pub fn diagnostics(&self) -> &[PreparedFeatureContributionDiagnostic] {
        &self.diagnostics
    }
}

pub fn validate_collector_resources(
    world: &runen_ecs::World,
    descriptor: &RenderFeatureContributionCollectorDescriptor,
) -> Result<(), PreparedFeatureContributionDiagnostic> {
    let mut seen = BTreeSet::<TypeId>::new();
    for requirement in &descriptor.required_resources {
        if !seen.insert(requirement.type_id) {
            return Err(PreparedFeatureContributionDiagnostic::error(
                descriptor.feature_id,
                format!(
                    "collector '{}' declares duplicate required resource '{}'",
                    descriptor.collector_id, requirement.type_name
                ),
            )
            .with_collector_id(descriptor.collector_id.clone())
            .with_payload_kind(descriptor.payload_kind.clone())
            .with_resource_type_name(requirement.type_name));
        }
        if world.resource_by_type_id(requirement.type_id).is_none() {
            return Err(PreparedFeatureContributionDiagnostic::error(
                descriptor.feature_id,
                format!(
                    "collector '{}' requires missing prepared resource '{}'",
                    descriptor.collector_id, requirement.type_name
                ),
            )
            .with_collector_id(descriptor.collector_id.clone())
            .with_payload_kind(descriptor.payload_kind.clone())
            .with_resource_type_name(requirement.type_name));
        }
    }
    Ok(())
}

pub fn validate_collected_contribution(
    descriptor: &RenderFeatureContributionCollectorDescriptor,
    contribution: &PreparedFeatureContribution,
) -> Result<(), PreparedFeatureContributionDiagnostic> {
    if let PreparedFeaturePayload::Registered(payload) = &contribution.payload
        && payload.kind() != &descriptor.payload_kind
    {
        return Err(PreparedFeatureContributionDiagnostic::error(
            descriptor.feature_id,
            format!(
                "collector '{}' emitted payload kind '{}' but registered '{}'",
                descriptor.collector_id,
                payload.kind(),
                descriptor.payload_kind
            ),
        )
        .with_collector_id(descriptor.collector_id.clone())
        .with_payload_kind(payload.kind().clone())
        .with_status(contribution.status));
    }
    Ok(())
}

fn scene_route_collector() -> RenderFeatureContributionCollector {
    RenderFeatureContributionCollector::new(
        RenderFeatureContributionCollectorDescriptor::new(
            SCENE_ROUTE_RENDER_FEATURE_ID,
            "scene.route.collector",
            "scene.route",
        )
        .with_fallback_policy(FeatureFallbackPolicy::EmptyContribution),
        collect_scene_route_contribution,
    )
}

fn collect_scene_route_contribution(
    context: &RenderFeatureContributionContext<'_>,
) -> Result<PreparedFeatureContribution, PreparedFeatureContributionDiagnostic> {
    let Some(scene_route) = context.scene_route().cloned() else {
        return Err(PreparedFeatureContributionDiagnostic::error(
            context.descriptor().feature_id,
            "scene route collector requires prepared scene route labels",
        )
        .with_collector_id(context.descriptor().collector_id.clone())
        .with_payload_kind(context.descriptor().payload_kind.clone()));
    };
    Ok(PreparedFeatureContribution {
        status: FeatureContributionStatus::Ready,
        fallback_policy: context.fallback_policy(),
        payload: PreparedFeaturePayload::SceneRoute(scene_route),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_feature_id(raw: u64) -> RenderFeatureId {
        RenderFeatureId::try_from_raw(raw).expect("test feature id should be non-zero")
    }

    fn noop_collector(
        context: &RenderFeatureContributionContext<'_>,
    ) -> Result<PreparedFeatureContribution, PreparedFeatureContributionDiagnostic> {
        Ok(PreparedFeatureContribution {
            status: FeatureContributionStatus::Ready,
            fallback_policy: context.fallback_policy(),
            payload: PreparedFeaturePayload::Registered(PreparedRegisteredFeaturePayload::new(
                StaticRegisteredFeaturePayload::new("test.payload", "noop"),
            )),
        })
    }

    #[test]
    fn render_feature_contributions_registry_rejects_duplicate_collector_id() {
        let feature_id = test_feature_id(20_001);
        let descriptor =
            RenderFeatureContributionCollectorDescriptor::new(feature_id, "duplicate", "first");
        let mut registry = RenderFeatureContributionCollectorRegistryResource {
            collectors: BTreeMap::new(),
            diagnostics: Vec::new(),
        };
        registry
            .try_register_collector(RenderFeatureContributionCollector::new(
                descriptor.clone(),
                noop_collector,
            ))
            .expect("first collector should register");

        let error = registry
            .try_register_collector(RenderFeatureContributionCollector::new(
                descriptor,
                noop_collector,
            ))
            .expect_err("duplicate collector id should fail");

        assert!(error.message.contains("duplicate"));
        assert_eq!(registry.diagnostics().len(), 1);
    }

    #[test]
    fn render_feature_contributions_registry_rejects_duplicate_payload_kind_for_feature() {
        let feature_id = test_feature_id(20_002);
        let mut registry = RenderFeatureContributionCollectorRegistryResource {
            collectors: BTreeMap::new(),
            diagnostics: Vec::new(),
        };
        registry
            .try_register_collector(RenderFeatureContributionCollector::new(
                RenderFeatureContributionCollectorDescriptor::new(
                    feature_id,
                    "first",
                    "shared.payload",
                ),
                noop_collector,
            ))
            .expect("first collector should register");

        let error = registry
            .try_register_collector(RenderFeatureContributionCollector::new(
                RenderFeatureContributionCollectorDescriptor::new(
                    feature_id,
                    "second",
                    "shared.payload",
                ),
                noop_collector,
            ))
            .expect_err("duplicate payload kind should fail");

        assert!(error.message.contains("duplicate"));
        assert_eq!(registry.diagnostics().len(), 1);
    }
}
