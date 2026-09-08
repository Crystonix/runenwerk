use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CanonicalF64(u64);

impl CanonicalF64 {
    pub(crate) fn new(value: f64, field: &'static str) -> Result<Self, RenderSemanticValueError> {
        if !value.is_finite() {
            return Err(RenderSemanticValueError::NonFinite { field });
        }
        let canonical = if value == 0.0 { 0.0 } else { value };
        Ok(Self(canonical.to_bits()))
    }

    pub(crate) const fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderSemanticValueError {
    NonFinite { field: &'static str },
    NonPositive { field: &'static str },
    InvalidBounds,
    InvalidInterval,
    MotionOutsideValidity,
}

impl fmt::Display for RenderSemanticValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field } => write!(f, "{field} must be finite"),
            Self::NonPositive { field } => write!(f, "{field} must be greater than zero"),
            Self::InvalidBounds => write!(f, "spatial bounds minimum must not exceed maximum"),
            Self::InvalidInterval => write!(f, "time interval start must not exceed end"),
            Self::MotionOutsideValidity => {
                write!(
                    f,
                    "motion interval must be contained by temporal validity"
                )
            }
        }
    }
}

impl Error for RenderSemanticValueError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderHandedness {
    Left,
    Right,
}

/// Unit scale and handedness for one renderer-semantic local coordinate frame.
///
/// Source coordinate systems remain source-owned. Adapters project their conventions into this
/// renderer-local vocabulary rather than leaking source spatial types into RunenRender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderSpaceSpec {
    meters_per_unit: CanonicalF64,
    handedness: RenderHandedness,
}

impl RenderSpaceSpec {
    pub fn new(
        meters_per_unit: f64,
        handedness: RenderHandedness,
    ) -> Result<Self, RenderSemanticValueError> {
        let meters_per_unit = CanonicalF64::new(meters_per_unit, "meters_per_unit")?;
        if meters_per_unit.get() <= 0.0 {
            return Err(RenderSemanticValueError::NonPositive {
                field: "meters_per_unit",
            });
        }
        Ok(Self {
            meters_per_unit,
            handedness,
        })
    }

    pub fn meters_per_unit(self) -> f64 {
        self.meters_per_unit.get()
    }

    pub const fn handedness(self) -> RenderHandedness {
        self.handedness
    }
}

/// A finite semantic affine transform from one renderer-local frame into another.
///
/// The 3x4 matrix is row-major. R2 validates finiteness but deliberately does not require
/// invertibility: a semantic source projection may be degenerate without implying a GPU numeric
/// realization or a representation-specific transform-validity guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderAffineTransform3 {
    row_major_3x4: [CanonicalF64; 12],
}

impl RenderAffineTransform3 {
    pub fn from_row_major_3x4(values: [f64; 12]) -> Result<Self, RenderSemanticValueError> {
        let mut row_major_3x4 = [CanonicalF64(0); 12];
        for (slot, value) in row_major_3x4.iter_mut().zip(values) {
            *slot = CanonicalF64::new(value, "affine_transform")?;
        }
        Ok(Self { row_major_3x4 })
    }

    pub const fn identity() -> Self {
        Self {
            row_major_3x4: [
                CanonicalF64(1.0f64.to_bits()),
                CanonicalF64(0),
                CanonicalF64(0),
                CanonicalF64(0),
                CanonicalF64(0),
                CanonicalF64(1.0f64.to_bits()),
                CanonicalF64(0),
                CanonicalF64(0),
                CanonicalF64(0),
                CanonicalF64(0),
                CanonicalF64(1.0f64.to_bits()),
                CanonicalF64(0),
            ],
        }
    }

    pub fn row_major_3x4(self) -> [f64; 12] {
        self.row_major_3x4.map(CanonicalF64::get)
    }
}

/// Renderer-semantic spatial support expressed in scene coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSpatialCoverage {
    kind: RenderSpatialCoverageKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RenderSpatialCoverageKind {
    Unbounded,
    AxisAlignedBounds {
        min: [CanonicalF64; 3],
        max: [CanonicalF64; 3],
    },
}

impl RenderSpatialCoverage {
    pub const fn unbounded() -> Self {
        Self {
            kind: RenderSpatialCoverageKind::Unbounded,
        }
    }

    pub fn axis_aligned_bounds(
        min: [f64; 3],
        max: [f64; 3],
    ) -> Result<Self, RenderSemanticValueError> {
        let canonical_min = [
            CanonicalF64::new(min[0], "spatial_bounds_min")?,
            CanonicalF64::new(min[1], "spatial_bounds_min")?,
            CanonicalF64::new(min[2], "spatial_bounds_min")?,
        ];
        let canonical_max = [
            CanonicalF64::new(max[0], "spatial_bounds_max")?,
            CanonicalF64::new(max[1], "spatial_bounds_max")?,
            CanonicalF64::new(max[2], "spatial_bounds_max")?,
        ];
        if canonical_min
            .iter()
            .zip(canonical_max.iter())
            .any(|(min, max)| min.get() > max.get())
        {
            return Err(RenderSemanticValueError::InvalidBounds);
        }
        Ok(Self {
            kind: RenderSpatialCoverageKind::AxisAlignedBounds {
                min: canonical_min,
                max: canonical_max,
            },
        })
    }

    pub fn axis_aligned_bounds_value(&self) -> Option<([f64; 3], [f64; 3])> {
        match &self.kind {
            RenderSpatialCoverageKind::Unbounded => None,
            RenderSpatialCoverageKind::AxisAlignedBounds { min, max } => {
                Some((min.map(CanonicalF64::get), max.map(CanonicalF64::get)))
            }
        }
    }

    pub fn is_unbounded(&self) -> bool {
        matches!(self.kind, RenderSpatialCoverageKind::Unbounded)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderTimePoint {
    seconds: CanonicalF64,
}

impl RenderTimePoint {
    pub fn from_seconds(seconds: f64) -> Result<Self, RenderSemanticValueError> {
        Ok(Self {
            seconds: CanonicalF64::new(seconds, "render_time")?,
        })
    }

    pub fn seconds(self) -> f64 {
        self.seconds.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderTimeInterval {
    start: RenderTimePoint,
    end: RenderTimePoint,
}

impl RenderTimeInterval {
    pub fn new(
        start: RenderTimePoint,
        end: RenderTimePoint,
    ) -> Result<Self, RenderSemanticValueError> {
        if start.seconds() > end.seconds() {
            return Err(RenderSemanticValueError::InvalidInterval);
        }
        Ok(Self { start, end })
    }

    pub fn instant(time: RenderTimePoint) -> Self {
        Self {
            start: time,
            end: time,
        }
    }

    pub const fn start(self) -> RenderTimePoint {
        self.start
    }

    pub const fn end(self) -> RenderTimePoint {
        self.end
    }

    pub fn contains(self, other: Self) -> bool {
        self.start.seconds() <= other.start.seconds() && self.end.seconds() >= other.end.seconds()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderTemporalSupport {
    interval: Option<RenderTimeInterval>,
}

impl RenderTemporalSupport {
    pub const fn unbounded() -> Self {
        Self { interval: None }
    }

    pub const fn interval(interval: RenderTimeInterval) -> Self {
        Self {
            interval: Some(interval),
        }
    }

    pub const fn bounded_interval(self) -> Option<RenderTimeInterval> {
        self.interval
    }

    pub const fn is_unbounded(self) -> bool {
        self.interval.is_none()
    }

    pub fn contains_interval(self, interval: RenderTimeInterval) -> bool {
        self.interval
            .is_none_or(|validity| validity.contains(interval))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderObjectSpatialState {
    local_space: RenderSpaceSpec,
    local_to_scene: RenderAffineTransform3,
    scene_coverage: RenderSpatialCoverage,
}

impl RenderObjectSpatialState {
    pub fn new(
        local_space: RenderSpaceSpec,
        local_to_scene: RenderAffineTransform3,
        scene_coverage: RenderSpatialCoverage,
    ) -> Self {
        Self {
            local_space,
            local_to_scene,
            scene_coverage,
        }
    }

    pub const fn local_space(&self) -> RenderSpaceSpec {
        self.local_space
    }

    pub const fn local_to_scene(&self) -> RenderAffineTransform3 {
        self.local_to_scene
    }

    pub const fn scene_coverage(&self) -> &RenderSpatialCoverage {
        &self.scene_coverage
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderObjectTemporalState {
    validity: RenderTemporalSupport,
    motion_interval: Option<RenderTimeInterval>,
}

impl RenderObjectTemporalState {
    pub fn new(
        validity: RenderTemporalSupport,
        motion_interval: Option<RenderTimeInterval>,
    ) -> Result<Self, RenderSemanticValueError> {
        if let Some(motion_interval) = motion_interval
            && !validity.contains_interval(motion_interval)
        {
            return Err(RenderSemanticValueError::MotionOutsideValidity);
        }
        Ok(Self {
            validity,
            motion_interval,
        })
    }

    pub const fn validity(self) -> RenderTemporalSupport {
        self.validity
    }

    pub const fn motion_interval(self) -> Option<RenderTimeInterval> {
        self.motion_interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_space_rejects_invalid_scale_and_transform_values() {
        assert_eq!(
            RenderSpaceSpec::new(0.0, RenderHandedness::Right),
            Err(RenderSemanticValueError::NonPositive {
                field: "meters_per_unit"
            })
        );
        assert!(matches!(
            RenderAffineTransform3::from_row_major_3x4([
                f64::NAN,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
            ]),
            Err(RenderSemanticValueError::NonFinite { .. })
        ));
    }

    #[test]
    fn bounds_and_time_intervals_validate_semantic_ordering() {
        assert_eq!(
            RenderSpatialCoverage::axis_aligned_bounds([1.0, 0.0, 0.0], [0.0, 1.0, 1.0]),
            Err(RenderSemanticValueError::InvalidBounds)
        );
        let start = RenderTimePoint::from_seconds(2.0).expect("finite time");
        let end = RenderTimePoint::from_seconds(1.0).expect("finite time");
        assert_eq!(
            RenderTimeInterval::new(start, end),
            Err(RenderSemanticValueError::InvalidInterval)
        );
    }

    #[test]
    fn motion_interval_must_fit_temporal_validity() {
        let validity = RenderTimeInterval::new(
            RenderTimePoint::from_seconds(1.0).expect("finite time"),
            RenderTimePoint::from_seconds(2.0).expect("finite time"),
        )
        .expect("ordered interval");
        let motion = RenderTimeInterval::new(
            RenderTimePoint::from_seconds(0.0).expect("finite time"),
            RenderTimePoint::from_seconds(2.0).expect("finite time"),
        )
        .expect("ordered interval");
        assert_eq!(
            RenderObjectTemporalState::new(RenderTemporalSupport::interval(validity), Some(motion)),
            Err(RenderSemanticValueError::MotionOutsideValidity)
        );
    }

    #[test]
    fn object_spatial_coverage_is_explicitly_scene_space() {
        let coverage = RenderSpatialCoverage::axis_aligned_bounds(
            [-2.0, -1.0, -1.0],
            [2.0, 1.0, 1.0],
        )
        .expect("valid bounds");
        let state = RenderObjectSpatialState::new(
            RenderSpaceSpec::new(0.01, RenderHandedness::Left).expect("valid local space"),
            RenderAffineTransform3::identity(),
            coverage.clone(),
        );
        assert_eq!(state.scene_coverage(), &coverage);
        assert_eq!(state.local_space().meters_per_unit(), 0.01);
        assert_eq!(state.local_space().handedness(), RenderHandedness::Left);
    }
}
