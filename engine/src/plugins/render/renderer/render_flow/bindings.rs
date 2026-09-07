use super::*;
use crate::plugins::render::pipelines::FlowPassPipelineDescriptor;
use crate::plugins::render::renderer::resource_descriptors::linear_sampler_descriptor;
use crate::plugins::render::{
    RenderBlendMode, RenderCullMode, RenderDepthPolicy, RenderFeatureId, RenderPassId,
    RenderPrimitiveTopology, RenderRasterState, RenderVertexFormat, RenderVertexStepMode,
};
use runen_gpu::{
    GpuAdmittedProgramSource, GpuBindingKey, GpuBindingLayoutRefinement,
    GpuBlendMode as GpuPipelineBlendMode, GpuColorTargetStateDescriptor, GpuColorWriteMask,
    GpuCompareFunction, GpuComputePipelineDescriptor, GpuCullMode as GpuPipelineCullMode,
    GpuDepthStencilStateDescriptor, GpuEntryPointName, GpuFragmentOutputStateDescriptor,
    GpuFrontFace, GpuIndexFormat, GpuMultisampleStateDescriptor, GpuPipelineConfiguration,
    GpuPrimitiveStateDescriptor, GpuPrimitiveTopology as GpuPipelinePrimitiveTopology,
    GpuProgramDescriptor, GpuRealizedPipelineLayout, GpuRealizedProgram, GpuRenderEntryPoints,
    GpuRenderPipelineDescriptor, GpuRenderPipelineStateDescriptor, GpuRuntimeBindingResource,
    GpuRuntimeBindingSet, GpuRuntimeBindingValue, GpuRuntimeBufferBinding,
    GpuRuntimeTextureViewBinding, GpuSamplerClass, GpuSamplerHandle, GpuSpecializationValueSet,
    GpuTextureFormat, GpuTextureSampleClass, GpuTextureViewDimension, GpuTextureViewHandle,
    GpuVertexAttribute, GpuVertexBufferLayoutDescriptor, GpuVertexFormat,
    GpuVertexInputStateDescriptor, GpuVertexStepMode,
};
use std::num::NonZeroU64;

enum RuntimeBindingResource {
    TextureView(GpuTextureViewHandle),
    Buffer {
        handle: runen_gpu::GpuBufferHandle,
        size: u64,
    },
    Sampler,
}

struct RuntimeBindingResolved {
    key: GpuBindingKey,
    resource: RuntimeBindingResource,
    refinement: Option<GpuBindingLayoutRefinement>,
}

/// G4C2/G4C3 program and pipeline-layout realization plus canonical runtime binding descriptors.
pub(in crate::plugins::render::renderer) struct RealizedFlowProgramBindings {
    pub(super) pipeline_key: FlowPassPipelineKey,
    pub(super) runtime_bindings: GpuRuntimeBindingSet,
    pub(super) program: GpuRealizedProgram,
    pub(super) pipeline_layout: GpuRealizedPipelineLayout,
}

impl Renderer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_compiled_bind_group(
        &mut self,
        context: &GpuContext,
        packet: &RendererPreparedPacket,
        flow: &CompiledRenderFlowPlan,
        pass_id: RenderPassId,
        pass_kind: FlowPassKind,
        pass_feature_id: Option<RenderFeatureId>,
        program_source: &GpuAdmittedProgramSource,
        specialization: GpuSpecializationValueSet,
        bindings: &CompiledPassBindings,
        allow_depth_sampling: bool,
        color_formats: Vec<GpuTextureFormat>,
        depth_format: Option<GpuTextureFormat>,
        runtime_resources: &FlowRuntimeResources,
    ) -> Result<RealizedFlowProgramBindings> {
        let mut resolved_entries = Vec::<RuntimeBindingResolved>::new();
        for entry in &bindings.bind_group.entries {
            match entry {
                CompiledBindingEntry::SampledTexture { key, resource } => {
                    let resource_key = runtime_resources.resolve_resource_key(
                        pass_id,
                        resource,
                        "sampled_texture",
                    )?;
                    let (texture_id, texture_view, is_depth) = match resource_key.clone() {
                        RuntimeResourceKey::DynamicTexture(key) => {
                            let texture =
                                self.dynamic_texture_targets.texture_ref(pass_id, &key)?;
                            (
                                texture.id.clone(),
                                resolved_binding_texture_view(&texture.id, texture.view_handle)?,
                                texture.is_depth,
                            )
                        }
                        _ => {
                            let (view_handle, _format, is_depth) = runtime_resources
                                .resolve_logical_texture_binding(pass_id, resource_key.clone())?;
                            (
                                resource_key,
                                RuntimeBindingResource::TextureView(view_handle.clone()),
                                is_depth,
                            )
                        }
                    };
                    if !allow_depth_sampling && is_depth {
                        bail!(
                            "pass '{}' samples depth texture '{}' but this pass type only supports color sampled textures",
                            pass_id,
                            texture_id
                        );
                    }
                    let refinement = (!is_depth).then(|| {
                        GpuBindingLayoutRefinement::new(*key)
                            .with_texture_sample_class(GpuTextureSampleClass::FloatFilterable)
                    });
                    resolved_entries.push(RuntimeBindingResolved {
                        key: *key,
                        resource: texture_view,
                        refinement,
                    });
                }
                CompiledBindingEntry::Sampler { key } => {
                    resolved_entries.push(RuntimeBindingResolved {
                        key: *key,
                        resource: RuntimeBindingResource::Sampler,
                        refinement: Some(
                            GpuBindingLayoutRefinement::new(*key)
                                .with_sampler_class(GpuSamplerClass::Filtering),
                        ),
                    });
                }
                CompiledBindingEntry::StorageTexture { key, resource, .. } => {
                    let resource_key = runtime_resources.resolve_resource_key(
                        pass_id,
                        resource,
                        "storage_texture",
                    )?;
                    let (texture_id, texture_view, is_depth) = match resource_key.clone() {
                        RuntimeResourceKey::DynamicTexture(key) => {
                            let texture =
                                self.dynamic_texture_targets.texture_ref(pass_id, &key)?;
                            (
                                texture.id.clone(),
                                resolved_binding_texture_view(&texture.id, texture.view_handle)?,
                                texture.is_depth,
                            )
                        }
                        _ => {
                            let (view_handle, _format, is_depth) = runtime_resources
                                .resolve_logical_texture_binding(pass_id, resource_key.clone())?;
                            (
                                resource_key,
                                RuntimeBindingResource::TextureView(view_handle.clone()),
                                is_depth,
                            )
                        }
                    };
                    if is_depth {
                        bail!(
                            "pass '{}' declares storage texture '{}' as depth; storage-texture bindings require color-like resources",
                            pass_id,
                            texture_id
                        );
                    }
                    resolved_entries.push(RuntimeBindingResolved {
                        key: *key,
                        resource: texture_view,
                        refinement: None,
                    });
                }
                CompiledBindingEntry::UniformBuffer { key, resource } => {
                    let buffer =
                        runtime_resources.resolve_uniform_buffer_for_pass(pass_id, *resource)?;
                    resolved_entries.push(RuntimeBindingResolved {
                        key: *key,
                        resource: RuntimeBindingResource::Buffer {
                            handle: buffer.handle.clone(),
                            size: buffer.size,
                        },
                        refinement: None,
                    });
                }
                CompiledBindingEntry::StorageBuffer { key, resource, .. } => {
                    let buffer = runtime_resources.resolve_storage_buffer_ref(pass_id, resource)?;
                    resolved_entries.push(RuntimeBindingResolved {
                        key: *key,
                        resource: RuntimeBindingResource::Buffer {
                            handle: buffer.handle.clone(),
                            size: buffer.size,
                        },
                        refinement: None,
                    });
                }
            }
        }

        let mut refinements = resolved_entries
            .iter()
            .filter_map(|entry| entry.refinement.clone())
            .collect::<Vec<_>>();
        refinements.extend(gpu_material_binding_refinements_for_pass(
            packet, flow, pass_id,
        )?);
        let render_pipeline_state =
            gpu_render_pipeline_state_for_pass(flow, pass_id, &color_formats, depth_format)?;
        let pipeline_descriptor = gpu_pipeline_descriptor_for_pass(
            program_source,
            pass_kind,
            render_pipeline_state,
            specialization,
            refinements,
        )?;

        let pipeline_key = FlowPassPipelineKey {
            flow_id: flow.flow_id,
            pass_id,
            pass_kind,
            feature_id: pass_feature_id,
            pipeline_descriptor,
        };

        let sampler = if resolved_entries
            .iter()
            .any(|entry| matches!(entry.resource, RuntimeBindingResource::Sampler))
        {
            Some(
                self.flow_pipeline_cache
                    .get_or_realize_sampler(
                        context,
                        pipeline_key.clone(),
                        linear_sampler_descriptor(
                            "engine_compiled_flow_sampler",
                            runen_gpu::GpuResourceLifetime::Retained,
                        )?,
                    )?
                    .handle()
                    .clone(),
            )
        } else {
            None
        };

        let primary_values = resolved_entries
            .iter()
            .map(|value| runtime_binding_value(value, sampler.as_ref()))
            .collect::<Result<Vec<_>>>()?;
        let material_values = gpu_material_runtime_binding_values_for_pass(packet, flow, pass_id)?;
        let runtime_bindings = GpuRuntimeBindingSet::new(
            pipeline_key.pipeline_descriptor.layout().clone(),
            primary_values.into_iter().chain(material_values),
        )?;

        let program = pollster::block_on(
            context.realize_program(pipeline_key.pipeline_descriptor.program()),
        )?;
        let realized_pipeline_layout = pollster::block_on(
            context.realize_pipeline_layout(pipeline_key.pipeline_descriptor.layout()),
        )?;
        Ok(RealizedFlowProgramBindings {
            pipeline_key,
            runtime_bindings,
            program,
            pipeline_layout: realized_pipeline_layout,
        })
    }
}

fn resolved_binding_texture_view(
    id: &RuntimeResourceKey,
    view_handle: Option<&GpuTextureViewHandle>,
) -> Result<RuntimeBindingResource> {
    let Some(view_handle) = view_handle else {
        bail!(
            "pass resource '{}' has no logical texture view for G4C2 shader binding realization",
            id
        );
    };
    Ok(RuntimeBindingResource::TextureView(view_handle.clone()))
}

fn runtime_binding_value(
    value: &RuntimeBindingResolved,
    sampler: Option<&GpuSamplerHandle>,
) -> Result<GpuRuntimeBindingValue> {
    let resource = match &value.resource {
        RuntimeBindingResource::TextureView(handle) => GpuRuntimeBindingResource::TextureView(
            GpuRuntimeTextureViewBinding::new(handle.clone(), GpuTextureViewDimension::D2),
        ),
        RuntimeBindingResource::Buffer { handle, size } => {
            let size = NonZeroU64::new(*size).ok_or_else(|| {
                anyhow::anyhow!(
                    "runtime binding '{}' resolved a zero-sized buffer, which cannot form a G4C2 binding range",
                    value.key
                )
            })?;
            GpuRuntimeBindingResource::Buffer(GpuRuntimeBufferBinding::new(
                handle.clone(),
                0,
                size,
                None,
            ))
        }
        RuntimeBindingResource::Sampler => GpuRuntimeBindingResource::Sampler(
            sampler
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "render-flow sampler binding '{}' lacks its G4C1 sampler handle",
                        value.key
                    )
                })?
                .clone(),
        ),
    };
    Ok(GpuRuntimeBindingValue::new(value.key, [resource])?)
}

fn gpu_pipeline_descriptor_for_pass(
    source: &GpuAdmittedProgramSource,
    pass_kind: FlowPassKind,
    render_state: Option<GpuRenderPipelineStateDescriptor>,
    specialization: GpuSpecializationValueSet,
    refinements: Vec<GpuBindingLayoutRefinement>,
) -> Result<FlowPassPipelineDescriptor> {
    match pass_kind {
        FlowPassKind::Compute => {
            if render_state.is_some() {
                bail!("compute pipeline descriptor cannot carry render state");
            }
            let entry_point = GpuEntryPointName::new("cs_main")?;
            let program =
                GpuProgramDescriptor::new(source.clone(), [entry_point.clone()], refinements)?;
            Ok(FlowPassPipelineDescriptor::Compute(
                GpuComputePipelineDescriptor::new(
                    program,
                    entry_point,
                    GpuPipelineConfiguration::new(Some(specialization), None),
                )?,
            ))
        }
        FlowPassKind::Fullscreen | FlowPassKind::Graphics => {
            let render_state = render_state.ok_or_else(|| {
                anyhow::anyhow!("render pipeline descriptor requires typed render state")
            })?;
            let vertex = GpuEntryPointName::new("vs_main")?;
            let fragment = GpuEntryPointName::new("fs_main")?;
            let program = GpuProgramDescriptor::new(
                source.clone(),
                [vertex.clone(), fragment.clone()],
                refinements,
            )?;
            Ok(FlowPassPipelineDescriptor::Render(
                GpuRenderPipelineDescriptor::new(
                    program,
                    GpuRenderEntryPoints::new(vertex, Some(fragment)),
                    render_state,
                    GpuPipelineConfiguration::new(Some(specialization), None),
                )?,
            ))
        }
        _ => bail!("pass kind '{pass_kind:?}' cannot construct a shader pipeline descriptor"),
    }
}

fn gpu_material_binding_refinements_for_pass(
    packet: &RendererPreparedPacket,
    flow: &CompiledRenderFlowPlan,
    pass_id: RenderPassId,
) -> Result<Vec<GpuBindingLayoutRefinement>> {
    let pass = flow
        .execution
        .passes
        .iter()
        .find(|pass| execution_pass_id(pass) == pass_id)
        .ok_or_else(|| {
            anyhow::anyhow!("pass '{pass_id}' is missing from compiled execution plan")
        })?;
    if !pass_consumes_material_resources(
        execution_pass_feature_id(pass),
        execution_pass_shader_reference(pass),
    ) {
        return Ok(Vec::new());
    }
    let Some(material) = packet.prepared_material.as_ref() else {
        return Ok(Vec::new());
    };
    gpu_material_binding_refinements(material)
}

fn sorted_material_texture_bindings(
    material: &crate::plugins::render::PreparedMaterialFeatureContribution,
) -> Vec<&crate::plugins::render::PreparedMaterialTextureBinding> {
    let mut bindings = material
        .instances
        .iter()
        .flat_map(|instance| instance.texture_bindings.iter())
        .collect::<Vec<_>>();
    bindings.sort_by_key(|binding| {
        (
            binding.bind_group,
            binding.texture_binding,
            binding.sampler_binding,
            binding.resource_slot_index,
        )
    });
    bindings
}

fn gpu_material_runtime_binding_values_for_pass(
    packet: &RendererPreparedPacket,
    flow: &CompiledRenderFlowPlan,
    pass_id: RenderPassId,
) -> Result<Vec<GpuRuntimeBindingValue>> {
    let pass = flow
        .execution
        .passes
        .iter()
        .find(|pass| execution_pass_id(pass) == pass_id)
        .ok_or_else(|| {
            anyhow::anyhow!("pass '{pass_id}' is missing from compiled execution plan")
        })?;
    if !pass_consumes_material_resources(
        execution_pass_feature_id(pass),
        execution_pass_shader_reference(pass),
    ) {
        return Ok(Vec::new());
    }

    let Some(material) = packet.prepared_material.as_ref() else {
        return Ok(Vec::new());
    };
    let bindings = sorted_material_texture_bindings(material);
    if bindings.is_empty() {
        return Ok(Vec::new());
    }
    let resources = packet.prepared_material_gpu_resources.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "pass '{}' requires material runtime bindings but material GPU resources are not resident",
            pass_id
        )
    })?;
    if bindings.len() != resources._texture_views.len()
        || bindings.len() != resources._samplers.len()
    {
        bail!(
            "pass '{}' material runtime binding cardinality diverged from prepared material resources: {} bindings, {} views, {} samplers",
            pass_id,
            bindings.len(),
            resources._texture_views.len(),
            resources._samplers.len()
        );
    }

    let mut values = Vec::with_capacity(bindings.len() * 2);
    for ((binding, view), sampler) in bindings
        .into_iter()
        .zip(&resources._texture_views)
        .zip(&resources._samplers)
    {
        let view_dimension = match binding.texture_kind {
            crate::plugins::render::PreparedMaterialTextureKind::Texture2D => {
                GpuTextureViewDimension::D2
            }
            crate::plugins::render::PreparedMaterialTextureKind::Texture3D => {
                GpuTextureViewDimension::D3
            }
        };
        values.push(GpuRuntimeBindingValue::new(
            GpuBindingKey::try_new(
                u64::from(binding.bind_group),
                u64::from(binding.texture_binding),
            )?,
            [GpuRuntimeBindingResource::TextureView(
                GpuRuntimeTextureViewBinding::new(view._handle.clone(), view_dimension),
            )],
        )?);
        values.push(GpuRuntimeBindingValue::new(
            GpuBindingKey::try_new(
                u64::from(binding.bind_group),
                u64::from(binding.sampler_binding),
            )?,
            [GpuRuntimeBindingResource::Sampler(sampler._handle.clone())],
        )?);
    }
    Ok(values)
}

fn gpu_material_binding_refinements(
    material: &crate::plugins::render::PreparedMaterialFeatureContribution,
) -> Result<Vec<GpuBindingLayoutRefinement>> {
    let bindings = sorted_material_texture_bindings(material);
    let mut refinements = Vec::with_capacity(bindings.len() * 2);
    for binding in bindings {
        if binding.bind_group != 1 {
            bail!(
                "material runtime binding uses group {}, but the renderer material contract requires exact group 1",
                binding.bind_group
            );
        }
        refinements.push(
            GpuBindingLayoutRefinement::new(GpuBindingKey::try_new(
                u64::from(binding.bind_group),
                u64::from(binding.texture_binding),
            )?)
            .with_texture_sample_class(GpuTextureSampleClass::FloatFilterable),
        );
        refinements.push(
            GpuBindingLayoutRefinement::new(GpuBindingKey::try_new(
                u64::from(binding.bind_group),
                u64::from(binding.sampler_binding),
            )?)
            .with_sampler_class(GpuSamplerClass::Filtering),
        );
    }
    refinements.sort_by_key(GpuBindingLayoutRefinement::key);
    if let Some(duplicate) = refinements
        .windows(2)
        .find(|pair| pair[0].key() == pair[1].key())
        .map(|pair| pair[0].key())
    {
        bail!("material runtime bindings duplicate final GPU key {duplicate}");
    }
    Ok(refinements)
}

fn gpu_render_pipeline_state_for_pass(
    flow: &CompiledRenderFlowPlan,
    pass_id: RenderPassId,
    color_formats: &[GpuTextureFormat],
    depth_format: Option<GpuTextureFormat>,
) -> Result<Option<GpuRenderPipelineStateDescriptor>> {
    let pass = flow
        .execution
        .passes
        .iter()
        .find(|pass| execution_pass_id(pass) == pass_id)
        .ok_or_else(|| {
            anyhow::anyhow!("pass '{pass_id}' is missing from compiled execution plan")
        })?;
    let vertex_input = gpu_vertex_input_state_for_execution_pass(pass, pass_id)?;

    match pass {
        CompiledPassExecutionPlan::Compute(_) => {
            if !color_formats.is_empty() || depth_format.is_some() {
                bail!(
                    "compute pass '{}' cannot carry render attachment state",
                    pass_id
                );
            }
            Ok(None)
        }
        CompiledPassExecutionPlan::Fullscreen(_) => {
            if depth_format.is_some() {
                bail!("fullscreen pass '{}' cannot carry depth state", pass_id);
            }
            let fragment_output = gpu_fragment_output_state(color_formats, RenderBlendMode::Alpha)?;
            Ok(Some(GpuRenderPipelineStateDescriptor::new(
                vertex_input,
                Some(fragment_output),
                GpuPrimitiveStateDescriptor::default(),
                None,
                GpuMultisampleStateDescriptor::default(),
            )?))
        }
        CompiledPassExecutionPlan::Graphics(plan) => {
            let fragment_output =
                gpu_fragment_output_state(color_formats, plan.raster_state.state.blend_mode)?;
            let primitive = gpu_primitive_state(plan.raster_state.state)?;
            let depth_stencil =
                gpu_depth_stencil_state(depth_format, plan.raster_state.state.depth_policy)?;
            Ok(Some(GpuRenderPipelineStateDescriptor::new(
                vertex_input,
                Some(fragment_output),
                primitive,
                depth_stencil,
                GpuMultisampleStateDescriptor::default(),
            )?))
        }
        _ => bail!(
            "pass '{}' cannot construct render-pipeline state for execution kind '{}'",
            pass_id,
            execution_pass_kind_name(pass)
        ),
    }
}

fn gpu_vertex_input_state_for_execution_pass(
    pass: &CompiledPassExecutionPlan,
    pass_id: RenderPassId,
) -> Result<GpuVertexInputStateDescriptor> {
    let layouts = match pass {
        CompiledPassExecutionPlan::Graphics(plan) => plan
            .draw_buffers
            .vertex_buffers
            .iter()
            .map(|binding| &binding.layout)
            .chain(plan.draw_buffers.instance_buffer_layouts.iter())
            .map(|layout| {
                GpuVertexBufferLayoutDescriptor::new(
                    layout.slot,
                    layout.array_stride,
                    gpu_vertex_step_mode(layout.step_mode),
                    layout.attributes.iter().map(|attribute| {
                        GpuVertexAttribute::new(
                            attribute.shader_location,
                            attribute.offset,
                            gpu_vertex_format(attribute.format),
                        )
                    }),
                )
            })
            .collect::<std::result::Result<Vec<_>, _>>()?,
        CompiledPassExecutionPlan::Compute(_) | CompiledPassExecutionPlan::Fullscreen(_) => {
            Vec::new()
        }
        _ => {
            bail!(
                "pass '{}' cannot construct pipeline vertex-input state for execution kind '{}'",
                pass_id,
                execution_pass_kind_name(pass)
            );
        }
    };

    Ok(GpuVertexInputStateDescriptor::new(layouts)?)
}

fn gpu_fragment_output_state(
    color_formats: &[GpuTextureFormat],
    blend_mode: RenderBlendMode,
) -> Result<GpuFragmentOutputStateDescriptor> {
    let targets = color_formats
        .iter()
        .copied()
        .map(|format| {
            let blend = if format == GpuTextureFormat::R32Uint
                || matches!(blend_mode, RenderBlendMode::Replace)
            {
                GpuPipelineBlendMode::Replace
            } else {
                GpuPipelineBlendMode::Alpha
            };
            Ok(GpuColorTargetStateDescriptor::new(
                format,
                blend,
                GpuColorWriteMask::ALL,
            )?)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(GpuFragmentOutputStateDescriptor::new(targets))
}

fn gpu_primitive_state(state: RenderRasterState) -> Result<GpuPrimitiveStateDescriptor> {
    let topology = match state.primitive_topology {
        RenderPrimitiveTopology::TriangleList => GpuPipelinePrimitiveTopology::TriangleList,
        RenderPrimitiveTopology::TriangleStrip => GpuPipelinePrimitiveTopology::TriangleStrip,
        RenderPrimitiveTopology::LineList => GpuPipelinePrimitiveTopology::LineList,
        RenderPrimitiveTopology::LineStrip => GpuPipelinePrimitiveTopology::LineStrip,
        RenderPrimitiveTopology::PointList => GpuPipelinePrimitiveTopology::PointList,
    };
    let strip_index_format = topology.is_strip().then_some(GpuIndexFormat::Uint32);
    let cull_mode = match state.cull_mode {
        RenderCullMode::None => GpuPipelineCullMode::None,
        RenderCullMode::Front => GpuPipelineCullMode::Front,
        RenderCullMode::Back => GpuPipelineCullMode::Back,
    };
    Ok(GpuPrimitiveStateDescriptor::new(
        topology,
        strip_index_format,
        GpuFrontFace::CounterClockwise,
        cull_mode,
    )?)
}

fn gpu_depth_stencil_state(
    depth_format: Option<GpuTextureFormat>,
    policy: RenderDepthPolicy,
) -> Result<Option<GpuDepthStencilStateDescriptor>> {
    let Some(format) = depth_format else {
        return Ok(None);
    };
    if matches!(policy, RenderDepthPolicy::Disabled) {
        return Ok(None);
    }
    Ok(Some(GpuDepthStencilStateDescriptor::new(
        format,
        !matches!(policy, RenderDepthPolicy::ReadOnly),
        GpuCompareFunction::LessEqual,
    )?))
}

fn gpu_vertex_step_mode(value: RenderVertexStepMode) -> GpuVertexStepMode {
    match value {
        RenderVertexStepMode::Vertex => GpuVertexStepMode::Vertex,
        RenderVertexStepMode::Instance => GpuVertexStepMode::Instance,
    }
}

fn gpu_vertex_format(value: RenderVertexFormat) -> GpuVertexFormat {
    match value {
        RenderVertexFormat::Float32 => GpuVertexFormat::Float32,
        RenderVertexFormat::Float32x2 => GpuVertexFormat::Float32x2,
        RenderVertexFormat::Float32x3 => GpuVertexFormat::Float32x3,
        RenderVertexFormat::Float32x4 => GpuVertexFormat::Float32x4,
        RenderVertexFormat::Uint32 => GpuVertexFormat::Uint32,
        RenderVertexFormat::Uint32x2 => GpuVertexFormat::Uint32x2,
        RenderVertexFormat::Uint32x3 => GpuVertexFormat::Uint32x3,
        RenderVertexFormat::Uint32x4 => GpuVertexFormat::Uint32x4,
        RenderVertexFormat::Sint32 => GpuVertexFormat::Sint32,
        RenderVertexFormat::Sint32x2 => GpuVertexFormat::Sint32x2,
        RenderVertexFormat::Sint32x3 => GpuVertexFormat::Sint32x3,
        RenderVertexFormat::Sint32x4 => GpuVertexFormat::Sint32x4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::{
        PreparedMaterialBindingTable, PreparedMaterialFeatureContribution,
        PreparedMaterialInstanceInput, PreparedMaterialParameterPayloadV1,
        PreparedMaterialTextureBinding, PreparedMaterialTextureBindingLocation,
        PreparedMaterialTextureKind,
    };

    #[test]
    fn missing_logical_texture_view_rejects_before_bind_group_realization() {
        let error = match resolved_binding_texture_view(&RuntimeResourceKey::SurfaceColor, None) {
            Err(error) => error,
            Ok(_) => panic!("missing logical view must reject before bind-group realization"),
        };
        assert!(
            error
                .to_string()
                .contains("has no logical texture view for G4C2 shader binding realization"),
            "unexpected missing-view rejection: {error}"
        );
    }

    fn material_with_bindings(
        texture_bindings: Vec<PreparedMaterialTextureBinding>,
    ) -> PreparedMaterialFeatureContribution {
        PreparedMaterialFeatureContribution {
            instances: vec![PreparedMaterialInstanceInput {
                material_instance_id: "material.product.1".to_string(),
                specialization_key_fragment: "material.first_slice".to_string(),
                parameter_payload: PreparedMaterialParameterPayloadV1::default(),
                texture_bindings,
            }],
            binding_table: PreparedMaterialBindingTable::default(),
            scene_bundle: None,
            model_mesh_material_selections: Vec::new(),
        }
    }

    fn texture_binding(
        resource_slot_index: u32,
        bind_group: u32,
        texture_binding: u32,
        sampler_binding: u32,
    ) -> PreparedMaterialTextureBinding {
        PreparedMaterialTextureBinding::new(
            resource_slot_index as u64 + 1,
            format!("texture_{resource_slot_index}"),
            PreparedMaterialTextureBindingLocation::new(
                resource_slot_index,
                bind_group,
                texture_binding,
                sampler_binding,
            ),
            format!("artifact.{resource_slot_index}"),
            ".runenwerk/artifacts/texture.ktx2",
            PreparedMaterialTextureKind::Texture2D,
            "texture-cache",
        )
    }

    #[test]
    fn material_group_one_refinements_use_transported_compiler_coordinates() {
        let material = material_with_bindings(vec![texture_binding(91, 1, 31, 47)]);

        let refinements =
            gpu_material_binding_refinements(&material).expect("typed refinements should form");
        assert_eq!(refinements.len(), 2);
        assert_eq!(refinements[0].key().group(), 1);
        assert_eq!(refinements[0].key().binding(), 31);
        assert_eq!(
            refinements[0].texture_sample_class(),
            Some(GpuTextureSampleClass::FloatFilterable)
        );
        assert_eq!(refinements[1].key().group(), 1);
        assert_eq!(refinements[1].key().binding(), 47);
        assert_eq!(
            refinements[1].sampler_class(),
            Some(GpuSamplerClass::Filtering)
        );
    }

    #[test]
    fn material_group_one_refinements_reject_invalid_or_duplicate_transported_keys() {
        let invalid_group = material_with_bindings(vec![texture_binding(0, 2, 31, 47)]);
        let error = gpu_material_binding_refinements(&invalid_group)
            .expect_err("material refinements must reject a non-group-one compiler coordinate");
        assert!(error.to_string().contains("exact group 1"));

        let duplicate = material_with_bindings(vec![
            texture_binding(0, 1, 31, 47),
            texture_binding(1, 1, 31, 47),
        ]);
        let error = gpu_material_binding_refinements(&duplicate)
            .expect_err("material refinements must reject duplicate final GPU keys");
        assert!(error.to_string().contains("duplicate final GPU key"));
    }
}
