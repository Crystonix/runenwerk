use editor_core::{ComponentTypeId, EditorMutationError, EntityId, ResourceTypeId};
use editor_inspector::{
    InspectorEditError, InspectorEditValue, InspectorPath, InspectorPathSegment,
    set_component_field_value, set_resource_field_value,
};
use editor_scene::{
    SceneComponentSnapshot, SceneEntitySnapshot, SceneRuntime, SceneTransform, SceneVec3,
    SdfPrimitiveKind, SdfPrimitiveSpec,
};
use scene::{LocalTransform, QuatValue, Vec3Value};

use crate::editor_runtime::{
    EDITOR_PRIMITIVE_COMPONENT_TYPE_ID, EditorPrimitive, EditorPrimitiveKind,
    EditorRuntimeIdRegistry, LOCAL_TRANSFORM_COMPONENT_TYPE_ID, RunenwerkEditorInspectorBridge,
    SceneDocumentState,
};

#[derive(ecs::Bundle)]
struct EmptyEntityBundle {}

pub struct RunenwerkEditorSceneRuntime<'a> {
    document: &'a mut SceneDocumentState,
    world: &'a mut ecs::World,
    ids: &'a mut EditorRuntimeIdRegistry,
}

impl<'a> RunenwerkEditorSceneRuntime<'a> {
    pub fn new(
        document: &'a mut SceneDocumentState,
        world: &'a mut ecs::World,
        ids: &'a mut EditorRuntimeIdRegistry,
    ) -> Self {
        Self {
            document,
            world,
            ids,
        }
    }
}

impl<'a> SceneRuntime for RunenwerkEditorSceneRuntime<'a> {
    fn create_entity(
        &mut self,
        parent: Option<EntityId>,
        display_name: &str,
    ) -> Result<EntityId, EditorMutationError> {
        if let Some(parent) = parent
            && !self.document.contains(parent)
        {
            return Err(EditorMutationError::runtime_rejected(
                "new parent entity is not registered",
            ));
        }

        let ecs_entity = self
            .world
            .spawn(EmptyEntityBundle {})
            .map_err(|_| EditorMutationError::runtime_rejected("failed to allocate ecs entity"))?;
        let editor_id = self.ids.allocate_entity_id();
        self.document
            .register_entity(editor_id, display_name.to_string(), parent)?;
        self.ids.register_entity(editor_id, ecs_entity);

        Ok(editor_id)
    }

    fn restore_entity(&mut self, snapshot: SceneEntitySnapshot) -> Result<(), EditorMutationError> {
        if let Some(parent) = snapshot.parent
            && !self.document.contains(parent)
        {
            return Err(EditorMutationError::runtime_rejected(
                "new parent entity is not registered",
            ));
        }

        self.document.restore_entity(snapshot.clone())?;

        if self.ids.resolve_entity(snapshot.id).is_none() {
            let ecs_entity = self.world.spawn(EmptyEntityBundle {}).map_err(|_| {
                EditorMutationError::runtime_rejected("failed to allocate ecs entity")
            })?;
            self.ids.register_entity(snapshot.id, ecs_entity);
        }

        Ok(())
    }

    fn delete_entity(
        &mut self,
        entity: EntityId,
    ) -> Result<SceneEntitySnapshot, EditorMutationError> {
        if self.document.has_children(entity) {
            return Err(EditorMutationError::runtime_rejected(
                "cannot delete entity while it still has children",
            ));
        }

        let snapshot =
            self.document
                .entity_snapshot(entity)
                .ok_or(EditorMutationError::runtime_rejected(
                    "editor entity is not registered",
                ))?;

        let ecs_entity =
            self.ids
                .resolve_entity(entity)
                .ok_or(EditorMutationError::runtime_rejected(
                    "editor entity is not registered",
                ))?;

        self.world
            .despawn(ecs_entity)
            .map_err(|_| EditorMutationError::runtime_rejected("failed to despawn ecs entity"))?;

        let _ = self.document.unregister_entity(entity);
        let _ = self.ids.unregister_entity(entity);

        Ok(snapshot)
    }

    fn children_of(&self, parent: Option<EntityId>) -> Vec<EntityId> {
        self.document.children_of(parent)
    }

    fn duplicate_entity_subtree(
        &mut self,
        source: EntityId,
        new_parent: Option<EntityId>,
        name_suffix: &str,
    ) -> Result<Vec<EntityId>, EditorMutationError> {
        let mut created = Vec::new();
        let source_snapshot =
            self.document
                .entity_snapshot(source)
                .ok_or(EditorMutationError::runtime_rejected(
                    "source entity is not registered",
                ))?;
        let root_parent = new_parent.or(source_snapshot.parent);
        duplicate_entity_recursive(self, source, root_parent, name_suffix, &mut created)?;
        Ok(created)
    }

    fn reparent_entity(
        &mut self,
        entity: EntityId,
        new_parent: Option<EntityId>,
    ) -> Result<Option<EntityId>, EditorMutationError> {
        self.document.reparent_entity(entity, new_parent)
    }

    fn add_component(
        &mut self,
        entity: EntityId,
        component_type: ComponentTypeId,
    ) -> Result<(), EditorMutationError> {
        let ecs_entity =
            self.ids
                .resolve_entity(entity)
                .ok_or(EditorMutationError::runtime_rejected(
                    "editor entity is not registered",
                ))?;

        self.ids
            .add_default_component(self.world, ecs_entity, component_type)
    }

    fn remove_component(
        &mut self,
        entity: EntityId,
        component_type: ComponentTypeId,
    ) -> Result<SceneComponentSnapshot, EditorMutationError> {
        let ecs_entity =
            self.ids
                .resolve_entity(entity)
                .ok_or(EditorMutationError::runtime_rejected(
                    "editor entity is not registered",
                ))?;

        let display_name = self
            .ids
            .component_display_name(component_type)
            .ok_or(EditorMutationError::runtime_rejected(
                "component type is not registered in editor runtime",
            ))?
            .to_string();

        self.ids
            .remove_component_and_capture(self.world, entity, ecs_entity, component_type)?;

        Ok(SceneComponentSnapshot::new(
            entity,
            component_type,
            display_name,
        ))
    }

    fn restore_component(
        &mut self,
        snapshot: SceneComponentSnapshot,
    ) -> Result<(), EditorMutationError> {
        let ecs_entity = self.ids.resolve_entity(snapshot.entity).ok_or(
            EditorMutationError::runtime_rejected("editor entity is not registered"),
        )?;

        self.ids.restore_removed_component(
            self.world,
            snapshot.entity,
            ecs_entity,
            snapshot.component_type,
        )
    }

    fn create_sdf_primitive(
        &mut self,
        parent: Option<EntityId>,
        display_name: &str,
        primitive: SdfPrimitiveSpec,
    ) -> Result<EntityId, EditorMutationError> {
        let entity = self.create_entity(parent, display_name)?;
        self.add_component(entity, LOCAL_TRANSFORM_COMPONENT_TYPE_ID)?;
        self.add_component(entity, EDITOR_PRIMITIVE_COMPONENT_TYPE_ID)?;
        self.write_transform(entity, primitive.transform)?;
        write_editor_primitive_kind(self, entity, primitive.kind)?;
        Ok(entity)
    }

    fn read_transform(&self, entity: EntityId) -> Result<SceneTransform, EditorMutationError> {
        let ecs_entity =
            self.ids
                .resolve_entity(entity)
                .ok_or(EditorMutationError::runtime_rejected(
                    "editor entity is not registered",
                ))?;
        let transform = self.world.get::<LocalTransform>(ecs_entity).ok_or(
            EditorMutationError::runtime_rejected("entity does not have LocalTransform"),
        )?;
        Ok(scene_transform_from_local(*transform))
    }

    fn write_transform(
        &mut self,
        entity: EntityId,
        transform: SceneTransform,
    ) -> Result<(), EditorMutationError> {
        let ecs_entity =
            self.ids
                .resolve_entity(entity)
                .ok_or(EditorMutationError::runtime_rejected(
                    "editor entity is not registered",
                ))?;
        self.world
            .insert(ecs_entity, local_transform_from_scene(transform))
            .map_err(|_| EditorMutationError::runtime_rejected("failed to write LocalTransform"))
    }

    fn read_component_field(
        &self,
        entity: EntityId,
        component_type: ComponentTypeId,
        path: &InspectorPath,
    ) -> Result<InspectorEditValue, InspectorEditError> {
        let ecs_entity = self
            .ids
            .resolve_entity(entity)
            .ok_or(InspectorEditError::TargetNotFound)?;

        let rust_type_id = self
            .ids
            .resolve_component_rust_type_id(component_type)
            .ok_or(InspectorEditError::TypeNotRegistered)?;

        let value = self
            .world
            .reflected_component_value_ref(ecs_entity, rust_type_id)
            .ok_or(InspectorEditError::ValueNotAvailable)?;

        read_edit_value_at_path(value, path)
    }

    fn write_component_field(
        &mut self,
        entity: EntityId,
        component_type: ComponentTypeId,
        path: &InspectorPath,
        value: InspectorEditValue,
    ) -> Result<(), InspectorEditError> {
        let ids = &*self.ids;
        let world = &mut *self.world;
        let bridge = RunenwerkEditorInspectorBridge::new(ids);

        set_component_field_value(world, &bridge, entity, component_type, path, value)
    }

    fn read_resource_field(
        &self,
        resource_type: ResourceTypeId,
        path: &InspectorPath,
    ) -> Result<InspectorEditValue, InspectorEditError> {
        let rust_type_id = self
            .ids
            .resolve_resource_rust_type_id(resource_type)
            .ok_or(InspectorEditError::TypeNotRegistered)?;

        let value = self
            .world
            .reflected_resource_value_ref(rust_type_id)
            .ok_or(InspectorEditError::ValueNotAvailable)?;

        read_edit_value_at_path(value, path)
    }

    fn write_resource_field(
        &mut self,
        resource_type: ResourceTypeId,
        path: &InspectorPath,
        value: InspectorEditValue,
    ) -> Result<(), InspectorEditError> {
        let ids = &*self.ids;
        let world = &mut *self.world;
        let bridge = RunenwerkEditorInspectorBridge::new(ids);

        set_resource_field_value(world, &bridge, resource_type, path, value)
    }

    fn rename_entity(
        &mut self,
        entity: EntityId,
        new_display_name: &str,
    ) -> Result<String, EditorMutationError> {
        self.document
            .rename_entity(entity, new_display_name.to_string())
    }
}

fn duplicate_entity_recursive(
    runtime: &mut RunenwerkEditorSceneRuntime<'_>,
    source: EntityId,
    new_parent: Option<EntityId>,
    name_suffix: &str,
    created: &mut Vec<EntityId>,
) -> Result<EntityId, EditorMutationError> {
    let source_snapshot =
        runtime
            .document
            .entity_snapshot(source)
            .ok_or(EditorMutationError::runtime_rejected(
                "source entity is not registered",
            ))?;
    let duplicate_name = format!("{}{}", source_snapshot.display_name, name_suffix);
    let duplicate = runtime.create_entity(new_parent, &duplicate_name)?;
    copy_common_scene_components(runtime, source, duplicate)?;
    created.push(duplicate);

    for child in runtime.document.children_of(Some(source)) {
        let _ = duplicate_entity_recursive(runtime, child, Some(duplicate), name_suffix, created)?;
    }

    Ok(duplicate)
}

fn copy_common_scene_components(
    runtime: &mut RunenwerkEditorSceneRuntime<'_>,
    source: EntityId,
    target: EntityId,
) -> Result<(), EditorMutationError> {
    let source_ecs =
        runtime
            .ids
            .resolve_entity(source)
            .ok_or(EditorMutationError::runtime_rejected(
                "source entity is not registered",
            ))?;
    let target_ecs =
        runtime
            .ids
            .resolve_entity(target)
            .ok_or(EditorMutationError::runtime_rejected(
                "target entity is not registered",
            ))?;

    if let Some(transform) = runtime.world.get::<LocalTransform>(source_ecs).copied() {
        runtime
            .world
            .insert(target_ecs, transform)
            .map_err(|_| EditorMutationError::runtime_rejected("failed to copy LocalTransform"))?;
    }

    if let Some(primitive) = runtime.world.get::<EditorPrimitive>(source_ecs).copied() {
        runtime
            .world
            .insert(target_ecs, primitive)
            .map_err(|_| EditorMutationError::runtime_rejected("failed to copy EditorPrimitive"))?;
    }

    Ok(())
}

fn write_editor_primitive_kind(
    runtime: &mut RunenwerkEditorSceneRuntime<'_>,
    entity: EntityId,
    kind: SdfPrimitiveKind,
) -> Result<(), EditorMutationError> {
    let ecs_entity =
        runtime
            .ids
            .resolve_entity(entity)
            .ok_or(EditorMutationError::runtime_rejected(
                "editor entity is not registered",
            ))?;
    let mut primitive = runtime
        .world
        .get::<EditorPrimitive>(ecs_entity)
        .copied()
        .unwrap_or_default();
    primitive.set_kind(editor_primitive_kind_from_sdf(kind));
    runtime
        .world
        .insert(ecs_entity, primitive)
        .map_err(|_| EditorMutationError::runtime_rejected("failed to write EditorPrimitive"))
}

fn editor_primitive_kind_from_sdf(kind: SdfPrimitiveKind) -> EditorPrimitiveKind {
    match kind {
        SdfPrimitiveKind::Box => EditorPrimitiveKind::Box,
        SdfPrimitiveKind::Sphere => EditorPrimitiveKind::Sphere,
        SdfPrimitiveKind::Capsule => EditorPrimitiveKind::Capsule,
        SdfPrimitiveKind::Cylinder => EditorPrimitiveKind::Cylinder,
        SdfPrimitiveKind::Torus => EditorPrimitiveKind::Torus,
        SdfPrimitiveKind::Plane => EditorPrimitiveKind::Plane,
    }
}

fn scene_transform_from_local(transform: LocalTransform) -> SceneTransform {
    SceneTransform::new(
        SceneVec3::new(
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
        ),
        editor_scene::SceneQuat::new(
            transform.rotation.x,
            transform.rotation.y,
            transform.rotation.z,
            transform.rotation.w,
        ),
        SceneVec3::new(transform.scale.x, transform.scale.y, transform.scale.z),
    )
}

fn local_transform_from_scene(transform: SceneTransform) -> LocalTransform {
    LocalTransform::new(
        Vec3Value::new(
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
        ),
        QuatValue::new(
            transform.rotation.x,
            transform.rotation.y,
            transform.rotation.z,
            transform.rotation.w,
        ),
        Vec3Value::new(transform.scale.x, transform.scale.y, transform.scale.z),
    )
}

fn read_edit_value_at_path(
    current: ecs::reflect::ReflectValueRef<'_>,
    path: &InspectorPath,
) -> Result<InspectorEditValue, InspectorEditError> {
    let mut current = current;

    for segment in path.segments() {
        match segment {
            InspectorPathSegment::Field(name) => {
                let struct_ref = current
                    .struct_ref()
                    .ok_or(InspectorEditError::InvalidPath)?;
                current = struct_ref
                    .field(name)
                    .ok_or(InspectorEditError::InvalidPath)?;
            }
            InspectorPathSegment::Index(_) => {
                return Err(InspectorEditError::UnsupportedPathSegment);
            }
        }
    }

    if let Some(value) = current.downcast_ref::<bool>() {
        return Ok(InspectorEditValue::Bool(*value));
    }
    if let Some(value) = current.downcast_ref::<i8>() {
        return Ok(InspectorEditValue::Integer(*value as i64));
    }
    if let Some(value) = current.downcast_ref::<i16>() {
        return Ok(InspectorEditValue::Integer(*value as i64));
    }
    if let Some(value) = current.downcast_ref::<i32>() {
        return Ok(InspectorEditValue::Integer(*value as i64));
    }
    if let Some(value) = current.downcast_ref::<i64>() {
        return Ok(InspectorEditValue::Integer(*value));
    }
    if let Some(value) = current.downcast_ref::<isize>() {
        return Ok(InspectorEditValue::Integer(*value as i64));
    }
    if let Some(value) = current.downcast_ref::<u8>() {
        return Ok(InspectorEditValue::Integer(*value as i64));
    }
    if let Some(value) = current.downcast_ref::<u16>() {
        return Ok(InspectorEditValue::Integer(*value as i64));
    }
    if let Some(value) = current.downcast_ref::<u32>() {
        return Ok(InspectorEditValue::Integer(*value as i64));
    }
    if let Some(value) = current.downcast_ref::<u64>() {
        return Ok(InspectorEditValue::Integer(*value as i64));
    }
    if let Some(value) = current.downcast_ref::<usize>() {
        return Ok(InspectorEditValue::Integer(*value as i64));
    }
    if let Some(value) = current.downcast_ref::<f32>() {
        return Ok(InspectorEditValue::Float(*value as f64));
    }
    if let Some(value) = current.downcast_ref::<f64>() {
        return Ok(InspectorEditValue::Float(*value));
    }
    if let Some(value) = current.downcast_ref::<String>() {
        return Ok(InspectorEditValue::Text(value.clone()));
    }

    Err(InspectorEditError::UnsupportedValueType {
        actual_type: current.type_info().display_name.to_string(),
    })
}
