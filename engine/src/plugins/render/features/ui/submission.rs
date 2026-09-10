use crate::plugins::render::api::ids::RenderFrameProducerId;
use crate::plugins::render::backend::RenderSurfaceId;
use std::collections::BTreeMap;
use ui_render_data::UiFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum SurfaceFrameRoute {
    #[default]
    Screen,
    ViewportOverlay,
    WorldProjected,
}

impl SurfaceFrameRoute {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Screen => "screen",
            Self::ViewportOverlay => "viewport_overlay",
            Self::WorldProjected => "world_projected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SurfaceFrameSubmissionOrder {
    pub layer: i32,
    pub priority: i32,
}

impl SurfaceFrameSubmissionOrder {
    pub const fn new(layer: i32, priority: i32) -> Self {
        Self { layer, priority }
    }
}

#[derive(Debug, Clone)]
pub struct SurfaceFrameSubmission {
    pub producer_id: RenderFrameProducerId,
    pub render_surface_id: Option<RenderSurfaceId>,
    pub route: SurfaceFrameRoute,
    pub order: SurfaceFrameSubmissionOrder,
    pub frame: UiFrame,
    pub rect_shader_asset_id: Option<String>,
}

impl SurfaceFrameSubmission {
    pub fn new(producer_id: impl Into<RenderFrameProducerId>) -> Self {
        Self {
            producer_id: producer_id.into(),
            render_surface_id: None,
            route: SurfaceFrameRoute::Screen,
            order: SurfaceFrameSubmissionOrder::default(),
            frame: UiFrame::default(),
            rect_shader_asset_id: None,
        }
    }

    pub fn with_route(mut self, route: SurfaceFrameRoute) -> Self {
        self.route = route;
        self
    }

    pub fn with_render_surface(mut self, render_surface_id: RenderSurfaceId) -> Self {
        self.render_surface_id = Some(render_surface_id);
        self
    }

    pub fn with_order(mut self, order: SurfaceFrameSubmissionOrder) -> Self {
        self.order = order;
        self
    }

    pub fn with_frame(mut self, frame: UiFrame) -> Self {
        self.frame = frame;
        self
    }

    pub fn with_rect_shader_asset_id(
        mut self,
        rect_shader_asset_id: impl Into<Option<String>>,
    ) -> Self {
        self.rect_shader_asset_id = rect_shader_asset_id.into();
        self
    }

    pub fn primitive_count_hint(&self) -> usize {
        self.frame
            .surfaces
            .iter()
            .map(|surface| {
                surface
                    .layers
                    .iter()
                    .map(|layer| layer.primitives.len())
                    .sum::<usize>()
            })
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.frame.is_empty()
    }
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct SurfaceFrameSubmissionRegistryResource {
    submissions: BTreeMap<(RenderFrameProducerId, Option<RenderSurfaceId>), SurfaceFrameSubmission>,
}

impl SurfaceFrameSubmissionRegistryResource {
    pub fn submission_count(&self) -> usize {
        self.submissions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.submissions.is_empty()
    }

    pub fn clear(&mut self) {
        self.submissions.clear();
    }

    pub fn replace(
        &mut self,
        submission: SurfaceFrameSubmission,
    ) -> Option<SurfaceFrameSubmission> {
        self.submissions.insert(
            (submission.producer_id, submission.render_surface_id),
            submission,
        )
    }

    pub fn replace_for_producer(
        &mut self,
        producer_id: impl Into<RenderFrameProducerId>,
        build: impl FnOnce(RenderFrameProducerId) -> SurfaceFrameSubmission,
    ) -> Option<SurfaceFrameSubmission> {
        let producer_id = producer_id.into();
        let submission = build(producer_id);
        debug_assert_eq!(
            submission.producer_id, producer_id,
            "submission producer_id must match replace_for_producer key",
        );
        self.submissions.insert((producer_id, None), submission)
    }

    pub fn replace_for_surface(
        &mut self,
        producer_id: impl Into<RenderFrameProducerId>,
        render_surface_id: RenderSurfaceId,
        build: impl FnOnce(RenderFrameProducerId) -> SurfaceFrameSubmission,
    ) -> Option<SurfaceFrameSubmission> {
        let producer_id = producer_id.into();
        let submission = build(producer_id).with_render_surface(render_surface_id);
        debug_assert_eq!(submission.producer_id, producer_id);
        self.submissions
            .insert((producer_id, Some(render_surface_id)), submission)
    }

    pub fn remove(
        &mut self,
        producer_id: &RenderFrameProducerId,
    ) -> Option<SurfaceFrameSubmission> {
        self.submissions.remove(&(*producer_id, None))
    }

    pub fn remove_for_surface(
        &mut self,
        producer_id: &RenderFrameProducerId,
        render_surface_id: RenderSurfaceId,
    ) -> Option<SurfaceFrameSubmission> {
        self.submissions
            .remove(&(*producer_id, Some(render_surface_id)))
    }

    pub fn get(&self, producer_id: &RenderFrameProducerId) -> Option<&SurfaceFrameSubmission> {
        self.submissions.get(&(*producer_id, None))
    }

    pub fn get_for_surface(
        &self,
        producer_id: &RenderFrameProducerId,
        render_surface_id: RenderSurfaceId,
    ) -> Option<&SurfaceFrameSubmission> {
        self.submissions
            .get(&(*producer_id, Some(render_surface_id)))
    }

    pub fn ordered_submissions(&self) -> Vec<&SurfaceFrameSubmission> {
        let mut values = self.submissions.values().collect::<Vec<_>>();
        sort_submissions(&mut values);
        values
    }

    pub fn ordered_submissions_for_surface(
        &self,
        render_surface_id: RenderSurfaceId,
    ) -> Vec<&SurfaceFrameSubmission> {
        let mut by_producer = BTreeMap::<RenderFrameProducerId, &SurfaceFrameSubmission>::new();
        for submission in self
            .submissions
            .values()
            .filter(|submission| submission.render_surface_id.is_none())
        {
            by_producer.insert(submission.producer_id, submission);
        }
        for submission in self
            .submissions
            .values()
            .filter(|submission| submission.render_surface_id == Some(render_surface_id))
        {
            by_producer.insert(submission.producer_id, submission);
        }
        let mut values = by_producer.into_values().collect::<Vec<_>>();
        sort_submissions(&mut values);
        values
    }
}

fn sort_submissions(values: &mut Vec<&SurfaceFrameSubmission>) {
    values.sort_by(|left, right| {
        left.route
            .cmp(&right.route)
            .then(left.order.layer.cmp(&right.order.layer))
            .then(left.order.priority.cmp(&right.order.priority))
            .then(left.producer_id.cmp(&right.producer_id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_frame_submission_replacement_is_keyed_by_generic_producer() {
        let mut registry = SurfaceFrameSubmissionRegistryResource::default();

        registry.replace(
            SurfaceFrameSubmission::new(RenderFrameProducerId::try_from_raw(1).unwrap())
                .with_order(SurfaceFrameSubmissionOrder::new(10, 0)),
        );
        registry.replace(
            SurfaceFrameSubmission::new(RenderFrameProducerId::try_from_raw(1).unwrap())
                .with_order(SurfaceFrameSubmissionOrder::new(20, 0)),
        );

        assert_eq!(registry.submission_count(), 1);
        let submission = registry
            .get(&RenderFrameProducerId::try_from_raw(1).unwrap())
            .expect("producer submission should exist");
        assert_eq!(submission.order.layer, 20);
    }

    #[test]
    fn surface_frame_submission_ordering_is_deterministic() {
        let mut registry = SurfaceFrameSubmissionRegistryResource::default();

        registry.replace(
            SurfaceFrameSubmission::new(RenderFrameProducerId::try_from_raw(2).unwrap())
                .with_route(SurfaceFrameRoute::Screen)
                .with_order(SurfaceFrameSubmissionOrder::new(100, 5)),
        );
        registry.replace(
            SurfaceFrameSubmission::new(RenderFrameProducerId::try_from_raw(1).unwrap())
                .with_route(SurfaceFrameRoute::Screen)
                .with_order(SurfaceFrameSubmissionOrder::new(10, 0)),
        );
        registry.replace(
            SurfaceFrameSubmission::new(RenderFrameProducerId::try_from_raw(3).unwrap())
                .with_route(SurfaceFrameRoute::ViewportOverlay)
                .with_order(SurfaceFrameSubmissionOrder::new(0, 0)),
        );

        let ordered = registry
            .ordered_submissions()
            .into_iter()
            .map(|value| value.producer_id)
            .collect::<Vec<_>>();

        assert_eq!(
            ordered,
            vec![
                RenderFrameProducerId::try_from_raw(1).unwrap(),
                RenderFrameProducerId::try_from_raw(2).unwrap(),
                RenderFrameProducerId::try_from_raw(3).unwrap(),
            ]
        );
    }

    #[test]
    fn surface_frame_submission_overrides_global_submission_for_same_producer() {
        let mut registry = SurfaceFrameSubmissionRegistryResource::default();
        let producer = RenderFrameProducerId::try_from_raw(1).unwrap();
        let primary = RenderSurfaceId::primary();
        registry.replace(
            SurfaceFrameSubmission::new(producer)
                .with_order(SurfaceFrameSubmissionOrder::new(1, 0)),
        );
        registry.replace_for_surface(producer, primary, |producer_id| {
            SurfaceFrameSubmission::new(producer_id)
                .with_order(SurfaceFrameSubmissionOrder::new(2, 0))
        });

        let submissions = registry.ordered_submissions_for_surface(primary);
        assert_eq!(submissions.len(), 1);
        assert_eq!(submissions[0].order.layer, 2);
        assert_eq!(submissions[0].render_surface_id, Some(primary));
    }

    #[test]
    fn surface_frame_submission_removes_surface_scoped_submission() {
        let mut registry = SurfaceFrameSubmissionRegistryResource::default();
        let producer = RenderFrameProducerId::try_from_raw(1).unwrap();
        let primary = RenderSurfaceId::primary();
        registry.replace_for_surface(producer, primary, SurfaceFrameSubmission::new);

        let removed = registry.remove_for_surface(&producer, primary);

        assert!(removed.is_some());
        assert!(registry.get_for_surface(&producer, primary).is_none());
        assert!(registry.is_empty());
    }
}
