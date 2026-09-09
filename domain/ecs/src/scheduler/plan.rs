use crate::scheduler::access::AccessConflict;
use crate::scheduler::label::{ScheduleKey, ScheduleLabel, SystemSetKey};
use crate::scheduler::system::{RegisteredSystem, SystemId};
use crate::telemetry;
use std::collections::BTreeSet;
use std::time::Instant;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionConflict {
    pub first_system_id: SystemId,
    pub first_system: String,
    pub second_system_id: SystemId,
    pub second_system: String,
    pub conflict: AccessConflict,
}

/// One semantic ordering layer in an ECS schedule.
///
/// Stages are formed only from explicit before/after set constraints. Access
/// incompatibilities are recorded separately and never create stage boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStage {
    pub index: usize,
    pub system_indices: Vec<usize>,
    pub system_ids: Vec<SystemId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPlan {
    pub label: ScheduleKey,
    pub stages: Vec<ExecutionStage>,
    pub conflicts: Vec<ExecutionConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ScheduleValidationError {
    #[error("schedule '{schedule}' has cyclic system ordering constraints")]
    OrderingCycle { schedule: &'static str },
}

/// ECS-owned registry for deterministic schedule planning and serial execution.
///
/// This is deliberately World-specific. It is not a generic scheduler framework.
pub(crate) struct ScheduleRegistry {
    systems: Vec<RegisteredSystem>,
    plans: Vec<ExecutionPlan>,
    dirty: bool,
    next_system_id: u64,
}

impl Default for ScheduleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ScheduleRegistry {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            plans: Vec::new(),
            dirty: true,
            next_system_id: 0,
        }
    }

    pub fn add_system(&mut self, mut system: RegisteredSystem) -> usize {
        let system_id = SystemId::from_raw(self.next_system_id);
        self.next_system_id = self.next_system_id.saturating_add(1);
        system.assign_id(system_id);
        let index = self.systems.len();
        self.systems.push(system);
        self.dirty = true;
        index
    }

    pub fn systems(&self) -> &[RegisteredSystem] {
        &self.systems
    }

    pub fn systems_mut(&mut self) -> &mut [RegisteredSystem] {
        &mut self.systems
    }

    pub fn plan_for<L: ScheduleLabel>(
        &mut self,
    ) -> Result<Option<&ExecutionPlan>, ScheduleValidationError> {
        self.rebuild_if_dirty()?;
        Ok(self.plans.iter().find(|plan| plan.label == L::key()))
    }

    fn rebuild_if_dirty(&mut self) -> Result<(), ScheduleValidationError> {
        if !self.dirty {
            return Ok(());
        }
        let mut labels = Vec::<ScheduleKey>::new();
        for system in &self.systems {
            if !labels.iter().any(|label| *label == system.label()) {
                labels.push(system.label());
            }
        }
        self.plans = labels
            .into_iter()
            .map(|label| self.build_plan(label))
            .collect::<Result<Vec<_>, _>>()?;
        self.dirty = false;
        Ok(())
    }

    fn build_plan(&self, label: ScheduleKey) -> Result<ExecutionPlan, ScheduleValidationError> {
        let build_start = Instant::now();
        let mut conflict_check_count = 0_u64;
        let scheduled_indices = self
            .systems
            .iter()
            .enumerate()
            .filter_map(|(index, system)| (system.label() == label).then_some(index))
            .collect::<Vec<_>>();

        let mut outgoing = vec![BTreeSet::<usize>::new(); scheduled_indices.len()];
        let mut incoming = vec![0usize; scheduled_indices.len()];
        for (source_pos, source_index) in scheduled_indices.iter().enumerate() {
            let source = &self.systems[*source_index];
            for (target_pos, target_index) in scheduled_indices.iter().enumerate() {
                if source_pos == target_pos {
                    continue;
                }
                let target = &self.systems[*target_index];
                if depends_on_set(source.after_sets(), target.sets())
                    && outgoing[target_pos].insert(source_pos)
                {
                    incoming[source_pos] = incoming[source_pos].saturating_add(1);
                }
                if depends_on_set(source.before_sets(), target.sets())
                    && outgoing[source_pos].insert(target_pos)
                {
                    incoming[target_pos] = incoming[target_pos].saturating_add(1);
                }
            }
        }

        let mut ready = BTreeSet::new();
        for (position, indegree) in incoming.iter().enumerate() {
            if *indegree == 0 {
                ready.insert(position);
            }
        }

        let mut stages = Vec::new();
        let mut scheduled_count = 0usize;
        while !ready.is_empty() {
            let stage_positions = ready.iter().copied().collect::<Vec<_>>();
            ready.clear();

            let mut system_indices = Vec::with_capacity(stage_positions.len());
            let mut system_ids = Vec::with_capacity(stage_positions.len());
            for position in &stage_positions {
                let system_index = scheduled_indices[*position];
                system_indices.push(system_index);
                system_ids.push(self.systems[system_index].id());
            }
            scheduled_count = scheduled_count.saturating_add(stage_positions.len());

            for position in stage_positions {
                for dependent in outgoing[position].iter().copied() {
                    incoming[dependent] = incoming[dependent].saturating_sub(1);
                    if incoming[dependent] == 0 {
                        ready.insert(dependent);
                    }
                }
            }

            stages.push(ExecutionStage {
                index: stages.len(),
                system_indices,
                system_ids,
            });
        }

        if scheduled_count != scheduled_indices.len() {
            return Err(ScheduleValidationError::OrderingCycle {
                schedule: label.name(),
            });
        }

        let mut conflicts = Vec::new();
        for (left_pos, left_index) in scheduled_indices.iter().enumerate() {
            let left = &self.systems[*left_index];
            for right_index in scheduled_indices.iter().skip(left_pos + 1) {
                let right = &self.systems[*right_index];
                conflict_check_count = conflict_check_count.saturating_add(1);
                for conflict in left.access().conflicts_with(right.access()) {
                    conflicts.push(ExecutionConflict {
                        first_system_id: left.id(),
                        first_system: left.name().to_string(),
                        second_system_id: right.id(),
                        second_system: right.name().to_string(),
                        conflict,
                    });
                }
            }
        }

        let plan = ExecutionPlan {
            label,
            stages,
            conflicts,
        };
        telemetry::record_schedule_plan_build(
            build_start.elapsed().as_nanos() as u64,
            conflict_check_count,
            plan.stages.len() as u64,
        );
        Ok(plan)
    }
}

fn depends_on_set(required_sets: &[SystemSetKey], assigned_sets: &[SystemSetKey]) -> bool {
    required_sets
        .iter()
        .any(|required| assigned_sets.iter().any(|assigned| assigned == required))
}
