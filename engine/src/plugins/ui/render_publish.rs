use crate::plugins::render::backend::RenderSurfaceId;
use crate::plugins::render::{
    RenderFrameProducerId, SurfaceFrameRoute, SurfaceFrameSubmission, SurfaceFrameSubmissionOrder,
    SurfaceFrameSubmissionRegistryResource,
};
use crate::runtime::{Res, ResMut};
use ui_math::{UiRect, UiSize};
use ui_render_data::{
    RectPrimitive, UiDrawKey, UiFrame, UiLayer, UiLayerId, UiPaint, UiPrimitive, UiSortKey,
    UiSurface, UiSurfaceId,
};

use super::{
    UiRuntimeDiagnostic, UiRuntimeDiagnosticsResource, UiRuntimeEvaluationResource,
    UiRuntimeFramePayload, UiRuntimeFramePublicationFailureReason, UiRuntimeFramePublicationReport,
    UiRuntimeFramePublicationResource, UiRuntimeTraceEvent, UiRuntimeTraceResource,
};

pub const UI_RUNTIME_FRAME_PRODUCER_ID: RenderFrameProducerId =
    ui_runtime_frame_producer_id(10_000);
const UI_RUNTIME_FRAME_DRAW_KEY: UiDrawKey = UiDrawKey::new(10_000, None);

const fn ui_runtime_frame_producer_id(raw: u64) -> RenderFrameProducerId {
    match RenderFrameProducerId::try_from_raw(raw) {
        Ok(id) => id,
        Err(_) => panic!("UI runtime frame producer id constants must be non-zero"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, runen_ecs::Resource)]
pub struct UiRuntimeFramePublicationTarget {
    producer_id: RenderFrameProducerId,
    render_surface_id: RenderSurfaceId,
    route: SurfaceFrameRoute,
    order: SurfaceFrameSubmissionOrder,
}

impl Default for UiRuntimeFramePublicationTarget {
    fn default() -> Self {
        Self {
            producer_id: UI_RUNTIME_FRAME_PRODUCER_ID,
            render_surface_id: RenderSurfaceId::primary(),
            route: SurfaceFrameRoute::Screen,
            order: SurfaceFrameSubmissionOrder::new(20, 0),
        }
    }
}

impl UiRuntimeFramePublicationTarget {
    pub fn new(producer_id: RenderFrameProducerId, render_surface_id: RenderSurfaceId) -> Self {
        Self {
            producer_id,
            render_surface_id,
            ..Self::default()
        }
    }

    pub fn producer_id(&self) -> RenderFrameProducerId {
        self.producer_id
    }

    pub fn render_surface_id(&self) -> RenderSurfaceId {
        self.render_surface_id
    }

    pub fn route(&self) -> SurfaceFrameRoute {
        self.route
    }

    pub fn order(&self) -> SurfaceFrameSubmissionOrder {
        self.order
    }

    pub fn with_route(mut self, route: SurfaceFrameRoute) -> Self {
        self.route = route;
        self
    }

    pub fn with_order(mut self, order: SurfaceFrameSubmissionOrder) -> Self {
        self.order = order;
        self
    }
}

pub fn publish_ui_runtime_frame_system(
    runtime: Res<UiRuntimeEvaluationResource>,
    target: Res<UiRuntimeFramePublicationTarget>,
    mut submissions: ResMut<SurfaceFrameSubmissionRegistryResource>,
    mut publications: ResMut<UiRuntimeFramePublicationResource>,
    mut trace: ResMut<UiRuntimeTraceResource>,
    mut diagnostics: ResMut<UiRuntimeDiagnosticsResource>,
) {
    publish_latest_ui_runtime_frame(
        &runtime,
        &target,
        &mut submissions,
        &mut publications,
        &mut trace,
        &mut diagnostics,
    );
}

pub fn publish_latest_ui_runtime_frame(
    runtime: &UiRuntimeEvaluationResource,
    target: &UiRuntimeFramePublicationTarget,
    submissions: &mut SurfaceFrameSubmissionRegistryResource,
    publications: &mut UiRuntimeFramePublicationResource,
    trace: &mut UiRuntimeTraceResource,
    diagnostics: &mut UiRuntimeDiagnosticsResource,
) -> UiRuntimeFramePublicationReport {
    let Some(evaluation) = runtime.latest_report() else {
        let producer_id = target.producer_id();
        let render_surface_id = target.render_surface_id();
        submissions.remove_for_surface(&producer_id, render_surface_id);
        let report = UiRuntimeFramePublicationReport::missing_runtime_evaluation(
            producer_id,
            render_surface_id,
        );
        diagnostics.push(UiRuntimeDiagnostic::frame_publication_rejected(
            producer_id,
            render_surface_id,
            UiRuntimeFramePublicationFailureReason::MissingRuntimeEvaluation,
        ));
        trace.record(UiRuntimeTraceEvent::frame_published(&report));
        publications.record(report.clone());
        return report;
    };

    let frame = frame_from_payload(evaluation.frame_payload());
    submissions.replace_for_surface(
        target.producer_id(),
        target.render_surface_id(),
        |producer_id| {
            SurfaceFrameSubmission::new(producer_id)
                .with_route(target.route())
                .with_order(target.order())
                .with_frame(frame)
        },
    );

    let report = UiRuntimeFramePublicationReport::published(
        evaluation,
        target.producer_id(),
        target.render_surface_id(),
    );
    trace.record(UiRuntimeTraceEvent::frame_published(&report));
    trace.record(UiRuntimeTraceEvent::frame_presented(&report));
    publications.record(report.clone());
    report
}

fn frame_from_payload(payload: &UiRuntimeFramePayload) -> UiFrame {
    let primitive_count = payload.primitive_count();
    if primitive_count == 0 {
        return UiFrame::new();
    }

    let primitives = (0..primitive_count)
        .map(|index| {
            let order = index.min(u32::MAX as usize) as u32;
            UiPrimitive::Rect(RectPrimitive::new(
                UiRect::new(index as f32, 0.0, 1.0, 1.0),
                0.0,
                UiPaint::rgba(0.16, 0.22, 0.28, 1.0),
                UI_RUNTIME_FRAME_DRAW_KEY,
                UiSortKey::new(0, 0, order),
            ))
        })
        .collect::<Vec<_>>();

    UiFrame::with_surfaces(vec![UiSurface::with_layers(
        UiSurfaceId(0),
        UiSize::new(primitive_count as f32, 1.0),
        vec![UiLayer::with_primitives(UiLayerId(0), primitives)],
    )])
}
