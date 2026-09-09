//! Runtime integration for the drawing app.

pub mod app;
pub mod gpu_ink;
pub mod ink;
pub mod ink_jobs;
pub mod plugin;
pub mod resources;
pub mod systems;

pub use app::{build_app, build_headless_app, run};
pub use gpu_ink::{
    DrawingInkGpuFlowResource, apply_drawing_ink_gpu_validation_report,
    prepare_drawing_ink_gpu_frame, process_drawing_ink_gpu_validation_report_system,
    register_drawing_ink_gpu_flow,
};
pub use ink::{
    DrawingPreviewInkJobProcessReport, process_drawing_preview_ink_jobs,
    publish_drawing_ink_products, publish_drawing_ink_products_at_boundary,
    publish_drawing_ink_products_with_executor_and_cache, publish_drawing_ink_query_snapshots,
    publish_drawing_ink_query_snapshots_at_boundary,
};
pub use ink_jobs::{
    DrawingCommittedInkTileJob, DrawingCommittedInkTileJobOutput, DrawingPreviewInkTileJob,
    DrawingPreviewInkTileJobOutput,
};
pub use plugin::{DrawingAppPlugin, DrawingRuntimeSet};
pub use resources::DrawingHostResource;
pub use systems::DRAWING_UI_FRAME_PRODUCER_ID;
