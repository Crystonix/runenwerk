//! Owner-controlled semantic planning and R5 admission for the maintained deterministic renderer.
//!
//! This boundary prevents a caller-created `RenderMethodContract` from becoming maintained
//! execution authority merely by reusing the renderer-local method ID. The exact method contract is
//! selected inside RunenRender, while the resulting ordinary `RenderPlan`/`AdmittedRenderPlan`
//! remain inspectable through their existing read-only APIs.

use super::admission::{
    AdmittedRenderPlan, RenderExecutionAdmissionFailure, RenderOutputBinding,
    RenderRepresentationAvailabilityFact, admit_render_plan_with_surface_inputs,
};
use super::maintained_method::maintained_deterministic_method;
use super::request::RenderRequest;
use super::scene::RenderSceneSnapshot;
use super::semantic_plan::{RenderPlanningFailure, plan_render};
use super::surface_input::RenderSurfaceSemanticInputBinding;
use runen_gpu::GpuContext;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderDeterministicAdmissionFailure {
    Planning(RenderPlanningFailure),
    Admission(RenderExecutionAdmissionFailure),
}

impl fmt::Display for RenderDeterministicAdmissionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Planning(error) => write!(formatter, "deterministic render planning failed: {error}"),
            Self::Admission(error) => write!(formatter, "deterministic render admission failed: {error}"),
        }
    }
}

impl Error for RenderDeterministicAdmissionFailure {}

/// R5-admitted work for the exact RunenRender-owned maintained deterministic method.
///
/// Construction is intentionally owner-controlled. The wrapper is not an execution/session ID and
/// carries no GPU submission lifecycle; it only proves that the enclosed admission came from the
/// maintained method authority rather than a same-ID caller contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedDeterministicRender {
    admitted: AdmittedRenderPlan,
}

impl AdmittedDeterministicRender {
    pub const fn admitted(&self) -> &AdmittedRenderPlan {
        &self.admitted
    }
}

pub fn admit_deterministic_render(
    scene: &RenderSceneSnapshot,
    request: &RenderRequest,
    semantic_inputs: &[RenderSurfaceSemanticInputBinding],
    availability: &[RenderRepresentationAvailabilityFact],
    output_bindings: &[RenderOutputBinding],
    context: &GpuContext,
) -> Result<AdmittedDeterministicRender, RenderDeterministicAdmissionFailure> {
    let method = maintained_deterministic_method();
    let plan = plan_render(scene, request, std::slice::from_ref(&method))
        .map_err(RenderDeterministicAdmissionFailure::Planning)?;
    let admitted = admit_render_plan_with_surface_inputs(
        &plan,
        semantic_inputs,
        availability,
        output_bindings,
        context,
    )
    .map_err(RenderDeterministicAdmissionFailure::Admission)?;
    Ok(AdmittedDeterministicRender { admitted })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::method::{
        RenderAbstractExecutionRequirement, RenderMethodContract, RenderMethodId,
        RenderMethodOutputContract, RenderMethodOutputGuarantee, RenderMethodOutputKind,
        RenderObservationKind, RenderSpectralRadianceSupport,
    };
    use crate::plugins::render::request::RenderDistanceConvention;

    #[test]
    fn same_method_id_is_not_the_maintained_method_authority() {
        let spectral = RenderSpectralRadianceSupport::new(500e-9, 600e-9).expect("range");
        let impostor = RenderMethodContract::new(
            RenderMethodId::new(1).expect("id"),
            vec![
                RenderMethodOutputContract::new(
                    RenderObservationKind::Probe,
                    RenderMethodOutputKind::Radiance { spectral },
                    RenderMethodOutputGuarantee::Exact,
                    vec![],
                    false,
                )
                .expect("output"),
            ],
            vec![RenderAbstractExecutionRequirement::GeneralParallelWork],
        )
        .expect("impostor contract");
        let maintained = maintained_deterministic_method();
        assert_eq!(impostor.id(), maintained.id());
        assert_ne!(impostor, maintained);
        assert!(maintained.output_contracts().iter().any(|contract| matches!(
            contract.output_kind(),
            RenderMethodOutputKind::Distance {
                convention: RenderDistanceConvention::ObservationForwardDepth
            }
        )));
    }
}
