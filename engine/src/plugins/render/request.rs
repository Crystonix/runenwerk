use super::space_time::{
    CanonicalF64, RenderAffineTransform3, RenderSemanticValueError, RenderTimeInterval,
};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderRequestValidationError {
    SemanticValue(RenderSemanticValueError),
    NonPositive { field: &'static str },
    PerspectiveFieldOfViewOutOfRange,
    SamplingConeOutOfRange,
    InvalidLatticeDimensions,
    IdentityToleranceMustBeExact,
    EmptyObservations,
    EmptyOutputs,
    OutputObservationOutOfRange { observation_index: usize },
    ObservationOutsideRenderInterval { observation_index: usize },
    ProbeRequiresScalarTopology { observation_index: usize },
    ObservationHasNoOutputs { observation_index: usize },
}

impl From<RenderSemanticValueError> for RenderRequestValidationError {
    fn from(value: RenderSemanticValueError) -> Self {
        Self::SemanticValue(value)
    }
}

impl fmt::Display for RenderRequestValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SemanticValue(error) => error.fmt(f),
            Self::NonPositive { field } => write!(f, "{field} must be greater than zero"),
            Self::PerspectiveFieldOfViewOutOfRange => {
                write!(f, "perspective field of view must be between zero and pi radians")
            }
            Self::SamplingConeOutOfRange => {
                write!(f, "sampling cone half-angle must be between zero and pi/2 radians")
            }
            Self::InvalidLatticeDimensions => {
                write!(f, "sample lattice dimensions must both be non-zero")
            }
            Self::IdentityToleranceMustBeExact => {
                write!(f, "object-identity output requires exact semantic tolerance")
            }
            Self::EmptyObservations => write!(f, "render request must contain an observation"),
            Self::EmptyOutputs => write!(f, "render request must contain an output"),
            Self::OutputObservationOutOfRange { observation_index } => write!(
                f,
                "requested output references missing observation index {observation_index}"
            ),
            Self::ObservationOutsideRenderInterval { observation_index } => write!(
                f,
                "observation index {observation_index} has shutter support outside the render interval"
            ),
            Self::ProbeRequiresScalarTopology { observation_index } => write!(
                f,
                "probe observation index {observation_index} requires scalar result topology"
            ),
            Self::ObservationHasNoOutputs { observation_index } => write!(
                f,
                "observation index {observation_index} has no requested outputs"
            ),
        }
    }
}

impl Error for RenderRequestValidationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderSamplingSupport {
    kind: RenderSamplingSupportKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RenderSamplingSupportKind {
    IdealRay,
    Cone { half_angle_radians: CanonicalF64 },
}

impl RenderSamplingSupport {
    pub const fn ideal_ray() -> Self {
        Self {
            kind: RenderSamplingSupportKind::IdealRay,
        }
    }

    pub fn cone(half_angle_radians: f64) -> Result<Self, RenderRequestValidationError> {
        let half_angle_radians = CanonicalF64::new(half_angle_radians, "half_angle_radians")?;
        if !(0.0..std::f64::consts::FRAC_PI_2).contains(&half_angle_radians.get()) {
            return Err(RenderRequestValidationError::SamplingConeOutOfRange);
        }
        Ok(Self {
            kind: RenderSamplingSupportKind::Cone { half_angle_radians },
        })
    }

    pub const fn is_ideal_ray(self) -> bool {
        matches!(self.kind, RenderSamplingSupportKind::IdealRay)
    }

    pub fn cone_half_angle_radians(self) -> Option<f64> {
        match self.kind {
            RenderSamplingSupportKind::IdealRay => None,
            RenderSamplingSupportKind::Cone { half_angle_radians } => {
                Some(half_angle_radians.get())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderPerspectiveObservation {
    observation_to_scene: RenderAffineTransform3,
    vertical_field_of_view_radians: CanonicalF64,
    aspect_ratio: CanonicalF64,
    shutter: RenderTimeInterval,
    sampling_support: RenderSamplingSupport,
}

impl RenderPerspectiveObservation {
    pub fn new(
        observation_to_scene: RenderAffineTransform3,
        vertical_field_of_view_radians: f64,
        aspect_ratio: f64,
        shutter: RenderTimeInterval,
        sampling_support: RenderSamplingSupport,
    ) -> Result<Self, RenderRequestValidationError> {
        let vertical_field_of_view_radians = CanonicalF64::new(
            vertical_field_of_view_radians,
            "vertical_field_of_view_radians",
        )?;
        if !(0.0..std::f64::consts::PI).contains(&vertical_field_of_view_radians.get()) {
            return Err(RenderRequestValidationError::PerspectiveFieldOfViewOutOfRange);
        }
        let aspect_ratio = CanonicalF64::new(aspect_ratio, "aspect_ratio")?;
        if aspect_ratio.get() <= 0.0 {
            return Err(RenderRequestValidationError::NonPositive {
                field: "aspect_ratio",
            });
        }
        Ok(Self {
            observation_to_scene,
            vertical_field_of_view_radians,
            aspect_ratio,
            shutter,
            sampling_support,
        })
    }

    pub const fn observation_to_scene(self) -> RenderAffineTransform3 {
        self.observation_to_scene
    }

    pub fn vertical_field_of_view_radians(self) -> f64 {
        self.vertical_field_of_view_radians.get()
    }

    pub fn aspect_ratio(self) -> f64 {
        self.aspect_ratio.get()
    }

    pub const fn shutter(self) -> RenderTimeInterval {
        self.shutter
    }

    pub const fn sampling_support(self) -> RenderSamplingSupport {
        self.sampling_support
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderProbeObservation {
    observation_to_scene: RenderAffineTransform3,
    shutter: RenderTimeInterval,
    sampling_support: RenderSamplingSupport,
}

impl RenderProbeObservation {
    pub const fn new(
        observation_to_scene: RenderAffineTransform3,
        shutter: RenderTimeInterval,
        sampling_support: RenderSamplingSupport,
    ) -> Self {
        Self {
            observation_to_scene,
            shutter,
            sampling_support,
        }
    }

    pub const fn observation_to_scene(self) -> RenderAffineTransform3 {
        self.observation_to_scene
    }

    pub const fn shutter(self) -> RenderTimeInterval {
        self.shutter
    }

    pub const fn sampling_support(self) -> RenderSamplingSupport {
        self.sampling_support
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderObservationSpec {
    Perspective(RenderPerspectiveObservation),
    Probe(RenderProbeObservation),
}

impl RenderObservationSpec {
    pub const fn shutter(self) -> RenderTimeInterval {
        match self {
            Self::Perspective(observation) => observation.shutter(),
            Self::Probe(observation) => observation.shutter(),
        }
    }

    pub const fn is_probe(self) -> bool {
        matches!(self, Self::Probe(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderRadiometricRepresentation {
    Monochromatic,
    Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderDistanceConvention {
    RayDistance,
    ObservationForwardDepth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderOutputValue {
    Radiance {
        representation: RenderRadiometricRepresentation,
    },
    Distance {
        convention: RenderDistanceConvention,
    },
    ObjectIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderResultTopology {
    kind: RenderResultTopologyKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RenderResultTopologyKind {
    Scalar,
    SampleLattice2D { width: u32, height: u32 },
}

impl RenderResultTopology {
    pub const fn scalar() -> Self {
        Self {
            kind: RenderResultTopologyKind::Scalar,
        }
    }

    pub fn sample_lattice_2d(
        width: u32,
        height: u32,
    ) -> Result<Self, RenderRequestValidationError> {
        if width == 0 || height == 0 {
            return Err(RenderRequestValidationError::InvalidLatticeDimensions);
        }
        Ok(Self {
            kind: RenderResultTopologyKind::SampleLattice2D { width, height },
        })
    }

    pub const fn is_scalar(self) -> bool {
        matches!(self.kind, RenderResultTopologyKind::Scalar)
    }

    pub const fn sample_lattice_dimensions(self) -> Option<(u32, u32)> {
        match self.kind {
            RenderResultTopologyKind::Scalar => None,
            RenderResultTopologyKind::SampleLattice2D { width, height } => Some((width, height)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderSemanticTolerance {
    kind: RenderSemanticToleranceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RenderSemanticToleranceKind {
    Exact,
    Absolute { max_error: CanonicalF64 },
    Relative { max_fraction: CanonicalF64 },
}

impl RenderSemanticTolerance {
    pub const fn exact() -> Self {
        Self {
            kind: RenderSemanticToleranceKind::Exact,
        }
    }

    pub fn absolute(max_error: f64) -> Result<Self, RenderRequestValidationError> {
        let max_error = CanonicalF64::new(max_error, "absolute_semantic_tolerance")?;
        if max_error.get() < 0.0 {
            return Err(RenderRequestValidationError::NonPositive {
                field: "absolute_semantic_tolerance",
            });
        }
        Ok(Self {
            kind: RenderSemanticToleranceKind::Absolute { max_error },
        })
    }

    pub fn relative(max_fraction: f64) -> Result<Self, RenderRequestValidationError> {
        let max_fraction = CanonicalF64::new(max_fraction, "relative_semantic_tolerance")?;
        if max_fraction.get() < 0.0 {
            return Err(RenderRequestValidationError::NonPositive {
                field: "relative_semantic_tolerance",
            });
        }
        Ok(Self {
            kind: RenderSemanticToleranceKind::Relative { max_fraction },
        })
    }

    pub const fn is_exact(self) -> bool {
        matches!(self.kind, RenderSemanticToleranceKind::Exact)
    }

    pub fn absolute_max_error(self) -> Option<f64> {
        match self.kind {
            RenderSemanticToleranceKind::Absolute { max_error } => Some(max_error.get()),
            _ => None,
        }
    }

    pub fn relative_max_fraction(self) -> Option<f64> {
        match self.kind {
            RenderSemanticToleranceKind::Relative { max_fraction } => Some(max_fraction.get()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderOutputSpec {
    value: RenderOutputValue,
    topology: RenderResultTopology,
    tolerance: RenderSemanticTolerance,
}

impl RenderOutputSpec {
    pub fn new(
        value: RenderOutputValue,
        topology: RenderResultTopology,
        tolerance: RenderSemanticTolerance,
    ) -> Result<Self, RenderRequestValidationError> {
        if matches!(value, RenderOutputValue::ObjectIdentity) && !tolerance.is_exact() {
            return Err(RenderRequestValidationError::IdentityToleranceMustBeExact);
        }
        Ok(Self {
            value,
            topology,
            tolerance,
        })
    }

    pub const fn value(self) -> RenderOutputValue {
        self.value
    }

    pub const fn topology(self) -> RenderResultTopology {
        self.topology
    }

    pub const fn tolerance(self) -> RenderSemanticTolerance {
        self.tolerance
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderRequestedOutput {
    observation_index: usize,
    spec: RenderOutputSpec,
}

impl RenderRequestedOutput {
    pub const fn new(observation_index: usize, spec: RenderOutputSpec) -> Self {
        Self {
            observation_index,
            spec,
        }
    }

    pub const fn observation_index(self) -> usize {
        self.observation_index
    }

    pub const fn spec(self) -> RenderOutputSpec {
        self.spec
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderRequest {
    render_interval: RenderTimeInterval,
    observations: Vec<RenderObservationSpec>,
    outputs: Vec<RenderRequestedOutput>,
}

impl RenderRequest {
    pub fn new(
        render_interval: RenderTimeInterval,
        observations: Vec<RenderObservationSpec>,
        outputs: Vec<RenderRequestedOutput>,
    ) -> Result<Self, RenderRequestValidationError> {
        if observations.is_empty() {
            return Err(RenderRequestValidationError::EmptyObservations);
        }
        if outputs.is_empty() {
            return Err(RenderRequestValidationError::EmptyOutputs);
        }

        for (observation_index, observation) in observations.iter().copied().enumerate() {
            if !render_interval.contains(observation.shutter()) {
                return Err(RenderRequestValidationError::ObservationOutsideRenderInterval {
                    observation_index,
                });
            }
        }

        let mut output_counts = vec![0usize; observations.len()];
        for output in &outputs {
            let observation_index = output.observation_index();
            let Some(observation) = observations.get(observation_index).copied() else {
                return Err(RenderRequestValidationError::OutputObservationOutOfRange {
                    observation_index,
                });
            };
            if observation.is_probe() && !output.spec().topology().is_scalar() {
                return Err(RenderRequestValidationError::ProbeRequiresScalarTopology {
                    observation_index,
                });
            }
            output_counts[observation_index] += 1;
        }

        if let Some(observation_index) = output_counts.iter().position(|count| *count == 0) {
            return Err(RenderRequestValidationError::ObservationHasNoOutputs {
                observation_index,
            });
        }

        Ok(Self {
            render_interval,
            observations,
            outputs,
        })
    }

    pub const fn render_interval(&self) -> RenderTimeInterval {
        self.render_interval
    }

    pub fn observations(&self) -> &[RenderObservationSpec] {
        &self.observations
    }

    pub fn outputs(&self) -> &[RenderRequestedOutput] {
        &self.outputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::space_time::RenderTimePoint;

    fn interval(start: f64, end: f64) -> RenderTimeInterval {
        RenderTimeInterval::new(
            RenderTimePoint::from_seconds(start).expect("finite time"),
            RenderTimePoint::from_seconds(end).expect("finite time"),
        )
        .expect("ordered interval")
    }

    fn radiance(topology: RenderResultTopology) -> RenderOutputSpec {
        RenderOutputSpec::new(
            RenderOutputValue::Radiance {
                representation: RenderRadiometricRepresentation::Rgb,
            },
            topology,
            RenderSemanticTolerance::relative(0.01).expect("valid tolerance"),
        )
        .expect("valid radiance output")
    }

    #[test]
    fn perspective_observation_is_independent_of_physical_surface_state() {
        let observation = RenderPerspectiveObservation::new(
            RenderAffineTransform3::identity(),
            std::f64::consts::FRAC_PI_2,
            16.0 / 9.0,
            interval(1.0, 1.01),
            RenderSamplingSupport::ideal_ray(),
        )
        .expect("valid perspective observation");
        assert_eq!(observation.aspect_ratio(), 16.0 / 9.0);
        assert!(observation.sampling_support().is_ideal_ray());
    }

    #[test]
    fn probe_request_uses_scalar_topology_without_image_lattice() {
        let render_interval = interval(0.0, 1.0);
        let probe = RenderObservationSpec::Probe(RenderProbeObservation::new(
            RenderAffineTransform3::identity(),
            interval(0.25, 0.25),
            RenderSamplingSupport::ideal_ray(),
        ));
        let output = RenderRequestedOutput::new(0, radiance(RenderResultTopology::scalar()));
        let request = RenderRequest::new(render_interval, vec![probe], vec![output])
            .expect("scalar probe request should validate");
        assert!(request.outputs()[0].spec().topology().is_scalar());
    }

    #[test]
    fn coordinated_observations_support_distinct_result_topologies() {
        let render_interval = interval(0.0, 1.0);
        let perspective = RenderObservationSpec::Perspective(
            RenderPerspectiveObservation::new(
                RenderAffineTransform3::identity(),
                std::f64::consts::FRAC_PI_2,
                1.0,
                interval(0.0, 0.5),
                RenderSamplingSupport::ideal_ray(),
            )
            .expect("valid perspective"),
        );
        let probe = RenderObservationSpec::Probe(RenderProbeObservation::new(
            RenderAffineTransform3::identity(),
            interval(0.5, 0.5),
            RenderSamplingSupport::ideal_ray(),
        ));
        let lattice = RenderResultTopology::sample_lattice_2d(640, 480).expect("valid lattice");
        let request = RenderRequest::new(
            render_interval,
            vec![perspective, probe],
            vec![
                RenderRequestedOutput::new(0, radiance(lattice)),
                RenderRequestedOutput::new(1, radiance(RenderResultTopology::scalar())),
            ],
        )
        .expect("coordinated request should validate");
        assert_eq!(request.observations().len(), 2);
        assert_eq!(
            request.outputs()[0].spec().topology().sample_lattice_dimensions(),
            Some((640, 480))
        );
    }

    #[test]
    fn probe_rejects_image_lattice_and_observation_shutter_must_fit_request() {
        let probe = RenderObservationSpec::Probe(RenderProbeObservation::new(
            RenderAffineTransform3::identity(),
            interval(2.0, 2.0),
            RenderSamplingSupport::ideal_ray(),
        ));
        let lattice = RenderResultTopology::sample_lattice_2d(4, 4).expect("valid lattice");
        assert_eq!(
            RenderRequest::new(
                interval(0.0, 1.0),
                vec![probe],
                vec![RenderRequestedOutput::new(0, radiance(lattice))]
            ),
            Err(RenderRequestValidationError::ObservationOutsideRenderInterval {
                observation_index: 0
            })
        );

        let probe = RenderObservationSpec::Probe(RenderProbeObservation::new(
            RenderAffineTransform3::identity(),
            interval(0.5, 0.5),
            RenderSamplingSupport::ideal_ray(),
        ));
        assert_eq!(
            RenderRequest::new(
                interval(0.0, 1.0),
                vec![probe],
                vec![RenderRequestedOutput::new(0, radiance(lattice))]
            ),
            Err(RenderRequestValidationError::ProbeRequiresScalarTopology {
                observation_index: 0
            })
        );
    }

    #[test]
    fn output_meaning_is_orthogonal_to_topology_and_numeric_storage() {
        let lattice = RenderResultTopology::sample_lattice_2d(2, 2).expect("valid lattice");
        let distance = RenderOutputSpec::new(
            RenderOutputValue::Distance {
                convention: RenderDistanceConvention::ObservationForwardDepth,
            },
            lattice,
            RenderSemanticTolerance::absolute(0.001).expect("valid tolerance"),
        )
        .expect("valid distance output");
        assert!(matches!(
            distance.value(),
            RenderOutputValue::Distance {
                convention: RenderDistanceConvention::ObservationForwardDepth
            }
        ));
        assert_eq!(distance.tolerance().absolute_max_error(), Some(0.001));
    }

    #[test]
    fn identity_output_requires_exact_semantic_tolerance() {
        let tolerance = RenderSemanticTolerance::relative(0.0).expect("valid tolerance");
        assert_eq!(
            RenderOutputSpec::new(
                RenderOutputValue::ObjectIdentity,
                RenderResultTopology::scalar(),
                tolerance
            ),
            Err(RenderRequestValidationError::IdentityToleranceMustBeExact)
        );
    }
}
