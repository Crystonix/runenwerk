use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SimulationProfile {
    #[default]
    LocalSinglePlayer,
    DeterministicLockstep,
    RollbackSession,
    DedicatedAuthority,
    HighThroughputAuthority,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AuthorityRole {
    #[default]
    Local,
    Client,
    Server,
    Peer,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DeterminismLevel {
    Strict,
    #[default]
    Validated,
    BestEffort,
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    runen_ecs::Component,
    runen_ecs::Resource,
)]
pub struct SimulationProfileConfig {
    pub profile: SimulationProfile,
    pub authority: AuthorityRole,
    pub determinism: DeterminismLevel,
}

impl Default for SimulationProfileConfig {
    fn default() -> Self {
        Self {
            profile: SimulationProfile::LocalSinglePlayer,
            authority: AuthorityRole::Local,
            determinism: DeterminismLevel::Validated,
        }
    }
}
