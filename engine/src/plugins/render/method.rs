//! R4 coherent renderer-method contracts.
//!
//! A method contract describes semantic compatibility and abstract future execution requirements.
//! It deliberately contains no current device capabilities, GPU handles, residency, surfaces,
//! pipelines, output bindings, or method-internal pass/work topology.

use super::request::{RenderObservationSpec, RenderOutputValue};
use super::space_time::{CanonicalF64, RenderSemanticValueError};
use std::error::Error;
use std::fmt;
use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderMethodId(NonZeroU32);

impl RenderMethodId {
    /// Runtime renderer-semantic method identity used to correlate conditional plan candidates.
    ///
    /// This is not persistence, wire, source, artifact, or RunenGPU identity.
    pub const fn new(raw: u32) -> Option<Self> {
        match NonZeroU32::new(raw) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    #[cfg(test)]
    pub(crate) const fn raw(self) -> u32 {
        self.0.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderObservationKind {
    Perspective,
    Probe,
}

impl RenderObservationKind {
    pub const fn of(observation: RenderObservationSpec) -> Self {
        match observation {
            RenderObservationSpec::Perspective(_) => Self::Perspective,
            RenderObservationSpec::Probe(_) => Self::Probe,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderMethodObservationSupport {
    perspective: bool,
    probe: bool,
}

impl RenderMethodObservationSupport {
    pub const fn perspective_only() -> Self {
        Self {
            perspective: true,
            probe: false,
        }
    }

    pub const fn probe_only() -> Self {
        Self {
            perspective: false,
            probe: true,
        }
    }

    pub const fn perspective_and_probe() -> Self {
        Self {
            perspective: true,
            probe: true,
        }
    }

    pub const fn supports(self, kind: RenderObservationKind) -> bool {
        match kind {
            RenderObservationKind::Perspective => self.perspective,
            RenderObservationKind::Probe => self.probe,
        }
    }

    const fn is_empty(self) -> bool {
        !self.perspective && !self.probe
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderSpectralRadianceSupport {
    minimum_wavelength_meters: CanonicalF64,
    maximum_wavelength_meters: CanonicalF64,
}

impl RenderSpectralRadianceSupport {
    pub fn new(
        minimum_wavelength_meters: f64,
        maximum_wavelength_meters: f64,
    ) -> Result<Self, RenderMethodValidationError> {
        let minimum_wavelength_meters = CanonicalF64::new(
            minimum_wavelength_meters,
            "method_minimum_wavelength_meters",
        )?;
        let maximum_wavelength_meters = CanonicalF64::new(
            maximum_wavelength_meters,
            "method_maximum_wavelength_meters",
        )?;
        if minimum_wavelength_meters.get() <= 0.0
            || maximum_wavelength_meters.get() <= 0.0
            || minimum_wavelength_meters.get() > maximum_wavelength_meters.get()
        {
            return Err(RenderMethodValidationError::InvalidSpectralRange);
        }
        Ok(Self {
            minimum_wavelength_meters,
            maximum_wavelength_meters,
        })
    }

    pub fn contains_wavelength_meters(self, wavelength_meters: f64) -> bool {
        wavelength_meters >= self.minimum_wavelength_meters.get()
            && wavelength_meters <= self.maximum_wavelength_meters.get()
    }

    pub fn range_meters(self) -> (f64, f64) {
        (
            self.minimum_wavelength_meters.get(),
            self.maximum_wavelength_meters.get(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderMethodOutputSupport {
    radiance: Option<RenderSpectralRadianceSupport>,
    distance: bool,
    object_identity: bool,
}

impl RenderMethodOutputSupport {
    pub const fn none() -> Self {
        Self {
            radiance: None,
            distance: false,
            object_identity: false,
        }
    }

    pub const fn with_radiance(mut self, support: RenderSpectralRadianceSupport) -> Self {
        self.radiance = Some(support);
        self
    }

    pub const fn with_distance(mut self) -> Self {
        self.distance = true;
        self
    }

    pub const fn with_object_identity(mut self) -> Self {
        self.object_identity = true;
        self
    }

    pub const fn radiance(self) -> Option<RenderSpectralRadianceSupport> {
        self.radiance
    }

    pub const fn supports_distance(self) -> bool {
        self.distance
    }

    pub const fn supports_object_identity(self) -> bool {
        self.object_identity
    }

    pub fn supports_value(self, value: RenderOutputValue) -> bool {
        match value {
            RenderOutputValue::Radiance { representation } => self.radiance.is_some_and(|support| {
                support.contains_wavelength_meters(representation.wavelength_meters())
            }),
            RenderOutputValue::Distance { .. } => self.distance,
            RenderOutputValue::ObjectIdentity => self.object_identity,
        }
    }

    const fn is_empty(self) -> bool {
        self.radiance.is_none() && !self.distance && !self.object_identity
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderRepresentationProtocolRequirement {
    SurfaceQuery { revision: u32 },
    FieldDistance { revision: u32 },
}

impl RenderRepresentationProtocolRequirement {
    pub const fn revision(self) -> u32 {
        match self {
            Self::SurfaceQuery { revision } | Self::FieldDistance { revision } => revision,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderMotionInputPolicy {
    None,
    /// A non-instant observation requires one later-admitted source-owned object-motion binding
    /// covering that shutter interval. R4 records the requirement; R5 owns binding values and
    /// admission.
    RequireForNonInstantShutter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderAbstractExecutionRequirement {
    /// Candidate lowering requires general-purpose parallel programmable work, but does not name a
    /// current device, capability set, pipeline, resource, or submission.
    GeneralParallelWork,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderMethodContract {
    id: RenderMethodId,
    observation_support: RenderMethodObservationSupport,
    output_support: RenderMethodOutputSupport,
    representation_requirement: RenderRepresentationProtocolRequirement,
    maximum_refinement_error_meters: Option<CanonicalF64>,
    motion_input_policy: RenderMotionInputPolicy,
    abstract_execution_requirements: Vec<RenderAbstractExecutionRequirement>,
}

impl RenderMethodContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: RenderMethodId,
        observation_support: RenderMethodObservationSupport,
        output_support: RenderMethodOutputSupport,
        representation_requirement: RenderRepresentationProtocolRequirement,
        maximum_refinement_error_meters: Option<f64>,
        motion_input_policy: RenderMotionInputPolicy,
        mut abstract_execution_requirements: Vec<RenderAbstractExecutionRequirement>,
    ) -> Result<Self, RenderMethodValidationError> {
        if observation_support.is_empty() {
            return Err(RenderMethodValidationError::NoObservations);
        }
        if output_support.is_empty() {
            return Err(RenderMethodValidationError::NoOutputs);
        }
        if representation_requirement.revision() == 0 {
            return Err(RenderMethodValidationError::InvalidProtocolRevision);
        }
        let maximum_refinement_error_meters = maximum_refinement_error_meters
            .map(|value| {
                let value = CanonicalF64::new(value, "method_maximum_refinement_error_meters")?;
                if value.get() < 0.0 {
                    return Err(RenderMethodValidationError::NegativeRefinementError);
                }
                Ok(value)
            })
            .transpose()?;
        abstract_execution_requirements.sort_unstable();
        abstract_execution_requirements.dedup();
        Ok(Self {
            id,
            observation_support,
            output_support,
            representation_requirement,
            maximum_refinement_error_meters,
            motion_input_policy,
            abstract_execution_requirements,
        })
    }

    pub const fn id(&self) -> RenderMethodId {
        self.id
    }

    pub const fn observation_support(&self) -> RenderMethodObservationSupport {
        self.observation_support
    }

    pub const fn output_support(&self) -> RenderMethodOutputSupport {
        self.output_support
    }

    pub const fn representation_requirement(&self) -> RenderRepresentationProtocolRequirement {
        self.representation_requirement
    }

    pub fn maximum_refinement_error_meters(&self) -> Option<f64> {
        self.maximum_refinement_error_meters.map(CanonicalF64::get)
    }

    pub const fn motion_input_policy(&self) -> RenderMotionInputPolicy {
        self.motion_input_policy
    }

    pub fn abstract_execution_requirements(&self) -> &[RenderAbstractExecutionRequirement] {
        &self.abstract_execution_requirements
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMethodValidationError {
    SemanticValue(RenderSemanticValueError),
    NoObservations,
    NoOutputs,
    InvalidSpectralRange,
    InvalidProtocolRevision,
    NegativeRefinementError,
}

impl From<RenderSemanticValueError> for RenderMethodValidationError {
    fn from(value: RenderSemanticValueError) -> Self {
        Self::SemanticValue(value)
    }
}

impl fmt::Display for RenderMethodValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SemanticValue(error) => fmt::Display::fmt(error, formatter),
            Self::NoObservations => {
                formatter.write_str("render method must support an observation")
            }
            Self::NoOutputs => formatter.write_str("render method must support an output"),
            Self::InvalidSpectralRange => formatter
                .write_str("render method spectral range must be finite, positive, and ordered"),
            Self::InvalidProtocolRevision => {
                formatter.write_str("render method protocol revision must be non-zero")
            }
            Self::NegativeRefinementError => {
                formatter.write_str("render method maximum refinement error must be non-negative")
            }
        }
    }
}

impl Error for RenderMethodValidationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::representation::RENDER_SURFACE_QUERY_PROTOCOL_REVISION;

    #[test]
    fn method_contract_canonicalizes_abstract_requirements_without_execution_state() {
        let method = RenderMethodContract::new(
            RenderMethodId::new(7).expect("non-zero method id"),
            RenderMethodObservationSupport::perspective_and_probe(),
            RenderMethodOutputSupport::none().with_distance(),
            RenderRepresentationProtocolRequirement::SurfaceQuery {
                revision: RENDER_SURFACE_QUERY_PROTOCOL_REVISION,
            },
            None,
            RenderMotionInputPolicy::None,
            vec![
                RenderAbstractExecutionRequirement::GeneralParallelWork,
                RenderAbstractExecutionRequirement::GeneralParallelWork,
            ],
        )
        .expect("valid method");
        assert_eq!(method.id().raw(), 7);
        assert_eq!(method.abstract_execution_requirements().len(), 1);
    }

    #[test]
    fn spectral_support_is_semantic_and_bounded() {
        let support = RenderSpectralRadianceSupport::new(400e-9, 700e-9)
            .expect("valid visible-like interval");
        assert!(support.contains_wavelength_meters(550e-9));
        assert!(!support.contains_wavelength_meters(800e-9));
        assert_eq!(
            RenderSpectralRadianceSupport::new(700e-9, 400e-9),
            Err(RenderMethodValidationError::InvalidSpectralRange)
        );
    }
}
