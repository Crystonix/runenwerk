use super::{
    RenderFragmentDiagnostic, RenderFragmentDiagnosticReport, RenderFragmentMergeReport,
    RenderFragmentPackageDescriptor, RenderFragmentPackageId, RenderFragmentRevision,
    validate_fragment_package,
};
use crate::plugins::render::RenderFlow;
use crate::plugins::render::graph::{RenderFragmentMergeError, merge_fragment_package_into_flow};
use runen_gpu::GpuCapabilities;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderFragmentPackageStatus {
    Active,
    Failed,
    Disabled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderFragmentPackageRecord {
    pub package_id: RenderFragmentPackageId,
    pub status: RenderFragmentPackageStatus,
    pub active_revision: Option<RenderFragmentRevision>,
    pub last_good_revision: Option<RenderFragmentRevision>,
    pub failed_revision: Option<RenderFragmentRevision>,
    pub active_package: Option<RenderFragmentPackageDescriptor>,
    pub diagnostics: Vec<RenderFragmentDiagnostic>,
}

impl RenderFragmentPackageRecord {
    pub fn active(package: RenderFragmentPackageDescriptor) -> Self {
        let revision = RenderFragmentRevision(package.source_revision);
        Self {
            package_id: package.package_id.clone(),
            status: RenderFragmentPackageStatus::Active,
            active_revision: Some(revision),
            last_good_revision: Some(revision),
            failed_revision: None,
            active_package: Some(package),
            diagnostics: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderFragmentReloadOutcome {
    pub package_id: RenderFragmentPackageId,
    pub status: RenderFragmentPackageStatus,
    pub active_revision: Option<RenderFragmentRevision>,
    pub last_good_revision: Option<RenderFragmentRevision>,
    pub failed_revision: Option<RenderFragmentRevision>,
    pub diagnostics: Vec<RenderFragmentDiagnostic>,
}

#[derive(Debug, Default, ecs::Resource)]
pub struct RenderFragmentRegistryResource {
    packages: BTreeMap<RenderFragmentPackageId, RenderFragmentPackageRecord>,
    revision: u64,
}

impl RenderFragmentRegistryResource {
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    pub fn record(
        &self,
        package_id: &RenderFragmentPackageId,
    ) -> Option<&RenderFragmentPackageRecord> {
        self.packages.get(package_id)
    }

    pub fn active_packages(&self) -> Vec<&RenderFragmentPackageDescriptor> {
        self.packages
            .values()
            .filter_map(|record| {
                if record.status != RenderFragmentPackageStatus::Disabled {
                    record.active_package.as_ref()
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn apply_package(
        &mut self,
        package: RenderFragmentPackageDescriptor,
    ) -> RenderFragmentReloadOutcome {
        let report = validate_fragment_package(&package);
        if report.is_ok() {
            let package_id = package.package_id.clone();
            let record = RenderFragmentPackageRecord::active(package);
            let outcome = outcome_from_record(&record);
            self.packages.insert(package_id, record);
            self.bump_revision();
            return outcome;
        }

        let package_id = package.package_id.clone();
        let failed_revision = Some(RenderFragmentRevision(package.source_revision));
        let diagnostics = report.diagnostics;
        let record = match self.packages.remove(&package_id) {
            Some(mut existing) => {
                existing.status = RenderFragmentPackageStatus::Failed;
                existing.failed_revision = failed_revision;
                existing.diagnostics = diagnostics;
                existing
            }
            None => RenderFragmentPackageRecord {
                package_id: package_id.clone(),
                status: RenderFragmentPackageStatus::Failed,
                active_revision: None,
                last_good_revision: None,
                failed_revision,
                active_package: None,
                diagnostics,
            },
        };
        let outcome = outcome_from_record(&record);
        self.packages.insert(package_id, record);
        self.bump_revision();
        outcome
    }

    pub fn disable_package(
        &mut self,
        package_id: &RenderFragmentPackageId,
    ) -> Option<RenderFragmentReloadOutcome> {
        let record = self.packages.get_mut(package_id)?;
        record.status = RenderFragmentPackageStatus::Disabled;
        let outcome = outcome_from_record(record);
        self.bump_revision();
        Some(outcome)
    }

    pub fn diagnostics(&self) -> RenderFragmentDiagnosticReport {
        RenderFragmentDiagnosticReport::new(
            self.packages
                .values()
                .flat_map(|record| record.diagnostics.clone())
                .collect(),
        )
    }

    pub fn merge_active_packages(
        &self,
        mut flow: RenderFlow,
        capabilities: &GpuCapabilities,
    ) -> Result<RenderFragmentRegistryMerge, RenderFragmentMergeError> {
        let mut reports = Vec::<RenderFragmentMergeReport>::new();
        for package in self.active_packages() {
            let merged = merge_fragment_package_into_flow(flow, package, capabilities)?;
            flow = merged.flow;
            reports.push(merged.report);
        }
        Ok(RenderFragmentRegistryMerge { flow, reports })
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }
}

#[derive(Debug)]
pub struct RenderFragmentRegistryMerge {
    pub flow: RenderFlow,
    pub reports: Vec<RenderFragmentMergeReport>,
}

fn outcome_from_record(record: &RenderFragmentPackageRecord) -> RenderFragmentReloadOutcome {
    RenderFragmentReloadOutcome {
        package_id: record.package_id.clone(),
        status: record.status,
        active_revision: record.active_revision,
        last_good_revision: record.last_good_revision,
        failed_revision: record.failed_revision,
        diagnostics: record.diagnostics.clone(),
    }
}
