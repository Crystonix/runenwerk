//! Focused R7 proof for request-scoped semantic surface binding laws activated by #566.
//!
//! This module proves protocol locality and one-binding temporal validity around the maintained R5
//! semantic-binding implementation. It does not create a second planning or binding authority.

use super::method::{
    RenderAbstractExecutionRequirement, RenderFieldDistanceInputRequirement, RenderMethodContract,
    RenderMethodId, RenderMethodOutputContract, RenderMethodOutputGuarantee, RenderMethodOutputKind,
    RenderMethodRepresentationRequirement, RenderObservationKind,
    RenderRepresentationProtocolRequirement,
};
use super::participation::RenderObjectParticipation;
use super::representation::{
    RENDER_FIELD_DISTANCE_PROTOCOL_REVISION, RENDER_ORIENTED_SURFACE_QUERY_PROTOCOL_REVISION,
    RENDER_SURFACE_QUERY_PROTOCOL_REVISION, RenderFieldDistanceGuarantee,
    RenderFieldDistanceProtocolEvidence, RenderOrientedSurfaceProtocolEvidence,
    RenderRefinementEvidence, RenderRepresentationId, RenderRepresentationProtocol,
    RenderRepresentationRecord, RenderSurfaceProtocolEvidence,
};
use super::request::{
    RenderDistanceConvention, RenderObservationSpec, RenderOutputSpec, RenderOutputValue,
    RenderProbeObservation, RenderRequest, RenderRequestedOutput, RenderResultTopology,
    RenderSamplingSupport, RenderSemanticTolerance,
};
use super::scene::{RenderObjectId, RenderObjectState, RenderSceneStore, RenderSceneUpdate};
use super::semantic_binding::{
    RenderNormalizedSurfaceSemanticInputs, RenderSemanticCandidateRejection,
};
use super::semantic_plan::{RenderPlan, plan_render};
use super::space_time::{
    RenderAffineTransform3, RenderHandedness, RenderObjectSpatialState, RenderObjectTemporalState,
    RenderSpaceSpec, RenderSpatialCoverage, RenderTemporalSupport, RenderTimeInterval,
    RenderTimePoint,
};
use super::surface_input::{
    RenderSurfaceSemanticInput, RenderSurfaceSemanticInputBinding,
    RenderSurfaceSemanticInputRequirement,
};

fn point(seconds: f64) -> RenderTimePoint {
    RenderTimePoint::from_seconds(seconds).expect("finite R7 proof time")
}

fn interval(start: f64, end: f64) -> RenderTimeInterval {
    RenderTimeInterval::new(point(start), point(end)).expect("ordered R7 proof interval")
}

fn object_state() -> RenderObjectState {
    RenderObjectState::new(
        RenderObjectSpatialState::new(
            RenderSpaceSpec::new(1.0, RenderHandedness::Right).expect("R7 proof space"),
            RenderAffineTransform3::identity(),
            RenderSpatialCoverage::unbounded(),
        ),
        RenderObjectTemporalState::new(RenderTemporalSupport::unbounded()),
    )
}

fn request_at_times(times: &[f64]) -> RenderRequest {
    let start = *times.first().expect("at least one observation time");
    let end = *times.last().expect("at least one observation time");
    let observations = times
        .iter()
        .copied()
        .map(|time| {
            RenderObservationSpec::Probe(
                RenderProbeObservation::new(
                    RenderAffineTransform3::identity(),
                    RenderTimeInterval::instant(point(time)),
                    RenderSamplingSupport::ideal_ray(),
                )
                .expect("R7 proof probe"),
            )
        })
        .collect::<Vec<_>>();
    let outputs = times
        .iter()
        .enumerate()
        .map(|(observation_index, _)| {
            RenderRequestedOutput::new(
                observation_index,
                RenderOutputSpec::new(
                    RenderOutputValue::Distance {
                        convention: RenderDistanceConvention::RayDistance,
                    },
                    RenderResultTopology::scalar(),
                    RenderSemanticTolerance::exact(),
                )
                .expect("R7 proof distance output"),
            )
        })
        .collect::<Vec<_>>();
    RenderRequest::new(interval(start, end), observations, outputs).expect("R7 proof request")
}

fn method(requirements: Vec<RenderMethodRepresentationRequirement>) -> RenderMethodContract {
    let output = RenderMethodOutputContract::new(
        RenderObservationKind::Probe,
        RenderMethodOutputKind::Distance {
            convention: RenderDistanceConvention::RayDistance,
        },
        RenderMethodOutputGuarantee::Exact,
        requirements,
        false,
    )
    .expect("R7 proof output contract");
    RenderMethodContract::new(
        RenderMethodId::new(1).expect("R7 proof method id"),
        vec![output],
        vec![RenderAbstractExecutionRequirement::GeneralParallelWork],
    )
    .expect("R7 proof method")
}

fn plan_for(
    request: RenderRequest,
    surface: Option<RenderSurfaceProtocolEvidence>,
    field: Option<RenderFieldDistanceProtocolEvidence>,
    requirements: Vec<RenderMethodRepresentationRequirement>,
) -> (RenderPlan, RenderRepresentationId, RenderObjectId) {
    let mut store = RenderSceneStore::new();
    let object_id = store.allocate_object_id().expect("R7 proof object id");
    let mut insert = RenderSceneUpdate::new();
    insert.insert_with_state(object_id, object_state());
    store.commit(insert).expect("insert R7 proof object");

    let representation_id = store
        .allocate_representation_id(object_id)
        .expect("R7 proof representation id");
    let representation = RenderRepresentationRecord::new(
        representation_id,
        RenderSpatialCoverage::unbounded(),
        RenderTemporalSupport::unbounded(),
        RenderRefinementEvidence::none(),
        surface,
        field,
    )
    .expect("R7 proof representation");
    let participation = RenderObjectParticipation::new(vec![representation], None, None)
        .expect("R7 proof participation");
    let mut attach = RenderSceneUpdate::new();
    attach.replace_participation(object_id, participation);
    store
        .commit(attach)
        .expect("attach R7 proof participation");

    let method = method(requirements);
    let plan = plan_render(&store.snapshot(), &request, std::slice::from_ref(&method))
        .expect("R7 proof plan");
    (plan, representation_id, object_id)
}

fn surface_requirement() -> RenderMethodRepresentationRequirement {
    RenderMethodRepresentationRequirement::new(
        RenderRepresentationProtocolRequirement::SurfaceQuery {
            revision: RENDER_SURFACE_QUERY_PROTOCOL_REVISION,
        },
        None,
    )
    .expect("surface requirement")
}

fn oriented_requirement() -> RenderMethodRepresentationRequirement {
    RenderMethodRepresentationRequirement::new(
        RenderRepresentationProtocolRequirement::OrientedSurfaceQuery {
            revision: RENDER_ORIENTED_SURFACE_QUERY_PROTOCOL_REVISION,
        },
        None,
    )
    .expect("oriented-surface requirement")
}

fn field_requirement() -> RenderMethodRepresentationRequirement {
    RenderMethodRepresentationRequirement::new(
        RenderRepresentationProtocolRequirement::FieldDistance {
            revision: RENDER_FIELD_DISTANCE_PROTOCOL_REVISION,
            input: RenderFieldDistanceInputRequirement::Exact,
        },
        None,
    )
    .expect("field requirement")
}

fn bound_surface_evidence() -> RenderSurfaceProtocolEvidence {
    RenderSurfaceProtocolEvidence::exact(RENDER_SURFACE_QUERY_PROTOCOL_REVISION)
        .expect("surface protocol")
        .with_oriented_surface(
            RenderOrientedSurfaceProtocolEvidence::exact(
                RENDER_ORIENTED_SURFACE_QUERY_PROTOCOL_REVISION,
            )
            .expect("oriented-surface protocol"),
        )
        .with_semantic_input_requirement(RenderSurfaceSemanticInputRequirement::current())
}

fn exact_field_evidence() -> RenderFieldDistanceProtocolEvidence {
    RenderFieldDistanceProtocolEvidence::new(
        RENDER_FIELD_DISTANCE_PROTOCOL_REVISION,
        RenderFieldDistanceGuarantee::exact(),
    )
    .expect("field protocol")
}

fn sphere(validity: RenderTemporalSupport) -> RenderSurfaceSemanticInput {
    RenderSurfaceSemanticInput::sphere([0.0; 3], 1.0, validity).expect("surface semantic input")
}

#[test]
fn surface_prerequisite_does_not_apply_to_field_use_on_same_representation() {
    let (plan, representation_id, _) = plan_for(
        request_at_times(&[0.0]),
        Some(bound_surface_evidence()),
        Some(exact_field_evidence()),
        vec![field_requirement()],
    );
    let inputs = RenderNormalizedSurfaceSemanticInputs::normalize(&plan, &[])
        .expect("field-only use needs no surface binding");
    let candidate = inputs
        .specialize_candidate(&plan, &plan.candidates()[0])
        .expect("field use remains semantically admissible");
    let uses = candidate.outputs()[0].objects()[0].uses();
    assert_eq!(uses.len(), 1);
    assert_eq!(uses[0].representation_id(), representation_id);
    assert_eq!(
        uses[0].requirement().protocol().protocol(),
        RenderRepresentationProtocol::FieldDistance
    );
}

#[test]
fn surface_and_oriented_uses_share_one_canonical_binding() {
    let (plan, representation_id, object_id) = plan_for(
        request_at_times(&[0.0]),
        Some(bound_surface_evidence()),
        None,
        vec![surface_requirement(), oriented_requirement()],
    );
    let binding = RenderSurfaceSemanticInputBinding::new(
        representation_id,
        sphere(RenderTemporalSupport::unbounded()),
    );
    let inputs = RenderNormalizedSurfaceSemanticInputs::normalize(&plan, &[binding])
        .expect("one canonical surface binding");
    let candidate = inputs
        .specialize_candidate(&plan, &plan.candidates()[0])
        .expect("both surface protocol alternatives remain admissible");
    let uses = candidate.outputs()[0].objects()[0].uses();
    assert_eq!(uses.len(), 2);
    let first = inputs
        .binding_for_selected_use(&plan, object_id, uses[0])
        .expect("first use binding");
    let second = inputs
        .binding_for_selected_use(&plan, object_id, uses[1])
        .expect("second use binding");
    assert!(std::ptr::eq(first, second));
    assert_eq!(first.representation_id(), representation_id);
}

#[test]
fn one_binding_must_cover_every_selected_observation_shutter() {
    let (plan, representation_id, _) = plan_for(
        request_at_times(&[0.0, 1.0]),
        Some(bound_surface_evidence()),
        None,
        vec![surface_requirement()],
    );
    let only_first = RenderNormalizedSurfaceSemanticInputs::normalize(
        &plan,
        &[RenderSurfaceSemanticInputBinding::new(
            representation_id,
            sphere(RenderTemporalSupport::interval(interval(0.0, 0.0))),
        )],
    )
    .expect("binding shape");
    assert!(matches!(
        only_first.specialize_candidate(&plan, &plan.candidates()[0]),
        Err(RenderSemanticCandidateRejection {
            output_index: 1,
            ..
        })
    ));

    let covers_both = RenderNormalizedSurfaceSemanticInputs::normalize(
        &plan,
        &[RenderSurfaceSemanticInputBinding::new(
            representation_id,
            sphere(RenderTemporalSupport::interval(interval(0.0, 1.0))),
        )],
    )
    .expect("binding shape");
    let candidate = covers_both
        .specialize_candidate(&plan, &plan.candidates()[0])
        .expect("one value valid for both shutters");
    assert_eq!(candidate.outputs().len(), 2);
}
