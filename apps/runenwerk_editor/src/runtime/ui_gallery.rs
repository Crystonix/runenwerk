//! Standalone artifact-backed UI gallery host.

use engine::plugins::render::{
    RenderFrameProducerId, SurfaceFrameRoute, SurfaceFrameSubmission, SurfaceFrameSubmissionOrder,
    SurfaceFrameSubmissionRegistryResource, UiFontAtlasResource,
};
use engine::prelude::*;
use ui_controls::{ControlPackageRegistry, runenwerk_control_package};
use ui_math::UiSize;
use ui_render_data::UiFrame;
use ui_runtime_view::ButtonRuntimeViewReport;
use ui_story::{
    NODE_PREVIEW_FRAME, UiStoryCliReportV2, UiStoryOutcomeV2, UiStoryRunnerV2,
    UiStoryWorkflowReportV2, checked_in_story_registry_v2,
};
use ui_theme::ThemeTokens;

use super::ui_gallery_diagnostics::{
    append_interactive_story_report_diagnostics, dedupe_gallery_diagnostics,
};
use super::ui_gallery_execution::{execute_gallery_story, registry_failure_execution};
use super::ui_gallery_frame::{compose_gallery_preview_frame, default_gallery_proof_size};
use super::workbench_close_policy::approve_primary_window_close_intent_system;

pub use super::ui_gallery_diagnostics::{
    UiGalleryDiagnostic, UiGalleryDiagnosticSeverity, UiGalleryStage,
};
pub use super::ui_gallery_execution::UiGalleryStoryExecution;

pub const UI_GALLERY_UI_PRODUCER_ID: RenderFrameProducerId = ui_frame_producer_id(5_101);

const fn ui_frame_producer_id(raw: u64) -> RenderFrameProducerId {
    match RenderFrameProducerId::try_from_raw(raw) {
        Ok(id) => id,
        Err(_) => panic!("ui frame producer id constants must be non-zero"),
    }
}

pub struct UiGalleryPlugin;

impl Plugin for UiGalleryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiGalleryResource>();
        app.add_systems(Update, approve_primary_window_close_intent_system);
        app.add_systems(Update, submit_ui_gallery_frame_system);
    }
}

#[derive(Clone, Debug, runen_ecs::Component, runen_ecs::Resource)]
pub struct UiGalleryResource {
    button_report: ButtonRuntimeViewReport,
    story_reports: Vec<UiStoryWorkflowReportV2>,
    diagnostics: Vec<UiGalleryDiagnostic>,
    frame: Option<UiFrame>,
    prepared_size: Option<UiSize>,
    diagnostics_logged: bool,
}

impl Default for UiGalleryResource {
    fn default() -> Self {
        Self::from_checked_in_stories()
    }
}

impl UiGalleryResource {
    pub fn from_checked_in_fixtures() -> Self {
        Self::from_checked_in_stories()
    }

    pub fn from_checked_in_stories() -> Self {
        let atlas = UiFontAtlasResource::default();
        Self::from_checked_in_stories_for_render_target(
            default_gallery_proof_size(),
            &ThemeTokens::default(),
            &atlas,
        )
    }

    pub fn from_checked_in_stories_for_render_target(
        size: UiSize,
        theme: &ThemeTokens,
        atlas: &UiFontAtlasResource,
    ) -> Self {
        let executions = run_checked_in_gallery_stories_for_render_target(
            default_gallery_proof_size(),
            theme,
            atlas,
        );
        Self::from_story_executions(executions, Some(size))
    }

    pub fn from_story_executions(
        executions: Vec<UiGalleryStoryExecution>,
        prepared_size: Option<UiSize>,
    ) -> Self {
        let mut diagnostics = Vec::new();
        let mut button_report = ButtonRuntimeViewReport::default();
        let mut mounted_previews = Vec::new();
        let mut story_reports = Vec::new();

        for execution in executions {
            let report_blocks_gallery = !execution.report.outcome().is_green();
            append_interactive_story_report_diagnostics(
                &execution.report,
                &mut diagnostics,
                report_blocks_gallery,
            );

            if let Some(report) = execution.button_report
                && !report_blocks_gallery
            {
                button_report.buttons.extend(report.buttons.clone());
            }

            if execution.report.outcome() == UiStoryOutcomeV2::Passed
                && execution.mount_decision.allowed
            {
                if let Some(mounted_frame) = execution.mounted_frame {
                    mounted_previews.push(mounted_frame);
                } else {
                    diagnostics.push(UiGalleryDiagnostic {
                        stage: UiGalleryStage::WorkflowNode(NODE_PREVIEW_FRAME.to_owned()),
                        story_id: Some(execution.report.story_id.as_str().to_owned()),
                        code: "ui_gallery.story.preview_frame.missing".to_owned(),
                        message: "passed story report did not carry a mounted preview frame"
                            .to_owned(),
                        severity: UiGalleryDiagnosticSeverity::Error,
                        source_map_index: None,
                        blocks_gallery: true,
                    });
                }
            }

            story_reports.push(execution.report);
        }
        dedupe_gallery_diagnostics(&mut diagnostics);
        let frame = compose_gallery_preview_frame(
            prepared_size.unwrap_or_else(default_gallery_proof_size),
            &mounted_previews,
        );

        Self {
            button_report,
            story_reports,
            diagnostics,
            frame,
            prepared_size,
            diagnostics_logged: false,
        }
    }

    pub fn passed(&self) -> bool {
        !self.has_blocking_diagnostics()
    }

    pub fn frame(&self) -> Option<&UiFrame> {
        self.frame.as_ref()
    }

    pub fn diagnostics(&self) -> &[UiGalleryDiagnostic] {
        &self.diagnostics
    }

    pub fn story_reports(&self) -> &[UiStoryWorkflowReportV2] {
        &self.story_reports
    }

    pub fn button_count(&self) -> usize {
        self.button_report.buttons.len()
    }

    fn has_blocking_diagnostics(&self) -> bool {
        self.diagnostics.iter().any(|diagnostic| {
            diagnostic.blocks_gallery && diagnostic.severity == UiGalleryDiagnosticSeverity::Error
        })
    }

    fn prepare_frame_if_needed(
        &mut self,
        size: UiSize,
        theme: &ThemeTokens,
        atlas: &UiFontAtlasResource,
    ) {
        if self.prepared_size == Some(size) {
            return;
        }

        *self = Self::from_checked_in_stories_for_render_target(size, theme, atlas);
    }

    fn log_diagnostics_once(&mut self) {
        if self.diagnostics_logged {
            return;
        }
        self.diagnostics_logged = true;

        if self.diagnostics.is_empty() {
            eprintln!(
                "[ui_gallery] prepared {} buttons from {} story reports",
                self.button_count(),
                self.story_reports.len()
            );
            return;
        }

        for diagnostic in &self.diagnostics {
            eprintln!(
                "[ui_gallery][{}][{}] {}{}{}",
                diagnostic.stage.as_str(),
                diagnostic.severity.as_str(),
                diagnostic.code,
                diagnostic
                    .story_id
                    .as_ref()
                    .map(|id| format!(" story={id}"))
                    .unwrap_or_default(),
                diagnostic
                    .source_map_index
                    .map(|index| format!(" source_map_index={index}"))
                    .unwrap_or_default()
            );
            eprintln!("[ui_gallery] {}", diagnostic.message);
        }
    }
}

pub fn inspect_checked_in_gallery_stories() -> UiStoryCliReportV2 {
    let executions = run_checked_in_gallery_stories();
    UiStoryCliReportV2::from_reports(
        executions
            .iter()
            .map(|execution| (&execution.report, execution.mount_policy)),
    )
}

pub fn run_checked_in_gallery_stories() -> Vec<UiGalleryStoryExecution> {
    let atlas = UiFontAtlasResource::default();
    run_checked_in_gallery_stories_for_render_target(
        default_gallery_proof_size(),
        &ThemeTokens::default(),
        &atlas,
    )
}

pub fn run_checked_in_gallery_stories_for_render_target(
    size: UiSize,
    theme: &ThemeTokens,
    atlas: &UiFontAtlasResource,
) -> Vec<UiGalleryStoryExecution> {
    let registry = match checked_in_story_registry_v2() {
        Ok(registry) => registry,
        Err(report) => {
            return report
                .diagnostics
                .into_iter()
                .map(registry_failure_execution)
                .collect();
        }
    };

    let runner = UiStoryRunnerV2::new(&registry);
    let control_registry = ControlPackageRegistry::new()
        .with_package(runenwerk_control_package())
        .expect("runenwerk controls package should register");
    let snapshot = control_registry.snapshot();

    registry
        .stories()
        .map(|story| {
            execute_gallery_story(story, &registry, &runner, &snapshot, size, theme, atlas)
        })
        .collect()
}

pub fn submit_ui_gallery_frame_system(
    window: Res<WindowState>,
    atlas: Res<UiFontAtlasResource>,
    mut gallery: ResMut<UiGalleryResource>,
    mut submissions: ResMut<SurfaceFrameSubmissionRegistryResource>,
) {
    let size = UiSize::new(window.size_px.0 as f32, window.size_px.1 as f32);
    gallery.prepare_frame_if_needed(size, &ThemeTokens::default(), &atlas);
    gallery.log_diagnostics_once();

    if let Some(frame) = gallery.frame().cloned() {
        submissions.replace(
            SurfaceFrameSubmission::new(UI_GALLERY_UI_PRODUCER_ID)
                .with_route(SurfaceFrameRoute::Screen)
                .with_order(SurfaceFrameSubmissionOrder::new(10, 0))
                .with_frame(frame),
        );
    } else {
        submissions.remove(&UI_GALLERY_UI_PRODUCER_ID);
    }
}
