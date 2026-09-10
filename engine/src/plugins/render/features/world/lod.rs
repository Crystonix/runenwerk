use runen_spatial::ChunkId;
use std::collections::BTreeMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq, runen_ecs::Resource)]
pub enum WorldLodBand {
    Far,
    Mid,
    Near,
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct WorldLodPolicyResource {
    pub near_distance_meters: f32,
    pub mid_distance_meters: f32,
    pub far_distance_meters: f32,
}

impl Default for WorldLodPolicyResource {
    fn default() -> Self {
        Self {
            near_distance_meters: 96.0,
            mid_distance_meters: 384.0,
            far_distance_meters: 2_048.0,
        }
    }
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct WorldLodSelectionResource {
    pub band_by_chunk: BTreeMap<ChunkId, WorldLodBand>,
}

impl WorldLodPolicyResource {
    pub fn classify_distance(&self, distance_meters: f32) -> WorldLodBand {
        if distance_meters <= self.near_distance_meters {
            WorldLodBand::Near
        } else if distance_meters <= self.mid_distance_meters {
            WorldLodBand::Mid
        } else {
            WorldLodBand::Far
        }
    }
}
