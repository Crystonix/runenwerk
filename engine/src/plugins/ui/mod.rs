//! Engine-owned UI runtime plugin foundation.
//!
//! This module establishes the plugin, resources, report, diagnostics, and
//! schedule labels that later UI runtime phases build on.

pub mod action;
pub mod app_ext;
pub mod diagnostics;
pub mod events;
pub mod host;
pub mod mount;
pub mod plugin;
pub mod render_publish;
#[cfg(test)]
pub(crate) mod render_scene;
pub mod report;
pub mod resources;
pub mod schedule;
pub mod screen;
pub mod source;
pub mod trace;

pub use action::*;
pub use app_ext::*;
pub use diagnostics::*;
pub use events::*;
pub use host::*;
pub use mount::*;
pub use plugin::UiPlugin;
pub use render_publish::*;
pub use report::*;
pub use resources::*;
pub use schedule::*;
pub use screen::*;
pub use source::*;
pub use trace::*;
