use crate::plugins::render::api::{
    BuiltinUiCompositePassBuilder, ComputePassBuilder, CopyPassBuilder, FullscreenPassBuilder,
    GraphicsPassBuilder, ParamProjectionError, PassUniformProjection, PresentPassBuilder,
    ProjectedUniformSet, RenderDoubleBuffer, RenderFixedStepIterationUniform,
    RenderFlowAuthoringError, RenderShaderBinding, RenderShaderBindingResource,
    project_uniform_bindings_for_pass,
};
use crate::plugins::render::graph::compile_flow_plan;
use crate::plugins::render::procedural::{
    ProceduralPassBuilder, ProceduralPassDescriptor, build_procedural_pass,
};
use crate::plugins::render::renderer::frame_bindings::RenderFrameDataRegistry;
use crate::plugins::render::{
    FlowValidationReport, GpuParams, GpuPrimitiveDispatchPlan, GpuPrimitiveExecutionPlan,
    IndirectDrawArgsBuffer, RenderFixedStepRegionId, RenderFixedStepRegionMembership,
    RenderFlowGraph, RenderFlowId, RenderFlowValidationError, RenderPassId, RenderPassIdSequence,
    RenderPassKind, RenderPassNode, RenderResourceDeclaration, RenderShaderReference,
    RenderTargetAliasKey, RenderTargetAliasKind, RenderTextureTargetFormat, U32ScanElement,
    validate_flow_graph,
};
use crate::runtime::{CatchupBudget, FixedTimeConfig, FixedTimeState};
use runen_gpu::{
    GpuBindingKey, GpuBufferHandle, GpuBufferInitialization, GpuResourceLifetime,
    GpuWorkResourceId, GpuWorkResourceIdAllocator,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

pub const SURFACE_COLOR_RESOURCE_LABEL: &str = "surface.color";
pub const SURFACE_DEPTH_RESOURCE_LABEL: &str = "surface.depth";

static NEXT_FLOW_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
struct PingPongStorageRegistration {
    a_id: GpuWorkResourceId,
    b_id: GpuWorkResourceId,
}

#[derive(Debug)]
pub struct RenderFlow {
    graph: RenderFlowGraph,
    pass_ids_by_label: BTreeMap<String, RenderPassId>,
    resource_ids_by_label: BTreeMap<String, GpuWorkResourceId>,
    ping_pong_storage: BTreeMap<String, PingPongStorageRegistration>,
    next_pass_id: RenderPassIdSequence,
    next_resource_id: GpuWorkResourceIdAllocator,
    next_fixed_step_region_id: u64,
}

impl RenderFlow {
    pub fn new(label: impl Into<String>) -> Self {
        let label = label.into();
        let flow_id = RenderFlowId::try_from_raw(NEXT_FLOW_ID.fetch_add(1, Ordering::Relaxed))
            .expect("render flow id sequence starts at one");

        Self {
            graph: RenderFlowGraph::new(flow_id, label),
            pass_ids_by_label: BTreeMap::new(),
            resource_ids_by_label: BTreeMap::new(),
            ping_pong_storage: BTreeMap::new(),
            next_pass_id: RenderPassIdSequence::default(),
            next_resource_id: GpuWorkResourceIdAllocator::new(),
            next_fixed_step_region_id: 1,
        }
    }

    pub fn with_state<T>(mut self) -> Self
    where
        T: runen_ecs::Resource + 'static,
    {
        self.graph.resources.add_state_resource::<T>();
        self
    }

    pub fn with_surface_color(mut self) -> Result<Self, RenderFlowAuthoringError> {
        self.ensure_surface_color_resource()?;
        Ok(self)
    }

    pub fn with_surface_depth(mut self) -> Result<Self, RenderFlowAuthoringError> {
        self.ensure_surface_depth_resource()?;
        Ok(self)
    }

    pub fn with_color_target(
        mut self,
        label: impl Into<String>,
    ) -> Result<Self, RenderFlowAuthoringError> {
        self.register_color_target(label.into())?;
        Ok(self)
    }

    pub fn with_color_target_exact(
        mut self,
        label: impl Into<String>,
        format: RenderTextureTargetFormat,
    ) -> Result<Self, RenderFlowAuthoringError> {
        self.register_color_target_exact(label.into(), format)?;
        Ok(self)
    }

    pub fn with_depth_target(
        mut self,
        label: impl Into<String>,
    ) -> Result<Self, RenderFlowAuthoringError> {
        self.register_depth_target(label.into())?;
        Ok(self)
    }

    pub fn with_history_texture(
        mut self,
        label: impl Into<String>,
    ) -> Result<Self, RenderFlowAuthoringError> {
        self.register_history_texture(label.into())?;
        Ok(self)
    }

    pub fn with_sampled_texture(
        mut self,
        label: impl Into<String>,
    ) -> Result<Self, RenderFlowAuthoringError> {
        self.register_sampled_texture(label.into())?;
        Ok(self)
    }

    pub fn with_storage_texture(
        mut self,
        label: impl Into<String>,
    ) -> Result<Self, RenderFlowAuthoringError> {
        self.register_storage_texture(label.into())?;
        Ok(self)
    }

    pub fn uniform_buffer<U>(
        mut self,
        label: impl Into<String>,
    ) -> Result<(Self, GpuBufferHandle), RenderFlowAuthoringError>
    where
        U: GpuParams + 'static,
    {
        let id = self.register_uniform_buffer::<U>(label.into())?;
        let handle = self.buffer_handle(id)?;
        Ok((self, handle))
    }

    pub fn with_target_alias(
        mut self,
        label: impl Into<String>,
        kind: RenderTargetAliasKind,
    ) -> Result<Self, RenderFlowAuthoringError> {
        self.register_target_alias(label.into(), kind)?;
        Ok(self)
    }

    pub fn with_color_target_alias(
        self,
        label: impl Into<String>,
    ) -> Result<Self, RenderFlowAuthoringError> {
        self.with_target_alias(label, RenderTargetAliasKind::Color)
    }

    pub fn with_depth_target_alias(
        self,
        label: impl Into<String>,
    ) -> Result<Self, RenderFlowAuthoringError> {
        self.with_target_alias(label, RenderTargetAliasKind::Depth)
    }

    pub fn with_builtin_ui(self) -> Self {
        self
    }

    pub fn storage_array<T>(
        mut self,
        label: impl Into<String>,
        len: u64,
    ) -> Result<(Self, GpuBufferHandle), RenderFlowAuthoringError>
    where
        T: GpuParams + 'static,
    {
        let id = self.register_storage_array::<T>(label.into(), len)?;
        let handle = self.buffer_handle(id)?;
        Ok((self, handle))
    }

    pub fn double_buffer_storage_array<T>(
        mut self,
        label: impl Into<String>,
        len: u64,
    ) -> Result<Self, RenderFlowAuthoringError>
    where
        T: GpuParams + 'static,
    {
        self.register_double_buffer_storage_array::<T>(label.into(), len)?;
        Ok(self)
    }

    pub fn double_buffer_storage_array_with_handle<T>(
        mut self,
        label: impl Into<String>,
        len: u64,
    ) -> Result<(Self, RenderDoubleBuffer), RenderFlowAuthoringError>
    where
        T: GpuParams + 'static,
    {
        let base_label = label.into();
        let (a_id, b_id) =
            self.register_double_buffer_storage_array::<T>(base_label.clone(), len)?;
        let handle = RenderDoubleBuffer::new(
            base_label,
            self.buffer_handle(a_id)?,
            self.buffer_handle(b_id)?,
        );
        Ok((self, handle))
    }

    pub fn compute_pass(self, label: impl Into<String>) -> ComputePassBuilder {
        ComputePassBuilder::new(self, label.into())
    }

    pub fn fullscreen_pass(self, label: impl Into<String>) -> FullscreenPassBuilder {
        FullscreenPassBuilder::new(self, label.into())
    }

    pub fn graphics_pass(self, label: impl Into<String>) -> GraphicsPassBuilder {
        GraphicsPassBuilder::new(self, label.into())
    }

    pub fn procedural_pass(
        self,
        descriptor: ProceduralPassDescriptor,
    ) -> Result<Self, RenderFlowAuthoringError> {
        build_procedural_pass(self, descriptor)
    }

    pub fn procedural_pass_builder(
        self,
        descriptor: ProceduralPassDescriptor,
    ) -> Result<ProceduralPassBuilder, RenderFlowAuthoringError> {
        ProceduralPassBuilder::new(self, descriptor)
    }

    pub fn gpu_primitive_plan(
        mut self,
        plan: &GpuPrimitiveExecutionPlan,
    ) -> Result<Self, RenderFlowAuthoringError> {
        let dispatch_plan =
            plan.dispatch_plan_with_temporary(|label, element_count, initialization| {
                let id = self.register_transient_storage_array::<U32ScanElement>(
                    label,
                    element_count,
                    initialization,
                )?;
                self.buffer_handle(id)
            })?;
        self.append_gpu_primitive_dispatch_plan(dispatch_plan)
    }

    pub fn fixed_step_region<I, S>(
        mut self,
        label: impl Into<String>,
        max_substeps: u32,
        pass_bindings: I,
    ) -> Result<Self, RenderFlowAuthoringError>
    where
        I: IntoIterator<Item = (S, GpuBindingKey)>,
        S: AsRef<str>,
    {
        let label = label.into();
        assert!(
            max_substeps > 0,
            "fixed-step region '{}' must allow at least one substep",
            label
        );
        self.graph.resources.add_state_resource::<FixedTimeConfig>();
        self.graph.resources.add_state_resource::<FixedTimeState>();
        self.graph.resources.add_state_resource::<CatchupBudget>();

        let iteration_uniform = self.register_uniform_buffer::<RenderFixedStepIterationUniform>(
            format!("{label}.fixed_step_iteration"),
        )?;
        let region_id = RenderFixedStepRegionId::new(self.next_fixed_step_region_id);
        self.next_fixed_step_region_id = self.next_fixed_step_region_id.saturating_add(1);
        let membership = RenderFixedStepRegionMembership {
            region_id,
            region_label: label.clone(),
            max_substeps,
            iteration_uniform,
        };
        let pass_bindings = pass_bindings
            .into_iter()
            .map(|(pass_label, binding)| {
                let pass_label = pass_label.as_ref();
                let pass_id = self.resolve_pass_id(pass_label).unwrap_or_else(|| {
                    panic!(
                        "pass label '{}' is not registered in flow '{}'",
                        pass_label,
                        self.label()
                    )
                });
                (pass_id, binding)
            })
            .collect::<Vec<_>>();
        assert!(
            !pass_bindings.is_empty(),
            "fixed-step region '{}' must include at least one pass",
            label
        );

        for (pass_id, binding) in pass_bindings {
            let pass = self
                .graph
                .passes
                .passes
                .iter_mut()
                .find(|pass| pass.id == pass_id)
                .expect("resolved pass should exist in flow graph");
            pass.fixed_step_region = Some(membership.clone());
            push_unique_resource_id(&mut pass.fixed_step_iteration_uniforms, iteration_uniform);
            pass.shader_bindings.push(RenderShaderBinding::new(
                binding,
                RenderShaderBindingResource::UniformBuffer(iteration_uniform),
            ));
        }

        Ok(self)
    }

    pub fn copy_pass(self, label: impl Into<String>) -> CopyPassBuilder {
        CopyPassBuilder::new(self, label.into())
    }

    pub fn present_pass(
        self,
        label: impl Into<String>,
    ) -> Result<PresentPassBuilder, RenderFlowAuthoringError> {
        PresentPassBuilder::new(self, label.into())
    }

    pub fn builtin_ui_composite_pass(
        self,
        label: impl Into<String>,
    ) -> Result<BuiltinUiCompositePassBuilder, RenderFlowAuthoringError> {
        BuiltinUiCompositePassBuilder::new(self, label.into())
    }

    pub fn validate(self) -> anyhow::Result<Self> {
        self.validation_report()
            .map_err(anyhow::Error::new)
            .map(|_| self)
    }

    pub fn validation_report(&self) -> Result<FlowValidationReport, RenderFlowValidationError> {
        validate_flow_graph(&self.graph)
    }

    /// Returns the render-owned lexical pass order after flow validation and compilation.
    /// Executable GPU ordering is established later by RunenGPU from resolved operations.
    pub fn lexical_pass_order(&self) -> Result<Vec<RenderPassId>, RenderFlowValidationError> {
        let plan = compile_flow_plan(self)?;
        Ok(plan
            .render_passes
            .iter()
            .map(|pass| pass.pass_id())
            .collect())
    }

    pub fn id(&self) -> RenderFlowId {
        self.graph.id
    }

    pub fn label(&self) -> &str {
        self.graph.label.as_str()
    }

    pub fn graph(&self) -> &RenderFlowGraph {
        &self.graph
    }

    pub fn resource_id(&self, label: &str) -> Option<GpuWorkResourceId> {
        self.resolve_resource_id(label)
    }

    pub fn pass_id(&self, label: &str) -> Option<RenderPassId> {
        self.resolve_pass_id(label)
    }

    pub fn project_uniforms(
        &self,
        frame_data: &RenderFrameDataRegistry<'_>,
        surface_size: (u32, u32),
    ) -> Result<ProjectedUniformSet, Vec<ParamProjectionError>> {
        let mut projections = Vec::<PassUniformProjection>::new();
        let mut errors = Vec::<ParamProjectionError>::new();

        for pass in &self.graph.passes.passes {
            match project_uniform_bindings_for_pass(
                pass,
                &self.graph.resources,
                frame_data,
                surface_size,
            ) {
                Ok(buffers) => {
                    if !buffers.is_empty() {
                        projections.push(PassUniformProjection {
                            pass_id: pass.id,
                            pass_label: pass.label.clone(),
                            buffers,
                        });
                    }
                }
                Err(mut pass_errors) => errors.append(&mut pass_errors),
            }
        }

        if errors.is_empty() {
            Ok(ProjectedUniformSet::from_passes(projections))
        } else {
            Err(errors)
        }
    }

    pub(crate) fn allocate_pass(&mut self, label: impl Into<String>) -> (RenderPassId, String) {
        let label = label.into();
        let id: RenderPassId = self.next_pass_id.allocate().into();
        self.pass_ids_by_label.insert(label.clone(), id);
        (id, label)
    }

    pub(crate) fn resolve_pass_id(&self, label: &str) -> Option<RenderPassId> {
        self.pass_ids_by_label.get(label).copied()
    }

    pub(crate) fn resolve_resource_id(&self, label: &str) -> Option<GpuWorkResourceId> {
        self.resource_ids_by_label.get(label).copied()
    }

    pub(crate) fn resource_ids_by_label(&self) -> &BTreeMap<String, GpuWorkResourceId> {
        &self.resource_ids_by_label
    }

    pub(crate) fn push_pass(mut self, pass: RenderPassNode) -> Self {
        self.graph.add_pass(pass);
        self
    }

    pub(crate) fn allocate_uniform_resource<U>(
        &mut self,
        pass_label: &str,
    ) -> Result<GpuBufferHandle, RenderFlowAuthoringError>
    where
        U: GpuParams + 'static,
    {
        let mut index = 0usize;
        loop {
            let label = format!("{pass_label}.uniform.{index}");
            if !self.resource_ids_by_label.contains_key(label.as_str()) {
                let declaration = RenderResourceDeclaration::declare_uniform::<U>(
                    &mut self.next_resource_id,
                    label.clone(),
                )?;
                let id = *declaration.id();
                self.upsert_labeled_resource(label, id, declaration);
                return self.buffer_handle(id);
            }
            index = index.saturating_add(1);
        }
    }

    pub(crate) fn ping_pong_storage_ids(
        &self,
        label: &str,
    ) -> Option<(GpuWorkResourceId, GpuWorkResourceId)> {
        self.ping_pong_storage
            .get(label)
            .map(|pair| (pair.a_id, pair.b_id))
    }

    pub(crate) fn ensure_surface_color_resource(
        &mut self,
    ) -> Result<GpuWorkResourceId, RenderFlowAuthoringError> {
        if let Some(id) = self.resolve_resource_id(SURFACE_COLOR_RESOURCE_LABEL) {
            return Ok(id);
        }

        let id = self.allocate_resource_id()?;
        self.upsert_labeled_resource(
            SURFACE_COLOR_RESOURCE_LABEL.to_string(),
            id,
            RenderResourceDeclaration::declare_imported_surface_color(
                id,
                SURFACE_COLOR_RESOURCE_LABEL,
            ),
        );
        Ok(id)
    }

    pub(crate) fn ensure_surface_depth_resource(
        &mut self,
    ) -> Result<GpuWorkResourceId, RenderFlowAuthoringError> {
        if let Some(id) = self.resolve_resource_id(SURFACE_DEPTH_RESOURCE_LABEL) {
            return Ok(id);
        }

        let id = self.allocate_resource_id()?;
        self.upsert_labeled_resource(
            SURFACE_DEPTH_RESOURCE_LABEL.to_string(),
            id,
            RenderResourceDeclaration::declare_imported_surface_depth(
                id,
                SURFACE_DEPTH_RESOURCE_LABEL,
            ),
        );
        Ok(id)
    }

    fn register_color_target(
        &mut self,
        label: String,
    ) -> Result<GpuWorkResourceId, RenderFlowAuthoringError> {
        if let Some(id) = self.resolve_resource_id(label.as_str()) {
            return Ok(id);
        }

        let id = self.allocate_resource_id()?;
        self.upsert_labeled_resource(
            label.clone(),
            id,
            RenderResourceDeclaration::declare_color_attachment(id, label),
        );
        Ok(id)
    }

    fn register_color_target_exact(
        &mut self,
        label: String,
        format: RenderTextureTargetFormat,
    ) -> Result<GpuWorkResourceId, RenderFlowAuthoringError> {
        if let Some(id) = self.resolve_resource_id(label.as_str()) {
            return Ok(id);
        }

        let id = self.allocate_resource_id()?;
        self.upsert_labeled_resource(
            label.clone(),
            id,
            RenderResourceDeclaration::declare_color_attachment_exact(id, label, format),
        );
        Ok(id)
    }

    fn register_depth_target(
        &mut self,
        label: String,
    ) -> Result<GpuWorkResourceId, RenderFlowAuthoringError> {
        if let Some(id) = self.resolve_resource_id(label.as_str()) {
            return Ok(id);
        }

        let id = self.allocate_resource_id()?;
        self.upsert_labeled_resource(
            label.clone(),
            id,
            RenderResourceDeclaration::declare_depth_attachment(id, label),
        );
        Ok(id)
    }

    fn register_history_texture(
        &mut self,
        label: String,
    ) -> Result<GpuWorkResourceId, RenderFlowAuthoringError> {
        if let Some(id) = self.resolve_resource_id(label.as_str()) {
            return Ok(id);
        }

        let id = self.allocate_resource_id()?;
        self.upsert_labeled_resource(
            label.clone(),
            id,
            RenderResourceDeclaration::declare_history_texture(id, label),
        );
        Ok(id)
    }

    fn register_sampled_texture(
        &mut self,
        label: String,
    ) -> Result<GpuWorkResourceId, RenderFlowAuthoringError> {
        if let Some(id) = self.resolve_resource_id(label.as_str()) {
            return Ok(id);
        }

        let id = self.allocate_resource_id()?;
        self.upsert_labeled_resource(
            label.clone(),
            id,
            RenderResourceDeclaration::declare_sampled_texture(id, label),
        );
        Ok(id)
    }

    fn register_storage_texture(
        &mut self,
        label: String,
    ) -> Result<GpuWorkResourceId, RenderFlowAuthoringError> {
        if let Some(id) = self.resolve_resource_id(label.as_str()) {
            return Ok(id);
        }

        let id = self.allocate_resource_id()?;
        self.upsert_labeled_resource(
            label.clone(),
            id,
            RenderResourceDeclaration::declare_storage_texture(id, label),
        );
        Ok(id)
    }

    fn register_uniform_buffer<U>(
        &mut self,
        label: String,
    ) -> Result<GpuWorkResourceId, RenderFlowAuthoringError>
    where
        U: GpuParams + 'static,
    {
        if let Some(id) = self.resolve_resource_id(label.as_str()) {
            return Ok(id);
        }

        let declaration = RenderResourceDeclaration::declare_uniform::<U>(
            &mut self.next_resource_id,
            label.clone(),
        )?;
        let id = *declaration.id();
        self.upsert_labeled_resource(label, id, declaration);
        Ok(id)
    }

    fn register_target_alias(
        &mut self,
        label: String,
        kind: RenderTargetAliasKind,
    ) -> Result<GpuWorkResourceId, RenderFlowAuthoringError> {
        let binding_key = RenderTargetAliasKey::new(label)?;
        if let Some(id) = self.resolve_resource_id(binding_key.as_str()) {
            return Ok(id);
        }

        let id = self.allocate_resource_id()?;
        self.upsert_labeled_resource(
            binding_key.as_str().to_string(),
            id,
            RenderResourceDeclaration::declare_target_alias_with_key(id, binding_key, kind),
        );
        Ok(id)
    }

    fn register_storage_array<T>(
        &mut self,
        label: String,
        len: u64,
    ) -> Result<GpuWorkResourceId, RenderFlowAuthoringError>
    where
        T: GpuParams + 'static,
    {
        self.register_storage_array_with_lifetime::<T>(label, len, GpuResourceLifetime::Retained)
    }

    fn register_transient_storage_array<T>(
        &mut self,
        label: String,
        len: u64,
        initialization: GpuBufferInitialization,
    ) -> Result<GpuWorkResourceId, RenderFlowAuthoringError>
    where
        T: GpuParams + 'static,
    {
        self.register_storage_array_with_lifetime_and_initialization::<T>(
            label,
            len,
            GpuResourceLifetime::Transient,
            initialization,
        )
    }

    fn register_storage_array_with_lifetime<T>(
        &mut self,
        label: String,
        len: u64,
        lifetime: GpuResourceLifetime,
    ) -> Result<GpuWorkResourceId, RenderFlowAuthoringError>
    where
        T: GpuParams + 'static,
    {
        self.register_storage_array_with_lifetime_and_initialization::<T>(
            label,
            len,
            lifetime,
            GpuBufferInitialization::Uninitialized,
        )
    }

    fn register_storage_array_with_lifetime_and_initialization<T>(
        &mut self,
        label: String,
        len: u64,
        lifetime: GpuResourceLifetime,
        initialization: GpuBufferInitialization,
    ) -> Result<GpuWorkResourceId, RenderFlowAuthoringError>
    where
        T: GpuParams + 'static,
    {
        if let Some(id) = self.resolve_resource_id(label.as_str()) {
            return Ok(id);
        }

        let declaration =
            RenderResourceDeclaration::declare_storage_array_with_lifetime_and_initialization::<T>(
                &mut self.next_resource_id,
                label.clone(),
                len,
                lifetime,
                initialization,
            )?;
        let id = *declaration.id();
        self.upsert_labeled_resource(label, id, declaration);
        Ok(id)
    }

    fn register_double_buffer_storage_array<T>(
        &mut self,
        base_label: String,
        len: u64,
    ) -> Result<(GpuWorkResourceId, GpuWorkResourceId), RenderFlowAuthoringError>
    where
        T: GpuParams + 'static,
    {
        if let Some(existing) = self.ping_pong_storage.get(base_label.as_str()) {
            return Ok((existing.a_id, existing.b_id));
        }

        let a_label = format!("{base_label}.a");
        let a = RenderResourceDeclaration::declare_storage_array::<T>(
            &mut self.next_resource_id,
            a_label.clone(),
            len,
        )?;
        let a_id = *a.id();

        let b_label = format!("{base_label}.b");
        let b = RenderResourceDeclaration::declare_storage_array::<T>(
            &mut self.next_resource_id,
            b_label.clone(),
            len,
        )?;
        let b_id = *b.id();
        self.upsert_labeled_resource(a_label, a_id, a);
        self.upsert_labeled_resource(b_label, b_id, b);

        self.ping_pong_storage.insert(
            base_label.clone(),
            PingPongStorageRegistration { a_id, b_id },
        );

        Ok((a_id, b_id))
    }

    fn allocate_resource_id(&mut self) -> Result<GpuWorkResourceId, RenderFlowAuthoringError> {
        self.next_resource_id.allocate().map_err(Into::into)
    }

    fn buffer_handle(
        &self,
        id: GpuWorkResourceId,
    ) -> Result<GpuBufferHandle, RenderFlowAuthoringError> {
        self.graph
            .resources
            .resources
            .iter()
            .find(|resource| *resource.id() == id)
            .and_then(RenderResourceDeclaration::buffer_handle)
            .cloned()
            .ok_or(RenderFlowAuthoringError::DeclaredBufferHandleMissing { resource_id: id })
    }

    pub(crate) fn indirect_buffer_element_count<T: IndirectDrawArgsBuffer + 'static>(
        &self,
        handle: &GpuBufferHandle,
    ) -> Result<u64, RenderFlowAuthoringError> {
        let resource_id = handle.diagnostic_identity();
        let expected = core::any::type_name::<T>();
        let Some(RenderResourceDeclaration::Storage(storage)) = self
            .graph
            .resources
            .resources
            .iter()
            .find(|resource| *resource.id() == resource_id)
        else {
            return Err(RenderFlowAuthoringError::BufferLayoutMismatch {
                resource_id,
                expected,
                actual: "non-storage or foreign buffer",
            });
        };
        if storage.params_type_id() != core::any::TypeId::of::<T>() {
            return Err(RenderFlowAuthoringError::BufferLayoutMismatch {
                resource_id,
                expected,
                actual: storage.params_type_name(),
            });
        }
        Ok(storage.element_count())
    }

    fn upsert_labeled_resource(
        &mut self,
        label: String,
        id: GpuWorkResourceId,
        descriptor: RenderResourceDeclaration,
    ) {
        self.resource_ids_by_label.insert(label, id);
        self.upsert_resource(descriptor);
    }

    fn upsert_resource(&mut self, descriptor: RenderResourceDeclaration) {
        let id = *descriptor.id();
        if self
            .graph
            .resources
            .resources
            .iter()
            .all(|existing| *existing.id() != id)
        {
            self.graph.add_resource(descriptor);
        }
    }

    fn append_gpu_primitive_dispatch_plan(
        mut self,
        plan: GpuPrimitiveDispatchPlan,
    ) -> Result<Self, RenderFlowAuthoringError> {
        for stage in plan.stages {
            let (pass_id, pass_label) = self.allocate_pass(stage.label);
            let mut pass = RenderPassNode::new(pass_id, pass_label, RenderPassKind::Compute);
            pass.shader = Some(RenderShaderReference::AssetPath(
                stage.shader_asset.to_string(),
            ));
            pass.shader_constants = stage.constants;
            pass.compute_dispatch =
                Some(crate::plugins::render::api::ComputeDispatchDescriptor::Fixed(stage.dispatch));
            for binding in &stage.shader_bindings {
                pass.shader_bindings.push(RenderShaderBinding::new(
                    binding.key(),
                    RenderShaderBindingResource::StorageBuffer {
                        resource: binding.buffer().diagnostic_identity(),
                        access: binding.access(),
                    },
                ));
            }
            for read in stage.reads {
                push_unique_resource_id(&mut pass.storage_reads, read.diagnostic_identity());
            }
            for write in stage.writes {
                push_unique_resource_id(&mut pass.storage_writes, write.diagnostic_identity());
            }
            self = self.push_pass(pass);
        }

        Ok(self)
    }
}

fn push_unique_resource_id<T: PartialEq + Copy>(resources: &mut Vec<T>, resource: T) {
    if !resources.contains(&resource) {
        resources.push(resource);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::{
        CompiledDrawSource, CompiledPassDescriptor, DrawIndexedIndirectArgs, DrawIndirectArgs,
        GpuStorage, GpuUniform, RenderFlowValidationIssue, RenderTextureFormatPolicy,
        RenderTextureSizePolicy, RenderTextureTargetFormat, RenderVertexBufferLayout,
        RenderVertexFormat, compile_flow_plan,
    };

    fn binding_in_group(group: u64, binding: u64) -> GpuBindingKey {
        GpuBindingKey::try_new(group, binding).expect("test binding key should be valid")
    }

    fn binding(index: u64) -> GpuBindingKey {
        binding_in_group(0, index)
    }

    #[derive(Debug, Clone, Copy, GpuStorage)]
    struct TestCell {
        value: u32,
    }

    #[derive(Debug, Clone, Copy, GpuUniform)]
    struct TestParams {
        value: u32,
    }

    #[derive(Debug, Clone, Copy, GpuUniform)]
    struct OtherTestParams {
        value: u32,
    }

    #[derive(Debug, Clone, runen_ecs::Resource)]
    struct TestState {
        value: u32,
    }

    impl TestState {
        fn params(&self) -> TestParams {
            TestParams { value: self.value }
        }

        fn other_params(&self) -> OtherTestParams {
            OtherTestParams { value: self.value }
        }

        fn dispatch(&self) -> [u32; 3] {
            [1, 1, 1]
        }
    }

    #[test]
    fn foreign_uniform_handle_is_rejected_even_when_local_components_match() {
        let (first_flow, foreign_handle) = RenderFlow::new("foreign.uniform.owner")
            .uniform_buffer::<TestParams>("uniform")
            .expect("render flow authoring should succeed");
        let (receiving_flow, receiving_handle) = RenderFlow::new("receiving.uniform.owner")
            .uniform_buffer::<TestParams>("uniform")
            .expect("render flow authoring should succeed");
        let foreign_id = foreign_handle.diagnostic_identity();
        let receiving_id = receiving_handle.diagnostic_identity();

        assert_eq!(
            foreign_id.diagnostic_parts().1,
            receiving_id.diagnostic_parts().1
        );
        assert_ne!(foreign_id, receiving_id);

        let receiving_flow = receiving_flow
            .with_state::<TestState>()
            .compute_pass("foreign.uniform")
            .uniform_from_state_to(binding(0), foreign_handle, TestState::params)
            .finish();
        let error = receiving_flow
            .validation_report()
            .expect_err("foreign uniform handle must be rejected");

        assert!(error.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::MissingUniformBuffer { uniform_id, .. }
                if *uniform_id == foreign_id
        )));
        drop(first_flow);
    }

    #[test]
    fn uniform_handle_projection_type_mismatch_is_rejected() {
        let (flow, handle) = RenderFlow::new("uniform.layout.mismatch")
            .with_state::<TestState>()
            .uniform_buffer::<TestParams>("uniform")
            .expect("render flow authoring should succeed");
        let flow = flow
            .compute_pass("uniform.layout.mismatch")
            .uniform_from_state_to(binding(0), handle, TestState::other_params)
            .finish();

        let error = flow
            .validation_report()
            .expect_err("same-size but different uniform parameter types must be rejected");
        assert!(error.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::UniformBufferTypeMismatch { .. }
        )));
    }

    #[test]
    fn uniform_handle_cannot_be_bound_as_storage() {
        let (flow, handle) = RenderFlow::new("uniform.storage.mismatch")
            .uniform_buffer::<TestParams>("uniform")
            .expect("render flow authoring should succeed");
        let flow = flow
            .compute_pass("uniform.storage.mismatch")
            .bind_storage(binding(0), handle)
            .finish();

        let error = flow
            .validation_report()
            .expect_err("uniform handles must not be compiled as storage bindings");
        assert!(error.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::UniformBufferUsedAsStorage { .. }
        )));
    }

    #[test]
    fn foreign_storage_handle_is_rejected_even_when_local_components_match() {
        let (first_flow, foreign_handle) = RenderFlow::new("foreign.storage.owner")
            .storage_array::<TestCell>("storage", 4)
            .expect("render flow authoring should succeed");
        let (receiving_flow, receiving_handle) = RenderFlow::new("receiving.storage.owner")
            .storage_array::<TestCell>("storage", 4)
            .expect("render flow authoring should succeed");
        let foreign_id = foreign_handle.diagnostic_identity();
        let receiving_id = receiving_handle.diagnostic_identity();

        assert_eq!(
            foreign_id.diagnostic_parts().1,
            receiving_id.diagnostic_parts().1
        );
        assert_ne!(foreign_id, receiving_id);

        let receiving_flow = receiving_flow
            .compute_pass("foreign.storage")
            .bind_storage(binding(0), foreign_handle)
            .finish();
        let error = receiving_flow
            .validation_report()
            .expect_err("foreign storage handle must be rejected");

        assert!(error.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::UnknownResourceReference { resource_id, .. }
                if *resource_id == foreign_id
        )));
        drop(first_flow);
    }

    #[test]
    fn public_authoring_path_supports_compute_graphics_copy_and_present() {
        let flow = RenderFlow::new("test.flow")
            .with_state::<TestState>()
            .with_surface_color()
            .expect("render flow authoring should succeed")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .with_history_texture("test.history")
            .expect("render flow authoring should succeed")
            .double_buffer_storage_array::<TestCell>("test.cells", 4)
            .expect("render flow authoring should succeed")
            .compute_pass("test.compute")
            .uniform_from_state(binding(0), TestState::params)
            .expect("render flow authoring should succeed")
            .bind_ping_pong_storage(binding(1), binding(2), "test.cells")
            .dispatch_from_state(TestState::dispatch)
            .finish()
            .graphics_pass("test.graphics")
            .uniform_from_state(binding(0), TestState::params)
            .expect("render flow authoring should succeed")
            .bind_ping_pong_storage(binding(1), binding(2), "test.cells")
            .write_color_target("test.color")
            .draw(3, 1)
            .finish()
            .copy_pass("test.history")
            .source("test.color")
            .destination("test.history")
            .finish()
            .present_pass("test.present")
            .expect("render flow authoring should succeed")
            .source("test.color")
            .order_after("test.history")
            .finish()
            .validate()
            .expect("public render-flow path should validate");

        let labels = flow
            .graph()
            .passes
            .passes
            .iter()
            .map(|pass| (pass.label.as_str(), pass.kind))
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec![
                ("test.compute", RenderPassKind::Compute),
                ("test.graphics", RenderPassKind::Graphics),
                ("test.history", RenderPassKind::Copy),
                ("test.present", RenderPassKind::Present),
            ]
        );

        let plan = compile_flow_plan(&flow).expect("validated flow should compile");
        assert!(matches!(
            plan.render_passes.as_slice(),
            [
                CompiledPassDescriptor::Compute(_),
                CompiledPassDescriptor::Graphics(_),
                CompiledPassDescriptor::Copy(_),
                CompiledPassDescriptor::Present(_),
            ]
        ));
    }

    #[test]
    fn fixed_step_region_compiles_graph_owned_repeat_metadata() {
        let (flow, cells) = RenderFlow::new("test.fixed")
            .with_state::<TestState>()
            .storage_array::<TestCell>("test.cells", 4)
            .expect("render flow authoring should succeed");
        let flow = flow
            .compute_pass("test.step.a")
            .uniform_from_state(binding(0), TestState::params)
            .expect("render flow authoring should succeed")
            .bind_storage(binding(1), cells.clone())
            .dispatch_from_state(TestState::dispatch)
            .finish()
            .compute_pass("test.step.b")
            .uniform_from_state(binding(0), TestState::params)
            .expect("render flow authoring should succeed")
            .bind_storage(binding(1), cells)
            .dispatch_from_state(TestState::dispatch)
            .finish()
            .fixed_step_region(
                "test.simulation",
                4,
                [("test.step.a", binding(2)), ("test.step.b", binding(2))],
            )
            .expect("render flow authoring should succeed")
            .validate()
            .expect("fixed-step region should validate");

        let plan = compile_flow_plan(&flow).expect("fixed-step flow should compile");
        assert_eq!(plan.execution.fixed_step_regions.len(), 1);
        let region = &plan.execution.fixed_step_regions[0];
        assert_eq!(region.region_label, "test.simulation");
        assert_eq!(region.max_substeps, 4);
        assert_eq!(region.pass_ids.len(), 2);

        for pass in &plan.execution.passes {
            let crate::plugins::render::CompiledPassExecutionPlan::Compute(pass) = pass else {
                panic!("test flow should only compile compute passes");
            };
            assert!(
                pass.bindings
                    .uniform_order
                    .contains(&region.iteration_uniform)
            );
        }
    }

    #[test]
    fn fixed_step_region_rejects_interleaved_pass_order() {
        let (flow, cells) = RenderFlow::new("test.fixed.interleaved")
            .with_state::<TestState>()
            .storage_array::<TestCell>("test.cells", 4)
            .expect("render flow authoring should succeed");
        let err = flow
            .compute_pass("test.step.a")
            .uniform_from_state(binding(0), TestState::params)
            .expect("render flow authoring should succeed")
            .bind_storage(binding(1), cells.clone())
            .dispatch_from_state(TestState::dispatch)
            .finish()
            .compute_pass("test.outside")
            .uniform_from_state(binding(0), TestState::params)
            .expect("render flow authoring should succeed")
            .bind_storage(binding(1), cells.clone())
            .dispatch_from_state(TestState::dispatch)
            .finish()
            .compute_pass("test.step.b")
            .uniform_from_state(binding(0), TestState::params)
            .expect("render flow authoring should succeed")
            .bind_storage(binding(1), cells)
            .dispatch_from_state(TestState::dispatch)
            .finish()
            .fixed_step_region(
                "test.simulation",
                4,
                [("test.step.a", binding(2)), ("test.step.b", binding(2))],
            )
            .expect("render flow authoring should succeed")
            .validation_report()
            .expect_err("interleaved repeat region must be rejected");

        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::FixedStepRegionPassesNotContiguous { .. }
        )));
    }

    #[test]
    fn shader_bindings_reject_duplicate_keys_and_non_primary_groups() {
        let (flow, cells) = RenderFlow::new("test.binding.duplicate")
            .with_state::<TestState>()
            .storage_array::<TestCell>("test.cells", 4)
            .expect("render flow authoring should succeed");
        let duplicate_key = binding(0);
        let duplicate = flow
            .compute_pass("test.binding.duplicate")
            .uniform_from_state(duplicate_key, TestState::params)
            .expect("render flow authoring should succeed")
            .bind_storage(duplicate_key, cells)
            .dispatch_from_state(TestState::dispatch)
            .finish()
            .validation_report()
            .expect_err("duplicate shader keys must fail flow validation");
        assert!(duplicate.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::DuplicateShaderBindingKey { key, .. }
                if *key == duplicate_key
        )));

        let non_primary_key = binding_in_group(1, 0);
        let non_primary = RenderFlow::new("test.binding.non-primary")
            .with_state::<TestState>()
            .compute_pass("test.binding.non-primary")
            .uniform_from_state(non_primary_key, TestState::params)
            .expect("render flow authoring should succeed")
            .dispatch_from_state(TestState::dispatch)
            .finish()
            .validation_report()
            .expect_err("non-primary shader groups must fail flow validation");
        assert!(non_primary.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::ShaderBindingOutsidePrimaryGroup { key, .. }
                if *key == non_primary_key
        )));
    }

    #[test]
    fn with_color_target_exact_declares_surface_sized_exact_format() {
        let flow = RenderFlow::new("test.exact.color")
            .with_color_target_exact("test.proof", RenderTextureTargetFormat::Rgba8Unorm)
            .expect("render flow authoring should succeed");

        let id = flow
            .resource_id("test.proof")
            .expect("exact color target should be registered");
        let resource = flow
            .graph()
            .resources
            .resources
            .iter()
            .find(|resource| *resource.id() == id)
            .expect("registered target should have a descriptor");

        let RenderResourceDeclaration::ColorAttachment(value) = resource else {
            panic!("exact color target should remain a color target");
        };
        assert_eq!(value.texture.size, RenderTextureSizePolicy::Surface);
        assert_eq!(
            value.texture.format,
            RenderTextureFormatPolicy::Exact(RenderTextureTargetFormat::Rgba8Unorm)
        );
    }

    #[test]
    fn exact_color_target_rejects_depth_format() {
        let err = RenderFlow::new("test.exact.color.depth")
            .with_color_target_exact("test.proof", RenderTextureTargetFormat::Depth32Float)
            .expect("render flow authoring should succeed")
            .validation_report()
            .expect_err("color target must not resolve to depth format");

        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::InvalidTextureFormatClass {
                resource_kind: "color_target",
                format: RenderTextureTargetFormat::Depth32Float,
                ..
            }
        )));
    }

    #[test]
    fn depth_target_rejects_color_format() {
        let mut flow = RenderFlow::new("test.depth.color")
            .with_depth_target("test.depth")
            .expect("render flow authoring should succeed");
        let id = flow
            .resource_id("test.depth")
            .expect("depth target should be registered");
        let resource = flow
            .graph
            .resources
            .resources
            .iter_mut()
            .find(|resource| *resource.id() == id)
            .expect("registered target should have a descriptor");

        let RenderResourceDeclaration::DepthAttachment(value) = resource else {
            panic!("registered resource should be a depth target");
        };
        value.texture.format =
            RenderTextureFormatPolicy::Exact(RenderTextureTargetFormat::Rgba8Unorm);

        let err = flow
            .validation_report()
            .expect_err("depth target must resolve to depth/stencil format");

        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::InvalidTextureFormatClass {
                resource_kind: "depth_target",
                format: RenderTextureTargetFormat::Rgba8Unorm,
                ..
            }
        )));
    }

    #[test]
    fn graphics_pass_with_vertex_buffer_layout_validates_and_plans_layout() {
        let (flow, vertices) = RenderFlow::new("test.graphics.vertex")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .storage_array::<TestCell>("test.vertices", 3)
            .expect("render flow authoring should succeed");

        let flow = flow
            .graphics_pass("test.draw")
            .vertex_buffer(
                vertices,
                RenderVertexBufferLayout::vertex(0, 4).attribute(0, 0, RenderVertexFormat::Uint32),
            )
            .write_color_target("test.color")
            .draw(3, 1)
            .finish()
            .validate()
            .expect("graphics pass with vertex buffer layout should validate");

        let plan = compile_flow_plan(&flow).expect("validated flow should compile");
        let Some(crate::plugins::render::CompiledPassExecutionPlan::Graphics(pass)) =
            plan.execution.passes.first()
        else {
            panic!("first execution pass should be graphics");
        };
        assert_eq!(pass.draw.expect("draw should compile").vertex_count, 3);
        assert_eq!(pass.draw_buffers.vertex_buffers.len(), 1);
        assert_eq!(pass.draw_buffers.vertex_buffers[0].layout.slot, 0);
        assert_eq!(
            pass.draw_buffers.vertex_buffers[0].layout.attributes[0].shader_location,
            0
        );
    }

    #[test]
    fn graphics_buffer_roles_require_matching_normalized_usage() {
        let (flow, uniform) = RenderFlow::new("test.graphics.invalid.buffer.role")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .uniform_buffer::<TestParams>("test.uniform")
            .expect("render flow authoring should succeed");

        let error = flow
            .graphics_pass("test.draw")
            .vertex_buffer(
                uniform,
                RenderVertexBufferLayout::vertex(0, 4).attribute(0, 0, RenderVertexFormat::Uint32),
            )
            .write_color_target("test.color")
            .draw(3, 1)
            .finish()
            .validation_report()
            .expect_err("uniform-only buffers must not validate as vertex buffers");

        assert!(error.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::MissingBufferRoleUsage { .. }
        )));
    }

    #[test]
    fn graphics_pass_with_instance_buffer_layout_validates() {
        let (flow, instances) = RenderFlow::new("test.graphics.instance")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .storage_array::<TestCell>("test.instances", 4)
            .expect("render flow authoring should succeed");

        flow.graphics_pass("test.draw")
            .instance_buffer(
                instances,
                RenderVertexBufferLayout::instance(0, 4).attribute(
                    0,
                    0,
                    RenderVertexFormat::Uint32,
                ),
            )
            .write_color_target("test.color")
            .draw(3, 4)
            .finish()
            .validate()
            .expect("graphics pass with instance buffer layout should validate");
    }

    #[test]
    fn graphics_pass_explicit_indirect_draw_compiles_draw_source() {
        let (flow, args) = RenderFlow::new("test.graphics.indirect")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .storage_array::<DrawIndirectArgs>("test.draw.args", 1)
            .expect("render flow authoring should succeed");

        let flow = flow
            .graphics_pass("test.draw")
            .write_color_target("test.color")
            .draw_indirect(args, 3, 64)
            .expect("declared draw-argument layout should match")
            .finish()
            .validate()
            .expect("graphics pass with explicit indirect draw should validate");

        let plan = compile_flow_plan(&flow).expect("validated flow should compile");
        let Some(crate::plugins::render::CompiledPassExecutionPlan::Graphics(pass)) =
            plan.execution.passes.first()
        else {
            panic!("first execution pass should be graphics");
        };
        let draw = pass.draw.expect("draw should compile");
        assert_eq!(draw.vertex_count, 3);
        assert_eq!(draw.instance_count, 64);
        assert!(matches!(
            draw.source,
            CompiledDrawSource::Indirect { byte_offset: 0, .. }
        ));
        assert_eq!(pass.draw_buffers.indirect_buffers.len(), 1);
    }

    #[test]
    fn graphics_pass_reports_mismatched_indirect_buffer_layout() {
        let (flow, args) = RenderFlow::new("test.graphics.indirect.layout")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .storage_array::<DrawIndexedIndirectArgs>("test.draw.indexed_args", 1)
            .expect("render flow authoring should succeed");

        assert!(matches!(
            flow.graphics_pass("test.draw")
                .write_color_target("test.color")
                .draw_indirect(args, 3, 1),
            Err(RenderFlowAuthoringError::BufferLayoutMismatch { .. })
        ));
    }

    #[test]
    fn graphics_pass_rejects_unaligned_indirect_draw_offset() {
        let (flow, args) = RenderFlow::new("test.graphics.indirect.unaligned")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .storage_array::<DrawIndirectArgs>("test.draw.args", 1)
            .expect("render flow authoring should succeed");

        let err = flow
            .graphics_pass("test.draw")
            .write_color_target("test.color")
            .draw_indirect_with_offsets(args, 3, 64, 0, 0, 2)
            .expect("declared draw-argument layout should match")
            .finish()
            .validation_report()
            .expect_err("unaligned indirect draw offset should be rejected");

        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::GraphicsPassInvalidIndirectDrawOffset { .. }
        )));
    }

    #[test]
    fn graphics_pass_rejects_indirect_buffer_sidecar_on_direct_draw() {
        let (flow, args) = RenderFlow::new("test.graphics.indirect.sidecar")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .storage_array::<DrawIndirectArgs>("test.draw.args", 1)
            .expect("render flow authoring should succeed");

        let err = flow
            .graphics_pass("test.draw")
            .write_color_target("test.color")
            .indirect_buffer(args)
            .draw(3, 64)
            .finish()
            .validation_report()
            .expect_err("direct draw with indirect buffer sidecar should be rejected");

        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::GraphicsPassIndirectBufferWithoutIndirectDraw { .. }
        )));
    }

    #[test]
    fn graphics_vertex_buffer_without_layout_is_rejected() {
        let (mut flow, vertices) = RenderFlow::new("test.graphics.missing_layout")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .storage_array::<TestCell>("test.vertices", 3)
            .expect("render flow authoring should succeed");

        flow = flow
            .graphics_pass("test.draw")
            .write_color_target("test.color")
            .draw(3, 1)
            .finish();
        let pass = flow
            .graph
            .passes
            .passes
            .iter_mut()
            .find(|pass| pass.label == "test.draw")
            .expect("draw pass should exist");
        pass.vertex_buffers.push(vertices.diagnostic_identity());

        let err = flow
            .validation_report()
            .expect_err("vertex buffer without layout should be rejected");
        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::GraphicsPassBufferLayoutCountMismatch { .. }
        )));
    }

    #[test]
    fn graphics_missing_draw_is_rejected() {
        let err = RenderFlow::new("test.graphics.missing_draw")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .graphics_pass("test.draw")
            .write_color_target("test.color")
            .finish()
            .validation_report()
            .expect_err("graphics pass without draw parameters should be rejected");

        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::GraphicsPassMissingDraw { .. }
        )));
    }

    #[test]
    fn graphics_invalid_vertex_layout_shape_is_rejected() {
        let (flow, vertices) = RenderFlow::new("test.graphics.zero_stride")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .storage_array::<TestCell>("test.vertices", 3)
            .expect("render flow authoring should succeed");

        let err = flow
            .graphics_pass("test.draw")
            .vertex_buffer(
                vertices,
                RenderVertexBufferLayout::vertex(0, 0).attribute(0, 0, RenderVertexFormat::Uint32),
            )
            .write_color_target("test.color")
            .draw(3, 1)
            .finish()
            .validation_report()
            .expect_err("zero vertex stride should be rejected");

        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::GraphicsPassInvalidVertexStride { .. }
        )));

        let (flow, vertices) = RenderFlow::new("test.graphics.invalid_layout")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .storage_array::<TestCell>("test.vertices", 3)
            .expect("render flow authoring should succeed");

        let err = flow
            .graphics_pass("test.draw")
            .vertex_buffer(
                vertices,
                RenderVertexBufferLayout::vertex(0, 4).attribute(
                    0,
                    0,
                    RenderVertexFormat::Float32x2,
                ),
            )
            .write_color_target("test.color")
            .draw(3, 1)
            .finish()
            .validation_report()
            .expect_err("vertex attribute extending beyond stride should be rejected");

        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::GraphicsPassInvalidVertexAttributeRange { .. }
        )));
    }

    #[test]
    fn graphics_duplicate_vertex_buffer_slots_are_rejected() {
        let (flow, vertices) = RenderFlow::new("test.graphics.duplicate_slots")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .storage_array::<TestCell>("test.vertices", 3)
            .expect("render flow authoring should succeed");
        let (flow, instances) = flow
            .storage_array::<TestCell>("test.instances", 4)
            .expect("render flow authoring should succeed");

        let err = flow
            .graphics_pass("test.draw")
            .vertex_buffer(
                vertices,
                RenderVertexBufferLayout::vertex(0, 4).attribute(0, 0, RenderVertexFormat::Uint32),
            )
            .instance_buffer(
                instances,
                RenderVertexBufferLayout::instance(0, 4).attribute(
                    1,
                    0,
                    RenderVertexFormat::Uint32,
                ),
            )
            .write_color_target("test.color")
            .draw(3, 4)
            .finish()
            .validation_report()
            .expect_err("duplicate vertex buffer slots should be rejected");

        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::GraphicsPassDuplicateVertexBufferSlot { .. }
        )));
    }

    #[test]
    fn graphics_duplicate_vertex_shader_locations_are_rejected() {
        let (flow, vertices) = RenderFlow::new("test.graphics.duplicate_locations")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .storage_array::<TestCell>("test.vertices", 3)
            .expect("render flow authoring should succeed");
        let (flow, instances) = flow
            .storage_array::<TestCell>("test.instances", 4)
            .expect("render flow authoring should succeed");

        let err = flow
            .graphics_pass("test.draw")
            .vertex_buffer(
                vertices,
                RenderVertexBufferLayout::vertex(0, 4).attribute(0, 0, RenderVertexFormat::Uint32),
            )
            .instance_buffer(
                instances,
                RenderVertexBufferLayout::instance(1, 4).attribute(
                    0,
                    0,
                    RenderVertexFormat::Uint32,
                ),
            )
            .write_color_target("test.color")
            .draw(3, 4)
            .finish()
            .validation_report()
            .expect_err("duplicate shader locations should be rejected");

        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::GraphicsPassDuplicateVertexShaderLocation { .. }
        )));
    }

    #[test]
    fn graphics_color_output_arity_matches_runtime_contract() {
        let err = RenderFlow::new("test.graphics.zero_color")
            .graphics_pass("test.draw")
            .draw(3, 1)
            .finish()
            .validation_report()
            .expect_err("graphics pass without color output should be rejected");
        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::GraphicsPassInvalidColorOutputArity { write_count: 0, .. }
        )));

        let err = RenderFlow::new("test.graphics.multiple_color")
            .with_color_target("test.a")
            .expect("render flow authoring should succeed")
            .with_color_target("test.b")
            .expect("render flow authoring should succeed")
            .graphics_pass("test.draw")
            .write_color_target("test.a")
            .write_color_target("test.b")
            .finish()
            .validation_report()
            .expect_err("graphics pass with multiple color outputs should be rejected");
        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::GraphicsPassInvalidColorOutputArity { write_count: 2, .. }
        )));
    }

    #[test]
    fn graphics_rejects_non_color_attachment_outputs() {
        let (flow, _cells) = RenderFlow::new("test.graphics.storage_output")
            .storage_array::<TestCell>("test.cells", 4)
            .expect("render flow authoring should succeed");

        let err = flow
            .graphics_pass("test.draw")
            .write_color_target("test.cells")
            .draw(3, 1)
            .finish()
            .validation_report()
            .expect_err("storage buffer cannot be a raster color output");
        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::InvalidRasterColorOutputResource { .. }
        )));

        let err = RenderFlow::new("test.graphics.depth_output")
            .with_depth_target("test.depth")
            .expect("render flow authoring should succeed")
            .graphics_pass("test.draw")
            .write_color_target("test.depth")
            .draw(3, 1)
            .finish()
            .validation_report()
            .expect_err("depth target cannot be a raster color output");
        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::InvalidRasterColorOutputResource { .. }
        )));
    }

    #[test]
    fn graphics_accepts_runtime_supported_color_outputs() {
        RenderFlow::new("test.graphics.color_output")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .graphics_pass("test.draw")
            .write_color_target("test.color")
            .draw(3, 1)
            .finish()
            .validate()
            .expect("flow-owned color target should validate as graphics color output");

        RenderFlow::new("test.graphics.surface_output")
            .with_surface_color()
            .expect("render flow authoring should succeed")
            .graphics_pass("test.draw")
            .write_surface_color()
            .expect("render flow authoring should succeed")
            .draw(3, 1)
            .finish()
            .validate()
            .expect("imported surface color should validate as graphics color output");
    }

    #[test]
    fn fullscreen_color_output_arity_matches_runtime_contract() {
        let err = RenderFlow::new("test.fullscreen.zero_color")
            .fullscreen_pass("test.compose")
            .finish()
            .validation_report()
            .expect_err("fullscreen pass without color output should be rejected");
        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::FullscreenPassInvalidColorOutputArity { write_count: 0, .. }
        )));

        let err = RenderFlow::new("test.fullscreen.multiple_color")
            .with_color_target("test.a")
            .expect("render flow authoring should succeed")
            .with_color_target("test.b")
            .expect("render flow authoring should succeed")
            .fullscreen_pass("test.compose")
            .write_color_target("test.a")
            .write_color_target("test.b")
            .finish()
            .validation_report()
            .expect_err("fullscreen pass with multiple color outputs should be rejected");
        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::FullscreenPassInvalidColorOutputArity { write_count: 2, .. }
        )));
    }

    #[test]
    fn fullscreen_rejects_non_color_attachment_outputs() {
        let (flow, _cells) = RenderFlow::new("test.fullscreen.storage_output")
            .storage_array::<TestCell>("test.cells", 4)
            .expect("render flow authoring should succeed");

        let err = flow
            .fullscreen_pass("test.compose")
            .write_color_target("test.cells")
            .finish()
            .validation_report()
            .expect_err("storage buffer cannot be a fullscreen color output");
        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::InvalidRasterColorOutputResource { .. }
        )));

        let err = RenderFlow::new("test.fullscreen.depth_output")
            .with_depth_target("test.depth")
            .expect("render flow authoring should succeed")
            .fullscreen_pass("test.compose")
            .write_color_target("test.depth")
            .finish()
            .validation_report()
            .expect_err("depth target cannot be a fullscreen color output");
        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::InvalidRasterColorOutputResource { .. }
        )));
    }

    #[test]
    fn fullscreen_accepts_runtime_supported_color_outputs() {
        RenderFlow::new("test.fullscreen.color_output")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .fullscreen_pass("test.compose")
            .write_color_target("test.color")
            .finish()
            .validate()
            .expect("flow-owned color target should validate as fullscreen color output");

        RenderFlow::new("test.fullscreen.surface_output")
            .with_surface_color()
            .expect("render flow authoring should succeed")
            .fullscreen_pass("test.compose")
            .write_surface_color()
            .expect("render flow authoring should succeed")
            .finish()
            .validate()
            .expect("imported surface color should validate as fullscreen color output");
    }

    #[test]
    fn fullscreen_clear_color_is_part_of_the_raster_load_contract() {
        let flow = RenderFlow::new("test.fullscreen.clear")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .fullscreen_pass("test.compose")
            .clear_color([0.1, 0.2, 0.3, 1.0])
            .write_color_target("test.color")
            .finish()
            .validate()
            .expect("fullscreen clear color should validate");
        let pass = flow
            .graph()
            .passes
            .passes
            .iter()
            .find(|pass| pass.label == "test.compose")
            .expect("fullscreen pass should be registered");

        assert_eq!(pass.clear_color, Some([0.1, 0.2, 0.3, 1.0]));
    }

    #[test]
    fn imported_surface_depth_is_rejected_as_graphics_depth_target() {
        let err = RenderFlow::new("test.graphics.surface_depth")
            .with_surface_depth()
            .expect("render flow authoring should succeed")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .graphics_pass("test.draw")
            .write_color_target("test.color")
            .depth_target(SURFACE_DEPTH_RESOURCE_LABEL)
            .draw(3, 1)
            .finish()
            .validation_report()
            .expect_err("imported surface depth is not runtime-backed");

        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::InvalidDepthTargetResource { .. }
        )));
    }

    #[test]
    fn copy_pass_rejects_mixed_texture_and_buffer_resources() {
        let (flow, _cells) = RenderFlow::new("test.copy.invalid")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .storage_array::<TestCell>("test.cells", 4)
            .expect("render flow authoring should succeed");

        let err = flow
            .copy_pass("test.copy")
            .source("test.color")
            .destination("test.cells")
            .finish()
            .validation_report()
            .expect_err("texture-to-buffer copy should be rejected");

        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::CopyPassMixedResourceClasses { .. }
        )));
    }

    #[test]
    fn present_pass_rejects_non_terminal_dependents() {
        let err = RenderFlow::new("test.present.invalid")
            .with_surface_color()
            .expect("render flow authoring should succeed")
            .with_color_target("test.color")
            .expect("render flow authoring should succeed")
            .with_color_target("test.after")
            .expect("render flow authoring should succeed")
            .fullscreen_pass("test.compose")
            .write_color_target("test.color")
            .finish()
            .present_pass("test.present")
            .expect("render flow authoring should succeed")
            .source("test.color")
            .finish()
            .fullscreen_pass("test.after")
            .write_color_target("test.after")
            .order_after("test.present")
            .finish()
            .validation_report()
            .expect_err("present pass should reject downstream dependents");

        assert!(err.issues.iter().any(|issue| matches!(
            issue,
            RenderFlowValidationIssue::PresentPassNotTerminal { .. }
        )));
    }
}
