use crate::plugins::render::RenderFlowId;
use crate::plugins::render::api::RenderFlow;
use crate::plugins::render::current_runtime_gpu_capabilities;
use crate::plugins::render::graph::{CompiledRenderFlowPlan, compile_flow_plan_checked};
use crate::runtime::ResMut;
use std::collections::BTreeMap;

#[derive(Debug, Default, runen_ecs::Resource)]
pub struct RenderFlowRegistryResource {
    flows: BTreeMap<RenderFlowId, RenderFlow>,
    compiled_flows: Vec<CompiledRenderFlowPlan>,
    revision: u64,
    applied_compiled_revision: u64,
}

impl RenderFlowRegistryResource {
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn upsert_flow(&mut self, flow: RenderFlow) {
        let key = flow.id();
        self.flows.insert(key, flow);
        self.bump_revision();
    }

    pub fn remove_flow(&mut self, id: RenderFlowId) -> bool {
        let removed = self.flows.remove(&id).is_some();
        if removed {
            self.bump_revision();
        }
        removed
    }

    pub fn clear(&mut self) {
        if self.flows.is_empty() {
            return;
        }
        self.flows.clear();
        self.bump_revision();
    }

    pub fn flow_count(&self) -> usize {
        self.flows.len()
    }

    pub fn compiled_flows(&self) -> &[CompiledRenderFlowPlan] {
        &self.compiled_flows
    }

    pub fn sync_compiled_flows(&mut self) {
        if self.applied_compiled_revision == self.revision {
            return;
        }

        let mut next_compiled = Vec::<CompiledRenderFlowPlan>::new();

        for flow in self.flows.values() {
            match compile_flow_plan_checked(flow, &current_runtime_gpu_capabilities()) {
                Ok(compiled) => next_compiled.push(compiled),
                Err(err) => tracing::warn!(
                    flow_id = ?flow.id(),
                    flow_label = flow.label(),
                    error = %err,
                    "skipping invalid render flow during compiled planning"
                ),
            }
        }

        self.compiled_flows = next_compiled;
        self.applied_compiled_revision = self.revision;
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }
}

pub(crate) fn sync_render_flow_registry_system(
    mut flow_registry: ResMut<RenderFlowRegistryResource>,
) {
    flow_registry.sync_compiled_flows();
}
