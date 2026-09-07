use super::*;
use crate::plugins::render::{RenderGpuWorkOccurrenceId, RenderPassId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
pub(super) struct ScheduledFixedStepIteration<'a> {
    pub(super) region: &'a CompiledFixedStepRegion,
    pub(super) schedule: RenderFixedStepIterationUniform,
    pub(super) substep_index: u32,
}

#[derive(Clone)]
pub(super) struct ExpandedRenderPassOccurrence<'a> {
    pub(super) occurrence_id: RenderGpuWorkOccurrenceId,
    pub(super) pass: &'a CompiledPassExecutionPlan,
    pub(super) fixed_step_iteration: Option<ScheduledFixedStepIteration<'a>>,
    pub(super) control_order_after: Vec<RenderGpuWorkOccurrenceId>,
}

/// Test-local per-invocation convenience wrapper over the frame-owned occurrence allocator.
#[cfg(test)]
fn expand_render_pass_occurrences<'a, F>(
    flow: &'a CompiledRenderFlowPlan,
    flow_inputs: &'a PreparedFlowInputs,
    include_pass: F,
) -> Result<Vec<ExpandedRenderPassOccurrence<'a>>>
where
    F: FnMut(&CompiledPassExecutionPlan) -> Result<bool>,
{
    let mut maximum_occurrence = 0_u64;
    expand_render_pass_occurrences_in_frame(
        flow,
        flow_inputs,
        &mut maximum_occurrence,
        include_pass,
    )
}

/// Expands one prepared render invocation into the pass executions that can become canonical GPU
/// work while allocating occurrence identity from one frame-owned monotonic space.
///
/// Lexical pass order is used only to locate contiguous fixed-step regions and to lift an authored
/// `order_after` request onto the latest actual predecessor occurrence. Arbitrary lexical adjacency
/// outside fixed-step regions never becomes execution order. The caller must retain the same
/// `maximum_occurrence` across every invocation that can enter one bounded frame GPU work graph.
pub(super) fn expand_render_pass_occurrences_in_frame<'a, F>(
    flow: &'a CompiledRenderFlowPlan,
    flow_inputs: &'a PreparedFlowInputs,
    maximum_occurrence: &mut u64,
    mut include_pass: F,
) -> Result<Vec<ExpandedRenderPassOccurrence<'a>>>
where
    F: FnMut(&CompiledPassExecutionPlan) -> Result<bool>,
{
    let included = flow
        .execution
        .passes
        .iter()
        .map(|pass| Ok((execution_pass_id(pass), include_pass(pass)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let passes_by_id = flow
        .execution
        .passes
        .iter()
        .map(|pass| (execution_pass_id(pass), pass))
        .collect::<BTreeMap<_, _>>();

    let mut consumed_region_passes = BTreeSet::<RenderPassId>::new();
    let mut occurrences = Vec::<ExpandedRenderPassOccurrence<'a>>::new();

    for pass in &flow.execution.passes {
        let pass_id = execution_pass_id(pass);
        if consumed_region_passes.contains(&pass_id) {
            continue;
        }

        if let Some(region) = fixed_step_region_starting_at(flow, pass_id) {
            let schedule = fixed_step_schedule_for_region(region, flow_inputs)?;
            let mut previous_region_occurrence = None;
            for substep_index in 0..schedule.submitted_substeps {
                for region_pass_id in &region.pass_ids {
                    let region_pass =
                        passes_by_id.get(region_pass_id).copied().ok_or_else(|| {
                            anyhow::anyhow!(
                                "fixed-step region '{}' references missing compiled pass '{}'",
                                region.region_label,
                                region_pass_id
                            )
                        })?;
                    if !included.get(region_pass_id).copied().unwrap_or(false) {
                        continue;
                    }
                    let occurrence_id = allocate_occurrence_id(maximum_occurrence)?;
                    let control_order_after = previous_region_occurrence.into_iter().collect();
                    occurrences.push(ExpandedRenderPassOccurrence {
                        occurrence_id,
                        pass: region_pass,
                        fixed_step_iteration: Some(ScheduledFixedStepIteration {
                            region,
                            schedule,
                            substep_index,
                        }),
                        control_order_after,
                    });
                    previous_region_occurrence = Some(occurrence_id);
                }
            }
            consumed_region_passes.extend(region.pass_ids.iter().copied());
            continue;
        }

        if included.get(&pass_id).copied().unwrap_or(false) {
            occurrences.push(ExpandedRenderPassOccurrence {
                occurrence_id: allocate_occurrence_id(maximum_occurrence)?,
                pass,
                fixed_step_iteration: None,
                control_order_after: Vec::new(),
            });
        }
    }

    lift_explicit_non_data_orders(flow, &mut occurrences);
    Ok(occurrences)
}

fn allocate_occurrence_id(maximum: &mut u64) -> Result<RenderGpuWorkOccurrenceId> {
    *maximum = maximum.checked_add(1).ok_or_else(|| {
        anyhow::anyhow!("render GPU execution occurrence identity space is exhausted")
    })?;
    Ok(RenderGpuWorkOccurrenceId::new(*maximum))
}

fn lift_explicit_non_data_orders(
    flow: &CompiledRenderFlowPlan,
    occurrences: &mut [ExpandedRenderPassOccurrence<'_>],
) {
    let authored_orders = flow
        .render_passes
        .iter()
        .map(|pass| (pass.pass_id(), pass.node().non_data_order_after.to_vec()))
        .collect::<BTreeMap<_, _>>();
    let occurrence_pass_ids = occurrences
        .iter()
        .map(|occurrence| execution_pass_id(occurrence.pass))
        .collect::<Vec<_>>();
    let occurrence_ids = occurrences
        .iter()
        .map(|occurrence| occurrence.occurrence_id)
        .collect::<Vec<_>>();

    for index in 0..occurrences.len() {
        let pass_id = occurrence_pass_ids[index];
        let Some(predecessor_passes) = authored_orders.get(&pass_id) else {
            continue;
        };
        for predecessor_pass in predecessor_passes {
            let predecessor = (0..index)
                .rev()
                .find(|candidate| occurrence_pass_ids[*candidate] == *predecessor_pass)
                .map(|candidate| occurrence_ids[candidate]);
            let Some(predecessor) = predecessor else {
                // A predecessor omitted by view/feature/fixed-step runtime control has no GPU
                // occurrence to order after. The authored requirement is vacuously satisfied.
                continue;
            };
            if !occurrences[index]
                .control_order_after
                .contains(&predecessor)
            {
                occurrences[index].control_order_after.push(predecessor);
            }
        }
    }
}

fn fixed_step_region_starting_at(
    flow: &CompiledRenderFlowPlan,
    pass_id: RenderPassId,
) -> Option<&CompiledFixedStepRegion> {
    flow.execution
        .fixed_step_regions
        .iter()
        .find(|region| region.pass_ids.first().copied() == Some(pass_id))
}

fn fixed_step_schedule_for_region(
    region: &CompiledFixedStepRegion,
    flow_inputs: &PreparedFlowInputs,
) -> Result<RenderFixedStepIterationUniform> {
    let bytes = flow_inputs
        .projected_uniform_bytes
        .get(&region.iteration_uniform)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "fixed-step region '{}' is missing prepared iteration uniform '{}'",
                region.region_label,
                region.iteration_uniform
            )
        })?;
    let mut schedule =
        RenderFixedStepIterationUniform::from_uniform_bytes(bytes).ok_or_else(|| {
            anyhow::anyhow!(
                "fixed-step region '{}' prepared iteration uniform '{}' has invalid byte shape",
                region.region_label,
                region.iteration_uniform
            )
        })?;
    schedule.submitted_substeps = schedule.submitted_substeps.min(region.max_substeps);
    schedule.max_substeps = region.max_substeps;
    Ok(schedule)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::{GpuStorage, RenderFlow, compile_flow_plan};
    use runen_gpu::GpuBindingKey;

    #[derive(Debug, Clone, Copy, GpuStorage)]
    struct TestCell {
        value: u32,
    }

    fn fixed_step_test_plan(with_tail_order: bool) -> CompiledRenderFlowPlan {
        let (flow, cells) = RenderFlow::new("fixed.step.occurrences")
            .storage_array::<TestCell>("cells", 4)
            .expect("render flow authoring should succeed");
        let storage_binding =
            GpuBindingKey::try_new(0, 0).expect("fixed-step storage binding should be valid");
        let iteration_binding =
            GpuBindingKey::try_new(0, 1).expect("fixed-step uniform binding should be valid");
        let flow = flow
            .compute_pass("step.a")
            .bind_storage(storage_binding, cells.clone())
            .dispatch([1, 1, 1])
            .finish()
            .compute_pass("step.b")
            .bind_storage(storage_binding, cells)
            .dispatch([1, 1, 1])
            .finish()
            .fixed_step_region(
                "simulation",
                4,
                [("step.a", iteration_binding), ("step.b", iteration_binding)],
            )
            .expect("fixed-step region should author");
        let flow = if with_tail_order {
            flow.compute_pass("tail")
                .dispatch([1, 1, 1])
                .order_after("step.b")
                .finish()
        } else {
            flow
        };
        compile_flow_plan(&flow.validate().expect("test flow should validate"))
            .expect("test flow should compile")
    }

    fn inputs_for_substeps(
        plan: &CompiledRenderFlowPlan,
        submitted_substeps: u32,
    ) -> PreparedFlowInputs {
        let mut inputs = PreparedFlowInputs::default();
        for pass in &plan.execution.passes {
            if let CompiledPassExecutionPlan::Compute(pass) = pass {
                inputs
                    .projected_dispatch_workgroups
                    .insert(pass.pass_id, [1, 1, 1]);
            }
        }
        let region = plan
            .execution
            .fixed_step_regions
            .first()
            .expect("test plan should contain fixed-step region");
        inputs.projected_uniform_bytes.insert(
            region.iteration_uniform,
            RenderFixedStepIterationUniform::new(
                0,
                submitted_substeps,
                region.max_substeps,
                0,
                1.0 / 60.0,
                0.0,
            )
            .to_uniform_bytes(),
        );
        inputs
    }

    fn pass_ids(occurrences: &[ExpandedRenderPassOccurrence<'_>]) -> Vec<RenderPassId> {
        occurrences
            .iter()
            .map(|occurrence| execution_pass_id(occurrence.pass))
            .collect()
    }

    #[test]
    fn fixed_step_expands_actual_zero_one_and_many_occurrences_before_gpu_preparation() {
        let plan = fixed_step_test_plan(false);
        let region = &plan.execution.fixed_step_regions[0];

        let zero_inputs = inputs_for_substeps(&plan, 0);
        let zero = expand_render_pass_occurrences(&plan, &zero_inputs, |_| Ok(true))
            .expect("zero-substep expansion should succeed");
        assert!(zero.is_empty());

        let one_inputs = inputs_for_substeps(&plan, 1);
        let one = expand_render_pass_occurrences(&plan, &one_inputs, |_| Ok(true))
            .expect("one-substep expansion should succeed");
        assert_eq!(pass_ids(&one), region.pass_ids);

        let many_inputs = inputs_for_substeps(&plan, 3);
        let many = expand_render_pass_occurrences(&plan, &many_inputs, |_| Ok(true))
            .expect("many-substep expansion should succeed");
        assert_eq!(many.len(), region.pass_ids.len() * 3);
        assert_eq!(&pass_ids(&many)[0..2], region.pass_ids.as_slice());
        assert_eq!(&pass_ids(&many)[2..4], region.pass_ids.as_slice());
        assert_eq!(&pass_ids(&many)[4..6], region.pass_ids.as_slice());
        for window in many.windows(2) {
            assert_eq!(
                window[1].control_order_after,
                vec![window[0].occurrence_id],
                "fixed-step execution occurrences must retain one continuous control sequence"
            );
        }
    }

    #[test]
    fn fixed_step_expansion_clamps_to_region_max_substeps() {
        let plan = fixed_step_test_plan(false);
        let region = &plan.execution.fixed_step_regions[0];
        let inputs = inputs_for_substeps(&plan, 99);
        let occurrences = expand_render_pass_occurrences(&plan, &inputs, |_| Ok(true))
            .expect("clamped expansion should succeed");
        assert_eq!(
            occurrences.len(),
            region.pass_ids.len() * region.max_substeps as usize
        );
    }

    #[test]
    fn omitted_passes_are_absent_and_fixed_step_control_chains_across_them() {
        let plan = fixed_step_test_plan(false);
        let region = &plan.execution.fixed_step_regions[0];
        let omitted = region.pass_ids[1];
        let inputs = inputs_for_substeps(&plan, 3);
        let occurrences = expand_render_pass_occurrences(&plan, &inputs, |pass| {
            Ok(execution_pass_id(pass) != omitted)
        })
        .expect("filtered expansion should succeed");

        assert_eq!(occurrences.len(), 3);
        assert!(
            pass_ids(&occurrences)
                .iter()
                .all(|pass| *pass == region.pass_ids[0])
        );
        assert!(occurrences[0].control_order_after.is_empty());
        assert_eq!(
            occurrences[1].control_order_after,
            vec![occurrences[0].occurrence_id]
        );
        assert_eq!(
            occurrences[2].control_order_after,
            vec![occurrences[1].occurrence_id]
        );
    }

    #[test]
    fn pass_level_order_after_lifts_to_last_actual_repeated_predecessor() {
        let plan = fixed_step_test_plan(true);
        let inputs = inputs_for_substeps(&plan, 2);
        let occurrences = expand_render_pass_occurrences(&plan, &inputs, |_| Ok(true))
            .expect("ordered expansion should succeed");
        let tail = occurrences.last().expect("tail occurrence should exist");
        assert_eq!(
            execution_pass_id(tail.pass),
            plan.execution.passes.last().map(execution_pass_id).unwrap()
        );
        let predecessor = &occurrences[occurrences.len() - 2];
        assert_eq!(
            tail.control_order_after,
            vec![predecessor.occurrence_id],
            "tail order_after(step.b) must target the last actual step.b occurrence"
        );
    }

    #[test]
    fn ordinary_lexical_neighbors_do_not_gain_control_edges() {
        let flow = RenderFlow::new("independent.lexical")
            .compute_pass("a")
            .dispatch([1, 1, 1])
            .finish()
            .compute_pass("b")
            .dispatch([1, 1, 1])
            .finish()
            .validate()
            .expect("independent flow should validate");
        let plan = compile_flow_plan(&flow).expect("independent flow should compile");
        let mut inputs = PreparedFlowInputs::default();
        for pass in &plan.execution.passes {
            inputs
                .projected_dispatch_workgroups
                .insert(execution_pass_id(pass), [1, 1, 1]);
        }
        let occurrences = expand_render_pass_occurrences(&plan, &inputs, |_| Ok(true))
            .expect("independent expansion should succeed");
        assert_eq!(occurrences.len(), 2);
        assert!(occurrences[0].control_order_after.is_empty());
        assert!(occurrences[1].control_order_after.is_empty());
    }

    #[test]
    fn frame_occurrence_allocator_does_not_reuse_ids_across_invocations() {
        let flow = RenderFlow::new("frame.occurrence.identity")
            .compute_pass("a")
            .dispatch([1, 1, 1])
            .finish()
            .compute_pass("b")
            .dispatch([1, 1, 1])
            .finish()
            .validate()
            .expect("frame occurrence flow should validate");
        let plan = compile_flow_plan(&flow).expect("frame occurrence flow should compile");
        let mut inputs = PreparedFlowInputs::default();
        for pass in &plan.execution.passes {
            inputs
                .projected_dispatch_workgroups
                .insert(execution_pass_id(pass), [1, 1, 1]);
        }

        let mut maximum_occurrence = 0_u64;
        let first = expand_render_pass_occurrences_in_frame(
            &plan,
            &inputs,
            &mut maximum_occurrence,
            |_| Ok(true),
        )
        .expect("first frame invocation should expand");
        let second = expand_render_pass_occurrences_in_frame(
            &plan,
            &inputs,
            &mut maximum_occurrence,
            |_| Ok(true),
        )
        .expect("second frame invocation should expand");

        assert_eq!(
            first
                .iter()
                .map(|occurrence| occurrence.occurrence_id.raw())
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(
            second
                .iter()
                .map(|occurrence| occurrence.occurrence_id.raw())
                .collect::<Vec<_>>(),
            vec![3, 4]
        );
        assert_eq!(maximum_occurrence, 4);
    }
}
