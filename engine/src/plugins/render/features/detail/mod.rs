use runen_spatial::ChunkId;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, runen_ecs::Resource)]
pub struct DetailCellPayload {
    pub chunk_id: ChunkId,
    pub cell_id: u32,
    pub instance_count: u32,
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct DetailPreparedCellResource {
    pub cells_by_chunk: BTreeMap<ChunkId, Vec<DetailCellPayload>>,
}
