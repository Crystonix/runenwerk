//! Runtime domain.
//!
//! Owns schedule labels, runtime system params, platform event normalization,
//! fixed-step state/resources, canonical fixed-step execution, canonical frame
//! lifecycle execution, publication policy, and window runtime helpers.

pub(crate) mod fixed_step_executor;
pub mod fixed_time;
pub(crate) mod frame_lifecycle;
pub mod frame_pacing;
pub mod jobs;
pub mod native_window_hooks;
pub mod param;
pub mod platform;
pub mod product_cache;
pub mod product_publication;
pub mod publication;
pub mod query_snapshot;
pub mod schedules;
pub mod system;
pub mod window;
pub mod winit_runner;

pub use fixed_time::*;
pub use frame_pacing::*;
pub use jobs::*;
pub use native_window_hooks::*;
pub use param::*;
pub use product_cache::*;
pub use product_publication::*;
pub use publication::*;
pub use query_snapshot::*;
pub use schedules::*;
pub use system::*;
pub use window::*;
