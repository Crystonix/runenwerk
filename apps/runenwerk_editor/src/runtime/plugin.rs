use ecs::{SystemSetKey, World};
use engine::plugins::render::backend::RenderSurfaceRegistryResource;
use engine::plugins::render::{
    PreparedRenderProductSelectionResource, RenderDynamicTextureTargetRequestRegistryResource,
    RenderDynamicTextureUploadRegistryResource, RenderGpuResidencyBudgetResource,
    RenderGpuResidencyResource, RenderRuntimeSet, SurfaceFrameSubmissionRegistryResource,
};
use engine::prelude::*;
use engine::runtime::{CoreSet, IntoSystemSetKey, SystemConfigExt, WindowStateRegistryResource};
use engine::runtime::{ProductPublicationRuntimeResource, PublicationBoundary};

use crate::asset_pipeline::publish_pending_field_product_publications;
use crate::material_lab::publish_pending_material_preview_publications;
use crate::runtime::composition::{
    EditorCompositionTransitionRuntimeResource, EditorTargetInputRuntimeResource,
    dispatch_editor_target_input_system, sync_editor_composition_transitions_system,
};
use crate::runtime::procgen::{
    publish_procgen_products_at_boundary, publish_procgen_query_snapshots_at_boundary,
    sync_procgen_viewport_overlay_system,
};
use crate::runtime::resources::{
    EditorHostResource, EditorInputBridgeState, EditorViewportRenderState,
    RuntimePreviewProcessResource,
};
use crate::runtime::systems::{
    bootstrap_editor_demo_system, dispatch_editor_input_system,
    prepare_material_preview_render_resource_system, produce_editor_picking_system,
    produce_material_preview_dynamic_uploads_system,
    produce_texture_preview_dynamic_uploads_system, seed_viewport_runtime_contracts_system,
    submit_editor_frame_system, sync_viewport_instances_system,
};
use crate::runtime::viewport::{
    MountedSurfaceRegistryResource, SurfaceDefinitionRegistryResource,
    ToolSurfaceRuntimeBindingRegistryResource, ViewportArtifactObservationResource,
    ViewportInstanceRegistryResource, ViewportLayoutMapResource, ViewportPickingResultsResource,
    ViewportPresentationStateResource, ViewportProductRegistryResource,
    ViewportProductTargetRegistryResource, ViewportRenderJobResource,
    ViewportRenderStateCommandQueueResource, ViewportRenderStateResource,
    ViewportRuntimeSettingsHydrationResource, ViewportSurfaceSetResource,
    apply_viewport_render_state_commands_system, prepare_viewport_render_product_selections_system,
    publish_viewport_query_snapshots_at_boundary, summarize_viewport_gpu_residency_system,
    sync_viewport_presentation_products_system, sync_viewport_product_targets_system,
    sync_viewport_render_jobs_system,
};
use crate::shell::EditorWindowPresentationBinding;

pub struct EditorAppPlugin;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EditorRuntimeSet {
    Picking,
    InputBridge,
    CompositionTransitions,
    TargetInput,
    WindowPresentationRequests,
    ViewportLifecycle,
    FrameSubmit,
    ViewportRenderStateCommands,
    ViewportPresentationSync,
    ProcgenViewportOverlay,
    ViewportProductTargets,
    ViewportRenderJobs,
    ViewportRenderProductSelection,
    MaterialPreviewRenderHandoff,
    MaterialPreviewProductUpload,
    TexturePreviewProductUpload,
    ViewportGpuResidencySummary,
}

impl IntoSystemSetKey for EditorRuntimeSet {
    fn system_set_key(&self) -> SystemSetKey {
        match self {
            Self::Picking => SystemSetKey::of::<EditorRuntimeSet>("EditorRuntimeSet::Picking"),
            Self::InputBridge => {
                SystemSetKey::of::<EditorRuntimeSet>("EditorRuntimeSet::InputBridge")
            }
            Self::CompositionTransitions => {
                SystemSetKey::of::<EditorRuntimeSet>("EditorRuntimeSet::CompositionTransitions")
            }
            Self::TargetInput => {
                SystemSetKey::of::<EditorRuntimeSet>("EditorRuntimeSet::TargetInput")
            }
            Self::WindowPresentationRequests => {
                SystemSetKey::of::<EditorRuntimeSet>("EditorRuntimeSet::WindowPresentationRequests")
            }
            Self::ViewportLifecycle => {
                SystemSetKey::of::<EditorRuntimeSet>("EditorRuntimeSet::ViewportLifecycle")
            }
            Self::FrameSubmit => {
                SystemSetKey::of::<EditorRuntimeSet>("EditorRuntimeSet::FrameSubmit")
            }
            Self::ViewportRenderStateCommands => SystemSetKey::of::<EditorRuntimeSet>(
                "EditorRuntimeSet::ViewportRenderStateCommands",
            ),
            Self::ViewportPresentationSync => {
                SystemSetKey::of::<EditorRuntimeSet>("EditorRuntimeSet::ViewportPresentationSync")
            }
            Self::ProcgenViewportOverlay => {
                SystemSetKey::of::<EditorRuntimeSet>("EditorRuntimeSet::ProcgenViewportOverlay")
            }
            Self::ViewportProductTargets => {
                SystemSetKey::of::<EditorRuntimeSet>("EditorRuntimeSet::ViewportProductTargets")
            }
            Self::ViewportRenderJobs => {
                SystemSetKey::of::<EditorRuntimeSet>("EditorRuntimeSet::ViewportRenderJobs")
            }
            Self::ViewportRenderProductSelection => SystemSetKey::of::<EditorRuntimeSet>(
                "EditorRuntimeSet::ViewportRenderProductSelection",
            ),
            Self::MaterialPreviewRenderHandoff => SystemSetKey::of::<EditorRuntimeSet>(
                "EditorRuntimeSet::MaterialPreviewRenderHandoff",
            ),
            Self::MaterialPreviewProductUpload => SystemSetKey::of::<EditorRuntimeSet>(
                "EditorRuntimeSet::MaterialPreviewProductUpload",
            ),
            Self::TexturePreviewProductUpload => SystemSetKey::of::<EditorRuntimeSet>(
                "EditorRuntimeSet::TexturePreviewProductUpload",
            ),
            Self::ViewportGpuResidencySummary => SystemSetKey::of::<EditorRuntimeSet>(
                "EditorRuntimeSet::ViewportGpuResidencySummary",
            ),
        }
    }
}

impl Plugin for EditorAppPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorHostResource>();
        app.init_resource::<RuntimePreviewProcessResource>();
        app.init_resource::<EditorInputBridgeState>();
        app.init_resource::<EditorViewportRenderState>();
        app.init_resource::<EditorCompositionTransitionRuntimeResource>();
        app.init_resource::<EditorTargetInputRuntimeResource>();
        app.init_resource::<ViewportProductRegistryResource>();
        app.init_resource::<ViewportProductTargetRegistryResource>();
        app.init_resource::<ViewportPresentationStateResource>();
        app.init_resource::<ViewportRuntimeSettingsHydrationResource>();
        app.init_resource::<ViewportArtifactObservationResource>();
        app.init_resource::<ViewportRenderJobResource>();
        app.init_resource::<ViewportRenderStateResource>();
        app.init_resource::<ViewportRenderStateCommandQueueResource>();
        app.init_resource::<ViewportInstanceRegistryResource>();
        app.init_resource::<ViewportLayoutMapResource>();
        app.init_resource::<ToolSurfaceRuntimeBindingRegistryResource>();
        app.init_resource::<SurfaceDefinitionRegistryResource>();
        app.init_resource::<MountedSurfaceRegistryResource>();
        app.init_resource::<ViewportSurfaceSetResource>();
        app.init_resource::<ViewportPickingResultsResource>();
        app.init_resource::<SurfaceFrameSubmissionRegistryResource>();
        app.init_resource::<RenderDynamicTextureTargetRequestRegistryResource>();
        app.init_resource::<RenderDynamicTextureUploadRegistryResource>();
        app.init_resource::<PreparedRenderProductSelectionResource>();
        app.init_resource::<RenderGpuResidencyResource>();
        app.init_resource::<RenderGpuResidencyBudgetResource>();
        app.add_product_publication_handler(publish_editor_field_products_at_boundary);
        app.add_product_publication_handler(publish_procgen_products_at_boundary);
        app.add_product_publication_handler(publish_editor_material_preview_products_at_boundary);
        app.add_query_snapshot_publication_handler(publish_viewport_query_snapshots_at_boundary);
        app.add_query_snapshot_publication_handler(publish_procgen_query_snapshots_at_boundary);

        app.add_systems(Startup, bootstrap_editor_demo_system);
        app.add_systems(Startup, seed_viewport_runtime_contracts_system);
        app.add_systems(
            Update,
            produce_editor_picking_system
                .in_set(EditorRuntimeSet::Picking)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            Update,
            dispatch_editor_input_system
                .in_set(EditorRuntimeSet::InputBridge)
                .after(EditorRuntimeSet::Picking)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            Update,
            apply_viewport_render_state_commands_system
                .in_set(EditorRuntimeSet::ViewportRenderStateCommands)
                .after(EditorRuntimeSet::InputBridge)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            Update,
            sync_editor_composition_transitions_system
                .in_set(EditorRuntimeSet::CompositionTransitions)
                .after(EditorRuntimeSet::InputBridge)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            Update,
            dispatch_editor_target_input_system
                .in_set(EditorRuntimeSet::TargetInput)
                .after(EditorRuntimeSet::CompositionTransitions)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            Update,
            sync_editor_window_presentation_requests_system
                .in_set(EditorRuntimeSet::WindowPresentationRequests)
                .after(EditorRuntimeSet::TargetInput)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            Update,
            sync_viewport_instances_system
                .in_set(EditorRuntimeSet::ViewportLifecycle)
                .after(EditorRuntimeSet::ViewportRenderStateCommands)
                .after(EditorRuntimeSet::WindowPresentationRequests)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            Update,
            submit_editor_frame_system
                .in_set(EditorRuntimeSet::FrameSubmit)
                .after(EditorRuntimeSet::ViewportLifecycle)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            Update,
            sync_viewport_presentation_products_system
                .in_set(EditorRuntimeSet::ViewportPresentationSync)
                .after(EditorRuntimeSet::FrameSubmit)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            Update,
            sync_procgen_viewport_overlay_system
                .in_set(EditorRuntimeSet::ProcgenViewportOverlay)
                .after(EditorRuntimeSet::ViewportPresentationSync)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            Update,
            sync_viewport_product_targets_system
                .in_set(EditorRuntimeSet::ViewportProductTargets)
                .after(EditorRuntimeSet::ProcgenViewportOverlay)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            Update,
            sync_viewport_render_jobs_system
                .in_set(EditorRuntimeSet::ViewportRenderJobs)
                .after(EditorRuntimeSet::ViewportProductTargets)
                .after(CoreSet::Input)
                .after(CoreSet::Time),
        );
        app.add_systems(
            RenderPrepare,
            prepare_viewport_render_product_selections_system
                .in_set(EditorRuntimeSet::ViewportRenderProductSelection)
                .before(RenderRuntimeSet::GpuResidency),
        );
        app.add_systems(
            RenderPrepare,
            prepare_material_preview_render_resource_system
                .in_set(EditorRuntimeSet::MaterialPreviewRenderHandoff)
                .after(EditorRuntimeSet::ViewportRenderProductSelection)
                .before(RenderRuntimeSet::FramePrepare),
        );
        app.add_systems(
            RenderPrepare,
            produce_material_preview_dynamic_uploads_system
                .in_set(EditorRuntimeSet::MaterialPreviewProductUpload)
                .after(EditorRuntimeSet::MaterialPreviewRenderHandoff)
                .before(RenderRuntimeSet::FramePrepare),
        );
        app.add_systems(
            RenderPrepare,
            produce_texture_preview_dynamic_uploads_system
                .in_set(EditorRuntimeSet::TexturePreviewProductUpload)
                .after(EditorRuntimeSet::MaterialPreviewProductUpload)
                .before(RenderRuntimeSet::FramePrepare),
        );
        app.add_systems(
            RenderPrepare,
            summarize_viewport_gpu_residency_system
                .in_set(EditorRuntimeSet::ViewportGpuResidencySummary)
                .after(RenderRuntimeSet::GpuResidency)
                .before(RenderRuntimeSet::FramePrepare),
        );
    }
}

pub(crate) fn sync_editor_window_presentation_requests_system(
    mut host: ResMut<EditorHostResource>,
    mut window_registry: ResMut<WindowStateRegistryResource>,
    mut surface_registry: ResMut<RenderSurfaceRegistryResource>,
) {
    let _ = sync_editor_window_presentation_requests(
        &mut host,
        &mut window_registry,
        &mut surface_registry,
    );
}

fn sync_editor_window_presentation_requests(
    host: &mut EditorHostResource,
    window_registry: &mut WindowStateRegistryResource,
    surface_registry: &mut RenderSurfaceRegistryResource,
) -> usize {
    let pending = host.shell_state.drain_pending_editor_window_presentations();
    let synced = pending.len();
    for editor_window_id in pending {
        let request = window_registry
            .request_window(format!("Runenwerk {}", editor_window_id.raw()), (1280, 720));
        let render_surface_id = surface_registry
            .ensure_surface_for_native_window(request.native_window_id, request.size_px);
        let binding = EditorWindowPresentationBinding {
            native_window_id: request.native_window_id,
            render_surface_id,
        };
        let _ = host
            .shell_state
            .bind_editor_window_presentation(editor_window_id, binding);
    }
    synced
}

fn publish_editor_field_products_at_boundary(
    boundary: &PublicationBoundary,
    world: &mut World,
) -> anyhow::Result<()> {
    let Some(mut host) = world.remove_resource::<EditorHostResource>() else {
        return Ok(());
    };
    let Some(mut publications) = world.remove_resource::<ProductPublicationRuntimeResource>()
    else {
        world.insert_resource(host);
        return Ok(());
    };

    publish_pending_field_product_publications(&mut host.app, &mut publications, boundary);

    world.insert_resource(publications);
    world.insert_resource(host);
    Ok(())
}

fn publish_editor_material_preview_products_at_boundary(
    boundary: &PublicationBoundary,
    world: &mut World,
) -> anyhow::Result<()> {
    let Some(mut host) = world.remove_resource::<EditorHostResource>() else {
        return Ok(());
    };
    let Some(mut publications) = world.remove_resource::<ProductPublicationRuntimeResource>()
    else {
        world.insert_resource(host);
        return Ok(());
    };

    publish_pending_material_preview_publications(&mut host.app, &mut publications, boundary);

    world.insert_resource(publications);
    world.insert_resource(host);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::runtime::{NativeWindowId, WindowState};

    #[test]
    fn window_presentation_requests_bind_editor_windows_to_native_surfaces() {
        let mut host = EditorHostResource::default();
        let editor_window_id = host.shell_state.open_editor_window_for_active_workspace();
        let mut window_registry =
            WindowStateRegistryResource::from_legacy(&WindowState::windowed("Runenwerk"));
        let mut surface_registry = RenderSurfaceRegistryResource::default();

        let synced = sync_editor_window_presentation_requests(
            &mut host,
            &mut window_registry,
            &mut surface_registry,
        );

        assert_eq!(synced, 1);
        let binding = host
            .shell_state
            .editor_window_binding(editor_window_id)
            .expect("secondary editor window should be bound to presentation ids");
        assert_ne!(binding.native_window_id, NativeWindowId::primary());
        assert!(window_registry.record(binding.native_window_id).is_some());
        assert_eq!(
            surface_registry.surface_for_native_window(binding.native_window_id),
            Some(binding.render_surface_id)
        );
        assert_eq!(window_registry.pending_creation_requests().len(), 1);
    }
}
