//! Engine plugin registry and default plugin stack helpers.
//!
//! See `docs-site/src/content/docs/engine/plugins/README.md` for plugin navigation.

pub mod debug_metrics;
pub mod diagnostics;
pub mod fixed_step;
pub mod grid;
pub mod input;
pub mod net;
pub mod render;
pub mod replay;
pub mod scene;
pub mod scheduler_diagnostics;
pub(crate) mod shared;
pub mod time;
pub mod ui;
pub mod world;
pub use debug_metrics::*;
pub use diagnostics::{
    DiagnosticsAttachment, DiagnosticsConfigResource, DiagnosticsConsoleFeedResource,
    DiagnosticsEntry, DiagnosticsFrameReport, DiagnosticsPlugin, DiagnosticsReportStoreResource,
    DiagnosticsSeverity, DiagnosticsStatus, DiagnosticsSummary, ResolvedDiagnosticsPlan,
};
pub use fixed_step::*;
pub use grid::*;
pub use input::*;
pub use render::*;
pub use replay::*;
pub use scene::plugin::ScenePlugin;
pub use scene::runtime::controls::*;
pub use scene::types::*;
pub use scheduler_diagnostics::*;
pub use time::TimePlugin;
pub use ui::{
    UiPlugin, UiRuntimeDiagnostic, UiRuntimeDiagnosticCode, UiRuntimeDiagnosticSeverity,
    UiRuntimeDiagnosticsResource, UiRuntimeInstallState, UiRuntimeReport, UiRuntimeReportResource,
    UiRuntimeResource, UiRuntimeSet,
};
pub use world::plugin::{
    WorldAuthorityState, WorldPlugin, WorldRuntimeConfig, WorldRuntimeMode, WorldRuntimeSet,
    WorldRuntimeState,
};

use crate::plugin::Plugin;

pub fn default_plugins() -> Vec<Box<dyn Plugin>> {
    vec![
        Box::new(TimePlugin),
        Box::new(FixedStepPlugin),
        Box::new(ReplayPlugin),
        Box::new(InputFinalizePlugin),
        Box::new(DiagnosticsPlugin),
    ]
}

pub fn default_plugins_with_diagnostics() -> Vec<Box<dyn Plugin>> {
    let mut plugins = default_plugins();
    plugins.push(Box::new(SchedulerDiagnosticsPlugin));
    plugins
}
