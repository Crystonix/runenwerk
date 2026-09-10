#[derive(Debug, Copy, Clone, PartialEq, Default, runen_ecs::Reflect)]
pub struct Vec3Value {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3Value {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    pub fn one() -> Self {
        Self::new(1.0, 1.0, 1.0)
    }

    pub fn translated(mut self, dx: f32, dy: f32, dz: f32) -> Self {
        self.x += dx;
        self.y += dy;
        self.z += dz;
        self
    }

    pub fn to_glam(self) -> glam::Vec3 {
        glam::Vec3::new(self.x, self.y, self.z)
    }

    pub fn from_glam(value: glam::Vec3) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Reflect)]
pub struct QuatValue {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl QuatValue {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn identity() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    pub fn to_glam(self) -> glam::Quat {
        glam::Quat::from_xyzw(self.x, self.y, self.z, self.w)
    }

    pub fn from_glam(value: glam::Quat) -> Self {
        Self::new(value.x, value.y, value.z, value.w)
    }
}

impl Default for QuatValue {
    fn default() -> Self {
        Self::identity()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component, runen_ecs::Reflect)]
pub struct LocalTransform {
    pub translation: Vec3Value,
    pub rotation: QuatValue,
    pub scale: Vec3Value,
}

impl LocalTransform {
    pub fn new(translation: Vec3Value, rotation: QuatValue, scale: Vec3Value) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    pub fn from_translation(translation: Vec3Value) -> Self {
        Self {
            translation,
            ..Self::default()
        }
    }

    pub fn translated(mut self, dx: f32, dy: f32, dz: f32) -> Self {
        self.translation = self.translation.translated(dx, dy, dz);
        self
    }
}

impl Default for LocalTransform {
    fn default() -> Self {
        Self {
            translation: Vec3Value::zero(),
            rotation: QuatValue::identity(),
            scale: Vec3Value::one(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, runen_ecs::Component, runen_ecs::Reflect)]
pub struct WorldTransform {
    pub translation: Vec3Value,
    pub rotation: QuatValue,
    pub scale: Vec3Value,
}

impl WorldTransform {
    pub fn new(translation: Vec3Value, rotation: QuatValue, scale: Vec3Value) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }
}

impl Default for WorldTransform {
    fn default() -> Self {
        Self {
            translation: Vec3Value::zero(),
            rotation: QuatValue::identity(),
            scale: Vec3Value::one(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_transform_defaults_to_identity() {
        let transform = LocalTransform::default();

        assert_eq!(transform.translation, Vec3Value::zero());
        assert_eq!(transform.rotation, QuatValue::identity());
        assert_eq!(transform.scale, Vec3Value::one());
    }

    #[test]
    fn local_transform_translated_applies_delta() {
        let transform = LocalTransform::from_translation(Vec3Value::new(1.0, 2.0, 3.0))
            .translated(2.0, -1.0, 0.5);

        assert_eq!(transform.translation, Vec3Value::new(3.0, 1.0, 3.5));
    }
}
