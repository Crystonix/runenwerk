//! R4 device-independent conditional semantic planning.
//!
//! This module consumes only accepted renderer-semantic scene/request/method/representation facts.
//! It does not inspect current availability/residency, output bindings, device capabilities, GPU
//! handles, pipelines, allocations, queues, submissions, ECS resources, or product fallback state.

use super::method::{
    RenderAbstractExecutionRequirement, RenderMethodContract, RenderMethodId,
    RenderMotionInputPolicy, RenderObservationKind, RenderRepresentationProtocolRequirement,
};
use super::representation::{
    RenderProtocolCompatibilityError, RenderRepresentationId, RenderRepresentationProtocol,
};
use super::request::{
    RenderObservationSpec, RenderOutputValue, RenderRequest, RenderRequestedOutput,
    RenderSemanticTolerance,
};
use super::scene::{RenderObjectId, RenderSceneRevision, RenderSceneSnapshot};
use super::space_time::{CanonicalF64, RenderTimeInterval};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderRepresentationApproximation {
    Exact,
    BoundedDistance {
        max_absolute_error_meters: CanonicalF64,
    },
}

impl RenderRepresentationApproximation {
    pub const fn exact() -> Self {
        Self::Exact
    }

    fn bounded_distance(max_absolute_error_meters: f64) -> Self {
        Self::BoundedDistance {
            max_absolute_error_meters: CanonicalF64::new(
                max_absolute_error_meters,
                "plan_bounded_distance_error_meters",
            )
            .expect("R3 guarantees expose finite non-negative field error"),
        }
    }

    pub fn max_absolute_distance_error_meters(self) -> Option<f64> {
        match self {
            Self::Exact => None,
            Self::BoundedDistance {
                max_absolute_error_meters,
            } => Some(max_absolute_error_meters.get()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderApplicableRepresentation {
    representation_id: RenderRepresentationId,
    approximation: RenderRepresentationApproximation,
}

impl RenderApplicableRepresentation {
    pub const fn representation_id(self) -> RenderRepresentationId {
        self.representation_id
    }

    pub const fn approximation(self) -> RenderRepresentationApproximation {
        self.approximation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderObjectRepresentationOptions {
    object_id: RenderObjectId,
    representations: Vec<RenderApplicableRepresentation>,
}

impl RenderObjectRepresentationOptions {
    pub const fn object_id(&self) -> RenderObjectId {
        self.object_id
    }

    pub fn representations(&self) -> &[RenderApplicableRepresentation] {
        &self.representations
    }
}

/// One R4-declared unresolved request-scoped semantic-input requirement.
///
/// A motion-aware method may require source-owned time-varying object-transform semantics over a
/// non-instant shutter. R4 records only the typed requirement. R5 owns actual values, source
/// generation/provenance, compatibility checking, and binding admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderMotionTransformInputRequirement {
    object_id: RenderObjectId,
    observation_index: usize,
    required_interval: RenderTimeInterval,
}

impl RenderMotionTransformInputRequirement {
    pub const fn object_id(self) -> RenderObjectId {
        self.object_id
    }

    pub const fn observation_index(self) -> usize {
        self.observation_index
    }

    pub const fn required_interval(self) -> RenderTimeInterval {
        self.required_interval
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPlanCandidate {
    method_id: RenderMethodId,
    object_representations: Vec<RenderObjectRepresentationOptions>,
    unresolved_motion_inputs: Vec<RenderMotionTransformInputRequirement>,
    abstract_execution_requirements: Vec<RenderAbstractExecutionRequirement>,
}

impl RenderPlanCandidate {
    pub const fn method_id(&self) -> RenderMethodId {
        self.method_id
    }

    pub fn object_representations(&self) -> &[RenderObjectRepresentationOptions] {
        &self.object_representations
    }

    pub fn unresolved_motion_inputs(&self) -> &[RenderMotionTransformInputRequirement] {
        &self.unresolved_motion_inputs
    }

    pub fn abstract_execution_requirements(&self) -> &[RenderAbstractExecutionRequirement] {
        &self.abstract_execution_requirements
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPlan {
    scene_revision: RenderSceneRevision,
    outputs: Vec<RenderRequestedOutput>,
    candidates: Vec<RenderPlanCandidate>,
    rejected_methods: Vec<RenderMethodRejection>,
}

impl RenderPlan {
    pub const fn scene_revision(&self) -> RenderSceneRevision {
        self.scene_revision
    }

    pub fn outputs(&self) -> &[RenderRequestedOutput] {
        &self.outputs
    }

    pub fn candidates(&self) -> &[RenderPlanCandidate] {
        &self.candidates
    }

    pub fn rejected_methods(&self) -> &[RenderMethodRejection] {
        &self.rejected_methods
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderMethodRejection {
    method_id: RenderMethodId,
    reason: RenderMethodRejectionReason,
}

impl RenderMethodRejection {
    pub const fn method_id(&self) -> RenderMethodId {
        self.method_id
    }

    pub const fn reason(&self) -> &RenderMethodRejectionReason {
        &self.reason
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderMethodRejectionReason {
    UnsupportedObservation {
        observation_index: usize,
        observation_kind: RenderObservationKind,
    },
    UnsupportedOutput {
        output_index: usize,
    },
    RadiometricDomainMismatch {
        output_index: usize,
    },
    NoApplicableRepresentation {
        object_id: RenderObjectId,
        representation_rejections: Vec<RenderRepresentationRejection>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderRepresentationRejection {
    representation_id: RenderRepresentationId,
    reason: RenderRepresentationRejectionReason,
}

impl RenderRepresentationRejection {
    pub const fn representation_id(&self) -> RenderRepresentationId {
        self.representation_id
    }

    pub const fn reason(&self) -> &RenderRepresentationRejectionReason {
        &self.reason
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderRepresentationRejectionReason {
    ProtocolUnsupported {
        protocol: RenderRepresentationProtocol,
    },
    ProtocolVersionMismatch {
        protocol: RenderRepresentationProtocol,
        requested_revision: u32,
        supported_revision: u32,
    },
    TemporalCoverage {
        observation_index: usize,
    },
    RefinementEvidenceMissing,
    RefinementInsufficient {
        required_max_error_meters: CanonicalF64,
        available_finest_error_meters: CanonicalF64,
    },
    ExactDistanceRequired,
    RelativeDistanceToleranceNotProvable,
    DistanceApproximationExceedsTolerance {
        max_absolute_error_meters: CanonicalF64,
        allowed_absolute_error_meters: CanonicalF64,
    },
    ApproximationNotProvableForOutput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderPlanningFailure {
    NoMethods,
    DuplicateMethodId { method_id: RenderMethodId },
    NoSemanticSolution { rejections: Vec<RenderMethodRejection> },
}

impl fmt::Display for RenderPlanningFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMethods => formatter.write_str("semantic planning requires a render method"),
            Self::DuplicateMethodId { method_id } => {
                write!(formatter, "duplicate render method identity {method_id:?}")
            }
            Self::NoSemanticSolution { .. } => {
                formatter.write_str("no render method can satisfy the requested semantics")
            }
        }
    }
}

impl Error for RenderPlanningFailure {}

/// Build one CPU-only, device-independent conditional semantic plan.
///
/// The implementation currently scans the explicitly supplied method set and represented objects.
/// The public plan contract stores eligible representation *sets* rather than materializing a
/// Cartesian product of representation choices, so it does not require all combination expansion
/// or expose an execution-selection topology.
pub fn plan_render(
    scene: &RenderSceneSnapshot,
    request: &RenderRequest,
    methods: &[RenderMethodContract],
) -> Result<RenderPlan, RenderPlanningFailure> {
    if methods.is_empty() {
        return Err(RenderPlanningFailure::NoMethods);
    }

    let mut methods = methods.iter().collect::<Vec<_>>();
    methods.sort_by_key(|method| method.id());
    for pair in methods.windows(2) {
        if pair[0].id() == pair[1].id() {
            return Err(RenderPlanningFailure::DuplicateMethodId {
                method_id: pair[0].id(),
            });
        }
    }

    let mut candidates = Vec::new();
    let mut rejected_methods = Vec::new();
    for method in methods {
        match plan_method(scene, request, method) {
            Ok(candidate) => candidates.push(candidate),
            Err(reason) => rejected_methods.push(RenderMethodRejection {
                method_id: method.id(),
                reason,
            }),
        }
    }

    if candidates.is_empty() {
        return Err(RenderPlanningFailure::NoSemanticSolution {
            rejections: rejected_methods,
        });
    }

    Ok(RenderPlan {
        scene_revision: scene.revision(),
        outputs: request.outputs().to_vec(),
        candidates,
        rejected_methods,
    })
}

fn plan_method(
    scene: &RenderSceneSnapshot,
    request: &RenderRequest,
    method: &RenderMethodContract,
) -> Result<RenderPlanCandidate, RenderMethodRejectionReason> {
    for (observation_index, observation) in request.observations().iter().copied().enumerate() {
        let observation_kind = RenderObservationKind::of(observation);
        if !method.observation_support().supports(observation_kind) {
            return Err(RenderMethodRejectionReason::UnsupportedObservation {
                observation_index,
                observation_kind,
            });
        }
    }

    for (output_index, output) in request.outputs().iter().copied().enumerate() {
        let value = output.spec().value();
        if method.output_support().supports_value(value) {
            continue;
        }
        if let RenderOutputValue::Radiance { representation } = value {
            if method.output_support().radiance().is_some() {
                let _ = representation;
                return Err(RenderMethodRejectionReason::RadiometricDomainMismatch { output_index });
            }
        }
        return Err(RenderMethodRejectionReason::UnsupportedOutput { output_index });
    }

    let mut object_representations = Vec::new();
    for object_id in scene.object_ids() {
        let Some(participation) = scene.object_participation(object_id) else {
            continue;
        };
        if participation.representations().is_empty() {
            continue;
        }

        let mut applicable = Vec::new();
        let mut representation_rejections = Vec::new();
        for representation in participation.representations() {
            match evaluate_representation(request, method, representation) {
                Ok(approximation) => applicable.push(RenderApplicableRepresentation {
                    representation_id: representation.id(),
                    approximation,
                }),
                Err(reason) => representation_rejections.push(RenderRepresentationRejection {
                    representation_id: representation.id(),
                    reason,
                }),
            }
        }

        if applicable.is_empty() {
            return Err(RenderMethodRejectionReason::NoApplicableRepresentation {
                object_id,
                representation_rejections,
            });
        }
        object_representations.push(RenderObjectRepresentationOptions {
            object_id,
            representations: applicable,
        });
    }

    let mut unresolved_motion_inputs = Vec::new();
    if method.motion_input_policy() == RenderMotionInputPolicy::RequireForNonInstantShutter {
        for object in &object_representations {
            for (observation_index, observation) in
                request.observations().iter().copied().enumerate()
            {
                let shutter = observation.shutter();
                if shutter.start() != shutter.end() {
                    unresolved_motion_inputs.push(RenderMotionTransformInputRequirement {
                        object_id: object.object_id,
                        observation_index,
                        required_interval: shutter,
                    });
                }
            }
        }
    }

    Ok(RenderPlanCandidate {
        method_id: method.id(),
        object_representations,
        unresolved_motion_inputs,
        abstract_execution_requirements: method.abstract_execution_requirements().to_vec(),
    })
}

fn evaluate_representation(
    request: &RenderRequest,
    method: &RenderMethodContract,
    representation: &super::representation::RenderRepresentationRecord,
) -> Result<RenderRepresentationApproximation, RenderRepresentationRejectionReason> {
    for (observation_index, observation) in request.observations().iter().copied().enumerate() {
        if !representation
            .temporal_support()
            .contains_interval(observation.shutter())
        {
            return Err(RenderRepresentationRejectionReason::TemporalCoverage {
                observation_index,
            });
        }
    }

    if let Some(required) = method.maximum_refinement_error_meters() {
        let Some(available) = representation.refinement().finest_absolute_error_meters() else {
            return Err(RenderRepresentationRejectionReason::RefinementEvidenceMissing);
        };
        if available > required {
            return Err(RenderRepresentationRejectionReason::RefinementInsufficient {
                required_max_error_meters: canonical_non_negative(required),
                available_finest_error_meters: canonical_non_negative(available),
            });
        }
    }

    match method.representation_requirement() {
        RenderRepresentationProtocolRequirement::SurfaceQuery { revision } => {
            representation
                .surface_query_protocol(revision)
                .map_err(protocol_rejection)?;
            Ok(RenderRepresentationApproximation::Exact)
        }
        RenderRepresentationProtocolRequirement::FieldDistance { revision } => {
            let evidence = representation
                .field_distance_protocol(revision)
                .map_err(protocol_rejection)?;
            let guarantee = evidence.guarantee();
            if guarantee.is_exact() {
                return Ok(RenderRepresentationApproximation::Exact);
            }
            let max_error = guarantee.max_absolute_error_meters();
            validate_distance_approximation(request, max_error)?;
            Ok(RenderRepresentationApproximation::bounded_distance(max_error))
        }
    }
}

fn protocol_rejection(error: RenderProtocolCompatibilityError) -> RenderRepresentationRejectionReason {
    match error {
        RenderProtocolCompatibilityError::Unsupported { protocol } => {
            RenderRepresentationRejectionReason::ProtocolUnsupported { protocol }
        }
        RenderProtocolCompatibilityError::VersionMismatch {
            protocol,
            requested_revision,
            supported_revision,
        } => RenderRepresentationRejectionReason::ProtocolVersionMismatch {
            protocol,
            requested_revision,
            supported_revision,
        },
    }
}

fn validate_distance_approximation(
    request: &RenderRequest,
    max_error_meters: f64,
) -> Result<(), RenderRepresentationRejectionReason> {
    for output in request.outputs().iter().copied() {
        match output.spec().value() {
            RenderOutputValue::Distance { .. } => {
                validate_distance_tolerance(output.spec().tolerance(), max_error_meters)?;
            }
            _ => return Err(RenderRepresentationRejectionReason::ApproximationNotProvableForOutput),
        }
    }
    Ok(())
}

fn validate_distance_tolerance(
    tolerance: RenderSemanticTolerance,
    max_error_meters: f64,
) -> Result<(), RenderRepresentationRejectionReason> {
    if tolerance.is_exact() {
        return Err(RenderRepresentationRejectionReason::ExactDistanceRequired);
    }
    if let Some(allowed) = tolerance.absolute_max_error() {
        if max_error_meters <= allowed {
            return Ok(());
        }
        return Err(
            RenderRepresentationRejectionReason::DistanceApproximationExceedsTolerance {
                max_absolute_error_meters: canonical_non_negative(max_error_meters),
                allowed_absolute_error_meters: canonical_non_negative(allowed),
            },
        );
    }
    Err(RenderRepresentationRejectionReason::RelativeDistanceToleranceNotProvable)
}

fn canonical_non_negative(value: f64) -> CanonicalF64 {
    CanonicalF64::new(value, "plan_non_negative_semantic_value")
        .expect("validated R2/R3 semantic values are finite")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::method::{
        RenderAbstractExecutionRequirement, RenderMethodObservationSupport,
        RenderMethodOutputSupport, RenderMotionInputPolicy, RenderRepresentationProtocolRequirement,
        RenderSpectralRadianceSupport,
    };
    use crate::plugins::render::participation::RenderObjectParticipation;
    use crate::plugins::render::representation::{
        RENDER_FIELD_DISTANCE_PROTOCOL_REVISION, RENDER_SURFACE_QUERY_PROTOCOL_REVISION,
        RenderFieldDistanceGuarantee, RenderFieldDistanceProtocolEvidence,
        RenderRefinementEvidence, RenderRepresentationRecord, RenderSurfaceProtocolEvidence,
    };
    use crate::plugins::render::request::{
        RenderDistanceConvention, RenderObservationSpec, RenderOutputSpec,
        RenderPerspectiveObservation, RenderProbeObservation, RenderRadiometricRepresentation,
        RenderResultTopology, RenderSamplingSupport,
    };
    use crate::plugins::render::scene::{RenderSceneStore, RenderSceneUpdate};
    use crate::plugins::render::space_time::{
        RenderAffineTransform3, RenderSpatialCoverage, RenderTemporalSupport, RenderTimePoint,
    };

    fn interval(start: f64, end: f64) -> RenderTimeInterval {
        RenderTimeInterval::new(
            RenderTimePoint::from_seconds(start).expect("time"),
            RenderTimePoint::from_seconds(end).expect("time"),
        )
        .expect("ordered interval")
    }

    fn distance_request(tolerance: RenderSemanticTolerance, shutter: RenderTimeInterval) -> RenderRequest {
        let observation = RenderObservationSpec::Probe(
            RenderProbeObservation::new(
                RenderAffineTransform3::identity(),
                shutter,
                RenderSamplingSupport::ideal_ray(),
            )
            .expect("probe"),
        );
        let output = RenderOutputSpec::new(
            RenderOutputValue::Distance {
                convention: RenderDistanceConvention::RayDistance,
            },
            RenderResultTopology::scalar(),
            tolerance,
        )
        .expect("distance output");
        RenderRequest::new(
            shutter,
            vec![observation],
            vec![RenderRequestedOutput::new(0, output)],
        )
        .expect("request")
    }

    fn surface_method(raw: u32) -> RenderMethodContract {
        RenderMethodContract::new(
            RenderMethodId::new(raw).expect("method id"),
            RenderMethodObservationSupport::perspective_and_probe(),
            RenderMethodOutputSupport::none().with_distance(),
            RenderRepresentationProtocolRequirement::SurfaceQuery {
                revision: RENDER_SURFACE_QUERY_PROTOCOL_REVISION,
            },
            None,
            RenderMotionInputPolicy::None,
            vec![RenderAbstractExecutionRequirement::GeneralParallelWork],
        )
        .expect("surface method")
    }

    fn field_method(raw: u32, motion: RenderMotionInputPolicy) -> RenderMethodContract {
        RenderMethodContract::new(
            RenderMethodId::new(raw).expect("method id"),
            RenderMethodObservationSupport::perspective_and_probe(),
            RenderMethodOutputSupport::none().with_distance(),
            RenderRepresentationProtocolRequirement::FieldDistance {
                revision: RENDER_FIELD_DISTANCE_PROTOCOL_REVISION,
            },
            Some(0.05),
            motion,
            vec![RenderAbstractExecutionRequirement::GeneralParallelWork],
        )
        .expect("field method")
    }

    fn insert_object_with_representations(
        store: &mut RenderSceneStore,
        field_error: f64,
        temporal_support: RenderTemporalSupport,
    ) -> RenderObjectId {
        let object_id = store.allocate_object_id().expect("object id");
        let mut insert = RenderSceneUpdate::new();
        insert.insert(object_id);
        store.commit(insert).expect("insert");

        let surface_id = store
            .allocate_representation_id(object_id)
            .expect("surface id");
        let field_id = store
            .allocate_representation_id(object_id)
            .expect("field id");
        let surface = RenderRepresentationRecord::new(
            surface_id,
            RenderSpatialCoverage::unbounded(),
            temporal_support,
            RenderRefinementEvidence::none(),
            Some(
                RenderSurfaceProtocolEvidence::exact(RENDER_SURFACE_QUERY_PROTOCOL_REVISION)
                    .expect("surface protocol"),
            ),
            None,
        )
        .expect("surface representation");
        let field = RenderRepresentationRecord::new(
            field_id,
            RenderSpatialCoverage::unbounded(),
            temporal_support,
            RenderRefinementEvidence::bounded(field_error).expect("refinement"),
            None,
            Some(
                RenderFieldDistanceProtocolEvidence::new(
                    RENDER_FIELD_DISTANCE_PROTOCOL_REVISION,
                    RenderFieldDistanceGuarantee::conservative(field_error)
                        .expect("field guarantee"),
                )
                .expect("field protocol"),
            ),
        )
        .expect("field representation");
        let participation = RenderObjectParticipation::new(vec![field, surface], None, None)
            .expect("participation");
        let mut attach = RenderSceneUpdate::new();
        attach.replace_participation(object_id, participation);
        store.commit(attach).expect("attach representations");
        object_id
    }

    #[test]
    fn deterministic_plan_has_multiple_legal_solution_families_and_no_gpu_state() {
        let mut store = RenderSceneStore::new();
        let shutter = interval(0.0, 0.0);
        insert_object_with_representations(
            &mut store,
            0.01,
            RenderTemporalSupport::interval(shutter),
        );
        let request = distance_request(
            RenderSemanticTolerance::absolute(0.02).expect("tolerance"),
            shutter,
        );
        let methods = vec![field_method(2, RenderMotionInputPolicy::None), surface_method(1)];
        let first = plan_render(&store.snapshot(), &request, &methods).expect("plan");
        let reversed = vec![methods[1].clone(), methods[0].clone()];
        let second = plan_render(&store.snapshot(), &request, &reversed).expect("plan");

        assert_eq!(first, second);
        assert_eq!(first.candidates().len(), 2);
        assert_eq!(first.candidates()[0].method_id().raw(), 1);
        assert_eq!(first.candidates()[1].method_id().raw(), 2);
        assert_eq!(
            first.candidates()[1].object_representations()[0].representations()[0]
                .approximation()
                .max_absolute_distance_error_meters(),
            Some(0.01)
        );
    }

    #[test]
    fn motion_binding_requirement_remains_explicit_and_unadmitted() {
        let mut store = RenderSceneStore::new();
        let shutter = interval(0.0, 0.5);
        let object_id = insert_object_with_representations(
            &mut store,
            0.01,
            RenderTemporalSupport::interval(shutter),
        );
        let request = distance_request(
            RenderSemanticTolerance::absolute(0.02).expect("tolerance"),
            shutter,
        );
        let plan = plan_render(
            &store.snapshot(),
            &request,
            &[field_method(
                1,
                RenderMotionInputPolicy::RequireForNonInstantShutter,
            )],
        )
        .expect("conditional plan");
        let requirement = plan.candidates()[0].unresolved_motion_inputs()[0];
        assert_eq!(requirement.object_id(), object_id);
        assert_eq!(requirement.observation_index(), 0);
        assert_eq!(requirement.required_interval(), shutter);
    }

    #[test]
    fn protocol_coverage_refinement_and_accuracy_failures_are_semantic() {
        let mut store = RenderSceneStore::new();
        let represented = interval(0.0, 0.25);
        insert_object_with_representations(
            &mut store,
            0.1,
            RenderTemporalSupport::interval(represented),
        );
        let outside = interval(0.5, 0.5);
        let request = distance_request(
            RenderSemanticTolerance::absolute(0.01).expect("tolerance"),
            outside,
        );
        let failure = plan_render(
            &store.snapshot(),
            &request,
            &[field_method(1, RenderMotionInputPolicy::None)],
        )
        .expect_err("coverage/refinement should reject");
        assert!(matches!(failure, RenderPlanningFailure::NoSemanticSolution { .. }));
    }

    #[test]
    fn conservative_field_cannot_silently_satisfy_exact_or_relative_distance() {
        let mut store = RenderSceneStore::new();
        let shutter = interval(0.0, 0.0);
        insert_object_with_representations(&mut store, 0.01, RenderTemporalSupport::unbounded());
        for tolerance in [
            RenderSemanticTolerance::exact(),
            RenderSemanticTolerance::relative(0.1).expect("relative tolerance"),
        ] {
            let request = distance_request(tolerance, shutter);
            assert!(matches!(
                plan_render(
                    &store.snapshot(),
                    &request,
                    &[field_method(1, RenderMotionInputPolicy::None)],
                ),
                Err(RenderPlanningFailure::NoSemanticSolution { .. })
            ));
        }
    }

    #[test]
    fn unsupported_output_and_radiometric_domain_do_not_become_fallback() {
        let mut store = RenderSceneStore::new();
        let shutter = interval(0.0, 0.0);
        insert_object_with_representations(&mut store, 0.01, RenderTemporalSupport::unbounded());
        let observation = RenderObservationSpec::Perspective(
            RenderPerspectiveObservation::new(
                RenderAffineTransform3::identity(),
                std::f64::consts::FRAC_PI_2,
                1.0,
                shutter,
                RenderSamplingSupport::ideal_ray(),
            )
            .expect("perspective"),
        );
        let radiance = RenderOutputSpec::new(
            RenderOutputValue::Radiance {
                representation: RenderRadiometricRepresentation::spectral_at_wavelength_meters(
                    800e-9,
                )
                .expect("wavelength"),
            },
            RenderResultTopology::sample_lattice_2d(2, 2).expect("lattice"),
            RenderSemanticTolerance::relative(0.01).expect("tolerance"),
        )
        .expect("radiance output");
        let request = RenderRequest::new(
            shutter,
            vec![observation],
            vec![RenderRequestedOutput::new(0, radiance)],
        )
        .expect("request");
        let radiance_support = RenderSpectralRadianceSupport::new(400e-9, 700e-9)
            .expect("spectral support");
        let method = RenderMethodContract::new(
            RenderMethodId::new(1).expect("method id"),
            RenderMethodObservationSupport::perspective_only(),
            RenderMethodOutputSupport::none().with_radiance(radiance_support),
            RenderRepresentationProtocolRequirement::SurfaceQuery {
                revision: RENDER_SURFACE_QUERY_PROTOCOL_REVISION,
            },
            None,
            RenderMotionInputPolicy::None,
            Vec::new(),
        )
        .expect("method");
        let failure = plan_render(&store.snapshot(), &request, &[method])
            .expect_err("out-of-domain radiance cannot be substituted");
        let RenderPlanningFailure::NoSemanticSolution { rejections } = failure else {
            panic!("expected semantic no-solution failure")
        };
        assert!(matches!(
            rejections[0].reason(),
            RenderMethodRejectionReason::RadiometricDomainMismatch { output_index: 0 }
        ));
    }

    #[test]
    fn old_snapshots_remain_independently_plannable_and_planning_is_revision_neutral() {
        let mut store = RenderSceneStore::new();
        let shutter = interval(0.0, 0.0);
        insert_object_with_representations(&mut store, 0.01, RenderTemporalSupport::unbounded());
        let retained = store.snapshot();
        let revision = store.revision();
        let request = distance_request(
            RenderSemanticTolerance::absolute(0.02).expect("tolerance"),
            shutter,
        );
        let method = surface_method(1);
        let first = plan_render(&retained, &request, std::slice::from_ref(&method)).expect("plan");
        let second = plan_render(&store.snapshot(), &request, &[method]).expect("plan");
        assert_eq!(first, second);
        assert_eq!(store.revision(), revision);
    }

    #[test]
    fn duplicate_method_identity_rejects_independent_of_caller_order() {
        let store = RenderSceneStore::new();
        let shutter = interval(0.0, 0.0);
        let request = distance_request(RenderSemanticTolerance::exact(), shutter);
        let method = surface_method(1);
        assert_eq!(
            plan_render(&store.snapshot(), &request, &[method.clone(), method]),
            Err(RenderPlanningFailure::DuplicateMethodId {
                method_id: RenderMethodId::new(1).expect("method id")
            })
        );
    }
}
