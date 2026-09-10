//! File: domain/editor/editor_inspector/src/bridge/ecs_adapter.rs
//! Purpose: ECS reflection-backed inspector adapter for components and resources.

use crate::{
    EcsInspectorBridge, InspectTarget, InspectorAdapter, InspectorAdapterError, InspectorField,
    InspectorPath, InspectorSection, InspectorValue,
};
use editor_core::{ComponentTypeId, EntityId, ResourceTypeId};
use runen_ecs::reflect::{ReflectValueRef, StructValueRef};

pub struct EcsInspectorAdapter<'a, B> {
    world: &'a runen_ecs::World,
    bridge: &'a B,
}

impl<'a, B> EcsInspectorAdapter<'a, B>
where
    B: EcsInspectorBridge,
{
    pub fn new(world: &'a runen_ecs::World, bridge: &'a B) -> Self {
        Self { world, bridge }
    }

    fn build_component_sections(
        &self,
        entity_id: EntityId,
        component_type: ComponentTypeId,
    ) -> Result<Vec<InspectorSection>, InspectorAdapterError> {
        let entity = self
            .bridge
            .resolve_entity(entity_id)
            .ok_or(InspectorAdapterError::TargetNotFound)?;

        let rust_type_id = self
            .bridge
            .resolve_component_rust_type_id(component_type)
            .ok_or(InspectorAdapterError::TypeNotRegistered)?;

        let type_info = self
            .world
            .reflected_component_type_info(rust_type_id)
            .ok_or(InspectorAdapterError::TypeNotRegistered)?;

        let reflected = self
            .world
            .reflected_component_value_ref(entity, rust_type_id)
            .ok_or(InspectorAdapterError::ValueNotAvailable)?;

        let root_field = build_reflect_field(
            type_info.display_name,
            type_info.display_name,
            InspectorPath::root(),
            reflected,
        );

        Ok(vec![
            InspectorSection::new(type_info.display_name, type_info.display_name)
                .with_field(root_field),
        ])
    }

    fn build_resource_sections(
        &self,
        resource_type: ResourceTypeId,
    ) -> Result<Vec<InspectorSection>, InspectorAdapterError> {
        let rust_type_id = self
            .bridge
            .resolve_resource_rust_type_id(resource_type)
            .ok_or(InspectorAdapterError::TypeNotRegistered)?;

        let type_info = self
            .world
            .reflected_resource_type_info(rust_type_id)
            .ok_or(InspectorAdapterError::TypeNotRegistered)?;

        let reflected = self
            .world
            .reflected_resource_value_ref(rust_type_id)
            .ok_or(InspectorAdapterError::ValueNotAvailable)?;

        let root_field = build_reflect_field(
            type_info.display_name,
            type_info.display_name,
            InspectorPath::root(),
            reflected,
        );

        Ok(vec![
            InspectorSection::new(type_info.display_name, type_info.display_name)
                .with_field(root_field),
        ])
    }
}

impl<'a, B> InspectorAdapter for EcsInspectorAdapter<'a, B>
where
    B: EcsInspectorBridge,
{
    type Error = InspectorAdapterError;

    fn supports(&self, target: &InspectTarget) -> bool {
        matches!(
            target,
            InspectTarget::Component { .. } | InspectTarget::Resource(_)
        )
    }

    fn build_sections(&self, target: &InspectTarget) -> Result<Vec<InspectorSection>, Self::Error> {
        match target {
            InspectTarget::Component {
                entity,
                component_type,
            } => self.build_component_sections(*entity, *component_type),
            InspectTarget::Resource(resource_type) => self.build_resource_sections(*resource_type),
            _ => Err(InspectorAdapterError::UnsupportedTarget),
        }
    }
}

fn build_reflect_field(
    stable_name: impl Into<String>,
    display_name: impl Into<String>,
    path: InspectorPath,
    value: ReflectValueRef<'_>,
) -> InspectorField {
    let stable_name = stable_name.into();
    let display_name = display_name.into();

    if let Some(struct_ref) = value.struct_ref() {
        return build_struct_field(stable_name, display_name, path, struct_ref);
    }
    if let Some(enum_ref) = value.enum_ref() {
        return build_enum_field(stable_name, display_name, path, enum_ref);
    }

    build_leaf_field(stable_name, display_name, path, value)
}

fn build_struct_field(
    stable_name: String,
    display_name: String,
    path: InspectorPath,
    struct_ref: StructValueRef<'_>,
) -> InspectorField {
    let mut field = InspectorField::new(
        stable_name,
        display_name,
        path.clone(),
        InspectorValue::Group,
    )
    .read_only(true);

    for meta in struct_ref.fields() {
        let child_path = path.child_field(meta.name);
        let Some(child_value) = struct_ref.field(meta.name) else {
            continue;
        };

        field = field.with_child(build_reflect_field(
            meta.name,
            meta.display_name,
            child_path,
            child_value,
        ));
    }

    field
}

fn build_enum_field(
    stable_name: String,
    display_name: String,
    path: InspectorPath,
    enum_ref: runen_ecs::reflect::EnumValueRef<'_>,
) -> InspectorField {
    let Some(current) = enum_ref.current_symbol() else {
        return InspectorField::new(
            stable_name,
            display_name,
            path,
            InspectorValue::Unsupported {
                type_name: "enum<unreadable>".to_string(),
            },
        )
        .read_only(true);
    };
    let options = enum_ref
        .variants()
        .iter()
        .map(|variant| variant.symbol.to_string())
        .collect::<Vec<_>>();
    InspectorField::new(
        stable_name,
        display_name,
        path,
        InspectorValue::Enum {
            current: current.to_string(),
            options,
        },
    )
}

fn build_leaf_field(
    stable_name: String,
    display_name: String,
    path: InspectorPath,
    value: ReflectValueRef<'_>,
) -> InspectorField {
    let inspector_value = if let Some(v) = value.downcast_ref::<bool>() {
        InspectorValue::Bool(*v)
    } else if let Some(v) = value.downcast_ref::<i8>() {
        InspectorValue::Integer(*v as i64)
    } else if let Some(v) = value.downcast_ref::<i16>() {
        InspectorValue::Integer(*v as i64)
    } else if let Some(v) = value.downcast_ref::<i32>() {
        InspectorValue::Integer(*v as i64)
    } else if let Some(v) = value.downcast_ref::<i64>() {
        InspectorValue::Integer(*v)
    } else if let Some(v) = value.downcast_ref::<isize>() {
        InspectorValue::Integer(*v as i64)
    } else if let Some(v) = value.downcast_ref::<u8>() {
        InspectorValue::Integer(*v as i64)
    } else if let Some(v) = value.downcast_ref::<u16>() {
        InspectorValue::Integer(*v as i64)
    } else if let Some(v) = value.downcast_ref::<u32>() {
        InspectorValue::Integer(*v as i64)
    } else if let Some(v) = value.downcast_ref::<u64>() {
        InspectorValue::Integer(*v as i64)
    } else if let Some(v) = value.downcast_ref::<usize>() {
        InspectorValue::Integer(*v as i64)
    } else if let Some(v) = value.downcast_ref::<f32>() {
        InspectorValue::Float(*v as f64)
    } else if let Some(v) = value.downcast_ref::<f64>() {
        InspectorValue::Float(*v)
    } else if let Some(v) = value.downcast_ref::<String>() {
        InspectorValue::Text(v.clone())
    } else {
        InspectorValue::Unsupported {
            type_name: value.type_info().display_name.to_string(),
        }
    };

    let is_read_only = matches!(inspector_value, InspectorValue::Unsupported { .. });
    InspectorField::new(stable_name, display_name, path, inspector_value).read_only(is_read_only)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StaticEcsInspectorBridge;

    #[derive(Debug, Clone, runen_ecs::Reflect)]
    struct Vec2 {
        x: f32,
        y: f32,
    }

    #[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Reflect)]
    struct Position {
        value: Vec2,
        speed: f32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, runen_ecs::Reflect)]
    enum TextureFilter {
        Nearest,
        Linear,
    }

    #[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Reflect)]
    struct SpritePreview {
        filter: TextureFilter,
    }

    #[derive(Debug, Clone, runen_ecs::Resource, runen_ecs::Reflect)]
    struct CameraSettings {
        zoom: f32,
        exposure: f32,
    }

    #[test]
    fn builds_component_sections_for_reflected_component() {
        let mut world = runen_ecs::World::new();
        world.register_reflected_component::<Position>();

        let entity = world
            .spawn(Position {
                value: Vec2 { x: 1.0, y: 2.0 },
                speed: 3.5,
            })
            .expect("spawn should succeed");

        let bridge = StaticEcsInspectorBridge::new()
            .with_entity(EntityId(1), entity)
            .with_component_type::<Position>(ComponentTypeId(10));

        let adapter = EcsInspectorAdapter::new(&world, &bridge);

        let sections = adapter
            .build_sections(&InspectTarget::Component {
                entity: EntityId(1),
                component_type: ComponentTypeId(10),
            })
            .expect("component sections should build");

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].fields.len(), 1);

        let root = &sections[0].fields[0];
        assert!(matches!(root.value, InspectorValue::Group));
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn builds_resource_sections_for_reflected_resource() {
        let mut world = runen_ecs::World::new();
        world.register_reflected_resource::<CameraSettings>();
        world.insert_resource(CameraSettings {
            zoom: 2.0,
            exposure: 1.25,
        });

        let bridge = StaticEcsInspectorBridge::new()
            .with_resource_type::<CameraSettings>(ResourceTypeId(20));

        let adapter = EcsInspectorAdapter::new(&world, &bridge);

        let sections = adapter
            .build_sections(&InspectTarget::Resource(ResourceTypeId(20)))
            .expect("resource sections should build");

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].fields.len(), 1);

        let root = &sections[0].fields[0];
        assert!(matches!(root.value, InspectorValue::Group));
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn builds_enum_fields_for_reflected_component() {
        let mut world = runen_ecs::World::new();
        world.register_reflected_component::<SpritePreview>();

        let entity = world
            .spawn(SpritePreview {
                filter: TextureFilter::Linear,
            })
            .expect("spawn should succeed");

        let bridge = StaticEcsInspectorBridge::new()
            .with_entity(EntityId(1), entity)
            .with_component_type::<SpritePreview>(ComponentTypeId(10));

        let adapter = EcsInspectorAdapter::new(&world, &bridge);

        let sections = adapter
            .build_sections(&InspectTarget::Component {
                entity: EntityId(1),
                component_type: ComponentTypeId(10),
            })
            .expect("component sections should build");

        let root = &sections[0].fields[0];
        let filter = root
            .children
            .iter()
            .find(|field| field.stable_name == "filter")
            .expect("filter field should be reflected");

        assert!(matches!(
            &filter.value,
            InspectorValue::Enum { current, options }
                if current == "Linear"
                    && options == &vec!["Nearest".to_string(), "Linear".to_string()]
        ));
    }
}
