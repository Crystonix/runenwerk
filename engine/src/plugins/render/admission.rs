//! R5 current availability, physical output binding, and execution admission.
//!
//! This module consumes an accepted R4 [`RenderPlan`] plus invocation-scoped operational facts.
//! It does not add current availability, GPU state, or physical destinations to R1-R4 semantic
//! authority. The founding R4 contracts declare no request-scoped semantic binding prerequisite,
//! so semantic-binding admission is intentionally vacuous here rather than represented by a
//! placeholder binding store.

use super::method::RenderAbstractExecutionRequirement;
use super::representation::RenderRepresentationId;
use super::scene::{RenderObjectId, RenderSceneRevision};
use super::semantic_plan::{
    RenderApplicableRepresentationUse, RenderOutputApproximation, RenderPlan, RenderPlanCandidate,
};
use runen_gpu::{
    GpuBufferHandle, GpuBufferUsage, GpuCapabilityFeature, GpuContext, GpuContextAffinity,
    GpuExecutionLifecycleState, GpuExecutionPolicy, GpuExecutionStats, GpuResourceOwnership,
    GpuTextureDimension, GpuTextureHandle, GpuTextureUsage,
};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// Point-in-time renderer interpretation of whether an R3 representation can currently be used.
///
/// This is not intrinsic representation evidence, physical GPU residency, or a long-lived registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderRepresentationAvailabilityState {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderRepresentationAvailabilityFact {
    representation_id: RenderRepresentationId,
    state: RenderRepresentationAvailabilityState,
}

impl RenderRepresentationAvailabilityFact {
    pub const fn new(
        representation_id: RenderRepresentationId,
        state: RenderRepresentationAvailabilityState,
    ) -> Self {
        Self {
            representation_id,
            state,
        }
    }

    pub const fn representation_id(self) -> RenderRepresentationId {
        self.representation_id
    }

    pub const fn state(self) -> RenderRepresentationAvailabilityState {
        self.state
    }
}

/// Physical destination family for one requested output.
///
/// The founding R5 slice deliberately does not define numeric channel/packing semantics. R2
/// semantic output meaning remains independent of this destination choice; method/lowering code
/// must establish any concrete numeric encoding before writing values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderOutputDestination {
    /// Founding scalar destination. The buffer must expose a physically writable usage.
    ScalarBuffer(GpuBufferHandle),
    /// Founding 2D-lattice destination. The whole base level must match the requested lattice.
    SampleLatticeTexture(GpuTextureHandle),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderOutputBinding {
    output_index: usize,
    destination: RenderOutputDestination,
}

impl RenderOutputBinding {
    pub const fn new(output_index: usize, destination: RenderOutputDestination) -> Self {
        Self {
            output_index,
            destination,
        }
    }

    pub const fn output_index(&self) -> usize {
        self.output_index
    }

    pub const fn destination(&self) -> &RenderOutputDestination {
        &self.destination
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderAdmittedObjectRepresentation {
    object_id: RenderObjectId,
    representation: RenderApplicableRepresentationUse,
}

impl RenderAdmittedObjectRepresentation {
    pub const fn object_id(&self) -> RenderObjectId {
        self.object_id
    }

    pub const fn representation(&self) -> RenderApplicableRepresentationUse {
        self.representation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderAdmittedOutput {
    output_index: usize,
    observation_index: usize,
    approximation: RenderOutputApproximation,
    object_representations: Vec<RenderAdmittedObjectRepresentation>,
    binding: RenderOutputBinding,
}

impl RenderAdmittedOutput {
    pub const fn output_index(&self) -> usize {
        self.output_index
    }

    pub const fn observation_index(&self) -> usize {
        self.observation_index
    }

    pub const fn approximation(&self) -> RenderOutputApproximation {
        self.approximation
    }

    pub fn object_representations(&self) -> &[RenderAdmittedObjectRepresentation] {
        &self.object_representations
    }

    pub const fn binding(&self) -> &RenderOutputBinding {
        &self.binding
    }
}

/// Point-in-time RunenGPU evidence used for one R5 admission decision.
///
/// This is evidence, not a capacity reservation. RunenGPU independently revalidates realization,
/// preparation, and submission against its current context state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderExecutionAdmissionEvidence {
    affinity: GpuContextAffinity,
    lifecycle: GpuExecutionLifecycleState,
    policy: GpuExecutionPolicy,
    stats: GpuExecutionStats,
}

impl RenderExecutionAdmissionEvidence {
    pub const fn affinity(self) -> GpuContextAffinity {
        self.affinity
    }

    pub const fn lifecycle(self) -> GpuExecutionLifecycleState {
        self.lifecycle
    }

    pub const fn policy(self) -> GpuExecutionPolicy {
        self.policy
    }

    pub const fn stats(self) -> GpuExecutionStats {
        self.stats
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderCandidateAdmissionRejection {
    candidate_index: usize,
    reason: RenderCandidateAdmissionRejectionReason,
}

impl RenderCandidateAdmissionRejection {
    pub const fn candidate_index(&self) -> usize {
        self.candidate_index
    }

    pub const fn reason(&self) -> &RenderCandidateAdmissionRejectionReason {
        &self.reason
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderCandidateAdmissionRejectionReason {
    ExecutionLifecycle {
        state: GpuExecutionLifecycleState,
    },
    RequiredCapabilityUnsupported {
        feature: GpuCapabilityFeature,
    },
    RequiredCapabilityNotEnabled {
        feature: GpuCapabilityFeature,
    },
    AvailabilityUnknown {
        output_index: usize,
        object_id: RenderObjectId,
        representation_id: RenderRepresentationId,
    },
    NoAvailableRepresentation {
        output_index: usize,
        object_id: RenderObjectId,
        representation_ids: Vec<RenderRepresentationId>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderAdmissionInputError {
    DuplicateAvailabilityFact {
        representation_id: RenderRepresentationId,
    },
    DuplicateOutputBinding {
        output_index: usize,
    },
    OutputBindingOutOfRange {
        output_index: usize,
        output_count: usize,
    },
    MissingOutputBinding {
        output_index: usize,
    },
    OutputDestinationKind {
        output_index: usize,
    },
    ScalarBufferNotWritable {
        output_index: usize,
    },
    LatticeTextureSurfaceAcquired {
        output_index: usize,
    },
    LatticeTextureDimension {
        output_index: usize,
        actual: GpuTextureDimension,
    },
    LatticeTextureExtent {
        output_index: usize,
        expected_width: u32,
        expected_height: u32,
        actual_width: u32,
        actual_height: u32,
        actual_depth_or_layers: u32,
    },
    LatticeTextureSampleCount {
        output_index: usize,
        actual: u32,
    },
    LatticeTextureNotWritable {
        output_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderExecutionAdmissionFailure {
    InvalidInput(RenderAdmissionInputError),
    NoExecutableCandidate {
        rejections: Vec<RenderCandidateAdmissionRejection>,
    },
}

impl fmt::Display for RenderExecutionAdmissionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(error) => {
                write!(formatter, "invalid render admission input: {error:?}")
            }
            Self::NoExecutableCandidate { .. } => {
                formatter.write_str("no semantically planned render candidate can execute now")
            }
        }
    }
}

impl Error for RenderExecutionAdmissionFailure {}

/// One R5-selected plan realization under point-in-time operational evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedRenderPlan {
    plan: RenderPlan,
    candidate_index: usize,
    outputs: Vec<RenderAdmittedOutput>,
    environment: RenderExecutionAdmissionEvidence,
    skipped_candidates: Vec<RenderCandidateAdmissionRejection>,
}

impl AdmittedRenderPlan {
    pub const fn scene_revision(&self) -> RenderSceneRevision {
        self.plan.scene_revision()
    }

    pub const fn plan(&self) -> &RenderPlan {
        &self.plan
    }

    pub fn selected_candidate(&self) -> &RenderPlanCandidate {
        &self.plan.candidates()[self.candidate_index]
    }

    pub fn outputs(&self) -> &[RenderAdmittedOutput] {
        &self.outputs
    }

    pub const fn environment(&self) -> RenderExecutionAdmissionEvidence {
        self.environment
    }

    pub fn skipped_candidates(&self) -> &[RenderCandidateAdmissionRejection] {
        &self.skipped_candidates
    }
}

#[derive(Debug, Clone, Copy)]
struct CurrentExecutionFacts {
    lifecycle: GpuExecutionLifecycleState,
    compute_supported: bool,
    compute_enabled: bool,
}

impl CurrentExecutionFacts {
    fn from_context(context: &GpuContext) -> Self {
        Self {
            lifecycle: context.execution_lifecycle_state(),
            compute_supported: context
                .adapter_facts()
                .supported()
                .supports(GpuCapabilityFeature::Compute),
            compute_enabled: context
                .device_facts()
                .is_enabled(GpuCapabilityFeature::Compute),
        }
    }
}

/// Admit one accepted R4 plan against current operational facts.
///
/// The function does not reserve RunenGPU capacity, allocate or realize resources, create work,
/// submit GPU operations, or alter the retained semantic scene/request/plan. Candidate ordering is
/// inherited from deterministic R4 planning; current fact insertion order is normalized here.
pub fn admit_render_plan(
    plan: &RenderPlan,
    availability: &[RenderRepresentationAvailabilityFact],
    output_bindings: &[RenderOutputBinding],
    context: &GpuContext,
) -> Result<AdmittedRenderPlan, RenderExecutionAdmissionFailure> {
    let availability = normalize_availability(availability)?;
    let bindings = normalize_output_bindings(plan, output_bindings)?;
    let current = CurrentExecutionFacts::from_context(context);
    let mut skipped_candidates = Vec::new();

    for (candidate_index, candidate) in plan.candidates().iter().enumerate() {
        match admit_candidate(candidate, &availability, &bindings, current) {
            Ok(outputs) => {
                return Ok(AdmittedRenderPlan {
                    plan: plan.clone(),
                    candidate_index,
                    outputs,
                    environment: RenderExecutionAdmissionEvidence {
                        affinity: context.affinity(),
                        lifecycle: current.lifecycle,
                        policy: context.execution_policy(),
                        stats: context.execution_stats(),
                    },
                    skipped_candidates,
                });
            }
            Err(reason) => skipped_candidates.push(RenderCandidateAdmissionRejection {
                candidate_index,
                reason,
            }),
        }
    }

    Err(RenderExecutionAdmissionFailure::NoExecutableCandidate {
        rejections: skipped_candidates,
    })
}

fn normalize_availability(
    facts: &[RenderRepresentationAvailabilityFact],
) -> Result<
    BTreeMap<RenderRepresentationId, RenderRepresentationAvailabilityState>,
    RenderExecutionAdmissionFailure,
> {
    let mut normalized = BTreeMap::new();
    for fact in facts.iter().copied() {
        if normalized
            .insert(fact.representation_id(), fact.state())
            .is_some()
        {
            return Err(RenderExecutionAdmissionFailure::InvalidInput(
                RenderAdmissionInputError::DuplicateAvailabilityFact {
                    representation_id: fact.representation_id(),
                },
            ));
        }
    }
    Ok(normalized)
}

fn normalize_output_bindings(
    plan: &RenderPlan,
    bindings: &[RenderOutputBinding],
) -> Result<Vec<RenderOutputBinding>, RenderExecutionAdmissionFailure> {
    let output_count = plan.outputs().len();
    let mut normalized = BTreeMap::new();
    for binding in bindings {
        let output_index = binding.output_index();
        if output_index >= output_count {
            return Err(RenderExecutionAdmissionFailure::InvalidInput(
                RenderAdmissionInputError::OutputBindingOutOfRange {
                    output_index,
                    output_count,
                },
            ));
        }
        if normalized.insert(output_index, binding.clone()).is_some() {
            return Err(RenderExecutionAdmissionFailure::InvalidInput(
                RenderAdmissionInputError::DuplicateOutputBinding { output_index },
            ));
        }
    }

    let mut ordered = Vec::with_capacity(output_count);
    for output_index in 0..output_count {
        let Some(binding) = normalized.remove(&output_index) else {
            return Err(RenderExecutionAdmissionFailure::InvalidInput(
                RenderAdmissionInputError::MissingOutputBinding { output_index },
            ));
        };
        validate_output_binding(plan, &binding)?;
        ordered.push(binding);
    }
    Ok(ordered)
}

fn validate_output_binding(
    plan: &RenderPlan,
    binding: &RenderOutputBinding,
) -> Result<(), RenderExecutionAdmissionFailure> {
    let output_index = binding.output_index();
    let topology = plan.outputs()[output_index].spec().topology();
    match (topology.sample_lattice_dimensions(), binding.destination()) {
        (None, RenderOutputDestination::ScalarBuffer(buffer)) if topology.is_scalar() => {
            let usages = buffer.descriptor().usages();
            if !usages.contains(GpuBufferUsage::Storage)
                && !usages.contains(GpuBufferUsage::CopyDestination)
            {
                return Err(RenderExecutionAdmissionFailure::InvalidInput(
                    RenderAdmissionInputError::ScalarBufferNotWritable { output_index },
                ));
            }
            Ok(())
        }
        (Some((width, height)), RenderOutputDestination::SampleLatticeTexture(texture)) => {
            let descriptor = texture.descriptor();
            if descriptor.common().ownership() == GpuResourceOwnership::SurfaceAcquired {
                return Err(RenderExecutionAdmissionFailure::InvalidInput(
                    RenderAdmissionInputError::LatticeTextureSurfaceAcquired { output_index },
                ));
            }
            if descriptor.dimension() != GpuTextureDimension::D2 {
                return Err(RenderExecutionAdmissionFailure::InvalidInput(
                    RenderAdmissionInputError::LatticeTextureDimension {
                        output_index,
                        actual: descriptor.dimension(),
                    },
                ));
            }
            let extent = descriptor.extent();
            if extent.width() != width || extent.height() != height || extent.depth_or_layers() != 1
            {
                return Err(RenderExecutionAdmissionFailure::InvalidInput(
                    RenderAdmissionInputError::LatticeTextureExtent {
                        output_index,
                        expected_width: width,
                        expected_height: height,
                        actual_width: extent.width(),
                        actual_height: extent.height(),
                        actual_depth_or_layers: extent.depth_or_layers(),
                    },
                ));
            }
            if descriptor.sample_count() != 1 {
                return Err(RenderExecutionAdmissionFailure::InvalidInput(
                    RenderAdmissionInputError::LatticeTextureSampleCount {
                        output_index,
                        actual: descriptor.sample_count(),
                    },
                ));
            }
            let usages = descriptor.usages();
            if !usages.contains(GpuTextureUsage::StorageWrite)
                && !usages.contains(GpuTextureUsage::ColorAttachment)
                && !usages.contains(GpuTextureUsage::CopyDestination)
            {
                return Err(RenderExecutionAdmissionFailure::InvalidInput(
                    RenderAdmissionInputError::LatticeTextureNotWritable { output_index },
                ));
            }
            Ok(())
        }
        _ => Err(RenderExecutionAdmissionFailure::InvalidInput(
            RenderAdmissionInputError::OutputDestinationKind { output_index },
        )),
    }
}

fn admit_candidate(
    candidate: &RenderPlanCandidate,
    availability: &BTreeMap<RenderRepresentationId, RenderRepresentationAvailabilityState>,
    bindings: &[RenderOutputBinding],
    current: CurrentExecutionFacts,
) -> Result<Vec<RenderAdmittedOutput>, RenderCandidateAdmissionRejectionReason> {
    validate_current_execution(candidate, current)?;

    let mut outputs = Vec::with_capacity(candidate.outputs().len());
    for planned_output in candidate.outputs() {
        let mut object_representations =
            Vec::with_capacity(planned_output.object_representations().len());
        for object in planned_output.object_representations() {
            let mut selected = None;
            let mut first_unknown = None;
            let mut representation_ids = Vec::with_capacity(object.uses().len());
            for representation in object.uses().iter().copied() {
                let representation_id = representation.representation_id();
                representation_ids.push(representation_id);
                match availability.get(&representation_id).copied() {
                    Some(RenderRepresentationAvailabilityState::Available) => {
                        selected = Some(representation);
                        break;
                    }
                    Some(RenderRepresentationAvailabilityState::Unavailable) => {}
                    None => {
                        first_unknown.get_or_insert(representation_id);
                    }
                }
            }

            let representation = match selected {
                Some(representation) => representation,
                None => {
                    if let Some(representation_id) = first_unknown {
                        return Err(
                            RenderCandidateAdmissionRejectionReason::AvailabilityUnknown {
                                output_index: planned_output.output_index(),
                                object_id: object.object_id(),
                                representation_id,
                            },
                        );
                    }
                    return Err(
                        RenderCandidateAdmissionRejectionReason::NoAvailableRepresentation {
                            output_index: planned_output.output_index(),
                            object_id: object.object_id(),
                            representation_ids,
                        },
                    );
                }
            };
            object_representations.push(RenderAdmittedObjectRepresentation {
                object_id: object.object_id(),
                representation,
            });
        }

        outputs.push(RenderAdmittedOutput {
            output_index: planned_output.output_index(),
            observation_index: planned_output.observation_index(),
            approximation: planned_output.approximation(),
            object_representations,
            binding: bindings[planned_output.output_index()].clone(),
        });
    }
    Ok(outputs)
}

fn validate_current_execution(
    candidate: &RenderPlanCandidate,
    current: CurrentExecutionFacts,
) -> Result<(), RenderCandidateAdmissionRejectionReason> {
    if current.lifecycle != GpuExecutionLifecycleState::Running {
        return Err(
            RenderCandidateAdmissionRejectionReason::ExecutionLifecycle {
                state: current.lifecycle,
            },
        );
    }
    for requirement in candidate.abstract_execution_requirements() {
        match requirement {
            RenderAbstractExecutionRequirement::GeneralParallelWork => {
                if !current.compute_supported {
                    return Err(
                        RenderCandidateAdmissionRejectionReason::RequiredCapabilityUnsupported {
                            feature: GpuCapabilityFeature::Compute,
                        },
                    );
                }
                if !current.compute_enabled {
                    return Err(
                        RenderCandidateAdmissionRejectionReason::RequiredCapabilityNotEnabled {
                            feature: GpuCapabilityFeature::Compute,
                        },
                    );
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::method::{
        RenderDistanceErrorBound, RenderMethodContract, RenderMethodId, RenderMethodOutputContract,
        RenderMethodOutputGuarantee, RenderMethodOutputKind, RenderMethodRepresentationRequirement,
        RenderObservationKind, RenderRepresentationProtocolRequirement,
    };
    use crate::plugins::render::participation::RenderObjectParticipation;
    use crate::plugins::render::representation::{
        RENDER_SURFACE_QUERY_PROTOCOL_REVISION, RenderRefinementEvidence,
        RenderRepresentationRecord, RenderSurfaceProtocolEvidence,
    };
    use crate::plugins::render::request::{
        RenderDistanceConvention, RenderObservationSpec, RenderOutputSpec, RenderOutputValue,
        RenderPerspectiveObservation, RenderProbeObservation, RenderRequestedOutput,
        RenderResultTopology, RenderSamplingSupport, RenderSemanticTolerance,
    };
    use crate::plugins::render::scene::{RenderObjectState, RenderSceneStore, RenderSceneUpdate};
    use crate::plugins::render::semantic_plan::plan_render;
    use crate::plugins::render::space_time::{
        RenderAffineTransform3, RenderHandedness, RenderObjectSpatialState,
        RenderObjectTemporalState, RenderSpaceSpec, RenderSpatialCoverage, RenderTemporalSupport,
        RenderTimeInterval, RenderTimePoint,
    };
    use runen_gpu::{
        GpuBufferDescriptor, GpuBufferInitialization, GpuBufferUsages, GpuMemoryIntent,
        GpuReconstruction, GpuResourceCommon, GpuResourceLabel, GpuResourceLifetime,
        GpuResourceProvenance, GpuTextureDescriptor, GpuTextureExtent, GpuTextureFormat,
        GpuTextureInitialization, GpuTextureUsages, GpuWorkResourceIdAllocator,
    };

    fn interval(time: f64) -> RenderTimeInterval {
        let point = RenderTimePoint::from_seconds(time).expect("time");
        RenderTimeInterval::new(point, point).expect("instant")
    }

    fn object_state() -> RenderObjectState {
        RenderObjectState::new(
            RenderObjectSpatialState::new(
                RenderSpaceSpec::new(1.0, RenderHandedness::Right).expect("space"),
                RenderAffineTransform3::identity(),
                RenderSpatialCoverage::unbounded(),
            ),
            RenderObjectTemporalState::new(RenderTemporalSupport::unbounded()),
        )
    }

    fn scalar_distance_request_with_tolerance(
        tolerance: RenderSemanticTolerance,
    ) -> super::super::request::RenderRequest {
        let shutter = interval(0.0);
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
        .expect("output");
        super::super::request::RenderRequest::new(
            shutter,
            vec![observation],
            vec![RenderRequestedOutput::new(0, output)],
        )
        .expect("request")
    }

    fn scalar_distance_request() -> super::super::request::RenderRequest {
        scalar_distance_request_with_tolerance(RenderSemanticTolerance::exact())
    }

    fn two_scalar_distance_request() -> super::super::request::RenderRequest {
        let shutter = interval(0.0);
        let probe = || {
            RenderObservationSpec::Probe(
                RenderProbeObservation::new(
                    RenderAffineTransform3::identity(),
                    shutter,
                    RenderSamplingSupport::ideal_ray(),
                )
                .expect("probe"),
            )
        };
        let output = || {
            RenderOutputSpec::new(
                RenderOutputValue::Distance {
                    convention: RenderDistanceConvention::RayDistance,
                },
                RenderResultTopology::scalar(),
                RenderSemanticTolerance::exact(),
            )
            .expect("output")
        };
        super::super::request::RenderRequest::new(
            shutter,
            vec![probe(), probe()],
            vec![
                RenderRequestedOutput::new(0, output()),
                RenderRequestedOutput::new(1, output()),
            ],
        )
        .expect("request")
    }

    fn lattice_distance_request(width: u32, height: u32) -> super::super::request::RenderRequest {
        let shutter = interval(0.0);
        let observation = RenderObservationSpec::Perspective(
            RenderPerspectiveObservation::new(
                RenderAffineTransform3::identity(),
                std::f64::consts::FRAC_PI_2,
                f64::from(width) / f64::from(height),
                shutter,
                RenderSamplingSupport::ideal_ray(),
            )
            .expect("perspective"),
        );
        let output = RenderOutputSpec::new(
            RenderOutputValue::Distance {
                convention: RenderDistanceConvention::RayDistance,
            },
            RenderResultTopology::sample_lattice_2d(width, height).expect("lattice topology"),
            RenderSemanticTolerance::exact(),
        )
        .expect("output");
        super::super::request::RenderRequest::new(
            shutter,
            vec![observation],
            vec![RenderRequestedOutput::new(0, output)],
        )
        .expect("request")
    }

    fn surface_requirement() -> RenderMethodRepresentationRequirement {
        RenderMethodRepresentationRequirement::new(
            RenderRepresentationProtocolRequirement::SurfaceQuery {
                revision: RENDER_SURFACE_QUERY_PROTOCOL_REVISION,
            },
            None,
        )
        .expect("requirement")
    }

    fn method_with_guarantee(
        observation_kind: RenderObservationKind,
        guarantee: RenderMethodOutputGuarantee,
    ) -> RenderMethodContract {
        let output = RenderMethodOutputContract::new(
            observation_kind,
            RenderMethodOutputKind::Distance {
                convention: RenderDistanceConvention::RayDistance,
            },
            guarantee,
            vec![surface_requirement()],
            false,
        )
        .expect("output contract");
        RenderMethodContract::new(
            RenderMethodId::new(1).expect("method id"),
            vec![output],
            vec![RenderAbstractExecutionRequirement::GeneralParallelWork],
        )
        .expect("method")
    }

    fn method(observation_kind: RenderObservationKind) -> RenderMethodContract {
        method_with_guarantee(observation_kind, RenderMethodOutputGuarantee::Exact)
    }

    fn plan_with_two_surface_representations_for(
        request: super::super::request::RenderRequest,
        guarantee: RenderMethodOutputGuarantee,
    ) -> (RenderPlan, RenderRepresentationId, RenderRepresentationId) {
        let mut store = RenderSceneStore::new();
        let object_id = store.allocate_object_id().expect("object id");
        let mut insert = RenderSceneUpdate::new();
        insert.insert_with_state(object_id, object_state());
        store.commit(insert).expect("insert");

        let first_id = store
            .allocate_representation_id(object_id)
            .expect("first representation id");
        let second_id = store
            .allocate_representation_id(object_id)
            .expect("second representation id");
        let representation = |id| {
            RenderRepresentationRecord::new(
                id,
                RenderSpatialCoverage::unbounded(),
                RenderTemporalSupport::unbounded(),
                RenderRefinementEvidence::none(),
                Some(
                    RenderSurfaceProtocolEvidence::exact(RENDER_SURFACE_QUERY_PROTOCOL_REVISION)
                        .expect("surface evidence"),
                ),
                None,
            )
            .expect("representation")
        };
        let participation = RenderObjectParticipation::new(
            vec![representation(second_id), representation(first_id)],
            None,
            None,
        )
        .expect("participation");
        let mut attach = RenderSceneUpdate::new();
        attach.replace_participation(object_id, participation);
        store.commit(attach).expect("attach");

        let plan = plan_render(
            &store.snapshot(),
            &request,
            &[method_with_guarantee(RenderObservationKind::Probe, guarantee)],
        )
        .expect("plan");
        (plan, first_id, second_id)
    }

    fn plan_with_two_surface_representations()
    -> (RenderPlan, RenderRepresentationId, RenderRepresentationId) {
        plan_with_two_surface_representations_for(
            scalar_distance_request(),
            RenderMethodOutputGuarantee::Exact,
        )
    }

    fn two_output_plan() -> (RenderPlan, RenderRepresentationId) {
        let mut store = RenderSceneStore::new();
        let object_id = store.allocate_object_id().expect("object id");
        let mut insert = RenderSceneUpdate::new();
        insert.insert_with_state(object_id, object_state());
        store.commit(insert).expect("insert");

        let representation_id = store
            .allocate_representation_id(object_id)
            .expect("representation id");
        let representation = RenderRepresentationRecord::new(
            representation_id,
            RenderSpatialCoverage::unbounded(),
            RenderTemporalSupport::unbounded(),
            RenderRefinementEvidence::none(),
            Some(
                RenderSurfaceProtocolEvidence::exact(RENDER_SURFACE_QUERY_PROTOCOL_REVISION)
                    .expect("surface evidence"),
            ),
            None,
        )
        .expect("representation");
        let participation = RenderObjectParticipation::new(vec![representation], None, None)
            .expect("participation");
        let mut attach = RenderSceneUpdate::new();
        attach.replace_participation(object_id, participation);
        store.commit(attach).expect("attach");

        let plan = plan_render(
            &store.snapshot(),
            &two_scalar_distance_request(),
            &[method(RenderObservationKind::Probe)],
        )
        .expect("plan");
        (plan, representation_id)
    }

    fn lattice_plan(width: u32, height: u32) -> RenderPlan {
        let mut store = RenderSceneStore::new();
        let object_id = store.allocate_object_id().expect("object id");
        let mut insert = RenderSceneUpdate::new();
        insert.insert_with_state(object_id, object_state());
        store.commit(insert).expect("insert");

        let representation_id = store
            .allocate_representation_id(object_id)
            .expect("representation id");
        let representation = RenderRepresentationRecord::new(
            representation_id,
            RenderSpatialCoverage::unbounded(),
            RenderTemporalSupport::unbounded(),
            RenderRefinementEvidence::none(),
            Some(
                RenderSurfaceProtocolEvidence::exact(RENDER_SURFACE_QUERY_PROTOCOL_REVISION)
                    .expect("surface evidence"),
            ),
            None,
        )
        .expect("representation");
        let participation = RenderObjectParticipation::new(vec![representation], None, None)
            .expect("participation");
        let mut attach = RenderSceneUpdate::new();
        attach.replace_participation(object_id, participation);
        store.commit(attach).expect("attach");

        plan_render(
            &store.snapshot(),
            &lattice_distance_request(width, height),
            &[method(RenderObservationKind::Perspective)],
        )
        .expect("plan")
    }

    fn available_execution() -> CurrentExecutionFacts {
        CurrentExecutionFacts {
            lifecycle: GpuExecutionLifecycleState::Running,
            compute_supported: true,
            compute_enabled: true,
        }
    }

    fn scalar_buffer_binding_for(output_index: usize, label_text: &str) -> RenderOutputBinding {
        let label = GpuResourceLabel::new(label_text).expect("label");
        let provenance = GpuResourceProvenance::new(label.clone(), None, None);
        let common = GpuResourceCommon::owned(
            label.clone(),
            GpuResourceLifetime::Transient,
            GpuMemoryIntent::Device,
            GpuReconstruction::SourceBacked,
            provenance,
        )
        .expect("common");
        let usages = GpuBufferUsages::new(&label, [GpuBufferUsage::Storage]).expect("usages");
        let descriptor =
            GpuBufferDescriptor::new(common, 16, usages, GpuBufferInitialization::Uninitialized)
                .expect("descriptor");
        let mut allocator = GpuWorkResourceIdAllocator::new();
        let buffer = allocator
            .allocate_buffer_handle(descriptor)
            .expect("buffer handle");
        RenderOutputBinding::new(output_index, RenderOutputDestination::ScalarBuffer(buffer))
    }

    fn scalar_buffer_binding() -> RenderOutputBinding {
        scalar_buffer_binding_for(0, "r5 scalar output")
    }

    fn lattice_texture(width: u32, height: u32, writable: bool) -> GpuTextureHandle {
        let label = GpuResourceLabel::new("r5 lattice output").expect("label");
        let provenance = GpuResourceProvenance::new(label.clone(), None, None);
        let common = GpuResourceCommon::owned(
            label.clone(),
            GpuResourceLifetime::Transient,
            GpuMemoryIntent::Device,
            GpuReconstruction::SourceBacked,
            provenance,
        )
        .expect("common");
        let usages = if writable {
            GpuTextureUsages::new(&label, [GpuTextureUsage::CopyDestination]).expect("usages")
        } else {
            GpuTextureUsages::new(&label, [GpuTextureUsage::Sampled]).expect("usages")
        };
        let extent = GpuTextureExtent::new(&label, GpuTextureDimension::D2, width, height, 1)
            .expect("extent");
        let descriptor = GpuTextureDescriptor::new(
            common,
            GpuTextureDimension::D2,
            extent,
            1,
            1,
            GpuTextureFormat::R8Unorm,
            usages,
            GpuTextureInitialization::Uninitialized,
        )
        .expect("descriptor");
        let mut allocator = GpuWorkResourceIdAllocator::new();
        allocator
            .allocate_texture_handle(descriptor)
            .expect("texture handle")
    }

    #[test]
    fn availability_is_invocation_scoped_and_order_independent() {
        let (plan, first_id, second_id) = plan_with_two_surface_representations();
        let before = plan.scene_revision();
        let bindings =
            normalize_output_bindings(&plan, &[scalar_buffer_binding()]).expect("bindings");
        let first_order = normalize_availability(&[
            RenderRepresentationAvailabilityFact::new(
                first_id,
                RenderRepresentationAvailabilityState::Unavailable,
            ),
            RenderRepresentationAvailabilityFact::new(
                second_id,
                RenderRepresentationAvailabilityState::Available,
            ),
        ])
        .expect("availability");
        let reversed = normalize_availability(&[
            RenderRepresentationAvailabilityFact::new(
                second_id,
                RenderRepresentationAvailabilityState::Available,
            ),
            RenderRepresentationAvailabilityFact::new(
                first_id,
                RenderRepresentationAvailabilityState::Unavailable,
            ),
        ])
        .expect("availability");
        let first = admit_candidate(
            plan.candidates().first().unwrap(),
            &first_order,
            &bindings,
            available_execution(),
        )
        .expect("candidate");
        let second = admit_candidate(
            plan.candidates().first().unwrap(),
            &reversed,
            &bindings,
            available_execution(),
        )
        .expect("candidate");

        assert_eq!(first, second);
        assert_eq!(
            first[0].object_representations()[0]
                .representation()
                .representation_id(),
            second_id
        );
        assert_eq!(plan.scene_revision(), before);
    }

    #[test]
    fn unavailable_is_not_unsupported_and_unknown_is_explicit() {
        let (plan, first_id, second_id) = plan_with_two_surface_representations();
        let bindings =
            normalize_output_bindings(&plan, &[scalar_buffer_binding()]).expect("bindings");
        let unavailable = normalize_availability(&[
            RenderRepresentationAvailabilityFact::new(
                first_id,
                RenderRepresentationAvailabilityState::Unavailable,
            ),
            RenderRepresentationAvailabilityFact::new(
                second_id,
                RenderRepresentationAvailabilityState::Unavailable,
            ),
        ])
        .expect("availability");
        assert!(matches!(
            admit_candidate(
                plan.candidates().first().unwrap(),
                &unavailable,
                &bindings,
                available_execution()
            ),
            Err(RenderCandidateAdmissionRejectionReason::NoAvailableRepresentation { .. })
        ));

        let incomplete = normalize_availability(&[RenderRepresentationAvailabilityFact::new(
            first_id,
            RenderRepresentationAvailabilityState::Unavailable,
        )])
        .expect("availability");
        assert!(matches!(
            admit_candidate(
                plan.candidates().first().unwrap(),
                &incomplete,
                &bindings,
                available_execution()
            ),
            Err(RenderCandidateAdmissionRejectionReason::AvailabilityUnknown {
                representation_id,
                ..
            }) if representation_id == second_id
        ));
    }

    #[test]
    fn duplicate_current_facts_and_bindings_are_rejected() {
        let (plan, first_id, _) = plan_with_two_surface_representations();
        let duplicate = RenderRepresentationAvailabilityFact::new(
            first_id,
            RenderRepresentationAvailabilityState::Available,
        );
        assert!(matches!(
            normalize_availability(&[duplicate, duplicate]),
            Err(RenderExecutionAdmissionFailure::InvalidInput(
                RenderAdmissionInputError::DuplicateAvailabilityFact { .. }
            ))
        ));

        let binding = scalar_buffer_binding();
        assert!(matches!(
            normalize_output_bindings(&plan, &[binding.clone(), binding]),
            Err(RenderExecutionAdmissionFailure::InvalidInput(
                RenderAdmissionInputError::DuplicateOutputBinding { output_index: 0 }
            ))
        ));
    }

    #[test]
    fn multi_output_bindings_are_correlated_by_index_not_input_order() {
        let (plan, representation_id) = two_output_plan();
        let first = scalar_buffer_binding_for(0, "r5 output zero");
        let second = scalar_buffer_binding_for(1, "r5 output one");
        let ordered = normalize_output_bindings(&plan, &[second.clone(), first.clone()])
            .expect("reversed bindings normalize by output index");
        assert_eq!(ordered, vec![first.clone(), second.clone()]);

        assert!(matches!(
            normalize_output_bindings(&plan, &[first.clone()]),
            Err(RenderExecutionAdmissionFailure::InvalidInput(
                RenderAdmissionInputError::MissingOutputBinding { output_index: 1 }
            ))
        ));

        let availability = normalize_availability(&[RenderRepresentationAvailabilityFact::new(
            representation_id,
            RenderRepresentationAvailabilityState::Available,
        )])
        .expect("availability");
        let admitted = admit_candidate(
            plan.candidates().first().unwrap(),
            &availability,
            &ordered,
            available_execution(),
        )
        .expect("candidate");
        assert_eq!(admitted.len(), 2);
        assert_eq!(admitted[0].output_index(), 0);
        assert_eq!(admitted[1].output_index(), 1);
        assert_eq!(admitted[0].binding(), &first);
        assert_eq!(admitted[1].binding(), &second);
    }

    #[test]
    fn bounded_r4_approximation_is_preserved_unchanged_through_admission() {
        let max_error_meters = RenderDistanceErrorBound::new(0.01).expect("distance bound");
        let request = scalar_distance_request_with_tolerance(
            RenderSemanticTolerance::absolute(0.02).expect("tolerance"),
        );
        let (plan, first_id, second_id) = plan_with_two_surface_representations_for(
            request,
            RenderMethodOutputGuarantee::BoundedAbsoluteDistance { max_error_meters },
        );
        let availability = normalize_availability(&[
            RenderRepresentationAvailabilityFact::new(
                first_id,
                RenderRepresentationAvailabilityState::Available,
            ),
            RenderRepresentationAvailabilityFact::new(
                second_id,
                RenderRepresentationAvailabilityState::Unavailable,
            ),
        ])
        .expect("availability");
        let bindings =
            normalize_output_bindings(&plan, &[scalar_buffer_binding()]).expect("bindings");
        let planned_approximation = plan.candidates()[0].outputs()[0].approximation();
        let admitted = admit_candidate(
            &plan.candidates()[0],
            &availability,
            &bindings,
            available_execution(),
        )
        .expect("candidate");

        assert_eq!(admitted[0].approximation(), planned_approximation);
        assert_eq!(
            admitted[0].approximation(),
            RenderOutputApproximation::BoundedAbsoluteDistance { max_error_meters }
        );
    }

    #[test]
    fn scalar_output_rejects_lattice_texture_destination() {
        let (plan, _, _) = plan_with_two_surface_representations();
        let wrong = RenderOutputBinding::new(
            0,
            RenderOutputDestination::SampleLatticeTexture(lattice_texture(1, 1, true)),
        );
        assert!(matches!(
            normalize_output_bindings(&plan, &[wrong]),
            Err(RenderExecutionAdmissionFailure::InvalidInput(
                RenderAdmissionInputError::OutputDestinationKind { output_index: 0 }
            ))
        ));
    }

    #[test]
    fn current_execution_requires_running_lifecycle_and_enabled_compute() {
        let (plan, _, _) = plan_with_two_surface_representations();
        let candidate = plan.candidates().first().unwrap();
        let mut current = available_execution();
        current.compute_supported = false;
        assert_eq!(
            validate_current_execution(candidate, current),
            Err(
                RenderCandidateAdmissionRejectionReason::RequiredCapabilityUnsupported {
                    feature: GpuCapabilityFeature::Compute,
                }
            )
        );

        let mut current = available_execution();
        current.compute_enabled = false;
        assert_eq!(
            validate_current_execution(candidate, current),
            Err(
                RenderCandidateAdmissionRejectionReason::RequiredCapabilityNotEnabled {
                    feature: GpuCapabilityFeature::Compute,
                }
            )
        );

        let mut current = available_execution();
        current.lifecycle = GpuExecutionLifecycleState::ShuttingDown;
        assert_eq!(
            validate_current_execution(candidate, current),
            Err(
                RenderCandidateAdmissionRejectionReason::ExecutionLifecycle {
                    state: GpuExecutionLifecycleState::ShuttingDown,
                }
            )
        );
    }

    #[test]
    fn lattice_binding_validation_uses_topology_extent_and_writability_not_semantic_format() {
        let plan = lattice_plan(4, 3);
        let valid = RenderOutputBinding::new(
            0,
            RenderOutputDestination::SampleLatticeTexture(lattice_texture(4, 3, true)),
        );
        normalize_output_bindings(&plan, &[valid]).expect("matching writable lattice binding");

        let wrong_extent = RenderOutputBinding::new(
            0,
            RenderOutputDestination::SampleLatticeTexture(lattice_texture(5, 3, true)),
        );
        assert!(matches!(
            normalize_output_bindings(&plan, &[wrong_extent]),
            Err(RenderExecutionAdmissionFailure::InvalidInput(
                RenderAdmissionInputError::LatticeTextureExtent {
                    output_index: 0,
                    expected_width: 4,
                    expected_height: 3,
                    ..
                }
            ))
        ));

        let not_writable = RenderOutputBinding::new(
            0,
            RenderOutputDestination::SampleLatticeTexture(lattice_texture(4, 3, false)),
        );
        assert!(matches!(
            normalize_output_bindings(&plan, &[not_writable]),
            Err(RenderExecutionAdmissionFailure::InvalidInput(
                RenderAdmissionInputError::LatticeTextureNotWritable { output_index: 0 }
            ))
        ));
    }
}
