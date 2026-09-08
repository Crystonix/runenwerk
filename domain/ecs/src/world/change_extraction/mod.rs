use crate::world::World;
use crate::world::change_tracking::{
    ComponentChangeKind, ComponentTypeKey, ResourceChangeKind, ResourceTypeKey,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ChangeExtractionWindow {
    pub tick_start_exclusive: u64,
    pub tick_end_inclusive: u64,
    pub frame_start_exclusive: u64,
    pub frame_end_inclusive: u64,
}

impl ChangeExtractionWindow {
    pub fn for_tick_window(start_exclusive: u64, end_inclusive: u64) -> Self {
        Self {
            tick_start_exclusive: start_exclusive,
            tick_end_inclusive: end_inclusive,
            frame_start_exclusive: u64::MAX,
            frame_end_inclusive: u64::MAX,
        }
    }

    pub fn for_frame_window(start_exclusive: u64, end_inclusive: u64) -> Self {
        Self {
            tick_start_exclusive: u64::MAX,
            tick_end_inclusive: u64::MAX,
            frame_start_exclusive: start_exclusive,
            frame_end_inclusive: end_inclusive,
        }
    }

    pub fn contains_tick(&self, tick: u64) -> bool {
        if self.tick_start_exclusive == u64::MAX && self.tick_end_inclusive == u64::MAX {
            return true;
        }
        tick > self.tick_start_exclusive && tick <= self.tick_end_inclusive
    }

    pub fn contains_frame(&self, frame: u64) -> bool {
        if self.frame_start_exclusive == u64::MAX && self.frame_end_inclusive == u64::MAX {
            return true;
        }
        frame > self.frame_start_exclusive && frame <= self.frame_end_inclusive
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentStructuralDelta {
    pub tick: u64,
    pub frame: u64,
    pub entity: crate::entity::Entity,
    pub component_key: ComponentTypeKey,
    pub component_name: &'static str,
    pub kind: ComponentChangeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceStructuralDelta {
    pub tick: u64,
    pub frame: u64,
    pub resource_key: ResourceTypeKey,
    pub resource_name: &'static str,
    pub kind: ResourceChangeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StructuralDeltaBatch {
    pub window: Option<ChangeExtractionWindow>,
    pub component_deltas: Vec<ComponentStructuralDelta>,
    pub resource_deltas: Vec<ResourceStructuralDelta>,
}

impl StructuralDeltaBatch {
    pub fn is_empty(&self) -> bool {
        self.component_deltas.is_empty() && self.resource_deltas.is_empty()
    }
}

#[derive(Default)]
pub struct ChangeExtractionFilter<'a> {
    pub component_key_filter: Option<&'a dyn Fn(ComponentTypeKey) -> bool>,
    pub resource_key_filter: Option<&'a dyn Fn(ResourceTypeKey) -> bool>,
}

fn component_kind_order(kind: ComponentChangeKind) -> u8 {
    match kind {
        ComponentChangeKind::Added => 0,
        ComponentChangeKind::Modified => 1,
        ComponentChangeKind::Removed => 2,
    }
}

fn resource_kind_order(kind: ResourceChangeKind) -> u8 {
    match kind {
        ResourceChangeKind::Inserted => 0,
        ResourceChangeKind::Modified => 1,
        ResourceChangeKind::Removed => 2,
    }
}

impl World {
    pub fn extract_structural_deltas(
        &self,
        window: ChangeExtractionWindow,
        filter: ChangeExtractionFilter<'_>,
    ) -> StructuralDeltaBatch {
        let mut component_deltas = Vec::<ComponentStructuralDelta>::new();
        for change in &self.component_change_log {
            if !window.contains_tick(change.tick) || !window.contains_frame(change.frame) {
                continue;
            }

            if let Some(component_key_filter) = filter.component_key_filter
                && !component_key_filter(change.component_key)
            {
                continue;
            }

            component_deltas.push(ComponentStructuralDelta {
                tick: change.tick,
                frame: change.frame,
                entity: change.entity,
                component_key: change.component_key,
                component_name: change.component_name,
                kind: change.kind,
            });
        }
        component_deltas.sort_by_key(|delta| {
            (
                delta.entity,
                delta.component_key,
                delta.tick,
                delta.frame,
                component_kind_order(delta.kind),
            )
        });

        let mut resource_deltas = Vec::<ResourceStructuralDelta>::new();
        for change in &self.resource_change_log {
            if !window.contains_tick(change.tick) || !window.contains_frame(change.frame) {
                continue;
            }

            if let Some(resource_key_filter) = filter.resource_key_filter
                && !resource_key_filter(change.resource_key)
            {
                continue;
            }

            resource_deltas.push(ResourceStructuralDelta {
                tick: change.tick,
                frame: change.frame,
                resource_key: change.resource_key,
                resource_name: change.resource_name,
                kind: change.kind,
            });
        }
        resource_deltas.sort_by_key(|delta| {
            (
                delta.resource_key,
                delta.tick,
                delta.frame,
                resource_kind_order(delta.kind),
            )
        });

        StructuralDeltaBatch {
            window: Some(window),
            component_deltas,
            resource_deltas,
        }
    }

    pub fn extract_structural_deltas_for_tick_window(
        &self,
        tick_start_exclusive: u64,
        tick_end_inclusive: u64,
        filter: ChangeExtractionFilter<'_>,
    ) -> StructuralDeltaBatch {
        self.extract_structural_deltas(
            ChangeExtractionWindow {
                tick_start_exclusive,
                tick_end_inclusive,
                frame_start_exclusive: u64::MAX,
                frame_end_inclusive: u64::MAX,
            },
            filter,
        )
    }

    pub fn extract_structural_deltas_for_frame_window(
        &self,
        frame_start_exclusive: u64,
        frame_end_inclusive: u64,
        filter: ChangeExtractionFilter<'_>,
    ) -> StructuralDeltaBatch {
        self.extract_structural_deltas(
            ChangeExtractionWindow {
                tick_start_exclusive: u64::MAX,
                tick_end_inclusive: u64::MAX,
                frame_start_exclusive,
                frame_end_inclusive,
            },
            filter,
        )
    }
}
