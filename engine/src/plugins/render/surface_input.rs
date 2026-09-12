//! Request-scoped semantic surface input for representations whose surface protocol depends on a
//! current source/adaptor value.
//!
//! The value is renderer-semantic input, not intrinsic representation evidence, source identity,
//! product scenario identity, or RunenGPU realization. Representation identity is supplied only by
//! [`RenderSurfaceSemanticInputBinding`], allowing the same immutable semantic value shape to be
//! bound independently to distinct representation records when their owning source contracts allow
//! it.

use super::derived_transform::{RenderCompiledObjectTransform, RenderCompiledObjectTransformError};
use super::representation::{
    RenderRepresentationId, RenderRepresentationValidationError, RenderSurfaceQuery,
};
use super::scene::RenderObjectState;
use super::space_time::{
    CanonicalF64, RenderSemanticValueError, RenderTemporalSupport, RenderTimeInterval,
};
use super::surface_result::RenderOrientedSurfaceQueryResult;
use std::error::Error;
use std::fmt;

/// Narrow typed declaration that one surface protocol requires a current request-scoped semantic
/// surface value before the representation use is semantically admissible.
///
/// This marker intentionally has no registry key, provider identity, generation, GPU handle, or
/// dynamic type token. A representation that can satisfy the same protocol intrinsically simply
/// omits this prerequisite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderSurfaceSemanticInputRequirement {
    _private: (),
}

impl RenderSurfaceSemanticInputRequirement {
    pub const fn current() -> Self {
        Self { _private: () }
    }
}

/// Immutable request-scoped surface semantic value.
///
/// The public construction surface is deliberately limited to the concrete pressure demonstrated by
/// the maintained deterministic renderer. This is not a closed representation-family ontology.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderSurfaceSemanticInput {
    kind: RenderSurfaceSemanticInputKind,
    validity: RenderTemporalSupport,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum RenderSurfaceSemanticInputKind {
    Sphere {
        center_local_units: [CanonicalF64; 3],
        radius_local_units: CanonicalF64,
    },
    Plane {
        point_local_units: [CanonicalF64; 3],
        normal_local: [CanonicalF64; 3],
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum RenderSurfaceSemanticInputShape {
    Sphere {
        center_local_units: [f64; 3],
        radius_local_units: f64,
    },
    Plane {
        point_local_units: [f64; 3],
        normal_local: [f64; 3],
    },
}

impl RenderSurfaceSemanticInput {
    pub fn sphere(
        center_local_units: [f64; 3],
        radius_local_units: f64,
        validity: RenderTemporalSupport,
    ) -> Result<Self, RenderSurfaceSemanticInputError> {
        Ok(Self {
            kind: RenderSurfaceSemanticInputKind::Sphere {
                center_local_units: canonical_point(center_local_units, "surface_input_sphere_center")?,
                radius_local_units: positive_value(
                    radius_local_units,
                    "surface_input_sphere_radius",
                )?,
            },
            validity,
        })
    }

    pub fn plane(
        point_local_units: [f64; 3],
        normal_local: [f64; 3],
        validity: RenderTemporalSupport,
    ) -> Result<Self, RenderSurfaceSemanticInputError> {
        Ok(Self {
            kind: RenderSurfaceSemanticInputKind::Plane {
                point_local_units: canonical_point(point_local_units, "surface_input_plane_point")?,
                normal_local: canonical_unit_direction(
                    normal_local,
                    "surface_input_plane_normal",
                )?,
            },
            validity,
        })
    }

    pub const fn validity(&self) -> RenderTemporalSupport {
        self.validity
    }

    pub(crate) fn shape(&self) -> RenderSurfaceSemanticInputShape {
        match &self.kind {
            RenderSurfaceSemanticInputKind::Sphere {
                center_local_units,
                radius_local_units,
            } => RenderSurfaceSemanticInputShape::Sphere {
                center_local_units: center_local_units.map(CanonicalF64::get),
                radius_local_units: radius_local_units.get(),
            },
            RenderSurfaceSemanticInputKind::Plane {
                point_local_units,
                normal_local,
            } => RenderSurfaceSemanticInputShape::Plane {
                point_local_units: point_local_units.map(CanonicalF64::get),
                normal_local: normal_local.map(CanonicalF64::get),
            },
        }
    }

    pub(crate) fn oriented_surface_query(
        &self,
        object_state: &RenderObjectState,
        query: RenderSurfaceQuery,
    ) -> Result<RenderOrientedSurfaceQueryResult, RenderSurfaceSemanticInputError> {
        if !self
            .validity
            .contains_interval(RenderTimeInterval::instant(query.time()))
        {
            return Err(RenderSurfaceSemanticInputError::QueryOutsideValidity);
        }
        let transform = RenderCompiledObjectTransform::compile(object_state.spatial())?;
        match self.shape() {
            RenderSurfaceSemanticInputShape::Sphere {
                center_local_units,
                radius_local_units,
            } => sphere_surface_query(
                query,
                &transform,
                center_local_units,
                radius_local_units,
            ),
            RenderSurfaceSemanticInputShape::Plane {
                point_local_units,
                normal_local,
            } => plane_surface_query(query, &transform, point_local_units, normal_local),
        }
    }
}

/// Invocation-local correlation between one exact representation identity and one immutable current
/// semantic surface value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderSurfaceSemanticInputBinding {
    representation_id: RenderRepresentationId,
    input: RenderSurfaceSemanticInput,
}

impl RenderSurfaceSemanticInputBinding {
    pub fn new(
        representation_id: RenderRepresentationId,
        input: RenderSurfaceSemanticInput,
    ) -> Self {
        Self {
            representation_id,
            input,
        }
    }

    pub const fn representation_id(&self) -> RenderRepresentationId {
        self.representation_id
    }

    pub const fn input(&self) -> &RenderSurfaceSemanticInput {
        &self.input
    }

    pub fn into_input(self) -> RenderSurfaceSemanticInput {
        self.input
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderSurfaceSemanticInputError {
    SemanticValue(RenderSemanticValueError),
    Representation(RenderRepresentationValidationError),
    NonPositiveRadius,
    ZeroNormal,
    NonInvertibleObjectTransform,
    QueryOutsideValidity,
    NonFiniteEvaluation,
}

impl From<RenderSemanticValueError> for RenderSurfaceSemanticInputError {
    fn from(value: RenderSemanticValueError) -> Self {
        Self::SemanticValue(value)
    }
}

impl From<RenderRepresentationValidationError> for RenderSurfaceSemanticInputError {
    fn from(value: RenderRepresentationValidationError) -> Self {
        Self::Representation(value)
    }
}

impl From<RenderCompiledObjectTransformError> for RenderSurfaceSemanticInputError {
    fn from(value: RenderCompiledObjectTransformError) -> Self {
        match value {
            RenderCompiledObjectTransformError::NonInvertibleObjectTransform => {
                Self::NonInvertibleObjectTransform
            }
        }
    }
}

impl fmt::Display for RenderSurfaceSemanticInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SemanticValue(error) => fmt::Display::fmt(error, formatter),
            Self::Representation(error) => fmt::Display::fmt(error, formatter),
            Self::NonPositiveRadius => write!(formatter, "surface-input sphere radius must be positive"),
            Self::ZeroNormal => write!(formatter, "surface-input plane normal must be non-zero"),
            Self::NonInvertibleObjectTransform => {
                write!(formatter, "surface-input query requires an invertible object transform")
            }
            Self::QueryOutsideValidity => {
                write!(formatter, "surface-input query time is outside source-declared validity")
            }
            Self::NonFiniteEvaluation => {
                write!(formatter, "surface-input query produced a non-finite semantic evaluation")
            }
        }
    }
}

impl Error for RenderSurfaceSemanticInputError {}

fn sphere_surface_query(
    query: RenderSurfaceQuery,
    transform: &RenderCompiledObjectTransform,
    center_local_units: [f64; 3],
    radius_local_units: f64,
) -> Result<RenderOrientedSurfaceQueryResult, RenderSurfaceSemanticInputError> {
    let origin_local = transform.local_point_from_scene(query.origin_scene_meters());
    let direction_local = transform.local_direction_per_scene_meter(query.direction_scene());
    let relative_origin = sub(origin_local, center_local_units);
    let a = dot(direction_local, direction_local);
    let half_b = dot(relative_origin, direction_local);
    let c = dot(relative_origin, relative_origin) - radius_local_units * radius_local_units;
    let discriminant = half_b * half_b - a * c;
    if !a.is_finite() || a <= 0.0 || !discriminant.is_finite() {
        return Err(RenderSurfaceSemanticInputError::NonFiniteEvaluation);
    }
    if discriminant < 0.0 {
        return Ok(RenderOrientedSurfaceQueryResult::miss());
    }
    let root = discriminant.sqrt();
    let near = (-half_b - root) / a;
    let far = (-half_b + root) / a;
    let distance_meters = if near >= 0.0 {
        near
    } else if far >= 0.0 {
        far
    } else {
        return Ok(RenderOrientedSurfaceQueryResult::miss());
    };
    if !distance_meters.is_finite() {
        return Err(RenderSurfaceSemanticInputError::NonFiniteEvaluation);
    }
    let local_hit = add(origin_local, scale(direction_local, distance_meters));
    let local_normal = normalize(sub(local_hit, center_local_units))
        .ok_or(RenderSurfaceSemanticInputError::NonFiniteEvaluation)?;
    Ok(RenderOrientedSurfaceQueryResult::hit_at_distance(
        query,
        distance_meters,
        transform.scene_normal_from_local(local_normal),
    )?)
}

fn plane_surface_query(
    query: RenderSurfaceQuery,
    transform: &RenderCompiledObjectTransform,
    point_local_units: [f64; 3],
    normal_local: [f64; 3],
) -> Result<RenderOrientedSurfaceQueryResult, RenderSurfaceSemanticInputError> {
    let point_scene = transform.scene_point_from_local(point_local_units);
    let normal_scene = transform.scene_normal_from_local(normal_local);
    let denominator = dot(normal_scene, query.direction_scene());
    if !denominator.is_finite() {
        return Err(RenderSurfaceSemanticInputError::NonFiniteEvaluation);
    }
    if denominator == 0.0 {
        return Ok(RenderOrientedSurfaceQueryResult::miss());
    }
    let numerator = dot(sub(point_scene, query.origin_scene_meters()), normal_scene);
    let distance_meters = numerator / denominator;
    if !distance_meters.is_finite() {
        return Err(RenderSurfaceSemanticInputError::NonFiniteEvaluation);
    }
    if distance_meters < 0.0 {
        return Ok(RenderOrientedSurfaceQueryResult::miss());
    }
    Ok(RenderOrientedSurfaceQueryResult::hit_at_distance(
        query,
        distance_meters,
        normal_scene,
    )?)
}

fn canonical_point(
    values: [f64; 3],
    field: &'static str,
) -> Result<[CanonicalF64; 3], RenderSurfaceSemanticInputError> {
    Ok([
        CanonicalF64::new(values[0], field)?,
        CanonicalF64::new(values[1], field)?,
        CanonicalF64::new(values[2], field)?,
    ])
}

fn positive_value(
    value: f64,
    field: &'static str,
) -> Result<CanonicalF64, RenderSurfaceSemanticInputError> {
    let value = CanonicalF64::new(value, field)?;
    if value.get() <= 0.0 {
        return Err(RenderSurfaceSemanticInputError::NonPositiveRadius);
    }
    Ok(value)
}

fn canonical_unit_direction(
    direction: [f64; 3],
    field: &'static str,
) -> Result<[CanonicalF64; 3], RenderSurfaceSemanticInputError> {
    let values = [
        CanonicalF64::new(direction[0], field)?.get(),
        CanonicalF64::new(direction[1], field)?.get(),
        CanonicalF64::new(direction[2], field)?.get(),
    ];
    let normalized = normalize(values).ok_or(RenderSurfaceSemanticInputError::ZeroNormal)?;
    Ok([
        CanonicalF64::new(normalized[0], field)?,
        CanonicalF64::new(normalized[1], field)?,
        CanonicalF64::new(normalized[2], field)?,
    ])
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn normalize(vector: [f64; 3]) -> Option<[f64; 3]> {
    let magnitude_squared = dot(vector, vector);
    if magnitude_squared <= 0.0 || !magnitude_squared.is_finite() {
        return None;
    }
    let magnitude = magnitude_squared.sqrt();
    if !magnitude.is_finite() {
        return None;
    }
    Some(scale(vector, magnitude.recip()))
}

fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn sub(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}
