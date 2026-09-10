use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use product::{ProductRatificationReport, RenderProductSelection, ratify_render_product_selection};

use crate::plugins::render::RenderFrameProducerId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRenderProductSelectionDiagnostic {
    pub producer_id: RenderFrameProducerId,
    pub view_id: String,
    pub message: String,
    pub issue_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedRenderProductSelectionError {
    DuplicateView {
        producer_id: RenderFrameProducerId,
        view_id: String,
    },
    DuplicateViewAcrossProducers {
        existing_producer_id: RenderFrameProducerId,
        replacement_producer_id: RenderFrameProducerId,
        view_id: String,
    },
    InvalidSelection {
        producer_id: RenderFrameProducerId,
        view_id: String,
        report: ProductRatificationReport,
    },
}

impl fmt::Display for PreparedRenderProductSelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateView {
                producer_id,
                view_id,
            } => write!(
                f,
                "render product selection producer '{producer_id:?}' published duplicate view '{view_id}'"
            ),
            Self::DuplicateViewAcrossProducers {
                existing_producer_id,
                replacement_producer_id,
                view_id,
            } => write!(
                f,
                "render product selection producer '{replacement_producer_id:?}' published view '{view_id}' already owned by producer '{existing_producer_id:?}'"
            ),
            Self::InvalidSelection {
                producer_id,
                view_id,
                report,
            } => write!(
                f,
                "render product selection producer '{producer_id:?}' published invalid view '{view_id}' with {} issue(s)",
                report.len()
            ),
        }
    }
}

impl std::error::Error for PreparedRenderProductSelectionError {}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct PreparedRenderProductSelectionResource {
    contributions: BTreeMap<RenderFrameProducerId, Vec<RenderProductSelection>>,
    diagnostics: Vec<PreparedRenderProductSelectionDiagnostic>,
}

impl PreparedRenderProductSelectionResource {
    pub fn replace_contribution(
        &mut self,
        producer_id: impl Into<RenderFrameProducerId>,
        selections: impl IntoIterator<Item = RenderProductSelection>,
    ) -> Result<Vec<RenderProductSelection>, PreparedRenderProductSelectionError> {
        let producer_id = producer_id.into();
        self.diagnostics
            .retain(|diagnostic| diagnostic.producer_id != producer_id);

        let mut selections = selections.into_iter().collect::<Vec<_>>();
        let mut view_ids = BTreeSet::<String>::new();
        for selection in &selections {
            if !view_ids.insert(selection.view_id.clone()) {
                let err = PreparedRenderProductSelectionError::DuplicateView {
                    producer_id,
                    view_id: selection.view_id.clone(),
                };
                self.diagnostics.push(diagnostic_for_error(
                    producer_id,
                    selection.view_id.clone(),
                    &err,
                ));
                return Err(err);
            }

            let report = ratify_render_product_selection(selection);
            if report.has_blocking_issues() {
                let err = PreparedRenderProductSelectionError::InvalidSelection {
                    producer_id,
                    view_id: selection.view_id.clone(),
                    report,
                };
                self.diagnostics.push(diagnostic_for_error(
                    producer_id,
                    selection.view_id.clone(),
                    &err,
                ));
                return Err(err);
            }
        }

        for (existing_producer_id, existing_selections) in &self.contributions {
            if *existing_producer_id == producer_id {
                continue;
            }
            for selection in &selections {
                if existing_selections
                    .iter()
                    .any(|existing| existing.view_id == selection.view_id)
                {
                    let err = PreparedRenderProductSelectionError::DuplicateViewAcrossProducers {
                        existing_producer_id: *existing_producer_id,
                        replacement_producer_id: producer_id,
                        view_id: selection.view_id.clone(),
                    };
                    self.diagnostics.push(diagnostic_for_error(
                        producer_id,
                        selection.view_id.clone(),
                        &err,
                    ));
                    return Err(err);
                }
            }
        }

        selections.sort_by(|left, right| left.view_id.cmp(&right.view_id));
        Ok(self
            .contributions
            .insert(producer_id, selections)
            .unwrap_or_default())
    }

    pub fn remove_contribution(
        &mut self,
        producer_id: impl Into<RenderFrameProducerId>,
    ) -> Option<Vec<RenderProductSelection>> {
        let producer_id = producer_id.into();
        self.diagnostics
            .retain(|diagnostic| diagnostic.producer_id != producer_id);
        self.contributions.remove(&producer_id)
    }

    pub fn clear(&mut self) {
        self.contributions.clear();
        self.diagnostics.clear();
    }

    pub fn contribution(
        &self,
        producer_id: impl Into<RenderFrameProducerId>,
    ) -> Option<&[RenderProductSelection]> {
        self.contributions
            .get(&producer_id.into())
            .map(Vec::as_slice)
    }

    pub fn diagnostics(&self) -> &[PreparedRenderProductSelectionDiagnostic] {
        &self.diagnostics
    }

    pub fn snapshot(&self) -> Vec<RenderProductSelection> {
        let mut snapshot = self
            .contributions
            .values()
            .flat_map(|selections| selections.iter().cloned())
            .collect::<Vec<_>>();
        snapshot.sort_by(|left, right| left.view_id.cmp(&right.view_id));
        snapshot
    }
}

fn diagnostic_for_error(
    producer_id: RenderFrameProducerId,
    view_id: String,
    err: &PreparedRenderProductSelectionError,
) -> PreparedRenderProductSelectionDiagnostic {
    let issue_count = match err {
        PreparedRenderProductSelectionError::DuplicateView { .. }
        | PreparedRenderProductSelectionError::DuplicateViewAcrossProducers { .. } => 1,
        PreparedRenderProductSelectionError::InvalidSelection { report, .. } => report.len(),
    };
    PreparedRenderProductSelectionDiagnostic {
        producer_id,
        view_id,
        message: err.to_string(),
        issue_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use product::{
        ProductAuthorityClass, ProductFreshness, ProductIdentity, ProductQueryPolicy,
        ProductResidency, ProductScaleBand, RenderSelectedProduct,
    };

    fn producer(raw: u64) -> RenderFrameProducerId {
        RenderFrameProducerId::try_from_raw(raw).expect("producer id should be non-zero")
    }

    fn selection(view_id: &str, product_id: u64, generation: u64) -> RenderProductSelection {
        RenderProductSelection::new(view_id).with_selected_product(RenderSelectedProduct {
            product_id: ProductIdentity::new(product_id),
            scale_band: ProductScaleBand::Preview,
            generation,
            freshness: ProductFreshness::Current,
            residency: ProductResidency::Resident,
            authority_class: ProductAuthorityClass::DeterministicDerived,
            query_policy: ProductQueryPolicy::StrictCurrentOnly,
        })
    }

    #[test]
    fn render_product_selection_contributions_snapshot_deterministically() {
        let mut resource = PreparedRenderProductSelectionResource::default();

        resource
            .replace_contribution(producer(2), [selection("a", 42, 9)])
            .expect("selection should be valid");
        resource
            .replace_contribution(producer(1), [selection("b", 41, 8)])
            .expect("selection should be valid");

        let snapshot = resource.snapshot();

        assert_eq!(snapshot[0].view_id, "a");
        assert_eq!(snapshot[1].view_id, "b");
        assert_eq!(snapshot[0].selected_products[0].generation, 9);
    }

    #[test]
    fn render_product_selection_rejects_duplicate_view_across_producers() {
        let mut resource = PreparedRenderProductSelectionResource::default();
        resource
            .replace_contribution(producer(1), [selection("main", 42, 9)])
            .expect("initial selection should be valid");

        let err = resource
            .replace_contribution(producer(2), [selection("main", 43, 1)])
            .expect_err("one render view must have one product selection producer");

        assert!(matches!(
            err,
            PreparedRenderProductSelectionError::DuplicateViewAcrossProducers { .. }
        ));
        assert_eq!(resource.snapshot().len(), 1);
        assert_eq!(resource.diagnostics().len(), 1);
    }

    #[test]
    fn render_product_selection_rejects_invalid_contributions_with_diagnostics() {
        let mut invalid = selection("main", 42, 9);
        invalid.selected_products[0].freshness = ProductFreshness::Stale;
        let mut resource = PreparedRenderProductSelectionResource::default();

        let err = resource
            .replace_contribution(producer(1), [invalid])
            .expect_err("invalid strict selection should be rejected");

        assert!(matches!(
            err,
            PreparedRenderProductSelectionError::InvalidSelection { .. }
        ));
        assert_eq!(resource.snapshot().len(), 0);
        assert_eq!(resource.diagnostics().len(), 1);
    }
}
