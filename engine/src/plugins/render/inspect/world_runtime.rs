#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct WorldRuntimeInspectorSnapshot {
    pub chunk_dirty_count: usize,
    pub queued_interactive: usize,
    pub queued_background: usize,
    pub integrated_build_outputs: u64,
    pub dropped_stale_outputs: u64,
    pub op_log_count: u64,
    pub ingress_operations: u64,
    pub invalidated_chunks: u64,
    pub collision_queries: u64,
    pub collision_authority_misses: u64,
    pub world_revision: u64,
    pub region_journal_latest_sequence: u64,
    pub region_journal_record_count: usize,
}
