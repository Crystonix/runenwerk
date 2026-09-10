use std::collections::VecDeque;

use editor_shell::{
    EditorCompositionDiagnosticCode as Code, EditorCompositionDiagnosticRecord as Record,
    EditorCompositionDiagnosticStage as Stage, EditorCompositionDiagnosticSubject as Subject,
    EditorCompositionIdentityAllocator, EditorCompositionRejection, EditorDockingIntent,
    EditorWindowId, PreparedEditorCompositionCommit, evaluate_editor_docking_intent,
    plan_editor_docking_transaction, plan_editor_target_close_transaction,
};
use engine::plugins::render::backend::RenderSurfaceRegistryResource;
use engine::runtime::{
    NativeWindowId, NativeWindowLifecycleState, ResMut, WindowStateRegistryResource,
};
use ui_composition::{CompositionPolicies, PresentationTargetId, TargetProfileId};

use crate::runtime::resources::EditorHostResource;
use crate::shell::EditorCompositionPolicy;
use crate::shell::EditorWindowPresentationBinding;

const EDITOR_DESKTOP_TARGET_PROFILE: &str = "runenwerk.editor.desktop";

#[derive(Debug, runen_ecs::Resource, Default)]
pub struct EditorCompositionTransitionRuntimeResource {
    queued: VecDeque<EditorDockingIntent>,
    pending_close_targets: VecDeque<PresentationTargetId>,
    pending: Option<PendingTargetCreation>,
    diagnostics: Vec<Record>,
}

impl EditorCompositionTransitionRuntimeResource {
    pub fn diagnostics(&self) -> &[Record] {
        &self.diagnostics
    }

    pub fn is_pending(&self) -> bool {
        self.pending.is_some() || !self.queued.is_empty() || !self.pending_close_targets.is_empty()
    }
}

#[derive(Debug)]
struct PendingTargetCreation {
    prepared: PreparedEditorCompositionCommit,
    identities: EditorCompositionIdentityAllocator,
    created_target: PresentationTargetId,
    detached_targets: Vec<PresentationTargetId>,
    editor_window_id: EditorWindowId,
}

pub fn sync_editor_composition_transitions_system(
    mut host: ResMut<EditorHostResource>,
    mut transitions: ResMut<EditorCompositionTransitionRuntimeResource>,
    mut windows: ResMut<WindowStateRegistryResource>,
    mut surfaces: ResMut<RenderSurfaceRegistryResource>,
) {
    sync_editor_composition_transitions(&mut host, &mut transitions, &mut windows, &mut surfaces);
}

fn sync_editor_composition_transitions(
    host: &mut EditorHostResource,
    transitions: &mut EditorCompositionTransitionRuntimeResource,
    windows: &mut WindowStateRegistryResource,
    surfaces: &mut RenderSurfaceRegistryResource,
) {
    transitions
        .queued
        .extend(host.shell_state.drain_docking_intents());
    collect_editor_window_close_intents(host, windows, transitions);
    refresh_coordination_pending(host, transitions);

    if transitions.pending.is_some() {
        finish_pending_target_creation(host, transitions, windows, surfaces);
        refresh_coordination_pending(host, transitions);
        return;
    }

    if let Some(target) = transitions.pending_close_targets.pop_front() {
        process_target_close(host, transitions, windows, target);
        refresh_coordination_pending(host, transitions);
        return;
    }

    let Some(intent) = transitions.queued.pop_front() else {
        refresh_coordination_pending(host, transitions);
        return;
    };
    let policy = EditorCompositionPolicy;
    let policies = CompositionPolicies {
        lifecycle: &policy,
        capability: &policy,
        target: &policy,
    };
    let result =
        evaluate_editor_docking_intent(host.shell_state.composition_runtime().snapshot(), intent)
            .and_then(|accepted| {
                plan_editor_docking_transaction(
                    host.shell_state.composition_runtime(),
                    accepted,
                    host.shell_state.composition_identity_allocator(),
                    TargetProfileId::new(EDITOR_DESKTOP_TARGET_PROFILE)
                        .expect("editor desktop target profile is a checked static identity"),
                )
            })
            .and_then(|plan| {
                if plan.detached_targets.iter().any(|target_id| {
                    host.shell_state
                        .composition_target_binding(*target_id)
                        .is_some_and(|binding| {
                            binding.native_window_id == NativeWindowId::primary()
                        })
                }) {
                    let target = plan.detached_targets.first().copied();
                    transitions.diagnostics.push(Record::error(
                Code::WindowPrimaryDetachDenied,
                Stage::Policy,
                target
                    .map(|target| Subject::Target(target.raw()))
                    .unwrap_or_else(|| Subject::General("primary-target".to_owned())),
                "Move another mounted unit into the primary target before detaching its last tab.",
            ));
                    return Ok(None);
                }
                let prepared = host
                    .shell_state
                    .composition_runtime()
                    .prepare_change(plan.change.clone(), policies)?;
                Ok(Some((plan, prepared)))
            });

    match result {
        Ok(Some((plan, prepared))) => {
            if let Some(created_target) = plan.created_target {
                let editor_window_id = host.shell_state.open_editor_window_for_active_workspace();
                transitions.pending = Some(PendingTargetCreation {
                    prepared,
                    identities: plan.identities,
                    created_target,
                    detached_targets: plan.detached_targets,
                    editor_window_id,
                });
            } else {
                let detached = detached_bindings(host, &plan.detached_targets);
                match host.shell_state.commit_prepared_composition(prepared, None) {
                    Ok(()) => {
                        host.shell_state
                            .replace_composition_identity_allocator(plan.identities);
                        close_detached_windows(host, detached, windows);
                    }
                    Err(rejection) => record_rejection(transitions, rejection),
                }
            }
        }
        Ok(None) => {}
        Err(rejection) => record_rejection(transitions, rejection),
    }
    refresh_coordination_pending(host, transitions);
}

fn refresh_coordination_pending(
    host: &mut EditorHostResource,
    transitions: &EditorCompositionTransitionRuntimeResource,
) {
    host.shell_state
        .set_composition_coordination_pending(transitions.is_pending());
}

fn finish_pending_target_creation(
    host: &mut EditorHostResource,
    transitions: &mut EditorCompositionTransitionRuntimeResource,
    windows: &mut WindowStateRegistryResource,
    surfaces: &mut RenderSurfaceRegistryResource,
) {
    let Some(pending) = transitions.pending.as_ref() else {
        return;
    };
    let Some(binding) = host
        .shell_state
        .editor_window_binding(pending.editor_window_id)
    else {
        return;
    };
    let lifecycle = windows
        .record(binding.native_window_id)
        .map(|record| record.lifecycle_state);
    match lifecycle {
        Some(NativeWindowLifecycleState::Requested) | None => {}
        Some(NativeWindowLifecycleState::CreationFailed) => {
            let pending = transitions
                .pending
                .take()
                .expect("pending transition exists");
            host.shell_state
                .remove_editor_window_presentation(pending.editor_window_id);
            surfaces.retire_surface_for_native_window(binding.native_window_id);
            windows.remove_window(binding.native_window_id);
            transitions.diagnostics.push(Record::error(
                Code::WindowCreationFailed,
                Stage::Projection,
                Subject::Target(pending.created_target.raw()),
                "Native window or GPU surface creation failed; the composition remained unchanged.",
            ));
        }
        Some(NativeWindowLifecycleState::Created) => {
            let pending = transitions
                .pending
                .take()
                .expect("pending transition exists");
            let detached = detached_bindings(host, &pending.detached_targets);
            match host.shell_state.commit_prepared_composition(
                pending.prepared,
                Some((pending.created_target, binding)),
            ) {
                Ok(()) => {
                    host.shell_state
                        .replace_composition_identity_allocator(pending.identities);
                    close_detached_windows(host, detached, windows);
                }
                Err(rejection) => {
                    if let Some(record) = windows.record_mut(binding.native_window_id) {
                        record.approve_close();
                    }
                    host.shell_state
                        .remove_editor_window_presentation(pending.editor_window_id);
                    record_rejection(transitions, rejection);
                }
            }
        }
        Some(NativeWindowLifecycleState::CloseIntentPending)
        | Some(NativeWindowLifecycleState::CloseApproved) => {}
    }
}

fn detached_bindings(
    host: &EditorHostResource,
    targets: &[PresentationTargetId],
) -> Vec<EditorWindowPresentationBinding> {
    targets
        .iter()
        .filter_map(|target| host.shell_state.composition_target_binding(*target))
        .collect()
}

fn close_detached_windows(
    host: &mut EditorHostResource,
    bindings: Vec<EditorWindowPresentationBinding>,
    windows: &mut WindowStateRegistryResource,
) {
    for binding in bindings {
        if binding.native_window_id == NativeWindowId::primary() {
            continue;
        }
        if let Some(record) = windows.record_mut(binding.native_window_id) {
            record.approve_close();
        }
        if let Some(editor_window_id) = host.shell_state.editor_window_for_binding(binding) {
            host.shell_state
                .remove_editor_window_presentation(editor_window_id);
        }
    }
}

fn collect_editor_window_close_intents(
    host: &EditorHostResource,
    windows: &mut WindowStateRegistryResource,
    transitions: &mut EditorCompositionTransitionRuntimeResource,
) {
    let close_intents = windows
        .records()
        .filter(|record| record.close_intent_pending)
        .map(|record| record.native_window_id)
        .collect::<Vec<_>>();
    for native_window_id in close_intents {
        let target = host
            .shell_state
            .composition_target_bindings()
            .find(|entry| entry.binding.native_window_id == native_window_id)
            .map(|entry| entry.target_id);
        if native_window_id == NativeWindowId::primary() {
            let has_dirty_documents = host
                .app
                .runtime()
                .session()
                .documents()
                .any(|document| document.is_dirty);
            if let Some(record) = windows.record_mut(native_window_id) {
                if has_dirty_documents {
                    record.veto_close();
                    transitions.diagnostics.push(Record::error(
                        Code::WindowDirtyQuitDenied,
                        Stage::Policy,
                        Subject::General("editor-quit".to_owned()),
                        "Save or explicitly discard dirty documents before closing the primary window.",
                    ));
                } else {
                    record.approve_close();
                }
            }
        } else if let Some(target) = target {
            if let Some(record) = windows.record_mut(native_window_id) {
                record.veto_close();
            }
            if !transitions.pending_close_targets.contains(&target) {
                transitions.pending_close_targets.push_back(target);
            }
        } else if let Some(record) = windows.record_mut(native_window_id) {
            record.veto_close();
            transitions.diagnostics.push(Record::error(
                Code::WindowUnboundCloseDenied,
                Stage::Policy,
                Subject::General(format!("native-window:{}", native_window_id.raw())),
                "Bind the native window to a composition target before retrying close.",
            ));
        }
    }
}

fn process_target_close(
    host: &mut EditorHostResource,
    transitions: &mut EditorCompositionTransitionRuntimeResource,
    windows: &mut WindowStateRegistryResource,
    target: PresentationTargetId,
) {
    let Some(binding) = host.shell_state.composition_target_binding(target) else {
        transitions.diagnostics.push(Record::error(
            Code::WindowTargetBindingMissing,
            Stage::Projection,
            Subject::Target(target.raw()),
            "Reconcile target bindings before retrying window close.",
        ));
        return;
    };
    let fallback_target = host
        .shell_state
        .composition_target_bindings()
        .find(|entry| {
            entry.target_id != target && entry.binding.native_window_id == NativeWindowId::primary()
        })
        .or_else(|| {
            host.shell_state
                .composition_target_bindings()
                .find(|entry| entry.target_id != target)
        })
        .map(|entry| entry.target_id);
    let Some(fallback_target) = fallback_target else {
        if let Some(record) = windows.record_mut(binding.native_window_id) {
            record.veto_close();
        }
        transitions.diagnostics.push(Record::error(
            Code::WindowCloseFallbackMissing,
            Stage::Policy,
            Subject::Target(target.raw()),
            "Create or retain another composition target before closing this window.",
        ));
        return;
    };
    let policy = EditorCompositionPolicy;
    let policies = CompositionPolicies {
        lifecycle: &policy,
        capability: &policy,
        target: &policy,
    };
    let result = plan_editor_target_close_transaction(
        host.shell_state.composition_runtime(),
        target,
        fallback_target,
        host.shell_state.composition_identity_allocator(),
    )
    .and_then(|plan| {
        host.shell_state
            .composition_runtime()
            .prepare_change(plan.change.clone(), policies)
            .map(|prepared| (plan, prepared))
    });
    match result {
        Ok((plan, prepared)) => {
            match host.shell_state.commit_prepared_composition(prepared, None) {
                Ok(()) => {
                    host.shell_state
                        .replace_composition_identity_allocator(plan.identities);
                    if let Some(record) = windows.record_mut(binding.native_window_id) {
                        record.approve_close();
                    }
                    if let Some(editor_window_id) =
                        host.shell_state.editor_window_for_binding(binding)
                    {
                        host.shell_state
                            .remove_editor_window_presentation(editor_window_id);
                    }
                }
                Err(rejection) => {
                    if let Some(record) = windows.record_mut(binding.native_window_id) {
                        record.veto_close();
                    }
                    record_rejection(transitions, rejection);
                }
            }
        }
        Err(rejection) => {
            if let Some(record) = windows.record_mut(binding.native_window_id) {
                record.veto_close();
            }
            record_rejection(transitions, rejection);
        }
    }
}

fn record_rejection(
    transitions: &mut EditorCompositionTransitionRuntimeResource,
    rejection: EditorCompositionRejection,
) {
    transitions
        .diagnostics
        .extend(rejection.diagnostics().iter().cloned());
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::runtime::WindowState;

    fn unit_from_multi_unit_stack(host: &EditorHostResource) -> ui_composition::MountedUnitId {
        host.shell_state
            .composition_runtime()
            .composition()
            .definition()
            .regions()
            .iter()
            .find_map(|region| match &region.kind {
                ui_composition::RegionKind::Stack { ordered_units, .. }
                    if ordered_units.len() > 1 =>
                {
                    ordered_units.first().copied()
                }
                _ => None,
            })
            .expect("default editor composition should contain a multi-unit stack")
    }

    fn bind_pending_native_window(
        host: &mut EditorHostResource,
        transitions: &EditorCompositionTransitionRuntimeResource,
        windows: &mut WindowStateRegistryResource,
        surfaces: &mut RenderSurfaceRegistryResource,
    ) -> (NativeWindowId, EditorWindowPresentationBinding) {
        let pending = transitions.pending.as_ref().expect("pending transition");
        let request = windows.request_window("Secondary", (900, 600));
        let render_surface_id =
            surfaces.ensure_surface_for_native_window(request.native_window_id, request.size_px);
        let binding = EditorWindowPresentationBinding {
            native_window_id: request.native_window_id,
            render_surface_id,
        };
        assert!(
            host.shell_state
                .bind_editor_window_presentation(pending.editor_window_id, binding,)
        );
        (request.native_window_id, binding)
    }

    #[test]
    fn target_creation_commits_only_after_native_window_is_created() {
        let mut host = EditorHostResource::default();
        let mut transitions = EditorCompositionTransitionRuntimeResource::default();
        let mut windows =
            WindowStateRegistryResource::from_legacy(&WindowState::windowed("Runenwerk"));
        let mut surfaces = RenderSurfaceRegistryResource::default();
        let source_revision = host
            .shell_state
            .composition_runtime()
            .composition()
            .revision();
        let unit = unit_from_multi_unit_stack(&host);
        host.shell_state
            .queue_docking_intent(EditorDockingIntent::detach_to_new_target(
                source_revision,
                unit,
            ));

        sync_editor_composition_transitions(
            &mut host,
            &mut transitions,
            &mut windows,
            &mut surfaces,
        );
        assert!(transitions.is_pending());
        assert_eq!(
            host.shell_state
                .composition_runtime()
                .composition()
                .revision(),
            source_revision
        );

        let (native_window_id, binding) =
            bind_pending_native_window(&mut host, &transitions, &mut windows, &mut surfaces);
        let mut created = WindowState::windowed("Secondary");
        created.size_px = (900, 600);
        windows.register_created_window(native_window_id, &created);
        sync_editor_composition_transitions(
            &mut host,
            &mut transitions,
            &mut windows,
            &mut surfaces,
        );

        assert!(!transitions.is_pending());
        assert_eq!(
            host.shell_state
                .composition_runtime()
                .composition()
                .definition()
                .targets()
                .len(),
            2
        );
        assert!(
            host.shell_state
                .composition_target_bindings()
                .any(|entry| entry.binding == binding)
        );
    }

    #[test]
    fn target_creation_failure_preserves_composition_revision() {
        let mut host = EditorHostResource::default();
        let mut transitions = EditorCompositionTransitionRuntimeResource::default();
        let mut windows =
            WindowStateRegistryResource::from_legacy(&WindowState::windowed("Runenwerk"));
        let mut surfaces = RenderSurfaceRegistryResource::default();
        let source_revision = host
            .shell_state
            .composition_runtime()
            .composition()
            .revision();
        host.shell_state
            .queue_docking_intent(EditorDockingIntent::detach_to_new_target(
                source_revision,
                unit_from_multi_unit_stack(&host),
            ));
        sync_editor_composition_transitions(
            &mut host,
            &mut transitions,
            &mut windows,
            &mut surfaces,
        );
        let (native_window_id, _) =
            bind_pending_native_window(&mut host, &transitions, &mut windows, &mut surfaces);
        windows
            .record_mut(native_window_id)
            .expect("requested native window")
            .mark_creation_failed("test failure");

        sync_editor_composition_transitions(
            &mut host,
            &mut transitions,
            &mut windows,
            &mut surfaces,
        );

        assert!(!transitions.is_pending());
        assert_eq!(
            host.shell_state
                .composition_runtime()
                .composition()
                .revision(),
            source_revision
        );
        assert!(
            transitions
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code() == Code::WindowCreationFailed)
        );
    }

    #[test]
    fn secondary_close_rehomes_content_before_native_teardown() {
        let mut host = EditorHostResource::default();
        let mut transitions = EditorCompositionTransitionRuntimeResource::default();
        let mut windows =
            WindowStateRegistryResource::from_legacy(&WindowState::windowed("Runenwerk"));
        let mut surfaces = RenderSurfaceRegistryResource::default();
        let source_revision = host
            .shell_state
            .composition_runtime()
            .composition()
            .revision();
        let unit = unit_from_multi_unit_stack(&host);
        host.shell_state
            .queue_docking_intent(EditorDockingIntent::detach_to_new_target(
                source_revision,
                unit,
            ));
        sync_editor_composition_transitions(
            &mut host,
            &mut transitions,
            &mut windows,
            &mut surfaces,
        );
        let (native_window_id, binding) =
            bind_pending_native_window(&mut host, &transitions, &mut windows, &mut surfaces);
        windows.register_created_window(native_window_id, &WindowState::windowed("Secondary"));
        sync_editor_composition_transitions(
            &mut host,
            &mut transitions,
            &mut windows,
            &mut surfaces,
        );
        let detached_target = host
            .shell_state
            .composition_target_bindings()
            .find(|entry| entry.binding == binding)
            .expect("secondary target binding")
            .target_id;
        let before_close_revision = host
            .shell_state
            .composition_runtime()
            .composition()
            .revision();
        let detached_root_ids = host
            .shell_state
            .composition_runtime()
            .composition()
            .definition()
            .roots()
            .iter()
            .filter(|root| root.target == detached_target)
            .map(|root| root.id)
            .collect::<Vec<_>>();
        let mounted_units = host
            .shell_state
            .composition_runtime()
            .composition()
            .definition()
            .mounted_units()
            .iter()
            .map(|mounted| mounted.id)
            .collect::<Vec<_>>();

        windows
            .record_mut(native_window_id)
            .expect("secondary native window record")
            .receive_close_intent();
        sync_editor_composition_transitions(
            &mut host,
            &mut transitions,
            &mut windows,
            &mut surfaces,
        );

        let composition = host.shell_state.composition_runtime().composition();
        assert_eq!(
            composition.revision().raw(),
            before_close_revision.raw() + 1,
            "target-close diagnostics: {:?}",
            transitions.diagnostics()
        );
        assert!(
            composition
                .definition()
                .targets()
                .iter()
                .all(|target| target.id != detached_target)
        );
        for root_id in detached_root_ids {
            let root = composition
                .definition()
                .roots()
                .iter()
                .find(|root| root.id == root_id)
                .expect("target close must preserve root identity and topology");
            assert_ne!(root.target, detached_target);
            assert!(!root.primary);
        }
        for mounted_unit in mounted_units {
            assert!(
                composition
                    .definition()
                    .regions()
                    .iter()
                    .any(|region| region.kind.mounted_units().contains(&mounted_unit))
            );
        }
        host.shell_state
            .composition_runtime()
            .extension()
            .validate_against(composition)
            .expect("core and editor extension remain atomically linked");
        let window = windows
            .record(native_window_id)
            .expect("Winit still owns the native record until teardown");
        assert!(window.close_requested);
        assert_eq!(
            window.lifecycle_state,
            NativeWindowLifecycleState::CloseApproved
        );
        assert_eq!(
            surfaces.surface_for_native_window(native_window_id),
            Some(binding.render_surface_id),
            "Winit teardown must detach the GPU surface before retiring the registry record"
        );
    }

    #[test]
    fn primary_close_vetoes_while_any_document_is_dirty() {
        let mut host = EditorHostResource::default();
        let document_id = host
            .app
            .runtime()
            .session()
            .documents()
            .next()
            .expect("default editor session should contain a document")
            .id;
        host.app
            .runtime_mut()
            .session_mut()
            .mark_document_dirty(document_id)
            .unwrap();
        let mut transitions = EditorCompositionTransitionRuntimeResource::default();
        let mut windows =
            WindowStateRegistryResource::from_legacy(&WindowState::windowed("Runenwerk"));
        let mut surfaces = RenderSurfaceRegistryResource::default();
        windows
            .record_mut(NativeWindowId::primary())
            .unwrap()
            .receive_close_intent();

        sync_editor_composition_transitions(
            &mut host,
            &mut transitions,
            &mut windows,
            &mut surfaces,
        );

        let primary = windows.record(NativeWindowId::primary()).unwrap();
        assert!(!primary.close_requested);
        assert_eq!(primary.lifecycle_state, NativeWindowLifecycleState::Created);
        assert!(
            transitions
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code() == Code::WindowDirtyQuitDenied)
        );
    }
}
