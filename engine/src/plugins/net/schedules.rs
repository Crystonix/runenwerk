use crate::runtime::IntoSystemSetKey;
use runen_ecs::SystemSetKey;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NetPreUpdateSet {
    Receive,
}

impl IntoSystemSetKey for NetPreUpdateSet {
    fn system_set_key(&self) -> SystemSetKey {
        match self {
            Self::Receive => SystemSetKey::of::<NetPreUpdateSet>("NetPreUpdateSet::Receive"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NetFixedSet {
    Prediction,
    Replication,
}

impl IntoSystemSetKey for NetFixedSet {
    fn system_set_key(&self) -> SystemSetKey {
        match self {
            Self::Prediction => SystemSetKey::of::<NetFixedSet>("NetFixedSet::Prediction"),
            Self::Replication => SystemSetKey::of::<NetFixedSet>("NetFixedSet::Replication"),
        }
    }
}
