use runen_gpu::{GpuPreparedWorkGraph, GpuResourceAccess, GpuWorkResourceId};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransientResourceWindow {
    pub resource_id: GpuWorkResourceId,
    pub first_node_index: usize,
    pub last_node_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransientAliasCandidate {
    pub left: GpuWorkResourceId,
    pub right: GpuWorkResourceId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransientAliasAssignment {
    pub resource_id: GpuWorkResourceId,
    pub slot_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransientAliasSlot {
    pub slot_index: usize,
    pub resources: Vec<GpuWorkResourceId>,
}

/// Derives later-phase transient allocation windows from prepared G3 order and
/// normalized access truth. This is allocation policy, not an alternate access
/// or dependency graph.
pub fn build_transient_windows(graph: &GpuPreparedWorkGraph) -> Vec<TransientResourceWindow> {
    let nodes_by_id = graph
        .nodes()
        .iter()
        .map(|prepared| (prepared.id(), prepared.node()))
        .collect::<BTreeMap<_, _>>();
    let mut node_usages = BTreeMap::<GpuWorkResourceId, (usize, usize)>::new();

    for (node_index, node_id) in graph.topological_order().iter().copied().enumerate() {
        let Some(node) = nodes_by_id.get(&node_id) else {
            continue;
        };
        for access in node.accesses() {
            let Some(id) = transient_resource_id(access) else {
                continue;
            };
            let entry = node_usages.entry(id).or_insert((node_index, node_index));
            entry.0 = entry.0.min(node_index);
            entry.1 = entry.1.max(node_index);
        }
    }

    node_usages
        .into_iter()
        .map(
            |(resource_id, (first_node_index, last_node_index))| TransientResourceWindow {
                resource_id,
                first_node_index,
                last_node_index,
            },
        )
        .collect()
}

pub fn find_aliasable_transients(
    windows: &[TransientResourceWindow],
) -> Vec<TransientAliasCandidate> {
    let mut candidates = Vec::<TransientAliasCandidate>::new();
    for i in 0..windows.len() {
        for j in (i + 1)..windows.len() {
            let left = &windows[i];
            let right = &windows[j];
            let disjoint = left.last_node_index < right.first_node_index
                || right.last_node_index < left.first_node_index;
            if disjoint {
                candidates.push(TransientAliasCandidate {
                    left: left.resource_id,
                    right: right.resource_id,
                });
            }
        }
    }
    candidates
}

pub fn build_transient_alias_assignments(
    windows: &[TransientResourceWindow],
) -> Vec<TransientAliasAssignment> {
    let mut ordered_windows = windows.to_vec();
    ordered_windows.sort_by_key(|window| (window.first_node_index, window.last_node_index));

    let mut slot_last_use = Vec::<usize>::new();
    let mut assignments = Vec::<TransientAliasAssignment>::new();

    for window in &ordered_windows {
        let mut selected_slot = None::<usize>;
        for (slot_index, last_use) in slot_last_use.iter().enumerate() {
            if *last_use < window.first_node_index {
                selected_slot = Some(slot_index);
                break;
            }
        }

        let slot_index = match selected_slot {
            Some(index) => {
                slot_last_use[index] = window.last_node_index;
                index
            }
            None => {
                slot_last_use.push(window.last_node_index);
                slot_last_use.len() - 1
            }
        };

        assignments.push(TransientAliasAssignment {
            resource_id: window.resource_id,
            slot_index,
        });
    }

    assignments
}

pub fn build_transient_alias_slots(
    assignments: &[TransientAliasAssignment],
) -> Vec<TransientAliasSlot> {
    let mut slots = BTreeMap::<usize, Vec<GpuWorkResourceId>>::new();
    for assignment in assignments {
        slots
            .entry(assignment.slot_index)
            .or_default()
            .push(assignment.resource_id);
    }

    slots
        .into_iter()
        .map(|(slot_index, resources)| TransientAliasSlot {
            slot_index,
            resources,
        })
        .collect()
}

fn transient_resource_id(access: &GpuResourceAccess) -> Option<GpuWorkResourceId> {
    let transient = match access {
        GpuResourceAccess::Buffer(access) => access
            .buffer()
            .descriptor()
            .common()
            .lifetime()
            .is_transient(),
        GpuResourceAccess::Texture(access) => access
            .normalized_texture()
            .descriptor()
            .common()
            .lifetime()
            .is_transient(),
        GpuResourceAccess::Query(access) => access
            .query_set()
            .descriptor()
            .common()
            .lifetime()
            .is_transient(),
        GpuResourceAccess::Sampler(access) => access
            .sampler()
            .descriptor()
            .common()
            .lifetime()
            .is_transient(),
    };
    transient.then(|| access.resource_identity())
}
