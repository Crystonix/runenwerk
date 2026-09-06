use crate::Entity;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EntityAllocationError {
    #[error("entity index space exhausted")]
    IndexExhausted,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EntityError {
    #[error("entity {entity:?} belongs to a different world")]
    ForeignWorld { entity: Entity },
    #[error("entity {entity:?} is unknown")]
    UnknownEntity { entity: Entity },
    #[error("entity {entity:?} has a stale generation; current generation is {current_generation}")]
    StaleGeneration {
        entity: Entity,
        current_generation: u32,
    },
    #[error("entity {entity:?} was already freed")]
    AlreadyFreed { entity: Entity },
    #[error("entity {entity:?} is missing component {component}")]
    MissingComponent {
        entity: Entity,
        component: &'static str,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ResourceError {
    #[error("resource {resource} does not exist")]
    Missing { resource: &'static str },
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error(transparent)]
    Entity(#[from] EntityError),
    #[error(transparent)]
    EntityAllocation(#[from] EntityAllocationError),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum QueryError {
    #[error("query expected exactly one result but found none")]
    NoResults,
    #[error("query expected exactly one result but found {count}")]
    MultipleResults { count: usize },
    #[error("query has conflicting {domain} borrows for {target}")]
    ConflictingBorrow {
        domain: &'static str,
        target: &'static str,
    },
}
