//! Public crate surface for runtime/app/plugin composition.
//!
//! Most downstream code should start with:
//! - [`crate::App`]
//! - [`crate::Plugin`]
//! - [`crate::prelude`]
//!
//! Net-specific entry surface:
//! - [`crate::net::prelude`]

extern crate self as engine;

pub mod app;
pub mod net;
pub mod plugin;
pub mod plugins;
pub mod prelude;
pub mod runtime;
pub mod state;
pub mod utils;

pub use app::*;
pub use engine_replay::*;
pub use engine_sim::*;
pub use plugin::*;
pub use runtime::*;
pub use state::*;
