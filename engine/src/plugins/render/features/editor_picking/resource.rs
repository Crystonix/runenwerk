#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorGizmoAxis {
    #[default]
    X,
    Y,
    Z,
}

impl EditorGizmoAxis {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorPickingTarget {
    #[default]
    None,
    Grid,
    Entity(u64),
    ComponentHandle {
        entity: u64,
        component_type: u64,
    },
    GizmoAxis(EditorGizmoAxis),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorPickingHit {
    pub target: EditorPickingTarget,
    pub distance: f32,
}

impl EditorPickingHit {
    pub const fn none() -> Self {
        Self {
            target: EditorPickingTarget::None,
            distance: f32::INFINITY,
        }
    }
}

impl Default for EditorPickingHit {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Debug, Clone, Copy, runen_ecs::Component, runen_ecs::Resource)]
pub struct EditorPickingResultResource {
    pub cursor_px: (f32, f32),
    pub viewport_bounds_px: (f32, f32, f32, f32),
    pub hit: EditorPickingHit,
    pub revision: u64,
}

impl Default for EditorPickingResultResource {
    fn default() -> Self {
        Self {
            cursor_px: (0.0, 0.0),
            viewport_bounds_px: (0.0, 0.0, 0.0, 0.0),
            hit: EditorPickingHit::default(),
            revision: 0,
        }
    }
}

impl EditorPickingResultResource {
    pub fn set_cursor(&mut self, cursor_px: (f32, f32), viewport_bounds_px: (f32, f32, f32, f32)) {
        self.cursor_px = cursor_px;
        self.viewport_bounds_px = viewport_bounds_px;
    }

    pub fn set_hit(&mut self, hit: EditorPickingHit) {
        self.hit = hit;
        self.revision = self.revision.saturating_add(1);
    }

    pub fn clear_hit(&mut self) {
        self.set_hit(EditorPickingHit::none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_hit_resets_target_to_none() {
        let mut resource = EditorPickingResultResource::default();
        resource.set_hit(EditorPickingHit {
            target: EditorPickingTarget::Entity(7),
            distance: 1.0,
        });

        resource.clear_hit();

        assert_eq!(resource.hit.target, EditorPickingTarget::None);
        assert!(resource.hit.distance.is_infinite());
    }
}
