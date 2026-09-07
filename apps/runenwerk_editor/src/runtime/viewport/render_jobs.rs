//! File: apps/runenwerk_editor/src/runtime/viewport/render_jobs.rs
//! Purpose: Per-viewport render jobs bound to dynamic product targets.

use std::collections::BTreeMap;

use editor_viewport::{ExpressionDimensions, ViewportId, ViewportSurfacePresentationSlot};
#[cfg(test)]
use engine::plugins::render::RenderTargetAliasKey;
use engine::plugins::render::{
    PreparedFlowInvocationRequest, PreparedRenderFrameRequestResource, PreparedViewFrame,
    RenderDynamicTextureTargetKey, RenderFlowId, RenderFlowRegistryResource,
    RenderProductSurfaceManifest, RenderProductSurfaceRequest, RenderProductSurfaceRequestBatch,
};
use engine::runtime::{Res, ResMut};
use runen_gpu::GpuWorkResourceId;
use ui_math::UiRect;

use crate::runtime::viewport::{
    EDITOR_MAIN_FLOW_ID, EDITOR_VIEWPORT_RENDER_PRODUCT_PRODUCER_ID,
    EDITOR_VIEWPORT_SCENE_PRODUCT_UNIFORM_ID, OVERLAY_PRODUCT_ID, PICKING_IDS_PRODUCT_ID,
    SCENE_COLOR_PRODUCT_ID, VIEWPORT_TARGET_ALIAS_OVERLAY, VIEWPORT_TARGET_ALIAS_PICKING_IDS,
    VIEWPORT_TARGET_ALIAS_SCENE_COLOR, ViewportProductTargetRegistryResource,
    ViewportRenderStateResource, expression_dimensions_for_bounds,
};

#[derive(Debug)]
pub struct ViewportRenderJob {
    pub viewport_id: ViewportId,
    pub bounds: UiRect,
    pub dimensions: ExpressionDimensions,
    pub scene_color_target: RenderDynamicTextureTargetKey,
    pub picking_ids_target: RenderDynamicTextureTargetKey,
    pub overlay_target: RenderDynamicTextureTargetKey,
    pub product_surface_request: RenderProductSurfaceRequest,
}

impl ViewportRenderJob {
    pub fn prepared_view(&self) -> &PreparedViewFrame {
        self.product_surface_request.view()
    }

    pub fn prepared_flow_invocation(&self) -> &PreparedFlowInvocationRequest {
        self.product_surface_request.flow_invocation()
    }
}

#[derive(Debug, Default, ecs::Component, ecs::Resource)]
pub struct ViewportRenderJobResource {
    jobs_by_viewport: BTreeMap<ViewportId, ViewportRenderJob>,
}

impl ViewportRenderJobResource {
    pub fn replace_jobs(&mut self, jobs: impl IntoIterator<Item = ViewportRenderJob>) {
        self.jobs_by_viewport = jobs.into_iter().map(|job| (job.viewport_id, job)).collect();
    }

    pub fn job_for(&self, viewport_id: ViewportId) -> Option<&ViewportRenderJob> {
        self.jobs_by_viewport.get(&viewport_id)
    }

    pub fn jobs(&self) -> impl Iterator<Item = &ViewportRenderJob> {
        self.jobs_by_viewport.values()
    }

    pub fn viewport_ids(&self) -> impl Iterator<Item = ViewportId> + '_ {
        self.jobs_by_viewport.keys().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs_by_viewport.is_empty()
    }
}

pub fn sync_viewport_render_jobs_system(
    flow_registry: Res<RenderFlowRegistryResource>,
    viewport_render_states: Res<ViewportRenderStateResource>,
    viewport_product_targets: Res<ViewportProductTargetRegistryResource>,
    mut viewport_render_jobs: ResMut<ViewportRenderJobResource>,
    mut prepared_frame_requests: ResMut<PreparedRenderFrameRequestResource>,
) {
    let Some((flow_id, scene_uniform_id)) = editor_main_flow_ids(&flow_registry) else {
        viewport_render_jobs.replace_jobs(Vec::new());
        let _ =
            prepared_frame_requests.remove_contribution(EDITOR_VIEWPORT_RENDER_PRODUCT_PRODUCER_ID);
        return;
    };

    let jobs = viewport_render_states
        .entries()
        .filter_map(|state| {
            build_viewport_render_job(
                flow_id,
                scene_uniform_id,
                &state.render_state,
                state.viewport_id,
                state.bounds,
                &viewport_product_targets,
            )
        })
        .collect::<Vec<_>>();
    let batch = RenderProductSurfaceRequestBatch::from_requests(
        jobs.iter().map(|job| job.product_surface_request.clone()),
    );
    let manifest = RenderProductSurfaceManifest::from_request_batch(
        EDITOR_VIEWPORT_RENDER_PRODUCT_PRODUCER_ID,
        "editor.viewport",
        batch,
    );
    debug_assert!(
        !manifest.has_error_diagnostics(),
        "viewport product-surface manifest should be structurally valid"
    );
    let (_, _, views, invocations) = manifest.into_render_parts();
    prepared_frame_requests
        .replace_contribution(
            EDITOR_VIEWPORT_RENDER_PRODUCT_PRODUCER_ID,
            views,
            invocations,
        )
        .expect("editor viewport prepared frame contribution must be unique");
    viewport_render_jobs.replace_jobs(jobs);
}

fn build_viewport_render_job(
    flow_id: RenderFlowId,
    scene_uniform_id: GpuWorkResourceId,
    viewport_render: &crate::runtime::resources::EditorViewportRenderState,
    viewport_id: ViewportId,
    bounds: UiRect,
    product_targets: &ViewportProductTargetRegistryResource,
) -> Option<ViewportRenderJob> {
    let dimensions = expression_dimensions_for_bounds(bounds);
    let scene_color_target = product_targets
        .record_for_product(
            viewport_id,
            ViewportSurfacePresentationSlot::Primary,
            SCENE_COLOR_PRODUCT_ID,
        )?
        .dynamic_key();
    let picking_ids_target = product_targets
        .record_for_product(
            viewport_id,
            ViewportSurfacePresentationSlot::Picking,
            PICKING_IDS_PRODUCT_ID,
        )?
        .dynamic_key();
    let overlay_target = product_targets
        .record_for_product(
            viewport_id,
            ViewportSurfacePresentationSlot::Overlay,
            OVERLAY_PRODUCT_ID,
        )?
        .dynamic_key();

    let view_id = prepared_view_id(viewport_id);
    let prepared_view = PreparedViewFrame::offscreen_product(
        view_id.clone(),
        (dimensions.width, dimensions.height),
    );
    let prepared_flow_invocation = PreparedFlowInvocationRequest::new(
        format!("editor.viewport.{}.{}", viewport_id.0, EDITOR_MAIN_FLOW_ID),
        flow_id,
        view_id,
    )
    .bind_dynamic_texture_alias(
        VIEWPORT_TARGET_ALIAS_SCENE_COLOR,
        scene_color_target.clone(),
    )
    .expect("scene-color alias key is a valid constant")
    .bind_dynamic_texture_alias(
        VIEWPORT_TARGET_ALIAS_PICKING_IDS,
        picking_ids_target.clone(),
    )
    .expect("picking-IDs alias key is a valid constant")
    .bind_dynamic_texture_alias(VIEWPORT_TARGET_ALIAS_OVERLAY, overlay_target.clone())
    .expect("overlay alias key is a valid constant")
    .with_uniform_override(
        scene_uniform_id,
        viewport_render.compose_scene_product_uniform_bytes((dimensions.width, dimensions.height)),
    );
    let product_surface_request =
        RenderProductSurfaceRequest::new(prepared_view, prepared_flow_invocation);

    Some(ViewportRenderJob {
        viewport_id,
        bounds,
        dimensions,
        scene_color_target,
        picking_ids_target,
        overlay_target,
        product_surface_request,
    })
}

pub fn prepared_view_id(viewport_id: ViewportId) -> String {
    format!("editor.viewport.{}.view", viewport_id.0)
}

fn editor_main_flow_ids(
    flow_registry: &RenderFlowRegistryResource,
) -> Option<(RenderFlowId, GpuWorkResourceId)> {
    let flow = flow_registry
        .compiled_flows()
        .iter()
        .find(|flow| flow.flow_label == EDITOR_MAIN_FLOW_ID)?;
    let uniform_id = flow
        .resource_ids_by_label
        .get(EDITOR_VIEWPORT_SCENE_PRODUCT_UNIFORM_ID)
        .copied()?;
    Some((flow.flow_id, uniform_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::app::test_gpu_work_resource_id;
    use crate::runtime::resources::EditorViewportRenderState;
    use crate::runtime::viewport::{
        ViewportProductRegistryResource, ViewportProductTargetRegistryResource,
        material_preview_descriptor,
    };
    use editor_core::RealityVersion;
    use editor_viewport::{
        ExpressionFormat, ExpressionFreshness, ExpressionPresentationHints,
        ExpressionProductDescriptor, ExpressionProductKind, ExpressionSourceRealityClass,
    };
    use engine::plugins::render::PreparedTargetBinding;

    fn descriptor(
        id: editor_viewport::ExpressionProductId,
        kind: ExpressionProductKind,
        format: ExpressionFormat,
    ) -> ExpressionProductDescriptor {
        ExpressionProductDescriptor::new(
            id,
            kind,
            ExpressionDimensions::new(320, 200),
            format,
            "test.producer",
            ExpressionSourceRealityClass::ObservedScene,
            RealityVersion(1),
            ExpressionFreshness::Current,
            ExpressionPresentationHints::default(),
            None,
        )
    }

    #[test]
    fn prepared_view_ids_are_stable_per_viewport() {
        assert_eq!(prepared_view_id(ViewportId(5)), "editor.viewport.5.view");
    }

    #[test]
    fn render_jobs_require_all_locked_product_targets() {
        let viewport_id = ViewportId(9);
        let mut product_registry = ViewportProductRegistryResource::default();
        product_registry.update_viewport_descriptors(
            viewport_id,
            vec![
                descriptor(
                    SCENE_COLOR_PRODUCT_ID,
                    ExpressionProductKind::SceneColor2D,
                    ExpressionFormat::Rgba8Unorm,
                ),
                descriptor(
                    PICKING_IDS_PRODUCT_ID,
                    ExpressionProductKind::PickingIds2D,
                    ExpressionFormat::R32Uint,
                ),
                descriptor(
                    OVERLAY_PRODUCT_ID,
                    ExpressionProductKind::Overlay2D,
                    ExpressionFormat::Rgba8Unorm,
                ),
            ],
        );
        let target_registry = ViewportProductTargetRegistryResource::from_descriptors_for_viewport(
            viewport_id,
            product_registry
                .descriptors_for(viewport_id)
                .expect("descriptors should exist"),
        );
        let flow_id = RenderFlowId::try_from_raw(1).expect("test flow id should be valid");
        let scene_uniform_id = test_gpu_work_resource_id("scene_uniform");
        let viewport_render = EditorViewportRenderState::default();
        let job = build_viewport_render_job(
            flow_id,
            scene_uniform_id,
            &viewport_render,
            viewport_id,
            UiRect::new(0.0, 0.0, 320.0, 200.0),
            &target_registry,
        )
        .expect("job should exist when all product targets exist");

        assert_eq!(job.viewport_id, viewport_id);
        assert_eq!(job.dimensions, ExpressionDimensions::new(320, 200));
        assert_eq!(
            job.prepared_view().view_id,
            prepared_view_id(viewport_id),
            "viewport jobs must carry the shared product-surface helper view"
        );
        assert!(
            job.product_surface_request.dynamic_targets().is_empty(),
            "viewport dynamic targets are published by the viewport target producer, not the request helper"
        );
        let scene_target = target_registry
            .record_for_product(
                viewport_id,
                ViewportSurfacePresentationSlot::Primary,
                SCENE_COLOR_PRODUCT_ID,
            )
            .expect("scene target should exist")
            .dynamic_key();
        assert_eq!(job.scene_color_target, scene_target);
        assert_eq!(
            job.prepared_flow_invocation().target_alias_bindings.get(
                &RenderTargetAliasKey::new(VIEWPORT_TARGET_ALIAS_SCENE_COLOR)
                    .expect("scene-color alias key is a valid constant"),
            ),
            Some(&PreparedTargetBinding::DynamicTexture(scene_target)),
        );
        assert_eq!(
            job.prepared_flow_invocation()
                .uniform_overrides
                .get(&scene_uniform_id),
            Some(&viewport_render.compose_scene_product_uniform_bytes((320, 200))),
            "viewport render jobs must carry target-local scene uniforms in the prepared invocation"
        );
    }

    #[test]
    fn selected_material_preview_does_not_retarget_scene_render_alias() {
        let viewport_id = ViewportId(9);
        let material_product_id = editor_viewport::ExpressionProductId(42);
        let mut descriptors = vec![
            descriptor(
                SCENE_COLOR_PRODUCT_ID,
                ExpressionProductKind::SceneColor2D,
                ExpressionFormat::Rgba8Unorm,
            ),
            descriptor(
                PICKING_IDS_PRODUCT_ID,
                ExpressionProductKind::PickingIds2D,
                ExpressionFormat::R32Uint,
            ),
            descriptor(
                OVERLAY_PRODUCT_ID,
                ExpressionProductKind::Overlay2D,
                ExpressionFormat::Rgba8Unorm,
            ),
        ];
        descriptors.push(material_preview_descriptor(
            material_product_id,
            ExpressionDimensions::new(320, 200),
            RealityVersion(1),
            "material.first_slice.render_material".to_string(),
        ));
        let target_registry = ViewportProductTargetRegistryResource::from_descriptors_for_viewport(
            viewport_id,
            &descriptors,
        );
        let flow_id = RenderFlowId::try_from_raw(1).expect("test flow id should be valid");
        let scene_uniform_id = test_gpu_work_resource_id("scene_uniform");
        let viewport_render = EditorViewportRenderState::default();

        let job = build_viewport_render_job(
            flow_id,
            scene_uniform_id,
            &viewport_render,
            viewport_id,
            UiRect::new(0.0, 0.0, 320.0, 200.0),
            &target_registry,
        )
        .expect("job should exist when scene/support targets exist");

        let material_target = target_registry
            .record_for_product(
                viewport_id,
                ViewportSurfacePresentationSlot::Primary,
                material_product_id,
            )
            .expect("material target should exist")
            .dynamic_key();
        let scene_target = target_registry
            .record_for_product(
                viewport_id,
                ViewportSurfacePresentationSlot::Primary,
                SCENE_COLOR_PRODUCT_ID,
            )
            .expect("scene target should exist")
            .dynamic_key();
        assert_eq!(job.scene_color_target, scene_target);
        assert_ne!(job.scene_color_target, material_target);
        assert_eq!(
            job.prepared_flow_invocation().target_alias_bindings.get(
                &RenderTargetAliasKey::new(VIEWPORT_TARGET_ALIAS_SCENE_COLOR)
                    .expect("scene-color alias key is a valid constant"),
            ),
            Some(&PreparedTargetBinding::DynamicTexture(scene_target)),
        );
    }
}
