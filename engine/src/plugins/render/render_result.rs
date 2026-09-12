use super::admission::AdmittedRenderPlan;
use super::method::RenderMethodId;
use super::request::RenderRequest;
use super::scene::{RenderObjectId, RenderSceneRevision, RenderSceneSnapshot};
use super::semantic_plan::{RenderApplicableRepresentationUse, RenderOutputApproximation};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

/// Renderer-owned evidence that one deterministic finite output satisfied the requested tolerance.
///
/// This is intentionally not public product API. A RunenRender method/evaluator may construct this
/// witness only after its concrete finite evaluation has established the requested
/// `RenderSemanticTolerance` for the correlated admitted output. Physical completion, numeric
/// format, or semantic/model approximation alone are not sufficient evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RenderDeterministicOutputFormationEvidence {
    output_index: usize,
}

impl RenderDeterministicOutputFormationEvidence {
    pub(super) const fn requested_tolerance_satisfied(output_index: usize) -> Self {
        Self { output_index }
    }

    const fn output_index(self) -> usize {
        self.output_index
    }
}

/// Immutable renderer-semantic representation provenance retained for one completed output.
///
/// Physical bindings, GPU execution evidence, readback identities, and output bytes deliberately do
/// not participate in this semantic result evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderResultObjectRepresentation {
    object_id: RenderObjectId,
    representation: RenderApplicableRepresentationUse,
}

impl RenderResultObjectRepresentation {
    pub(crate) const fn object_id(&self) -> RenderObjectId {
        self.object_id
    }

    pub(crate) const fn representation(&self) -> RenderApplicableRepresentationUse {
        self.representation
    }
}

/// Semantic evidence for one successfully formed requested output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderResultOutputEvidence {
    output_index: usize,
    observation_index: usize,
    approximation: RenderOutputApproximation,
    object_representations: Vec<RenderResultObjectRepresentation>,
}

impl RenderResultOutputEvidence {
    pub(crate) const fn output_index(&self) -> usize {
        self.output_index
    }

    pub(crate) const fn observation_index(&self) -> usize {
        self.observation_index
    }

    pub(crate) const fn approximation(&self) -> RenderOutputApproximation {
        self.approximation
    }

    pub(crate) fn object_representations(&self) -> &[RenderResultObjectRepresentation] {
        &self.object_representations
    }
}

/// Complete RunenRender semantic outcome evidence for one admitted execution.
///
/// This is intentionally not a value container. Output values may remain in physical bindings,
/// retained renderer products, readback results, or presentation destinations. The result retains
/// only immutable semantic provenance projected from the exact admitted plan that produced the work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderResult {
    scene: RenderSceneSnapshot,
    request: RenderRequest,
    method_id: RenderMethodId,
    outputs: Vec<RenderResultOutputEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RenderResultFormationError {
    OutputOutOfRange {
        output_index: usize,
        output_count: usize,
    },
    DuplicateOutput {
        output_index: usize,
    },
    MissingOutput {
        output_index: usize,
    },
}

impl fmt::Display for RenderResultFormationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputOutOfRange {
                output_index,
                output_count,
            } => write!(
                formatter,
                "formed render output {output_index} is outside {output_count} admitted outputs"
            ),
            Self::DuplicateOutput { output_index } => {
                write!(
                    formatter,
                    "render output {output_index} was formed more than once"
                )
            }
            Self::MissingOutput { output_index } => {
                write!(formatter, "render output {output_index} was not formed")
            }
        }
    }
}

impl Error for RenderResultFormationError {}

impl RenderResult {
    /// Form complete semantic result evidence for one deterministic admitted execution.
    ///
    /// Every admitted output requires one renderer-owned witness that its concrete finite value has
    /// satisfied the requested deterministic tolerance. This constructor deliberately does not
    /// depend on RunenGPU submission/readback types and does not infer evaluation fidelity from GPU
    /// completion, numeric format, or the admitted semantic/model approximation.
    pub(super) fn complete_deterministic(
        admitted: &AdmittedRenderPlan,
        output_evidence: impl IntoIterator<Item = RenderDeterministicOutputFormationEvidence>,
    ) -> Result<Self, RenderResultFormationError> {
        let output_count = admitted.outputs().len();
        let mut formed = BTreeSet::new();
        for evidence in output_evidence {
            let output_index = evidence.output_index();
            if output_index >= output_count {
                return Err(RenderResultFormationError::OutputOutOfRange {
                    output_index,
                    output_count,
                });
            }
            if !formed.insert(output_index) {
                return Err(RenderResultFormationError::DuplicateOutput { output_index });
            }
        }

        for output in admitted.outputs() {
            let output_index = output.output_index();
            if !formed.contains(&output_index) {
                return Err(RenderResultFormationError::MissingOutput { output_index });
            }
        }

        let outputs = admitted
            .outputs()
            .iter()
            .map(|output| RenderResultOutputEvidence {
                output_index: output.output_index(),
                observation_index: output.observation_index(),
                approximation: output.approximation(),
                object_representations: output
                    .object_representations()
                    .iter()
                    .map(|object| RenderResultObjectRepresentation {
                        object_id: object.object_id(),
                        representation: object.representation(),
                    })
                    .collect(),
            })
            .collect();

        Ok(Self {
            scene: admitted.plan().scene().clone(),
            request: admitted.plan().request().clone(),
            method_id: admitted.selected_candidate().method_id(),
            outputs,
        })
    }

    pub(crate) const fn scene_revision(&self) -> RenderSceneRevision {
        self.scene.revision()
    }

    pub(crate) const fn scene(&self) -> &RenderSceneSnapshot {
        &self.scene
    }

    pub(crate) const fn request(&self) -> &RenderRequest {
        &self.request
    }

    pub(crate) const fn method_id(&self) -> RenderMethodId {
        self.method_id
    }

    pub(crate) fn outputs(&self) -> &[RenderResultOutputEvidence] {
        &self.outputs
    }
}
