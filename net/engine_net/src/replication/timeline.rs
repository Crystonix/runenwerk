use crate::protocol::{
    ComponentRemove, ComponentUpsert, DeltaSnapshot, DeltaSnapshotPayload, EntityDespawn,
    EntitySpawn, Snapshot, SnapshotPayload,
};
use crate::simulation::NetworkEntityId;
use engine_sim::SimulationTick;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Serialize,
    Deserialize,
    runen_ecs::Component,
    runen_ecs::Resource,
)]
pub struct SnapshotCursor(pub u64);

#[derive(Debug, Clone)]
pub struct SnapshotTimeline {
    next_cursor: u64,
    snapshots: BTreeMap<SnapshotCursor, Snapshot>,
}

impl Default for SnapshotTimeline {
    fn default() -> Self {
        Self {
            next_cursor: 1,
            snapshots: BTreeMap::new(),
        }
    }
}

impl SnapshotTimeline {
    pub fn build_full_snapshot(
        &mut self,
        tick: SimulationTick,
        payload: SnapshotPayload,
    ) -> Result<Snapshot, postcard::Error> {
        let cursor = SnapshotCursor(self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        let entity_ids = collect_entity_ids_from_full(&payload);
        let encoded = postcard::to_allocvec(&payload)?;
        let last_applied = self.latest_cursor().unwrap_or_default();
        let snapshot = Snapshot {
            tick,
            cursor,
            last_applied,
            entity_ids,
            payload: encoded,
        };
        self.snapshots.insert(cursor, snapshot.clone());
        Ok(snapshot)
    }

    pub fn build_delta_snapshot(
        &mut self,
        tick: SimulationTick,
        base: SnapshotCursor,
        current: &SnapshotPayload,
    ) -> Result<Option<DeltaSnapshot>, postcard::Error> {
        let Some(base_snapshot) = self.snapshots.get(&base) else {
            return Ok(None);
        };
        let base_payload: SnapshotPayload = postcard::from_bytes(&base_snapshot.payload)?;
        let delta_payload = build_delta_payload(&base_payload, current);
        let cursor = SnapshotCursor(self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        let entity_ids = collect_entity_ids_from_delta(&delta_payload);
        let encoded = postcard::to_allocvec(&delta_payload)?;
        let delta = DeltaSnapshot {
            tick,
            base,
            cursor,
            entity_ids,
            payload: encoded,
        };
        let merged_payload = postcard::to_allocvec(current)?;
        self.snapshots.insert(
            cursor,
            Snapshot {
                tick,
                cursor,
                last_applied: base,
                entity_ids: collect_entity_ids_from_full(current),
                payload: merged_payload,
            },
        );
        Ok(Some(delta))
    }

    pub fn latest_cursor(&self) -> Option<SnapshotCursor> {
        self.snapshots.keys().next_back().copied()
    }

    pub fn get_snapshot(&self, cursor: SnapshotCursor) -> Option<&Snapshot> {
        self.snapshots.get(&cursor)
    }

    pub fn contains_cursor(&self, cursor: SnapshotCursor) -> bool {
        self.snapshots.contains_key(&cursor)
    }

    pub fn prune_before(&mut self, min_cursor: SnapshotCursor) {
        self.snapshots.retain(|cursor, _| *cursor >= min_cursor);
    }
}

pub fn apply_delta_payload(
    base: &SnapshotPayload,
    delta: &DeltaSnapshotPayload,
) -> SnapshotPayload {
    let delta = normalize_delta_payload(delta);
    let mut spawns = base.spawns.clone();
    let mut despawns = base.despawns.clone();
    let mut upserts = base.upserts.clone();
    let mut removes = base.removes.clone();

    for item in &delta.spawns {
        if let Some(existing) = spawns
            .iter_mut()
            .find(|e| e.net_entity_id == item.net_entity_id)
        {
            *existing = item.clone();
        } else {
            spawns.push(item.clone());
        }
    }
    for item in &delta.despawns {
        spawns.retain(|spawn| spawn.net_entity_id != item.net_entity_id);
        upserts.retain(|upsert| upsert.net_entity_id != item.net_entity_id);
        removes.retain(|remove| remove.net_entity_id != item.net_entity_id);
        if !despawns
            .iter()
            .any(|e| e.net_entity_id == item.net_entity_id)
        {
            despawns.push(item.clone());
        }
    }
    for item in &delta.upserts {
        if despawns
            .iter()
            .any(|despawn| despawn.net_entity_id == item.net_entity_id)
        {
            continue;
        }
        if let Some(existing) = upserts.iter_mut().find(|e| {
            e.net_entity_id == item.net_entity_id && e.component_name == item.component_name
        }) {
            *existing = item.clone();
        } else {
            upserts.push(item.clone());
        }
    }
    for item in &delta.removes {
        if despawns
            .iter()
            .any(|despawn| despawn.net_entity_id == item.net_entity_id)
        {
            continue;
        }
        if !removes.iter().any(|e| {
            e.net_entity_id == item.net_entity_id && e.component_name == item.component_name
        }) {
            removes.push(item.clone());
        }
    }

    SnapshotPayload {
        spawns,
        despawns,
        upserts,
        removes,
    }
}

pub fn normalize_delta_payload(delta: &DeltaSnapshotPayload) -> DeltaSnapshotPayload {
    let despawned = delta
        .despawns
        .iter()
        .map(|entry| entry.net_entity_id)
        .collect::<BTreeSet<_>>();
    let mut emitted_despawns = BTreeSet::new();

    DeltaSnapshotPayload {
        spawns: delta
            .spawns
            .iter()
            .filter(|entry| !despawned.contains(&entry.net_entity_id))
            .cloned()
            .collect(),
        despawns: delta
            .despawns
            .iter()
            .filter(|entry| emitted_despawns.insert(entry.net_entity_id))
            .cloned()
            .collect(),
        upserts: delta
            .upserts
            .iter()
            .filter(|entry| !despawned.contains(&entry.net_entity_id))
            .cloned()
            .collect(),
        removes: delta
            .removes
            .iter()
            .filter(|entry| !despawned.contains(&entry.net_entity_id))
            .cloned()
            .collect(),
    }
}

fn build_delta_payload(base: &SnapshotPayload, current: &SnapshotPayload) -> DeltaSnapshotPayload {
    let mut delta = DeltaSnapshotPayload::default();

    let base_spawns = base
        .spawns
        .iter()
        .map(|entry| (entry.net_entity_id, entry))
        .collect::<BTreeMap<_, _>>();
    for spawn in &current.spawns {
        if !base_spawns
            .get(&spawn.net_entity_id)
            .is_some_and(|base_spawn| *base_spawn == spawn)
        {
            delta.spawns.push(spawn.clone());
        }
    }

    let base_despawns = base
        .despawns
        .iter()
        .map(|entry| entry.net_entity_id)
        .collect::<BTreeSet<_>>();
    for despawn in &current.despawns {
        if !base_despawns.contains(&despawn.net_entity_id) {
            delta.despawns.push(despawn.clone());
        }
    }

    let base_upserts = base
        .upserts
        .iter()
        .map(|entry| ((entry.net_entity_id, entry.component_name.clone()), entry))
        .collect::<BTreeMap<_, _>>();
    for upsert in &current.upserts {
        let key = (upsert.net_entity_id, upsert.component_name.clone());
        if !base_upserts
            .get(&key)
            .is_some_and(|base_upsert| *base_upsert == upsert)
        {
            delta.upserts.push(upsert.clone());
        }
    }

    let base_removes = base
        .removes
        .iter()
        .map(|entry| (entry.net_entity_id, entry.component_name.clone()))
        .collect::<BTreeSet<_>>();
    for remove in &current.removes {
        let key = (remove.net_entity_id, remove.component_name.clone());
        if !base_removes.contains(&key) {
            delta.removes.push(remove.clone());
        }
    }

    normalize_delta_payload(&delta)
}

fn collect_entity_ids_from_full(payload: &SnapshotPayload) -> Vec<NetworkEntityId> {
    let mut ids = BTreeSet::new();
    for EntitySpawn { net_entity_id, .. } in &payload.spawns {
        ids.insert(*net_entity_id);
    }
    for EntityDespawn { net_entity_id } in &payload.despawns {
        ids.insert(*net_entity_id);
    }
    for ComponentUpsert { net_entity_id, .. } in &payload.upserts {
        ids.insert(*net_entity_id);
    }
    for ComponentRemove { net_entity_id, .. } in &payload.removes {
        ids.insert(*net_entity_id);
    }
    ids.into_iter().collect()
}

fn collect_entity_ids_from_delta(payload: &DeltaSnapshotPayload) -> Vec<NetworkEntityId> {
    let mut ids = BTreeSet::new();
    for EntitySpawn { net_entity_id, .. } in &payload.spawns {
        ids.insert(*net_entity_id);
    }
    for EntityDespawn { net_entity_id } in &payload.despawns {
        ids.insert(*net_entity_id);
    }
    for ComponentUpsert { net_entity_id, .. } in &payload.upserts {
        ids.insert(*net_entity_id);
    }
    for ComponentRemove { net_entity_id, .. } in &payload.removes {
        ids.insert(*net_entity_id);
    }
    ids.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::{SnapshotCursor, SnapshotTimeline, apply_delta_payload, normalize_delta_payload};
    use crate::protocol::{
        ComponentRemove, ComponentUpsert, DeltaSnapshotPayload, EntityDespawn, EntitySpawn,
        SnapshotPayload,
    };
    use engine_sim::{NetEntityId, SimulationTick};

    #[test]
    fn timeline_builds_full_and_delta_snapshots() {
        let mut timeline = SnapshotTimeline::default();
        let base = SnapshotPayload {
            upserts: vec![ComponentUpsert {
                net_entity_id: NetEntityId(1),
                component_name: "Transform".to_string(),
                payload: vec![1],
            }],
            ..SnapshotPayload::default()
        };
        let full = timeline
            .build_full_snapshot(SimulationTick(10), base.clone())
            .expect("full snapshot should encode");
        assert_eq!(full.cursor, SnapshotCursor(1));

        let current = SnapshotPayload {
            upserts: vec![ComponentUpsert {
                net_entity_id: NetEntityId(1),
                component_name: "Transform".to_string(),
                payload: vec![2],
            }],
            ..SnapshotPayload::default()
        };
        let delta = timeline
            .build_delta_snapshot(SimulationTick(11), full.cursor, &current)
            .expect("delta build should not fail")
            .expect("base cursor is valid");
        assert_eq!(delta.base, full.cursor);
        assert!(timeline.contains_cursor(delta.cursor));
    }

    #[test]
    fn apply_delta_overrides_component_payload() {
        let base = SnapshotPayload {
            upserts: vec![ComponentUpsert {
                net_entity_id: NetEntityId(9),
                component_name: "Health".to_string(),
                payload: vec![100],
            }],
            ..SnapshotPayload::default()
        };
        let delta = DeltaSnapshotPayload {
            upserts: vec![ComponentUpsert {
                net_entity_id: NetEntityId(9),
                component_name: "Health".to_string(),
                payload: vec![80],
            }],
            ..DeltaSnapshotPayload::default()
        };
        let merged = apply_delta_payload(&base, &delta);
        assert_eq!(merged.upserts[0].payload, vec![80]);
    }

    #[test]
    fn delta_despawn_removes_existing_component_state() {
        let base = SnapshotPayload {
            spawns: vec![EntitySpawn {
                net_entity_id: NetEntityId(9),
                prefab: Some("Actor".to_string()),
            }],
            upserts: vec![ComponentUpsert {
                net_entity_id: NetEntityId(9),
                component_name: "Health".to_string(),
                payload: vec![100],
            }],
            ..SnapshotPayload::default()
        };
        let delta = DeltaSnapshotPayload {
            despawns: vec![EntityDespawn {
                net_entity_id: NetEntityId(9),
            }],
            upserts: vec![ComponentUpsert {
                net_entity_id: NetEntityId(9),
                component_name: "Health".to_string(),
                payload: vec![0],
            }],
            removes: vec![ComponentRemove {
                net_entity_id: NetEntityId(9),
                component_name: "Health".to_string(),
            }],
            ..DeltaSnapshotPayload::default()
        };

        let merged = apply_delta_payload(&base, &delta);

        assert!(merged.spawns.is_empty());
        assert!(merged.upserts.is_empty());
        assert!(merged.removes.is_empty());
        assert_eq!(
            merged.despawns,
            vec![EntityDespawn {
                net_entity_id: NetEntityId(9)
            }]
        );
    }

    #[test]
    fn normalize_delta_payload_makes_same_delta_despawn_win() {
        let payload = DeltaSnapshotPayload {
            spawns: vec![EntitySpawn {
                net_entity_id: NetEntityId(12),
                prefab: Some("Projectile".to_string()),
            }],
            despawns: vec![EntityDespawn {
                net_entity_id: NetEntityId(12),
            }],
            upserts: vec![ComponentUpsert {
                net_entity_id: NetEntityId(12),
                component_name: "Transform".to_string(),
                payload: vec![1],
            }],
            removes: vec![ComponentRemove {
                net_entity_id: NetEntityId(12),
                component_name: "Transform".to_string(),
            }],
        };

        let normalized = normalize_delta_payload(&payload);

        assert!(normalized.spawns.is_empty());
        assert_eq!(
            normalized.despawns,
            vec![EntityDespawn {
                net_entity_id: NetEntityId(12)
            }]
        );
        assert!(normalized.upserts.is_empty());
        assert!(normalized.removes.is_empty());
    }

    #[test]
    fn timeline_supports_chained_delta_baselines() {
        let mut timeline = SnapshotTimeline::default();
        let base = SnapshotPayload {
            upserts: vec![ComponentUpsert {
                net_entity_id: NetEntityId(3),
                component_name: "Transform".to_string(),
                payload: vec![1],
            }],
            ..SnapshotPayload::default()
        };
        let full = timeline
            .build_full_snapshot(SimulationTick(1), base.clone())
            .expect("full snapshot should build");
        let first_next = SnapshotPayload {
            upserts: vec![ComponentUpsert {
                net_entity_id: NetEntityId(3),
                component_name: "Transform".to_string(),
                payload: vec![2],
            }],
            ..SnapshotPayload::default()
        };
        let first_delta = timeline
            .build_delta_snapshot(SimulationTick(2), full.cursor, &first_next)
            .expect("first delta should build")
            .expect("first baseline exists");
        let second_next = SnapshotPayload {
            upserts: vec![ComponentUpsert {
                net_entity_id: NetEntityId(3),
                component_name: "Transform".to_string(),
                payload: vec![3],
            }],
            ..SnapshotPayload::default()
        };
        let second_delta = timeline
            .build_delta_snapshot(SimulationTick(3), first_delta.cursor, &second_next)
            .expect("second delta should build")
            .expect("delta cursor should be tracked as baseline");
        assert_eq!(second_delta.base, first_delta.cursor);
    }
}
