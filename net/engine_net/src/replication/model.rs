use crate::replication::interest::InterestPolicy;
use crate::replication::profile::{
    BandwidthPriority, PredictionMode, Reliability, ReplicationDirection, ReplicationProfile,
    ReplicationProfilePreset,
};
use engine_sim::NetEntityId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct Replicated;

pub trait Replicate:
    serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Sync + 'static
{
}

impl<T> Replicate for T where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Sync + 'static
{
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuthorityModel {
    Server,
    Client,
}

pub trait NetEntity: Send + Sync + 'static {}

pub trait NetComponentMetadata: Send + Sync + 'static {
    fn replication_descriptor() -> ReplicatedComponentDescriptor;
}

pub trait NetEntityMetadata: Send + Sync + 'static {
    fn replication_descriptor() -> ReplicatedEntityDescriptor;
}

pub trait NetResourceMetadata: Send + Sync + 'static {
    fn replication_descriptor() -> ReplicatedResourceDescriptor;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicationSemantics {
    pub direction: ReplicationDirection,
    pub reliability: Reliability,
    pub frequency_hz: u16,
    pub prediction: PredictionMode,
    pub priority: BandwidthPriority,
}

impl ReplicationSemantics {
    pub fn from_profile(profile: ReplicationProfilePreset) -> Self {
        let profile = ReplicationProfile::from_preset(profile);
        Self {
            direction: profile.direction,
            reliability: profile.reliability,
            frequency_hz: profile.frequency_hz,
            prediction: profile.prediction,
            priority: profile.priority,
        }
    }

    pub fn with_explicit_overrides(mut self, overrides: ReplicationSemanticsOverrides) -> Self {
        if let Some(direction) = overrides.direction {
            self.direction = direction;
        }
        if let Some(reliability) = overrides.reliability {
            self.reliability = reliability;
        }
        if let Some(frequency_hz) = overrides.frequency_hz {
            self.frequency_hz = frequency_hz;
        }
        if let Some(prediction) = overrides.prediction {
            self.prediction = prediction;
        }
        if let Some(priority) = overrides.priority {
            self.priority = priority;
        }
        self
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ReplicationSemanticsOverrides {
    pub direction: Option<ReplicationDirection>,
    pub reliability: Option<Reliability>,
    pub frequency_hz: Option<u16>,
    pub prediction: Option<PredictionMode>,
    pub priority: Option<BandwidthPriority>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicatedComponentDescriptor {
    pub component_name: String,
    pub authority: AuthorityModel,
    pub profile: ReplicationProfilePreset,
    pub interest: InterestPolicy,
    pub owner_prediction: bool,
    pub direction: ReplicationDirection,
    pub reliability: Reliability,
    pub frequency_hz: u16,
    pub prediction: PredictionMode,
    pub priority: BandwidthPriority,
    pub explicit_semantics: ReplicationSemanticsOverrides,
}

impl ReplicatedComponentDescriptor {
    pub fn new(
        component_name: impl Into<String>,
        authority: AuthorityModel,
        profile: ReplicationProfilePreset,
        interest: InterestPolicy,
        owner_prediction: bool,
        explicit_semantics: ReplicationSemanticsOverrides,
    ) -> Self {
        let semantics =
            ReplicationSemantics::from_profile(profile).with_explicit_overrides(explicit_semantics);
        Self {
            component_name: component_name.into(),
            authority,
            profile,
            interest,
            owner_prediction,
            direction: semantics.direction,
            reliability: semantics.reliability,
            frequency_hz: semantics.frequency_hz,
            prediction: semantics.prediction,
            priority: semantics.priority,
            explicit_semantics,
        }
    }

    pub fn resolved_semantics(&self) -> ReplicationSemantics {
        ReplicationSemantics {
            direction: self.direction,
            reliability: self.reliability,
            frequency_hz: self.frequency_hz,
            prediction: self.prediction,
            priority: self.priority,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicatedEntityDescriptor {
    pub entity_name: String,
    pub profile: ReplicationProfilePreset,
    pub semantics: ReplicationSemantics,
    pub explicit_semantics: ReplicationSemanticsOverrides,
}

impl ReplicatedEntityDescriptor {
    pub fn new(
        entity_name: impl Into<String>,
        profile: ReplicationProfilePreset,
        explicit_semantics: ReplicationSemanticsOverrides,
    ) -> Self {
        let semantics =
            ReplicationSemantics::from_profile(profile).with_explicit_overrides(explicit_semantics);
        Self {
            entity_name: entity_name.into(),
            profile,
            semantics,
            explicit_semantics,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicatedResourceDescriptor {
    pub resource_name: String,
    pub profile: ReplicationProfilePreset,
    pub semantics: ReplicationSemantics,
    pub explicit_semantics: ReplicationSemanticsOverrides,
}

impl ReplicatedResourceDescriptor {
    pub fn new(
        resource_name: impl Into<String>,
        profile: ReplicationProfilePreset,
        explicit_semantics: ReplicationSemanticsOverrides,
    ) -> Self {
        let semantics =
            ReplicationSemantics::from_profile(profile).with_explicit_overrides(explicit_semantics);
        Self {
            resource_name: resource_name.into(),
            profile,
            semantics,
            explicit_semantics,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReplicationRegistry {
    by_component: BTreeMap<String, ReplicatedComponentDescriptor>,
    by_entity: BTreeMap<String, ReplicatedEntityDescriptor>,
    by_resource: BTreeMap<String, ReplicatedResourceDescriptor>,
}

impl ReplicationRegistry {
    pub fn register_component_descriptor(&mut self, descriptor: ReplicatedComponentDescriptor) {
        self.by_component
            .insert(descriptor.component_name.clone(), descriptor);
    }

    pub fn register(&mut self, descriptor: ReplicatedComponentDescriptor) {
        self.register_component_descriptor(descriptor);
    }

    pub fn register_component<T: NetComponentMetadata>(&mut self) {
        self.register_component_descriptor(T::replication_descriptor());
    }

    pub fn register_entity_descriptor(&mut self, descriptor: ReplicatedEntityDescriptor) {
        self.by_entity
            .insert(descriptor.entity_name.clone(), descriptor);
    }

    pub fn register_entity<T: NetEntityMetadata>(&mut self) {
        self.register_entity_descriptor(T::replication_descriptor());
    }

    pub fn register_resource_descriptor(&mut self, descriptor: ReplicatedResourceDescriptor) {
        self.by_resource
            .insert(descriptor.resource_name.clone(), descriptor);
    }

    pub fn register_resource<T: NetResourceMetadata>(&mut self) {
        self.register_resource_descriptor(T::replication_descriptor());
    }

    pub fn descriptor(&self, component_name: &str) -> Option<&ReplicatedComponentDescriptor> {
        self.by_component.get(component_name)
    }

    pub fn component_descriptor(
        &self,
        component_name: &str,
    ) -> Option<&ReplicatedComponentDescriptor> {
        self.by_component.get(component_name)
    }

    pub fn entity_descriptor(&self, entity_name: &str) -> Option<&ReplicatedEntityDescriptor> {
        self.by_entity.get(entity_name)
    }

    pub fn resource_descriptor(
        &self,
        resource_name: &str,
    ) -> Option<&ReplicatedResourceDescriptor> {
        self.by_resource.get(resource_name)
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &ReplicatedComponentDescriptor> {
        self.by_component.values()
    }

    pub fn component_descriptors(&self) -> impl Iterator<Item = &ReplicatedComponentDescriptor> {
        self.by_component.values()
    }

    pub fn entity_descriptors(&self) -> impl Iterator<Item = &ReplicatedEntityDescriptor> {
        self.by_entity.values()
    }

    pub fn resource_descriptors(&self) -> impl Iterator<Item = &ReplicatedResourceDescriptor> {
        self.by_resource.values()
    }

    pub fn len(&self) -> usize {
        self.by_component.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_component.is_empty() && self.by_entity.is_empty() && self.by_resource.is_empty()
    }
}

static NEXT_NET_ENTITY_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetEntityMapEvent {
    Assigned {
        ecs_entity: u64,
        net_entity: NetEntityId,
    },
    Removed {
        ecs_entity: u64,
        net_entity: NetEntityId,
    },
}

#[derive(Debug, Clone, Default)]
pub struct NetEntityMap {
    ecs_to_net: BTreeMap<u64, NetEntityId>,
    net_to_ecs: BTreeMap<NetEntityId, u64>,
}

impl NetEntityMap {
    pub fn get_or_assign(&mut self, ecs_entity: u64) -> NetEntityId {
        self.get_or_assign_with_event(ecs_entity).0
    }

    pub fn get_or_assign_with_event(
        &mut self,
        ecs_entity: u64,
    ) -> (NetEntityId, Option<NetEntityMapEvent>) {
        if let Some(existing) = self.ecs_to_net.get(&ecs_entity).copied() {
            return (existing, None);
        }
        let id = NetEntityId(NEXT_NET_ENTITY_ID.fetch_add(1, Ordering::Relaxed));
        self.ecs_to_net.insert(ecs_entity, id);
        self.net_to_ecs.insert(id, ecs_entity);
        (
            id,
            Some(NetEntityMapEvent::Assigned {
                ecs_entity,
                net_entity: id,
            }),
        )
    }

    pub fn remove_by_ecs(&mut self, ecs_entity: u64) -> Option<NetEntityId> {
        self.remove_by_ecs_with_event(ecs_entity)
            .map(|event| match event {
                NetEntityMapEvent::Removed { net_entity, .. } => net_entity,
                NetEntityMapEvent::Assigned { net_entity, .. } => net_entity,
            })
    }

    pub fn remove_by_ecs_with_event(&mut self, ecs_entity: u64) -> Option<NetEntityMapEvent> {
        let id = self.ecs_to_net.remove(&ecs_entity)?;
        self.net_to_ecs.remove(&id);
        Some(NetEntityMapEvent::Removed {
            ecs_entity,
            net_entity: id,
        })
    }

    pub fn resolve_ecs(&self, net_entity: NetEntityId) -> Option<u64> {
        self.net_to_ecs.get(&net_entity).copied()
    }

    pub fn resolve_net(&self, ecs_entity: u64) -> Option<NetEntityId> {
        self.ecs_to_net.get(&ecs_entity).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AuthorityModel, NetComponentMetadata, NetEntityMap, NetEntityMapEvent,
        ReplicatedComponentDescriptor, ReplicatedEntityDescriptor, ReplicatedResourceDescriptor,
        ReplicationRegistry, ReplicationSemanticsOverrides,
    };
    use crate::replication::{
        BandwidthPriority, InterestPolicy, PredictionMode, Reliability, ReplicationDirection,
        ReplicationProfilePreset,
    };

    #[test]
    fn net_entity_map_round_trips_ids() {
        let mut map = NetEntityMap::default();
        let id = map.get_or_assign(42);
        assert_eq!(map.resolve_net(42), Some(id));
        assert_eq!(map.resolve_ecs(id), Some(42));
        assert_eq!(map.remove_by_ecs(42), Some(id));
        assert_eq!(map.resolve_ecs(id), None);
    }

    #[test]
    fn net_entity_map_emits_events_for_assignment_and_removal() {
        let mut map = NetEntityMap::default();
        let (assigned, assignment_event) = map.get_or_assign_with_event(9);
        assert!(matches!(
            assignment_event,
            Some(NetEntityMapEvent::Assigned {
                ecs_entity: 9,
                net_entity
            }) if net_entity == assigned
        ));
        let removal_event = map.remove_by_ecs_with_event(9);
        assert!(matches!(
            removal_event,
            Some(NetEntityMapEvent::Removed {
                ecs_entity: 9,
                net_entity
            }) if net_entity == assigned
        ));
    }

    #[test]
    fn registry_replaces_descriptor_by_component_name() {
        let mut registry = ReplicationRegistry::default();
        registry.register(ReplicatedComponentDescriptor::new(
            "PlayerState",
            AuthorityModel::Server,
            ReplicationProfilePreset::PredictedMovement,
            InterestPolicy::Global,
            true,
            ReplicationSemanticsOverrides::default(),
        ));
        registry.register(ReplicatedComponentDescriptor::new(
            "PlayerState",
            AuthorityModel::Server,
            ReplicationProfilePreset::ReliableState,
            InterestPolicy::OwnerOnly,
            false,
            ReplicationSemanticsOverrides::default(),
        ));
        assert_eq!(registry.len(), 1);
        assert_eq!(
            registry
                .descriptor("PlayerState")
                .expect("descriptor exists")
                .profile,
            ReplicationProfilePreset::ReliableState
        );
    }

    #[test]
    fn explicit_semantics_override_profile_defaults() {
        let descriptor = ReplicatedComponentDescriptor::new(
            "Projectile",
            AuthorityModel::Server,
            ReplicationProfilePreset::ReliableState,
            InterestPolicy::Global,
            false,
            ReplicationSemanticsOverrides {
                reliability: Some(Reliability::Unreliable),
                frequency_hz: Some(120),
                priority: Some(BandwidthPriority::High),
                prediction: Some(PredictionMode::OwnerPredicted),
                direction: Some(ReplicationDirection::Bidirectional),
            },
        );
        assert_eq!(descriptor.direction, ReplicationDirection::Bidirectional);
        assert_eq!(descriptor.reliability, Reliability::Unreliable);
        assert_eq!(descriptor.frequency_hz, 120);
        assert_eq!(descriptor.priority, BandwidthPriority::High);
        assert_eq!(descriptor.prediction, PredictionMode::OwnerPredicted);
    }

    #[test]
    fn registry_supports_entity_and_resource_metadata() {
        let mut registry = ReplicationRegistry::default();
        registry.register_entity_descriptor(ReplicatedEntityDescriptor::new(
            "Avatar",
            ReplicationProfilePreset::PredictedMovement,
            ReplicationSemanticsOverrides::default(),
        ));
        registry.register_resource_descriptor(ReplicatedResourceDescriptor::new(
            "WeatherState",
            ReplicationProfilePreset::SparseEvent,
            ReplicationSemanticsOverrides {
                reliability: Some(Reliability::Reliable),
                ..ReplicationSemanticsOverrides::default()
            },
        ));

        assert!(registry.entity_descriptor("Avatar").is_some());
        assert!(registry.resource_descriptor("WeatherState").is_some());
    }

    #[test]
    fn registry_registers_component_through_metadata_trait() {
        struct Health;

        impl NetComponentMetadata for Health {
            fn replication_descriptor() -> ReplicatedComponentDescriptor {
                ReplicatedComponentDescriptor::new(
                    "Health",
                    AuthorityModel::Server,
                    ReplicationProfilePreset::ReliableState,
                    InterestPolicy::Global,
                    false,
                    ReplicationSemanticsOverrides::default(),
                )
            }
        }

        let mut registry = ReplicationRegistry::default();
        registry.register_component::<Health>();
        assert!(registry.descriptor("Health").is_some());
    }

    #[test]
    fn registry_registers_macro_annotated_component() {
        #[engine_net_macros::net_component(
            authority = Server,
            profile = PredictedMovement,
            interest = OwnerOnly,
            owner_prediction = true,
            reliability = Reliable,
            frequency_hz = 120,
            prediction = OwnerPredicted,
            priority = High,
            direction = Bidirectional
        )]
        #[derive(Clone, serde::Serialize, serde::Deserialize)]
        struct PlayerState;

        let mut registry = ReplicationRegistry::default();
        registry.register_component::<PlayerState>();
        let descriptor = registry
            .descriptor("PlayerState")
            .expect("descriptor should be generated by macro");
        assert_eq!(descriptor.authority, AuthorityModel::Server);
        assert_eq!(
            descriptor.profile,
            ReplicationProfilePreset::PredictedMovement
        );
        assert_eq!(descriptor.interest, InterestPolicy::OwnerOnly);
        assert!(descriptor.owner_prediction);
        assert_eq!(descriptor.reliability, Reliability::Reliable);
        assert_eq!(descriptor.frequency_hz, 120);
        assert_eq!(descriptor.prediction, PredictionMode::OwnerPredicted);
        assert_eq!(descriptor.priority, BandwidthPriority::High);
        assert_eq!(descriptor.direction, ReplicationDirection::Bidirectional);
    }

    #[test]
    fn net_entity_macro_implements_entity_trait() {
        #[engine_net_macros::net_entity]
        struct PlayerEntity;

        fn assert_net_entity<T: super::NetEntity>() {}
        assert_net_entity::<PlayerEntity>();
    }
}
