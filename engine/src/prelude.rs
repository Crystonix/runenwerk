//! Common imports for engine app/plugin authoring.
//!
//! This prelude focuses on runtime composition and ecs ergonomics.
//! Feature-heavy or domain-specific APIs stay in their owning modules
//! (for example `engine::plugins::net` or `engine::plugins::render`).

pub use crate::app::*;
pub use crate::plugin::Plugin;
pub use crate::plugins::fixed_step::FixedStepPlugin;
pub use crate::plugins::input::domain::InputState;
pub use crate::plugins::net::{
    NetworkClientOutbox, NetworkInboundQueue, NetworkServerOutbox, NetworkSessionStatus,
    RoundTripMetrics,
};
pub use crate::plugins::replay::*;
pub use crate::plugins::scene::*;
pub use crate::plugins::time::domain::Time;
pub use crate::plugins::ui::AppUiExt;
pub use crate::plugins::{default_plugins, default_plugins_with_diagnostics};
pub use crate::runtime::*;
pub use crate::state::*;
pub use engine_replay::*;
pub use engine_sim::*;
pub use runen_ecs::{Bundle, Component, Entity, Resource, SystemSet, World};
