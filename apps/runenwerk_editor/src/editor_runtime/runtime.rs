use std::collections::HashSet;

use editor_core::{
    ComponentTypeId, DocumentDescriptor, DocumentId, DocumentKind, EditorMutationError,
    EditorSession, EntityId, ResourceTypeId,
};
use editor_inspector::{InspectTarget, InspectorEditError, InspectorEditValue, InspectorPath};
use editor_scene::{
    SceneComponentDescriptor, SceneMaterialAssignmentState, SceneMaterialSlotId,
    SceneModelMeshMaterialRegionSourceId, SceneRuntime, SdfPrimitiveSourceId,
};

use crate::editor_runtime::{
    AuthoredSceneReality, DocumentTabRuntimeRecord, DocumentTabRuntimeState,
    EditorRuntimeIdRegistry, HierarchySnapshot, InstantiatedSceneReality, OutlinerTree,
    RatifiedChangeLog, RetainedSceneTransaction, RunenwerkEditorInspectorBridge,
    RunenwerkEditorSceneRuntime, SceneComponentSnapshotRecord, SceneDocumentState, SceneEntityView,
    SceneFieldSnapshot, SceneResourceSnapshotRecord, SceneRetentionStore, SceneRuntimeSnapshot,
    SessionReality, SimulatedSceneReality, all_entity_views, build_hierarchy_snapshot,
    outliner_tree_from_hierarchy_snapshot, primary_selected_entity,
    resolve_primary_inspect_target_from_runtime, validate_reparent,
};

struct SceneRealityStore {
    authored: SceneDocumentState,
    material_assignments: SceneMaterialAssignmentState,
    instantiated: ecs::World,
    identities: EditorRuntimeIdRegistry,
}

impl SceneRealityStore {
    fn new() -> Self {
        Self {
            authored: SceneDocumentState::new(),
            material_assignments: SceneMaterialAssignmentState::default(),
            instantiated: ecs::World::new(),
            identities: EditorRuntimeIdRegistry::new(),
        }
    }
}

pub struct RunenwerkEditorRuntime {
    session: EditorSession,
    document_tabs: DocumentTabRuntimeState,
    scene_realities: SceneRealityStore,
    retention_store: SceneRetentionStore,
    ratified_changes: RatifiedChangeLog,
    session_changes: editor_core::SessionChangeLog,
    session_share_outbox: editor_core::SessionShareOutbox,
    session_share_policy: editor_core::SessionSharePolicy,
    shared_changes: editor_core::SharedChangeOutbox,
    sharing_policy: editor_core::SharingPolicy,
    workflow: editor_core::WorkflowLog,
    next_command_id: u64,
    next_transaction_id: u64,
    next_ratification_id: u64,
    next_causality_id: u64,
    next_session_change_id: u64,
    next_session_share_sequence: u64,
    next_shared_change_sequence: u64,
    next_workflow_event_id: u64,
    scene_reality_version: u64,
}

impl Default for RunenwerkEditorRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl RunenwerkEditorRuntime {
    pub fn new() -> Self {
        let mut session = EditorSession::new();
        ensure_default_scene_document(&mut session);
        let mut document_tabs = DocumentTabRuntimeState::new();
        document_tabs.upsert(DocumentTabRuntimeRecord::new(
            DocumentId(1),
            DocumentKind::Scene,
            true,
        ));
        Self {
            session,
            document_tabs,
            scene_realities: SceneRealityStore::new(),
            retention_store: SceneRetentionStore::new(),
            ratified_changes: RatifiedChangeLog::new(),
            session_changes: editor_core::SessionChangeLog::new(),
            session_share_outbox: editor_core::SessionShareOutbox::new(),
            session_share_policy: editor_core::SessionSharePolicy::Disabled,
            shared_changes: editor_core::SharedChangeOutbox::new(),
            sharing_policy: editor_core::SharingPolicy::LocalOnly,
            workflow: editor_core::WorkflowLog::new(),
            next_command_id: 1,
            next_transaction_id: 1,
            next_ratification_id: 1,
            next_causality_id: 1,
            next_session_change_id: 1,
            next_session_share_sequence: 1,
            next_shared_change_sequence: 1,
            next_workflow_event_id: 1,
            scene_reality_version: 0,
        }
    }

    pub fn session(&self) -> &EditorSession {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut EditorSession {
        &mut self.session
    }

    pub fn document_tabs(&self) -> &DocumentTabRuntimeState {
        &self.document_tabs
    }

    pub fn activate_default_material_graph_document(&mut self) -> Result<(), EditorMutationError> {
        self.activate_or_open_document(
            DocumentDescriptor::new(
                DEFAULT_MATERIAL_GRAPH_DOCUMENT_ID,
                DocumentKind::MaterialGraph,
                "Material Graph",
            ),
            true,
        )
    }

    pub fn activate_or_open_document(
        &mut self,
        descriptor: DocumentDescriptor,
        provider_compatible: bool,
    ) -> Result<(), EditorMutationError> {
        let document_id = descriptor.id;
        let document_kind = descriptor.kind.clone();
        self.session.upsert_document(descriptor);
        self.session.activate_document(document_id)?;
        self.document_tabs.upsert(DocumentTabRuntimeRecord::new(
            document_id,
            document_kind,
            provider_compatible,
        ));
        Ok(())
    }

    pub fn session_reality(&self) -> SessionReality<'_> {
        SessionReality::new(&self.session)
    }

    pub fn authored_reality(&self) -> AuthoredSceneReality<'_> {
        AuthoredSceneReality::new(&self.scene_realities.authored)
    }

    pub fn instantiated_reality(&self) -> InstantiatedSceneReality<'_> {
        InstantiatedSceneReality::new(
            &self.scene_realities.instantiated,
            &self.scene_realities.identities,
        )
    }

    pub fn simulated_reality(&self) -> SimulatedSceneReality<'_> {
        SimulatedSceneReality::new(&self.scene_realities.instantiated)
    }

    pub(crate) fn set_active_tool_with_origin(
        &mut self,
        tool_id: Option<editor_core::ToolId>,
        origin: editor_core::ChangeOrigin,
    ) {
        self.session.set_active_tool(tool_id);
        self.record_session_change(
            origin,
            editor_core::SessionChangeKind::ActiveToolSet { tool_id },
        );
    }

    pub(crate) fn set_selection_single_with_origin(
        &mut self,
        target: editor_core::SelectionTarget,
        origin: editor_core::ChangeOrigin,
    ) {
        self.session.select_single(target.clone());
        self.record_session_change(
            origin,
            editor_core::SessionChangeKind::SelectionSetSingle { target },
        );
    }

    pub(crate) fn clear_selection_with_origin(
        &mut self,
        origin: editor_core::ChangeOrigin,
    ) -> bool {
        if self.session.selection().is_empty() {
            return false;
        }

        self.session.clear_selection();
        self.record_session_change(origin, editor_core::SessionChangeKind::SelectionCleared);
        true
    }

    pub(crate) fn pop_undo_history_entry(&mut self) -> Option<editor_core::HistoryEntry> {
        self.session.history_mut().pop_undo()
    }

    pub(crate) fn pop_redo_history_entry(&mut self) -> Option<editor_core::HistoryEntry> {
        self.session.history_mut().pop_redo()
    }

    pub(crate) fn push_applied_history_entry(&mut self, entry: editor_core::HistoryEntry) {
        self.session.history_mut().push_applied(entry);
    }

    pub(crate) fn push_redo_history_entry(&mut self, entry: editor_core::HistoryEntry) {
        self.session.history_mut().push_redo(entry);
    }

    pub fn world(&self) -> &ecs::World {
        &self.scene_realities.instantiated
    }

    #[cfg(test)]
    pub(crate) fn spawn_world_entity<T>(&mut self, component: T) -> ecs::Entity
    where
        T: ecs::Component + 'static,
    {
        self.scene_realities
            .instantiated
            .spawn(component)
            .expect("test world entity spawn should succeed")
    }

    pub(crate) fn insert_component_for_editor_entity<T>(
        &mut self,
        editor_entity: EntityId,
        component: T,
    ) -> Result<(), EditorMutationError>
    where
        T: ecs::Component + 'static,
    {
        let ecs_entity = self
            .scene_realities
            .identities
            .resolve_entity(editor_entity)
            .ok_or(EditorMutationError::runtime_rejected(
                "editor entity is not registered",
            ))?;

        self.scene_realities
            .instantiated
            .insert(ecs_entity, component)
            .map_err(|_| {
                EditorMutationError::runtime_rejected(
                    "failed to insert component for editor entity",
                )
            })
    }

    #[cfg(test)]
    pub(crate) fn remove_component_for_editor_entity<T>(
        &mut self,
        editor_entity: EntityId,
    ) -> Result<T, EditorMutationError>
    where
        T: ecs::Component + 'static,
    {
        let ecs_entity = self
            .scene_realities
            .identities
            .resolve_entity(editor_entity)
            .ok_or(EditorMutationError::runtime_rejected(
                "editor entity is not registered",
            ))?;

        self.scene_realities
            .instantiated
            .remove::<T>(ecs_entity)
            .map_err(|_| {
                EditorMutationError::runtime_rejected(
                    "failed to remove component for editor entity",
                )
            })
    }

    pub fn document(&self) -> &SceneDocumentState {
        &self.scene_realities.authored
    }

    pub fn scene_material_assignments(&self) -> &SceneMaterialAssignmentState {
        &self.scene_realities.material_assignments
    }

    pub(crate) fn replace_scene_material_assignments(
        &mut self,
        assignments: SceneMaterialAssignmentState,
    ) {
        self.scene_realities.material_assignments = assignments;
        self.advance_scene_reality_version();
    }

    pub fn assign_sdf_primitive_material_slot(
        &mut self,
        entity_id: EntityId,
        slot_id: SceneMaterialSlotId,
    ) -> Result<(), String> {
        self.scene_realities
            .material_assignments
            .assign_sdf_primitive_material_slot(SdfPrimitiveSourceId::new(entity_id), slot_id)?;
        self.advance_scene_reality_version();
        Ok(())
    }

    pub fn material_slot_index_for_entity(&self, entity_id: EntityId) -> u32 {
        self.scene_realities
            .material_assignments
            .resolve_material_slot_for_sdf_primitive(SdfPrimitiveSourceId::new(entity_id))
            .material_table_index
    }

    pub fn material_slot_index_for_model_mesh_region(
        &self,
        material_region: &SceneModelMeshMaterialRegionSourceId,
    ) -> u32 {
        self.scene_realities
            .material_assignments
            .resolve_material_slot_for_model_mesh_region(material_region)
            .material_table_index
    }

    pub fn ids(&self) -> &EditorRuntimeIdRegistry {
        &self.scene_realities.identities
    }

    pub fn retention_store(&self) -> &SceneRetentionStore {
        &self.retention_store
    }

    pub(crate) fn clear_scene_entities_only(&mut self) {
        let entity_ids = self
            .scene_realities
            .authored
            .entity_ids()
            .collect::<Vec<_>>();

        for entity_id in entity_ids {
            if let Some(ecs_entity) = self.scene_realities.identities.resolve_entity(entity_id) {
                let _ = self.scene_realities.instantiated.despawn(ecs_entity);
            }
        }

        self.scene_realities.authored.clear_entities();
        self.scene_realities.material_assignments = SceneMaterialAssignmentState::default();
        self.scene_realities.identities.clear_scene_entities();
    }

    pub(crate) fn capture_scene_snapshot(&self) -> SceneRuntimeSnapshot {
        let mut snapshot = SceneRuntimeSnapshot {
            entities: self
                .scene_realities
                .authored
                .entity_ids()
                .filter_map(|entity| self.scene_realities.authored.entity_snapshot(entity))
                .collect(),
            material_assignments: self.scene_realities.material_assignments.clone(),
            ..SceneRuntimeSnapshot::default()
        };

        for entity in self.scene_realities.authored.entity_ids() {
            for component_type in self.scene_realities.identities.component_type_ids() {
                if !self.entity_has_component(entity, component_type) {
                    continue;
                }

                let Some(ecs_entity) = self.scene_realities.identities.resolve_entity(entity)
                else {
                    continue;
                };
                let Some(rust_type_id) = self
                    .scene_realities
                    .identities
                    .resolve_component_rust_type_id(component_type)
                else {
                    continue;
                };
                let Some(value) = self
                    .scene_realities
                    .instantiated
                    .reflected_component_value_ref(ecs_entity, rust_type_id)
                else {
                    continue;
                };

                let mut fields = Vec::new();
                collect_reflected_leaf_fields(value, InspectorPath::root(), &mut fields);
                snapshot.components.push(SceneComponentSnapshotRecord::new(
                    entity,
                    component_type,
                    fields,
                ));
            }
        }

        for resource_type in self.scene_realities.identities.resource_type_ids() {
            let Some(rust_type_id) = self
                .scene_realities
                .identities
                .resolve_resource_rust_type_id(resource_type)
            else {
                continue;
            };
            let Some(value) = self
                .scene_realities
                .instantiated
                .reflected_resource_value_ref(rust_type_id)
            else {
                continue;
            };

            let mut fields = Vec::new();
            collect_reflected_leaf_fields(value, InspectorPath::root(), &mut fields);
            snapshot
                .resources
                .push(SceneResourceSnapshotRecord::new(resource_type, fields));
        }

        snapshot
    }

    pub(crate) fn restore_scene_snapshot(
        &mut self,
        snapshot: &SceneRuntimeSnapshot,
    ) -> Result<(), EditorMutationError> {
        let target_entity_ids = snapshot
            .entities
            .iter()
            .map(|entity| entity.id)
            .collect::<HashSet<_>>();

        let existing_entity_ids = self
            .scene_realities
            .authored
            .entity_ids()
            .collect::<Vec<_>>();
        for entity_id in existing_entity_ids {
            if target_entity_ids.contains(&entity_id) {
                continue;
            }

            if let Some(ecs_entity) = self.scene_realities.identities.resolve_entity(entity_id) {
                let _ = self.scene_realities.instantiated.despawn(ecs_entity);
            }
            let _ = self.scene_realities.authored.unregister_entity(entity_id);
            let _ = self.scene_realities.identities.unregister_entity(entity_id);
        }

        self.scene_realities.authored.clear_entities();

        let mut pending_entities = snapshot.entities.clone();
        while !pending_entities.is_empty() {
            let mut progressed = false;
            let mut remaining = Vec::new();

            for entity in pending_entities {
                if let Some(parent) = entity.parent
                    && !self.scene_realities.authored.contains(parent)
                {
                    remaining.push(entity);
                    continue;
                }

                self.scene_runtime()
                    .restore_entity(entity)
                    .map_err(|error| EditorMutationError::runtime_rejected(error.message))?;
                progressed = true;
            }

            if !progressed {
                return Err(EditorMutationError::runtime_rejected(
                    "failed to restore scene hierarchy",
                ));
            }
            pending_entities = remaining;
        }

        let component_types = self
            .scene_realities
            .identities
            .component_type_ids()
            .collect::<Vec<_>>();
        for entity in snapshot.entities.iter().map(|entity| entity.id) {
            for component_type in &component_types {
                if !self.entity_has_component(entity, *component_type) {
                    continue;
                }

                self.scene_runtime()
                    .remove_component(entity, *component_type)
                    .map_err(|error| EditorMutationError::runtime_rejected(error.message))?;
            }
        }
        self.scene_realities
            .identities
            .clear_removed_component_cache();

        for component in &snapshot.components {
            self.scene_runtime()
                .add_component(component.entity, component.component_type)
                .map_err(|error| EditorMutationError::runtime_rejected(error.message))?;

            for field in &component.fields {
                self.scene_runtime()
                    .write_component_field(
                        component.entity,
                        component.component_type,
                        &field.path,
                        field.value.clone(),
                    )
                    .map_err(map_inspector_edit_error)?;
            }
        }

        for resource in &snapshot.resources {
            for field in &resource.fields {
                self.scene_runtime()
                    .write_resource_field(resource.resource_type, &field.path, field.value.clone())
                    .map_err(map_inspector_edit_error)?;
            }
        }

        self.scene_realities
            .identities
            .clear_removed_component_cache();
        self.scene_realities.material_assignments = snapshot.material_assignments.clone();

        Ok(())
    }

    pub(crate) fn prepare_for_scene_load(&mut self) {
        self.clear_scene_entities_only();
        self.session = EditorSession::new();
        ensure_default_scene_document(&mut self.session);
        self.document_tabs = DocumentTabRuntimeState::new();
        self.document_tabs.upsert(DocumentTabRuntimeRecord::new(
            DocumentId(1),
            DocumentKind::Scene,
            true,
        ));
        self.scene_realities.material_assignments = SceneMaterialAssignmentState::default();
        self.retention_store = SceneRetentionStore::new();
        self.ratified_changes = RatifiedChangeLog::new();
        self.session_changes = editor_core::SessionChangeLog::new();
        self.session_share_outbox = editor_core::SessionShareOutbox::new();
        self.shared_changes = editor_core::SharedChangeOutbox::new();
        self.workflow = editor_core::WorkflowLog::new();
        self.next_command_id = 1;
        self.next_transaction_id = 1;
        self.next_ratification_id = 1;
        self.next_causality_id = 1;
        self.next_session_change_id = 1;
        self.next_session_share_sequence = 1;
        self.next_shared_change_sequence = 1;
        self.next_workflow_event_id = 1;
        self.scene_reality_version = 0;
    }

    pub(crate) fn clear_redo_retained_transactions(&mut self) {
        self.retention_store.clear_redo();
    }

    pub(crate) fn store_applied_retained_transaction(
        &mut self,
        transaction: RetainedSceneTransaction,
    ) {
        self.retention_store.store_applied(transaction);
    }

    pub(crate) fn take_applied_retained_transaction(
        &mut self,
        transaction_id: editor_core::TransactionId,
    ) -> Option<RetainedSceneTransaction> {
        self.retention_store.take_applied(transaction_id)
    }

    pub(crate) fn store_redo_retained_transaction(
        &mut self,
        transaction: RetainedSceneTransaction,
    ) {
        self.retention_store.store_redo(transaction);
    }

    pub(crate) fn take_redo_retained_transaction(
        &mut self,
        transaction_id: editor_core::TransactionId,
    ) -> Option<RetainedSceneTransaction> {
        self.retention_store.take_redo(transaction_id)
    }

    pub fn ratified_change_log(&self) -> &RatifiedChangeLog {
        &self.ratified_changes
    }

    pub fn session_change_log(&self) -> &editor_core::SessionChangeLog {
        &self.session_changes
    }

    pub fn workflow_log(&self) -> &editor_core::WorkflowLog {
        &self.workflow
    }

    pub fn session_share_policy(&self) -> editor_core::SessionSharePolicy {
        self.session_share_policy
    }

    pub fn set_session_share_policy(&mut self, policy: editor_core::SessionSharePolicy) {
        self.session_share_policy = policy;
    }

    pub fn queued_session_share_count(&self) -> usize {
        self.session_share_outbox.len()
    }

    pub fn drain_session_share_changes(&mut self) -> Vec<editor_core::SessionShareEnvelope> {
        self.session_share_outbox.drain()
    }

    pub fn sharing_policy(&self) -> editor_core::SharingPolicy {
        self.sharing_policy
    }

    pub fn set_sharing_policy(&mut self, policy: editor_core::SharingPolicy) {
        self.sharing_policy = policy;
    }

    pub fn queued_shared_change_count(&self) -> usize {
        self.shared_changes.len()
    }

    pub fn drain_shared_changes(&mut self) -> Vec<editor_core::SharedChangeEnvelope> {
        self.shared_changes.drain()
    }

    pub fn propagate_shared_changes<S>(
        &mut self,
        sink: &mut S,
    ) -> Result<usize, <S as editor_core::SharedChangePropagationSink>::Error>
    where
        S: editor_core::SharedChangePropagationSink,
    {
        let mut propagated = 0usize;
        while let Some(envelope) = self.shared_changes.pop_front() {
            if let Err(error) = sink.push_shared_change(envelope.clone()) {
                self.shared_changes.enqueue_front(envelope);
                return Err(error);
            }
            propagated += 1;
        }

        Ok(propagated)
    }

    pub fn reconcile_shared_change(
        &mut self,
        envelope: editor_core::SharedChangeEnvelope,
    ) -> editor_core::ReconciliationDecision {
        let current_version = self.current_scene_reality_version();
        let decision = editor_core::evaluate_reconciliation(&envelope.change, current_version);

        self.record_workflow_event(editor_core::WorkflowEventKind::SharedChangeReconciled {
            sequence: envelope.sequence,
            result: decision.result,
        });

        if decision.is_accepted() {
            self.ratified_changes.push(envelope.change.clone());
            self.adopt_scene_reality_version(envelope.change.result_version);
            self.record_workflow_event(editor_core::WorkflowEventKind::RatifiedChangeRecorded {
                ratification_id: envelope.change.ratification_id,
            });
        }

        decision
    }

    pub fn last_ratified_change(&self) -> Option<&editor_core::RatifiedChange> {
        self.ratified_changes.last()
    }

    pub(crate) fn record_ratified_change(&mut self, change: editor_core::RatifiedChange) {
        self.ratified_changes.push(change.clone());

        if change.propagation_structure == editor_core::PropagationStructure::SessionBroadcast {
            let sequence = editor_core::SharedChangeSequence(self.next_shared_change_sequence);
            self.next_shared_change_sequence += 1;
            self.shared_changes
                .enqueue(editor_core::SharedChangeEnvelope::new(
                    sequence,
                    change.clone(),
                ));
            self.record_workflow_event(editor_core::WorkflowEventKind::SharedChangeQueued {
                sequence,
            });
        }

        self.record_workflow_event(editor_core::WorkflowEventKind::RatifiedChangeRecorded {
            ratification_id: change.ratification_id,
        });
    }

    pub(crate) fn record_session_change(
        &mut self,
        origin: editor_core::ChangeOrigin,
        kind: editor_core::SessionChangeKind,
    ) {
        let id = editor_core::SessionChangeId(self.next_session_change_id);
        self.next_session_change_id += 1;
        self.session_changes
            .push(editor_core::SessionChange::new(id, origin, kind.clone()));

        if self.session_share_policy == editor_core::SessionSharePolicy::ObservationSafe
            && let Some(shared_kind) = map_session_change_to_share(&kind)
        {
            let sequence = editor_core::SessionShareSequence(self.next_session_share_sequence);
            self.next_session_share_sequence += 1;
            self.session_share_outbox
                .enqueue(editor_core::SessionShareEnvelope::new(
                    sequence,
                    editor_core::SessionShareEntry::new(origin, shared_kind),
                ));
        }
    }

    pub(crate) fn record_workflow_event(&mut self, kind: editor_core::WorkflowEventKind) {
        let id = editor_core::WorkflowEventId(self.next_workflow_event_id);
        self.next_workflow_event_id += 1;
        self.workflow
            .push(editor_core::WorkflowEvent::new(id, kind));
    }

    pub fn inspector_bridge(&self) -> RunenwerkEditorInspectorBridge<'_> {
        RunenwerkEditorInspectorBridge::new(&self.scene_realities.identities)
    }

    pub(crate) fn scene_runtime(&mut self) -> RunenwerkEditorSceneRuntime<'_> {
        RunenwerkEditorSceneRuntime::new(
            &mut self.scene_realities.authored,
            &mut self.scene_realities.instantiated,
            &mut self.scene_realities.identities,
        )
    }

    pub(crate) fn with_scene_command_context<R>(
        &mut self,
        run: impl FnOnce(&mut editor_scene::SceneCommandContext<'_>) -> R,
    ) -> R {
        let session = &mut self.session;
        let mut scene_runtime = RunenwerkEditorSceneRuntime::new(
            &mut self.scene_realities.authored,
            &mut self.scene_realities.instantiated,
            &mut self.scene_realities.identities,
        );
        let mut context = editor_scene::SceneCommandContext::new(session, &mut scene_runtime);
        run(&mut context)
    }

    pub fn register_entity(
        &mut self,
        editor_id: EntityId,
        ecs_entity: ecs::Entity,
        display_name: impl Into<String>,
        parent: Option<EntityId>,
    ) {
        self.scene_realities
            .authored
            .register_entity(editor_id, display_name.into(), parent)
            .expect("document entity registration failed");
        self.scene_realities
            .identities
            .register_entity(editor_id, ecs_entity);
    }

    pub fn register_component_type<T>(&mut self, editor_id: ComponentTypeId)
    where
        T: ecs::Component + ecs::Reflect + Default + 'static,
    {
        self.scene_realities
            .instantiated
            .register_reflected_component::<T>();
        self.scene_realities
            .identities
            .register_component_type::<T>(editor_id);
    }

    pub fn register_resource_type<T>(&mut self, editor_id: ResourceTypeId)
    where
        T: ecs::Resource + ecs::Reflect + 'static,
    {
        self.scene_realities
            .instantiated
            .register_reflected_resource::<T>();
        self.scene_realities
            .identities
            .register_resource_type::<T>(editor_id);
    }

    pub fn list_scene_entities(&self) -> Vec<SceneEntityView> {
        all_entity_views(&self.scene_realities.authored)
    }

    pub fn hierarchy_snapshot(&self) -> HierarchySnapshot {
        build_hierarchy_snapshot(&self.scene_realities.authored)
    }

    pub fn outliner_tree(&self) -> OutlinerTree {
        outliner_tree_from_hierarchy_snapshot(&self.hierarchy_snapshot())
    }

    pub fn selected_entity(&self) -> Option<EntityId> {
        primary_selected_entity(self)
    }

    pub fn primary_inspect_target(&self) -> Option<InspectTarget> {
        resolve_primary_inspect_target_from_runtime(self)
    }

    pub fn validate_reparent(
        &self,
        entity: EntityId,
        new_parent: Option<EntityId>,
    ) -> Result<(), EditorMutationError> {
        validate_reparent(&self.scene_realities.authored, entity, new_parent)
    }

    pub fn entity_has_component(&self, entity: EntityId, component_type: ComponentTypeId) -> bool {
        let Some(ecs_entity) = self.scene_realities.identities.resolve_entity(entity) else {
            return false;
        };

        let Some(rust_type_id) = self
            .scene_realities
            .identities
            .resolve_component_rust_type_id(component_type)
        else {
            return false;
        };

        self.scene_realities
            .instantiated
            .reflected_component_value_ref(ecs_entity, rust_type_id)
            .is_some()
    }

    pub fn list_entity_components(&self, entity: EntityId) -> Vec<SceneComponentDescriptor> {
        let mut components = self
            .scene_realities
            .identities
            .component_type_ids()
            .filter(|component_type| self.entity_has_component(entity, *component_type))
            .filter_map(|component_type| {
                let display_name = self
                    .scene_realities
                    .identities
                    .component_display_name(component_type)?;
                Some(SceneComponentDescriptor::new(
                    entity,
                    component_type,
                    display_name.to_string(),
                ))
            })
            .collect::<Vec<_>>();

        components.sort_by(|left, right| {
            left.display_name
                .cmp(&right.display_name)
                .then_with(|| left.component_type.cmp(&right.component_type))
        });

        components
    }

    pub(crate) fn allocate_command_id(&mut self) -> editor_core::CommandId {
        let id = editor_core::CommandId(self.next_command_id);
        self.next_command_id += 1;
        id
    }

    pub(crate) fn allocate_transaction_id(&mut self) -> editor_core::TransactionId {
        let id = editor_core::TransactionId(self.next_transaction_id);
        self.next_transaction_id += 1;
        id
    }

    pub(crate) fn allocate_ratification_id(&mut self) -> editor_core::RatificationId {
        let id = editor_core::RatificationId(self.next_ratification_id);
        self.next_ratification_id += 1;
        id
    }

    pub(crate) fn allocate_causality_id(&mut self) -> editor_core::CausalityId {
        let id = editor_core::CausalityId(self.next_causality_id);
        self.next_causality_id += 1;
        id
    }

    pub fn current_scene_reality_version(&self) -> editor_core::RealityVersion {
        editor_core::RealityVersion(self.scene_reality_version)
    }

    pub(crate) fn advance_scene_reality_version(&mut self) -> editor_core::RealityVersion {
        self.scene_reality_version += 1;
        editor_core::RealityVersion(self.scene_reality_version)
    }

    pub(crate) fn adopt_scene_reality_version(&mut self, version: editor_core::RealityVersion) {
        if version.0 > self.scene_reality_version {
            self.scene_reality_version = version.0;
        }
    }

    pub fn list_registered_component_types(&self) -> Vec<(ComponentTypeId, String)> {
        let mut component_types = self
            .scene_realities
            .identities
            .component_type_ids()
            .filter_map(|component_type| {
                let display_name = self
                    .scene_realities
                    .identities
                    .component_display_name(component_type)?;
                Some((component_type, display_name.to_string()))
            })
            .collect::<Vec<_>>();

        component_types
            .sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));

        component_types
    }
}

fn ensure_default_scene_document(session: &mut EditorSession) -> DocumentId {
    let scene_document_id = DocumentId(1);
    session.upsert_document(DocumentDescriptor::new(
        scene_document_id,
        DocumentKind::Scene,
        "Scene",
    ));
    session
        .set_active_document(Some(scene_document_id))
        .expect("default scene document should be registered before activation");
    scene_document_id
}

const DEFAULT_MATERIAL_GRAPH_DOCUMENT_ID: DocumentId = DocumentId(2);

fn map_session_change_to_share(
    kind: &editor_core::SessionChangeKind,
) -> Option<editor_core::SessionShareKind> {
    match kind {
        editor_core::SessionChangeKind::ActiveToolSet { tool_id } => {
            Some(editor_core::SessionShareKind::ActiveToolSet { tool_id: *tool_id })
        }
        editor_core::SessionChangeKind::ModeSet { mode } => {
            Some(editor_core::SessionShareKind::ModeSet { mode: *mode })
        }
        editor_core::SessionChangeKind::SelectionSetSingle { target } => {
            Some(editor_core::SessionShareKind::SelectionSetSingle {
                target: target.clone(),
            })
        }
        editor_core::SessionChangeKind::SelectionCleared => {
            Some(editor_core::SessionShareKind::SelectionCleared)
        }
    }
}

fn map_inspector_edit_error(error: InspectorEditError) -> EditorMutationError {
    match error {
        InspectorEditError::TargetNotFound => {
            EditorMutationError::runtime_rejected("inspector target not found")
        }
        InspectorEditError::TypeNotRegistered => {
            EditorMutationError::runtime_rejected("inspector type not registered")
        }
        InspectorEditError::ValueNotAvailable => {
            EditorMutationError::runtime_rejected("inspector value not available")
        }
        InspectorEditError::InvalidPath => {
            EditorMutationError::runtime_rejected("invalid inspector path")
        }
        InspectorEditError::UnsupportedPathSegment => {
            EditorMutationError::runtime_rejected("unsupported inspector path segment")
        }
        InspectorEditError::UnsupportedValueType { .. } => {
            EditorMutationError::runtime_rejected("unsupported inspector value type")
        }
        InspectorEditError::IntegerOutOfRange { .. } => {
            EditorMutationError::runtime_rejected("integer out of range")
        }
        InspectorEditError::FloatOutOfRange { .. } => {
            EditorMutationError::runtime_rejected("float out of range")
        }
        InspectorEditError::ExpectedEnumField => {
            EditorMutationError::runtime_rejected("expected inspector enum field")
        }
        InspectorEditError::InvalidEnumOption { .. } => {
            EditorMutationError::runtime_rejected("invalid inspector enum option")
        }
    }
}

fn collect_reflected_leaf_fields(
    value: ecs::reflect::ReflectValueRef<'_>,
    path: InspectorPath,
    out: &mut Vec<SceneFieldSnapshot>,
) {
    if let Some(bool_value) = value.downcast_ref::<bool>() {
        out.push(SceneFieldSnapshot::new(
            path,
            InspectorEditValue::Bool(*bool_value),
        ));
        return;
    }
    if let Some(int_value) = value.downcast_ref::<i8>() {
        out.push(SceneFieldSnapshot::new(
            path,
            InspectorEditValue::Integer(*int_value as i64),
        ));
        return;
    }
    if let Some(int_value) = value.downcast_ref::<i16>() {
        out.push(SceneFieldSnapshot::new(
            path,
            InspectorEditValue::Integer(*int_value as i64),
        ));
        return;
    }
    if let Some(int_value) = value.downcast_ref::<i32>() {
        out.push(SceneFieldSnapshot::new(
            path,
            InspectorEditValue::Integer(*int_value as i64),
        ));
        return;
    }
    if let Some(int_value) = value.downcast_ref::<i64>() {
        out.push(SceneFieldSnapshot::new(
            path,
            InspectorEditValue::Integer(*int_value),
        ));
        return;
    }
    if let Some(int_value) = value.downcast_ref::<isize>() {
        out.push(SceneFieldSnapshot::new(
            path,
            InspectorEditValue::Integer(*int_value as i64),
        ));
        return;
    }
    if let Some(int_value) = value.downcast_ref::<u8>() {
        out.push(SceneFieldSnapshot::new(
            path,
            InspectorEditValue::Integer(*int_value as i64),
        ));
        return;
    }
    if let Some(int_value) = value.downcast_ref::<u16>() {
        out.push(SceneFieldSnapshot::new(
            path,
            InspectorEditValue::Integer(*int_value as i64),
        ));
        return;
    }
    if let Some(int_value) = value.downcast_ref::<u32>() {
        out.push(SceneFieldSnapshot::new(
            path,
            InspectorEditValue::Integer(*int_value as i64),
        ));
        return;
    }
    if let Some(int_value) = value.downcast_ref::<u64>() {
        out.push(SceneFieldSnapshot::new(
            path,
            InspectorEditValue::Integer(*int_value as i64),
        ));
        return;
    }
    if let Some(int_value) = value.downcast_ref::<usize>() {
        out.push(SceneFieldSnapshot::new(
            path,
            InspectorEditValue::Integer(*int_value as i64),
        ));
        return;
    }
    if let Some(float_value) = value.downcast_ref::<f32>() {
        out.push(SceneFieldSnapshot::new(
            path,
            InspectorEditValue::Float(*float_value as f64),
        ));
        return;
    }
    if let Some(float_value) = value.downcast_ref::<f64>() {
        out.push(SceneFieldSnapshot::new(
            path,
            InspectorEditValue::Float(*float_value),
        ));
        return;
    }
    if let Some(text_value) = value.downcast_ref::<String>() {
        out.push(SceneFieldSnapshot::new(
            path,
            InspectorEditValue::Text(text_value.clone()),
        ));
        return;
    }

    if let Some(struct_ref) = value.struct_ref() {
        for field in struct_ref.fields() {
            let Some(field_value) = struct_ref.field(field.name) else {
                continue;
            };
            collect_reflected_leaf_fields(field_value, path.clone().child_field(field.name), out);
        }
    }
}
