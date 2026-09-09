// Owner: ecs World - Runtime and Query Entry APIs
use super::World;
use crate::commands::Commands;
use crate::query::{QueryFilter, QueryOrphanedState, QuerySpec, QueryState};

impl World {
    pub fn commands(&self) -> Commands<'static> {
        Commands::new()
    }

    pub fn query_state<Q: QuerySpec, F: QueryFilter>(&self) -> QueryState<Q, F> {
        QueryState::new(self)
    }

    pub fn query_orphaned_state<T: crate::component::Component>(&self) -> QueryOrphanedState<T> {
        QueryOrphanedState::new(self)
    }

    pub fn current_change_tick(&self) -> u64 {
        self.change_tick
    }
}
