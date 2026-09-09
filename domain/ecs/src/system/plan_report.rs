use crate::scheduler::access::{AccessDomain, ConflictKind};
use crate::scheduler::plan::ExecutionPlan;
use crate::scheduler::system::{RegisteredSystem, SystemId};

use super::param_metadata::{ParamSlotMetadata, param_slot_metadata_for_descriptors};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePlanReport {
    pub schedule_label: &'static str,
    pub stages: Vec<RuntimePlanStageReport>,
    pub conflicts: Vec<RuntimePlanConflictReport>,
}

impl RuntimePlanReport {
    pub(crate) fn from_plan(plan: &ExecutionPlan, systems: &[RegisteredSystem]) -> Self {
        Self {
            schedule_label: plan.label.name(),
            stages: plan
                .stages
                .iter()
                .map(|stage| RuntimePlanStageReport {
                    index: stage.index,
                    systems: system_reports_for_indices(systems, &stage.system_indices),
                    missing_system_indices: missing_system_indices(systems, &stage.system_indices),
                })
                .collect(),
            conflicts: plan
                .conflicts
                .iter()
                .map(|conflict| RuntimePlanConflictReport {
                    first_system_id: conflict.first_system_id,
                    first_system: conflict.first_system.clone(),
                    second_system_id: conflict.second_system_id,
                    second_system: conflict.second_system.clone(),
                    access_domain: conflict.conflict.key.domain(),
                    access_name: conflict.conflict.key.name(),
                    conflict_kind: conflict.conflict.kind,
                    message: conflict.conflict.diagnostic_message(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePlanStageReport {
    pub index: usize,
    pub systems: Vec<RuntimePlanSystemReport>,
    pub missing_system_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePlanSystemReport {
    pub system_index: usize,
    pub system_id: SystemId,
    pub name: String,
    pub param_slots: Vec<ParamSlotMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePlanConflictReport {
    pub first_system_id: SystemId,
    pub first_system: String,
    pub second_system_id: SystemId,
    pub second_system: String,
    pub access_domain: AccessDomain,
    pub access_name: &'static str,
    pub conflict_kind: ConflictKind,
    pub message: String,
}

fn system_reports_for_indices(
    systems: &[RegisteredSystem],
    indices: &[usize],
) -> Vec<RuntimePlanSystemReport> {
    indices
        .iter()
        .filter_map(|system_index| system_report_for_index(systems, *system_index))
        .collect()
}

fn system_report_for_index(
    systems: &[RegisteredSystem],
    system_index: usize,
) -> Option<RuntimePlanSystemReport> {
    let system = systems.get(system_index)?;
    let system_id = system.id();
    Some(RuntimePlanSystemReport {
        system_index,
        system_id,
        name: system.name().to_string(),
        param_slots: param_slot_metadata_for_descriptors(system_id, system.param_slots()),
    })
}

fn missing_system_indices(systems: &[RegisteredSystem], indices: &[usize]) -> Vec<usize> {
    indices
        .iter()
        .copied()
        .filter(|system_index| systems.get(*system_index).is_none())
        .collect()
}
