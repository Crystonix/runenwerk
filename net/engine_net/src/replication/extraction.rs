use ecs::{
    ChangeExtractionFilter, ChangeExtractionWindow, ComponentTypeKey, ResourceTypeKey,
    StructuralDeltaBatch,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default)]
pub struct ReplicationExtractionFilter {
    pub component_keys: Option<BTreeSet<ComponentTypeKey>>,
    pub resource_keys: Option<BTreeSet<ResourceTypeKey>>,
}

pub fn extract_replication_deltas(
    world: &ecs::World,
    window: ChangeExtractionWindow,
    filter: &ReplicationExtractionFilter,
) -> StructuralDeltaBatch {
    let component_keys = filter.component_keys.as_ref();
    let resource_keys = filter.resource_keys.as_ref();

    let component_key_filter = |key: ComponentTypeKey| {
        component_keys
            .map(|keys| keys.contains(&key))
            .unwrap_or(true)
    };
    let resource_key_filter = |key: ResourceTypeKey| {
        resource_keys
            .map(|keys| keys.contains(&key))
            .unwrap_or(true)
    };

    world.extract_structural_deltas(
        window,
        ChangeExtractionFilter {
            component_key_filter: Some(&component_key_filter),
            resource_key_filter: Some(&resource_key_filter),
        },
    )
}
