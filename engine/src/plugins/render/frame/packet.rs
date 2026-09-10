use super::{
    PreparedFrameContext, PreparedFrameContributions, PreparedUiFrameContribution,
    PreparedViewFrame,
};
use crate::plugins::render::{
    RenderDynamicTextureTargetDescriptor, RenderDynamicTextureTargetKey,
    RenderDynamicTextureUploadDescriptor, RenderFlowId, RenderFrameProducerId,
    RenderGpuResourceAdapterError, RenderPassId, RenderTargetAliasKey, backend::RenderSurfaceId,
};
use crate::runtime::NativeWindowId;
use product::RenderProductSelection;
use runen_gpu::GpuWorkResourceId;
use std::collections::{BTreeMap, BTreeSet};
use ui_render_data::ViewportSurfaceBindingRegistry;

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct PreparedRenderFrameResource {
    frames: BTreeMap<RenderSurfaceId, PreparedRenderFrame>,
    next_frame_index: u64,
    next_prepare_epoch: u64,
}

impl PreparedRenderFrameResource {
    pub fn publish(&mut self, frame: PreparedRenderFrame) {
        self.next_frame_index = frame.context.frame_index.saturating_add(1);
        self.next_prepare_epoch = frame.context.prepare_epoch.saturating_add(1);
        self.frames.clear();
        self.frames.insert(frame.surface.render_surface_id, frame);
    }

    pub fn publish_set(&mut self, frames: impl IntoIterator<Item = PreparedRenderFrame>) {
        self.frames.clear();
        for frame in frames {
            self.next_frame_index = self
                .next_frame_index
                .max(frame.context.frame_index.saturating_add(1));
            self.next_prepare_epoch = self
                .next_prepare_epoch
                .max(frame.context.prepare_epoch.saturating_add(1));
            self.frames.insert(frame.surface.render_surface_id, frame);
        }
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }

    pub fn frame(&self) -> Option<&PreparedRenderFrame> {
        self.frames
            .get(&RenderSurfaceId::primary())
            .or_else(|| self.frames.values().next())
    }

    pub fn frames(&self) -> impl Iterator<Item = &PreparedRenderFrame> {
        self.frames.values()
    }

    pub fn take(&mut self) -> Option<PreparedRenderFrame> {
        let render_surface_id = if self.frames.contains_key(&RenderSurfaceId::primary()) {
            RenderSurfaceId::primary()
        } else {
            self.frames.keys().next().copied()?
        };
        self.frames.remove(&render_surface_id)
    }

    pub fn take_all(&mut self) -> Vec<PreparedRenderFrame> {
        std::mem::take(&mut self.frames).into_values().collect()
    }

    pub fn allocate_frame_index(&mut self) -> u64 {
        let frame_index = self.next_frame_index;
        self.next_frame_index = self.next_frame_index.saturating_add(1);
        frame_index
    }

    pub fn allocate_prepare_epoch(&mut self) -> u64 {
        let prepare_epoch = self.next_prepare_epoch;
        self.next_prepare_epoch = self.next_prepare_epoch.saturating_add(1);
        prepare_epoch
    }
}

#[derive(Debug, Clone)]
pub struct PreparedRenderFrame {
    pub context: PreparedFrameContext,
    pub surface: PreparedSurfaceInfo,
    pub views: Vec<PreparedViewFrame>,
    pub flows: BTreeMap<RenderFlowId, PreparedFlowInputs>,
    pub flow_invocations: Vec<PreparedFlowInvocation>,
    pub dynamic_texture_targets: Vec<RenderDynamicTextureTargetDescriptor>,
    pub dynamic_texture_uploads: Vec<RenderDynamicTextureUploadDescriptor>,
    pub product_selections: Vec<RenderProductSelection>,
    pub viewport_surface_bindings: ViewportSurfaceBindingRegistry,
    pub contributions: PreparedFrameContributions,
    pub shader: PreparedShaderSnapshot,
}

impl PreparedRenderFrame {
    pub fn flow_inputs(&self, flow_id: RenderFlowId) -> Option<&PreparedFlowInputs> {
        self.flows.get(&flow_id)
    }

    pub fn main_view(&self) -> Option<&PreparedViewFrame> {
        self.views
            .iter()
            .find(|view| matches!(view.kind, super::PreparedViewKind::MainSurface))
            .or_else(|| self.views.first())
    }

    pub fn view(&self, view_id: &str) -> Option<&PreparedViewFrame> {
        self.views.iter().find(|view| view.view_id == view_id)
    }

    pub fn flow_invocations_for_flow(
        &self,
        flow_id: RenderFlowId,
    ) -> impl Iterator<Item = &PreparedFlowInvocation> {
        self.flow_invocations
            .iter()
            .filter(move |invocation| invocation.flow_id == flow_id)
    }

    pub fn ui(&self) -> Option<&PreparedUiFrameContribution> {
        self.contributions.ui()
    }

    pub fn scene_route_labels(&self) -> Option<(&str, &str)> {
        self.contributions.scene_route_labels()
    }

    pub fn dynamic_target_history_signatures(
        &self,
    ) -> anyhow::Result<BTreeMap<RenderDynamicTextureTargetKey, String>> {
        let mut signatures = BTreeMap::<RenderDynamicTextureTargetKey, String>::new();
        for invocation in &self.flow_invocations {
            let view_signature = self
                .view(invocation.view_id.as_str())
                .and_then(|view| view.history_signature.as_ref());
            let Some(signature) = invocation
                .history_signature
                .as_ref()
                .or(view_signature)
                .cloned()
            else {
                continue;
            };
            for binding in invocation.target_alias_bindings.values() {
                let PreparedTargetBinding::DynamicTexture(key) = binding else {
                    continue;
                };
                if let Some(existing) = signatures.get(key)
                    && existing != &signature
                {
                    anyhow::bail!(
                        "dynamic target '{}' has incompatible history signatures '{}' and '{}'",
                        key,
                        existing,
                        signature
                    );
                }
                signatures.insert(key.clone(), signature.clone());
            }
        }
        Ok(signatures)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PreparedSurfaceInfo {
    pub render_surface_id: RenderSurfaceId,
    pub native_window_id: Option<NativeWindowId>,
    pub target_size_px: (u32, u32),
}

impl PreparedSurfaceInfo {
    pub fn primary(target_size_px: (u32, u32)) -> Self {
        Self {
            render_surface_id: RenderSurfaceId::primary(),
            native_window_id: Some(NativeWindowId::primary()),
            target_size_px,
        }
    }

    pub fn for_surface(
        render_surface_id: RenderSurfaceId,
        native_window_id: NativeWindowId,
        target_size_px: (u32, u32),
    ) -> Self {
        Self {
            render_surface_id,
            native_window_id: Some(native_window_id),
            target_size_px,
        }
    }

    pub fn target_size_px(&self) -> (u32, u32) {
        self.target_size_px
    }
}

#[derive(Debug, Clone, Default)]
pub struct PreparedFlowInputs {
    pub projected_uniform_bytes: BTreeMap<GpuWorkResourceId, Vec<u8>>,
    pub projected_dispatch_workgroups: BTreeMap<RenderPassId, [u32; 3]>,
    pub required_state_types: Vec<PreparedStateTypeInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PreparedFlowInvocationId(pub String);

impl PreparedFlowInvocationId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl From<String> for PreparedFlowInvocationId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for PreparedFlowInvocationId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for PreparedFlowInvocationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct PreparedFlowInvocation {
    pub invocation_id: PreparedFlowInvocationId,
    pub flow_id: RenderFlowId,
    pub view_id: String,
    pub inputs: PreparedFlowInputs,
    pub target_alias_bindings: BTreeMap<RenderTargetAliasKey, PreparedTargetBinding>,
    pub history_signature: Option<String>,
}

impl PreparedFlowInvocation {
    pub fn main(flow_id: RenderFlowId, inputs: PreparedFlowInputs) -> Self {
        Self {
            invocation_id: PreparedFlowInvocationId::new(format!("{flow_id}.main")),
            flow_id,
            view_id: "main".to_string(),
            inputs,
            target_alias_bindings: BTreeMap::new(),
            history_signature: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedTargetBinding {
    DynamicTexture(RenderDynamicTextureTargetKey),
    SurfaceColor,
    SurfaceDepth,
    FlowOwned(GpuWorkResourceId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedFlowInvocationRequest {
    pub invocation_id: PreparedFlowInvocationId,
    pub flow_id: RenderFlowId,
    pub view_id: String,
    pub target_alias_bindings: BTreeMap<RenderTargetAliasKey, PreparedTargetBinding>,
    pub uniform_overrides: BTreeMap<GpuWorkResourceId, Vec<u8>>,
    pub history_signature: Option<String>,
}

impl PreparedFlowInvocationRequest {
    pub fn new(
        invocation_id: impl Into<PreparedFlowInvocationId>,
        flow_id: RenderFlowId,
        view_id: impl Into<String>,
    ) -> Self {
        Self {
            invocation_id: invocation_id.into(),
            flow_id,
            view_id: view_id.into(),
            target_alias_bindings: BTreeMap::new(),
            uniform_overrides: BTreeMap::new(),
            history_signature: None,
        }
    }

    pub fn bind_target_alias(
        mut self,
        alias: impl Into<String>,
        binding: PreparedTargetBinding,
    ) -> Result<Self, RenderGpuResourceAdapterError> {
        self.target_alias_bindings
            .insert(RenderTargetAliasKey::new(alias)?, binding);
        Ok(self)
    }

    pub fn bind_dynamic_texture_alias(
        self,
        alias: impl Into<String>,
        key: RenderDynamicTextureTargetKey,
    ) -> Result<Self, RenderGpuResourceAdapterError> {
        self.bind_target_alias(alias, PreparedTargetBinding::DynamicTexture(key))
    }

    pub fn bind_surface_color_alias(
        self,
        alias: impl Into<String>,
    ) -> Result<Self, RenderGpuResourceAdapterError> {
        self.bind_target_alias(alias, PreparedTargetBinding::SurfaceColor)
    }

    pub fn bind_surface_depth_alias(
        self,
        alias: impl Into<String>,
    ) -> Result<Self, RenderGpuResourceAdapterError> {
        self.bind_target_alias(alias, PreparedTargetBinding::SurfaceDepth)
    }

    pub fn bind_flow_owned_alias(
        self,
        alias: impl Into<String>,
        resource_id: GpuWorkResourceId,
    ) -> Result<Self, RenderGpuResourceAdapterError> {
        self.bind_target_alias(alias, PreparedTargetBinding::FlowOwned(resource_id))
    }

    pub fn with_history_signature(mut self, signature: impl Into<String>) -> Self {
        self.history_signature = Some(signature.into());
        self
    }

    pub fn with_uniform_override(mut self, uniform_id: GpuWorkResourceId, bytes: Vec<u8>) -> Self {
        self.uniform_overrides.insert(uniform_id, bytes);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparedRenderFrameRequestKind {
    View,
    Invocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRenderFrameRequestDiagnostic {
    pub producer_id: RenderFrameProducerId,
    pub existing_producer_id: Option<RenderFrameProducerId>,
    pub view_id: Option<String>,
    pub invocation_id: Option<PreparedFlowInvocationId>,
    pub request_kind: PreparedRenderFrameRequestKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PreparedRenderFrameRequestError {
    #[error(
        "prepared render frame producer {producer_id:?} publishes duplicate view '{view_id}' within one contribution"
    )]
    DuplicateViewWithinProducer {
        producer_id: RenderFrameProducerId,
        view_id: String,
    },
    #[error(
        "prepared render frame producer {producer_id:?} publishes view '{view_id}' already owned by producer {existing_producer_id:?}"
    )]
    DuplicateViewAcrossProducers {
        producer_id: RenderFrameProducerId,
        existing_producer_id: RenderFrameProducerId,
        view_id: String,
    },
    #[error(
        "prepared render frame producer {producer_id:?} publishes duplicate invocation '{invocation_id}' within one contribution"
    )]
    DuplicateInvocationWithinProducer {
        producer_id: RenderFrameProducerId,
        invocation_id: PreparedFlowInvocationId,
    },
    #[error(
        "prepared render frame producer {producer_id:?} publishes invocation '{invocation_id}' already owned by producer {existing_producer_id:?}"
    )]
    DuplicateInvocationAcrossProducers {
        producer_id: RenderFrameProducerId,
        existing_producer_id: RenderFrameProducerId,
        invocation_id: PreparedFlowInvocationId,
    },
}

impl PreparedRenderFrameRequestError {
    pub fn diagnostic(&self) -> PreparedRenderFrameRequestDiagnostic {
        match self {
            Self::DuplicateViewWithinProducer {
                producer_id,
                view_id,
            } => PreparedRenderFrameRequestDiagnostic {
                producer_id: *producer_id,
                existing_producer_id: None,
                view_id: Some(view_id.clone()),
                invocation_id: None,
                request_kind: PreparedRenderFrameRequestKind::View,
                message: self.to_string(),
            },
            Self::DuplicateViewAcrossProducers {
                producer_id,
                existing_producer_id,
                view_id,
            } => PreparedRenderFrameRequestDiagnostic {
                producer_id: *producer_id,
                existing_producer_id: Some(*existing_producer_id),
                view_id: Some(view_id.clone()),
                invocation_id: None,
                request_kind: PreparedRenderFrameRequestKind::View,
                message: self.to_string(),
            },
            Self::DuplicateInvocationWithinProducer {
                producer_id,
                invocation_id,
            } => PreparedRenderFrameRequestDiagnostic {
                producer_id: *producer_id,
                existing_producer_id: None,
                view_id: None,
                invocation_id: Some(invocation_id.clone()),
                request_kind: PreparedRenderFrameRequestKind::Invocation,
                message: self.to_string(),
            },
            Self::DuplicateInvocationAcrossProducers {
                producer_id,
                existing_producer_id,
                invocation_id,
            } => PreparedRenderFrameRequestDiagnostic {
                producer_id: *producer_id,
                existing_producer_id: Some(*existing_producer_id),
                view_id: None,
                invocation_id: Some(invocation_id.clone()),
                request_kind: PreparedRenderFrameRequestKind::Invocation,
                message: self.to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct PreparedRenderFrameRequestResource {
    contributions: BTreeMap<RenderFrameProducerId, PreparedRenderFrameRequestContribution>,
    diagnostics: Vec<PreparedRenderFrameRequestDiagnostic>,
}

#[derive(Debug, Clone, Default)]
pub struct PreparedRenderFrameRequestContribution {
    views: BTreeMap<String, PreparedViewFrame>,
    flow_invocations: Vec<PreparedFlowInvocationRequest>,
}

impl PreparedRenderFrameRequestResource {
    pub fn clear(&mut self) {
        self.contributions.clear();
        self.diagnostics.clear();
    }

    pub fn remove_contribution(
        &mut self,
        producer_id: impl Into<RenderFrameProducerId>,
    ) -> Option<PreparedRenderFrameRequestContribution> {
        let producer_id = producer_id.into();
        let removed = self.contributions.remove(&producer_id);
        self.clear_diagnostics_for_producer(&producer_id);
        removed
    }

    pub fn replace_contribution(
        &mut self,
        producer_id: impl Into<RenderFrameProducerId>,
        views: impl IntoIterator<Item = PreparedViewFrame>,
        flow_invocations: impl IntoIterator<Item = PreparedFlowInvocationRequest>,
    ) -> Result<Option<PreparedRenderFrameRequestContribution>, PreparedRenderFrameRequestError>
    {
        let producer_id = producer_id.into();
        self.clear_diagnostics_for_producer(&producer_id);
        let contribution = match PreparedRenderFrameRequestContribution::from_requests(
            &producer_id,
            views,
            flow_invocations,
        ) {
            Ok(contribution) => contribution,
            Err(error) => {
                self.record_error(&error);
                return Err(error);
            }
        };
        if let Err(error) = self.validate_replacement(&producer_id, &contribution) {
            self.record_error(&error);
            return Err(error);
        }
        Ok(self.contributions.insert(producer_id, contribution))
    }

    pub fn diagnostics(&self) -> &[PreparedRenderFrameRequestDiagnostic] {
        &self.diagnostics
    }

    pub fn requested_views(&self) -> Vec<&PreparedViewFrame> {
        self.contributions
            .values()
            .flat_map(|contribution| contribution.views.values())
            .collect()
    }

    pub fn requested_flow_invocations(&self) -> Vec<&PreparedFlowInvocationRequest> {
        self.contributions
            .values()
            .flat_map(|contribution| contribution.flow_invocations.iter())
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.contributions.is_empty()
    }

    fn validate_replacement(
        &self,
        producer_id: &RenderFrameProducerId,
        replacement: &PreparedRenderFrameRequestContribution,
    ) -> Result<(), PreparedRenderFrameRequestError> {
        let mut view_ids = BTreeMap::<&str, &RenderFrameProducerId>::new();
        let mut invocation_ids =
            BTreeMap::<&PreparedFlowInvocationId, &RenderFrameProducerId>::new();

        for (existing_producer_id, contribution) in &self.contributions {
            if existing_producer_id == producer_id {
                continue;
            }
            for view_id in contribution.views.keys() {
                view_ids.insert(view_id.as_str(), existing_producer_id);
            }
            for request in &contribution.flow_invocations {
                invocation_ids.insert(&request.invocation_id, existing_producer_id);
            }
        }

        for view_id in replacement.views.keys() {
            if let Some(existing_producer_id) = view_ids.get(view_id.as_str()) {
                return Err(
                    PreparedRenderFrameRequestError::DuplicateViewAcrossProducers {
                        producer_id: *producer_id,
                        existing_producer_id: **existing_producer_id,
                        view_id: view_id.clone(),
                    },
                );
            }
        }
        for request in &replacement.flow_invocations {
            if let Some(existing_producer_id) = invocation_ids.get(&request.invocation_id) {
                return Err(
                    PreparedRenderFrameRequestError::DuplicateInvocationAcrossProducers {
                        producer_id: *producer_id,
                        existing_producer_id: **existing_producer_id,
                        invocation_id: request.invocation_id.clone(),
                    },
                );
            }
        }

        Ok(())
    }

    fn record_error(&mut self, error: &PreparedRenderFrameRequestError) {
        self.diagnostics.push(error.diagnostic());
    }

    fn clear_diagnostics_for_producer(&mut self, producer_id: &RenderFrameProducerId) {
        self.diagnostics.retain(|diagnostic| {
            diagnostic.producer_id != *producer_id
                && diagnostic.existing_producer_id != Some(*producer_id)
        });
    }
}

impl PreparedRenderFrameRequestContribution {
    fn from_requests(
        producer_id: &RenderFrameProducerId,
        views: impl IntoIterator<Item = PreparedViewFrame>,
        flow_invocations: impl IntoIterator<Item = PreparedFlowInvocationRequest>,
    ) -> Result<Self, PreparedRenderFrameRequestError> {
        let mut view_map = BTreeMap::<String, PreparedViewFrame>::new();
        for view in views {
            let view_id = view.view_id.clone();
            if view_map.insert(view_id.clone(), view).is_some() {
                return Err(
                    PreparedRenderFrameRequestError::DuplicateViewWithinProducer {
                        producer_id: *producer_id,
                        view_id,
                    },
                );
            }
        }

        let mut invocation_ids = BTreeSet::<PreparedFlowInvocationId>::new();
        let flow_invocations = flow_invocations.into_iter().collect::<Vec<_>>();
        for request in &flow_invocations {
            if !invocation_ids.insert(request.invocation_id.clone()) {
                return Err(
                    PreparedRenderFrameRequestError::DuplicateInvocationWithinProducer {
                        producer_id: *producer_id,
                        invocation_id: request.invocation_id.clone(),
                    },
                );
            }
        }

        Ok(Self {
            views: view_map,
            flow_invocations,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedStateTypeInfo {
    pub type_name: &'static str,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PreparedShaderSnapshot {
    pub registry_revision: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_frame(index: u64) -> PreparedRenderFrame {
        PreparedRenderFrame {
            context: PreparedFrameContext {
                frame_index: index,
                flow_registry_revision: 7,
                shader_registry_revision: 11,
                prepare_epoch: 3,
            },
            surface: PreparedSurfaceInfo::primary((1280, 720)),
            views: vec![PreparedViewFrame::main((1280, 720))],
            flows: BTreeMap::new(),
            flow_invocations: Vec::new(),
            dynamic_texture_targets: Vec::new(),
            dynamic_texture_uploads: Vec::new(),
            product_selections: Vec::new(),
            viewport_surface_bindings: ViewportSurfaceBindingRegistry::default(),
            contributions: PreparedFrameContributions::default(),
            shader: PreparedShaderSnapshot {
                registry_revision: 11,
            },
        }
    }

    #[test]
    fn prepared_frame_resource_allocates_monotonic_indices() {
        let mut resource = PreparedRenderFrameResource::default();
        assert_eq!(resource.allocate_frame_index(), 0);
        assert_eq!(resource.allocate_frame_index(), 1);
    }

    #[test]
    fn prepared_frame_resource_publish_and_take_roundtrip() {
        let mut resource = PreparedRenderFrameResource::default();
        resource.publish(dummy_frame(4));
        assert_eq!(
            resource
                .frame()
                .expect("frame should be present after publish")
                .context
                .frame_index,
            4
        );
        assert_eq!(resource.allocate_frame_index(), 5);

        let taken = resource.take().expect("take should return a frame");
        assert_eq!(taken.context.frame_index, 4);
        assert!(resource.frame().is_none());
    }
}
