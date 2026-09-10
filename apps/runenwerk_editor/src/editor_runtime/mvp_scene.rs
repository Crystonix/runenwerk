use std::{any::TypeId, collections::BTreeSet};

use editor_core::{ComponentTypeId, EditorMutationError};
use editor_scene::SceneCommandIntent;
use scene::{LocalTransform, Vec3Value};

use crate::editor_runtime::{RunenwerkEditorRuntime, execute_scene_intent};

pub const LOCAL_TRANSFORM_COMPONENT_TYPE_ID: ComponentTypeId = ComponentTypeId(500);
pub const EDITOR_PRIMITIVE_COMPONENT_TYPE_ID: ComponentTypeId = ComponentTypeId(501);
pub const MVP_GRAYBOX_ENTITY_NAME: &str = "Graybox";
pub const MVP_GROUND_PLANE_ENTITY_NAME: &str = "Ground Plane";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorPrimitiveKind {
    #[default]
    Box,
    Sphere,
    Capsule,
    Cylinder,
    Torus,
    Plane,
}

impl EditorPrimitiveKind {
    pub fn as_u32(self) -> u32 {
        match self {
            Self::Box => 0,
            Self::Sphere => 1,
            Self::Capsule => 2,
            Self::Cylinder => 3,
            Self::Torus => 4,
            Self::Plane => 5,
        }
    }

    pub fn from_u32(value: u32) -> Self {
        match value {
            1 => Self::Sphere,
            2 => Self::Capsule,
            3 => Self::Cylinder,
            4 => Self::Torus,
            5 => Self::Plane,
            _ => Self::Box,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, runen_ecs::Component, runen_ecs::Reflect)]
pub struct EditorPrimitive {
    pub primitive_kind: u32,
    pub box_half_extents: Vec3Value,
    pub sphere_radius: f32,
    pub capsule_radius: f32,
    pub capsule_half_height: f32,
}

impl EditorPrimitive {
    pub fn kind(self) -> EditorPrimitiveKind {
        EditorPrimitiveKind::from_u32(self.primitive_kind)
    }

    pub fn set_kind(&mut self, kind: EditorPrimitiveKind) {
        self.primitive_kind = kind.as_u32();
    }

    pub fn box_extents_for_picking(self) -> Vec3Value {
        match self.kind() {
            EditorPrimitiveKind::Box => self.box_half_extents,
            EditorPrimitiveKind::Sphere => Vec3Value::new(
                self.sphere_radius.max(0.05),
                self.sphere_radius.max(0.05),
                self.sphere_radius.max(0.05),
            ),
            EditorPrimitiveKind::Capsule => Vec3Value::new(
                self.capsule_radius.max(0.05),
                self.capsule_radius.max(0.05) + self.capsule_half_height.max(0.05),
                self.capsule_radius.max(0.05),
            ),
            EditorPrimitiveKind::Cylinder => Vec3Value::new(
                self.capsule_radius.max(0.05),
                self.capsule_half_height.max(0.05),
                self.capsule_radius.max(0.05),
            ),
            EditorPrimitiveKind::Torus => Vec3Value::new(
                self.sphere_radius.max(0.05) * 2.0,
                self.sphere_radius.max(0.05) * 0.5,
                self.sphere_radius.max(0.05) * 2.0,
            ),
            EditorPrimitiveKind::Plane => self.box_half_extents,
        }
    }
}

impl Default for EditorPrimitive {
    fn default() -> Self {
        Self {
            primitive_kind: EditorPrimitiveKind::Box.as_u32(),
            box_half_extents: Vec3Value::new(0.5, 0.5, 0.5),
            sphere_radius: 0.6,
            capsule_radius: 0.35,
            capsule_half_height: 0.75,
        }
    }
}

pub fn register_mvp_component_types(runtime: &mut RunenwerkEditorRuntime) {
    if runtime
        .ids()
        .resolve_component_rust_type_id(LOCAL_TRANSFORM_COMPONENT_TYPE_ID)
        .is_none()
    {
        runtime.register_component_type::<LocalTransform>(LOCAL_TRANSFORM_COMPONENT_TYPE_ID);
    }

    if runtime
        .ids()
        .resolve_component_rust_type_id(EDITOR_PRIMITIVE_COMPONENT_TYPE_ID)
        .is_none()
    {
        runtime.register_component_type::<EditorPrimitive>(EDITOR_PRIMITIVE_COMPONENT_TYPE_ID);
    }
}

pub fn bootstrap_mvp_scene_if_empty(
    runtime: &mut RunenwerkEditorRuntime,
) -> Result<(), EditorMutationError> {
    if runtime.document().entity_ids().next().is_some() {
        return Ok(());
    }

    create_mvp_primitive_entity(
        runtime,
        MVP_GRAYBOX_ENTITY_NAME,
        LocalTransform::default(),
        EditorPrimitive::default(),
    )?;

    let mut ground_plane = EditorPrimitive {
        box_half_extents: Vec3Value::new(8.0, 0.02, 8.0),
        ..Default::default()
    };
    ground_plane.set_kind(EditorPrimitiveKind::Plane);
    create_mvp_primitive_entity(
        runtime,
        MVP_GROUND_PLANE_ENTITY_NAME,
        LocalTransform::from_translation(Vec3Value::new(0.0, -0.55, 0.0)),
        ground_plane,
    )?;

    Ok(())
}

fn create_mvp_primitive_entity(
    runtime: &mut RunenwerkEditorRuntime,
    display_name: &str,
    transform: LocalTransform,
    primitive: EditorPrimitive,
) -> Result<editor_core::EntityId, EditorMutationError> {
    let before = runtime.document().entity_ids().collect::<BTreeSet<_>>();
    let create_command_id = runtime.allocate_command_id();
    execute_scene_intent(
        runtime,
        create_command_id,
        SceneCommandIntent::CreateEntity {
            parent: None,
            display_name: display_name.to_string(),
        },
    )?;

    let entity = runtime
        .document()
        .entity_ids()
        .find(|entity| !before.contains(entity))
        .ok_or(EditorMutationError::runtime_rejected(
            "expected created entity in document",
        ))?;

    let add_transform_command_id = runtime.allocate_command_id();
    execute_scene_intent(
        runtime,
        add_transform_command_id,
        SceneCommandIntent::AddComponent {
            entity,
            component_type: LOCAL_TRANSFORM_COMPONENT_TYPE_ID,
        },
    )?;

    let add_primitive_command_id = runtime.allocate_command_id();
    execute_scene_intent(
        runtime,
        add_primitive_command_id,
        SceneCommandIntent::AddComponent {
            entity,
            component_type: EDITOR_PRIMITIVE_COMPONENT_TYPE_ID,
        },
    )?;

    runtime.insert_component_for_editor_entity(entity, transform)?;
    runtime.insert_component_for_editor_entity(entity, primitive)?;

    Ok(entity)
}

pub fn is_local_transform_component(
    runtime: &RunenwerkEditorRuntime,
    component_type: ComponentTypeId,
) -> bool {
    runtime.ids().resolve_component_rust_type_id(component_type)
        == Some(TypeId::of::<LocalTransform>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::systems::extract_viewport_scene_render_packet;

    #[test]
    fn viewport_bootstrap_creates_graybox_and_ground_plane_entities() {
        let mut runtime = RunenwerkEditorRuntime::new();
        register_mvp_component_types(&mut runtime);

        bootstrap_mvp_scene_if_empty(&mut runtime).expect("mvp bootstrap should succeed");

        let entities = runtime.document().entity_ids().collect::<Vec<_>>();
        assert_eq!(entities.len(), 2);

        let names = entities
            .iter()
            .filter_map(|entity| runtime.document().entity_snapshot(*entity))
            .map(|snapshot| snapshot.display_name)
            .collect::<Vec<_>>();
        assert!(names.contains(&MVP_GRAYBOX_ENTITY_NAME.to_string()));
        assert!(names.contains(&MVP_GROUND_PLANE_ENTITY_NAME.to_string()));

        let packet = extract_viewport_scene_render_packet(&runtime, None);
        assert_eq!(packet.len(), 2);
        assert!(
            packet
                .primitives()
                .iter()
                .any(|primitive| primitive.primitive_kind == EditorPrimitiveKind::Plane),
            "the default ground plane must enter the viewport scene packet as a normal entity"
        );
    }
}
