//! Drawing app runtime plugin.

use ecs::SystemSetKey;
use engine::plugins::render::SurfaceFrameSubmissionRegistryResource;
use engine::prelude::*;
use engine::runtime::{
    CoreSet, IntoSystemSetKey, RuntimeJobExecutorConfig, RuntimeJobExecutorResource,
    SystemConfigExt,
};

use crate::runtime::gpu_ink::{
    DrawingInkGpuValidationReportCursorResource, process_drawing_ink_gpu_validation_report_system,
};
use crate::runtime::ink::{
    publish_drawing_ink_products_at_boundary, publish_drawing_ink_query_snapshots_at_boundary,
};
use crate::runtime::resources::{DrawingHostResource, DrawingInkUploadTrackerResource};
use crate::runtime::systems::{
    process_draw_preview_ink_jobs_system, route_draw_input_system, submit_draw_frame_system,
};

pub struct DrawingAppPlugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawingRuntimeSet {
    InputRoute,
    PreviewJobs,
    GpuValidation,
    FrameSubmit,
}

impl IntoSystemSetKey for DrawingRuntimeSet {
    fn system_set_key(&self) -> SystemSetKey {
        match self {
            Self::InputRoute => {
                SystemSetKey::of::<DrawingRuntimeSet>("DrawingRuntimeSet::InputRoute")
            }
            Self::PreviewJobs => {
                SystemSetKey::of::<DrawingRuntimeSet>("DrawingRuntimeSet::PreviewJobs")
            }
            Self::GpuValidation => {
                SystemSetKey::of::<DrawingRuntimeSet>("DrawingRuntimeSet::GpuValidation")
            }
            Self::FrameSubmit => {
                SystemSetKey::of::<DrawingRuntimeSet>("DrawingRuntimeSet::FrameSubmit")
            }
        }
    }
}

impl Plugin for DrawingAppPlugin {
    fn build(&self, app: &mut App) {
        install_draw_runtime_job_executor(app);
        app.init_resource::<DrawingHostResource>();
        app.init_resource::<DrawingInkUploadTrackerResource>();
        app.init_resource::<DrawingInkGpuValidationReportCursorResource>();
        app.init_resource::<SurfaceFrameSubmissionRegistryResource>();
        app.add_product_publication_handler(publish_drawing_ink_products_at_boundary);
        app.add_query_snapshot_publication_handler(publish_drawing_ink_query_snapshots_at_boundary);

        app.add_systems(
            Update,
            route_draw_input_system
                .in_set(DrawingRuntimeSet::InputRoute)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            Update,
            process_draw_preview_ink_jobs_system
                .in_set(DrawingRuntimeSet::PreviewJobs)
                .after(DrawingRuntimeSet::InputRoute)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            Update,
            process_drawing_ink_gpu_validation_report_system
                .in_set(DrawingRuntimeSet::GpuValidation)
                .after(DrawingRuntimeSet::PreviewJobs)
                .after(DrawingRuntimeSet::InputRoute)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            Update,
            submit_draw_frame_system
                .in_set(DrawingRuntimeSet::FrameSubmit)
                .after(DrawingRuntimeSet::GpuValidation)
                .after(DrawingRuntimeSet::PreviewJobs)
                .after(DrawingRuntimeSet::InputRoute)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
    }
}

fn install_draw_runtime_job_executor(app: &mut App) {
    let should_install_draw_default = app
        .world()
        .resource::<RuntimeJobExecutorResource>()
        .map(|executor| executor.config() == &RuntimeJobExecutorConfig::default())
        .unwrap_or(true);
    if should_install_draw_default {
        app.insert_resource(RuntimeJobExecutorResource::with_config(
            RuntimeJobExecutorConfig::worker_pool(2, 64),
        ));
    }
}
