use std::collections::BTreeSet;
use world_sdf::CaveSectorId;

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct CaveRenderVisibilityResource {
    pub visible_sectors: BTreeSet<CaveSectorId>,
}
