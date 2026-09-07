use super::*;
use crate::plugins::render::pipelines::FlowPassPipelineDescriptor;
use crate::plugins::render::{RenderPassId, RenderShaderConstant};
use runen_gpu::{
    GpuAdmittedProgramSource, GpuCapabilityRequirements, GpuProgramSourceKey,
    GpuProgramSourceProvenance, GpuSpecializationDeclaration, GpuSpecializationEntry,
    GpuSpecializationKey, GpuSpecializationSchema, GpuSpecializationValueSet,
};

impl Renderer {
    /// First half of the renderer's two-phase integration: realize every G4C1/G4C2/G4C3
    /// dependency while no raw device/queue or physical surface object is required.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::plugins::render::renderer::render_flow) fn realize_compiled_pass(
        &mut self,
        context: &GpuContext,
        packet: &RendererPreparedPacket,
        flow: &CompiledRenderFlowPlan,
        flow_inputs: &PreparedFlowInputs,
        pass: &CompiledPassExecutionPlan,
        shader_registry: &ShaderRegistryResource,
        runtime_resources: &FlowRuntimeResources,
    ) -> Result<Option<PreparedPipelinePass>> {
        match pass {
            CompiledPassExecutionPlan::Compute(value) => {
                let shader = resolve_shader_material(
                    value.shader.as_ref(),
                    shader_registry,
                    DEFAULT_COMPUTE_SHADER,
                    "builtin:compute",
                );
                let specialization =
                    compute_specialization_from_constants(&value.shader_constants)?;
                flow_inputs
                    .projected_dispatch_workgroups
                    .get(&value.pass_id)
                    .copied()
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "missing prepared dispatch for pass '{}' in flow '{}'",
                            value.pass_id,
                            flow.flow_id
                        )
                    })?;
                let admitted_source = admit_resolved_program_source(
                    &mut self.flow_pipeline_cache,
                    &shader,
                    format!("compute pass {}", value.pass_id),
                )?;
                let bindings = self.resolve_compiled_bind_group(
                    context,
                    packet,
                    flow,
                    value.pass_id,
                    FlowPassKind::Compute,
                    value.feature_id,
                    &admitted_source,
                    specialization,
                    &value.bindings,
                    true,
                    Vec::new(),
                    None,
                    runtime_resources,
                )?;
                let pipeline = match &bindings.pipeline_key.pipeline_descriptor {
                    FlowPassPipelineDescriptor::Compute(descriptor) => {
                        PreparedFlowPipeline::Compute(pollster::block_on(
                            context.realize_compute_pipeline(
                                descriptor,
                                &bindings.program,
                                &bindings.pipeline_layout,
                            ),
                        )?)
                    }
                    FlowPassPipelineDescriptor::Render(_) => {
                        bail!(
                            "compute pass '{}' resolved a render pipeline descriptor",
                            value.pass_id
                        )
                    }
                };
                Ok(Some(PreparedPipelinePass {
                    bindings,
                    pipeline,
                    shader_id: shader.shader_id,
                    shader_revision: shader.revision,
                    fallback_used: shader.fallback_used,
                }))
            }
            CompiledPassExecutionPlan::Fullscreen(value) => {
                if !value.draw_buffers.vertex_buffers.is_empty()
                    || !value.draw_buffers.index_buffers.is_empty()
                    || !value.draw_buffers.instance_buffers.is_empty()
                    || !value.draw_buffers.indirect_buffers.is_empty()
                {
                    bail!(
                        "fullscreen pass '{}' cannot bind graphics vertex/index/instance/indirect buffers",
                        value.pass_id
                    );
                }
                let color_format = self.resolve_color_target_format_from_plan(
                    runtime_resources,
                    value.pass_id,
                    &value.targets,
                    packet.surface_format,
                )?;
                let shader = resolve_shader_material_for_packet(
                    value.shader.as_ref(),
                    packet,
                    shader_registry,
                    DEFAULT_FULLSCREEN_SHADER,
                    "builtin:fullscreen",
                );
                reject_material_shader_fallback(
                    value.feature_id,
                    value.shader.as_ref(),
                    value.pass_id,
                    &shader,
                )?;
                reject_unresident_material_textures(
                    packet,
                    value.feature_id,
                    value.shader.as_ref(),
                    value.pass_id,
                )?;
                let admitted_source = admit_resolved_program_source(
                    &mut self.flow_pipeline_cache,
                    &shader,
                    format!("fullscreen pass {}", value.pass_id),
                )?;
                let bindings = self.resolve_compiled_bind_group(
                    context,
                    packet,
                    flow,
                    value.pass_id,
                    FlowPassKind::Fullscreen,
                    value.feature_id,
                    &admitted_source,
                    empty_specialization_value_set()?,
                    &value.bindings,
                    true,
                    vec![color_format],
                    None,
                    runtime_resources,
                )?;
                let pipeline = match &bindings.pipeline_key.pipeline_descriptor {
                    FlowPassPipelineDescriptor::Render(descriptor) => PreparedFlowPipeline::Render(
                        pollster::block_on(context.realize_render_pipeline(
                            descriptor,
                            &bindings.program,
                            &bindings.pipeline_layout,
                        ))?,
                    ),
                    FlowPassPipelineDescriptor::Compute(_) => {
                        bail!(
                            "fullscreen pass '{}' resolved a compute pipeline descriptor",
                            value.pass_id
                        )
                    }
                };
                Ok(Some(PreparedPipelinePass {
                    bindings,
                    pipeline,
                    shader_id: shader.shader_id,
                    shader_revision: shader.revision,
                    fallback_used: shader.fallback_used,
                }))
            }
            CompiledPassExecutionPlan::Graphics(value) => {
                let color_format = self.resolve_color_target_format_from_plan(
                    runtime_resources,
                    value.pass_id,
                    &value.targets,
                    packet.surface_format,
                )?;
                let depth_format = self.resolve_depth_target_format_from_plan(
                    runtime_resources,
                    value.pass_id,
                    &value.targets,
                )?;
                let shader = resolve_shader_material_for_packet(
                    value.shader.as_ref(),
                    packet,
                    shader_registry,
                    DEFAULT_GRAPHICS_SHADER,
                    "builtin:graphics",
                );
                reject_material_shader_fallback(
                    value.feature_id,
                    value.shader.as_ref(),
                    value.pass_id,
                    &shader,
                )?;
                reject_unresident_material_textures(
                    packet,
                    value.feature_id,
                    value.shader.as_ref(),
                    value.pass_id,
                )?;
                let admitted_source = admit_resolved_program_source(
                    &mut self.flow_pipeline_cache,
                    &shader,
                    format!("graphics pass {}", value.pass_id),
                )?;
                let bindings = self.resolve_compiled_bind_group(
                    context,
                    packet,
                    flow,
                    value.pass_id,
                    FlowPassKind::Graphics,
                    value.feature_id,
                    &admitted_source,
                    empty_specialization_value_set()?,
                    &value.bindings,
                    true,
                    vec![color_format],
                    depth_format,
                    runtime_resources,
                )?;
                let pipeline = match &bindings.pipeline_key.pipeline_descriptor {
                    FlowPassPipelineDescriptor::Render(descriptor) => PreparedFlowPipeline::Render(
                        pollster::block_on(context.realize_render_pipeline(
                            descriptor,
                            &bindings.program,
                            &bindings.pipeline_layout,
                        ))?,
                    ),
                    FlowPassPipelineDescriptor::Compute(_) => {
                        bail!(
                            "graphics pass '{}' resolved a compute pipeline descriptor",
                            value.pass_id
                        )
                    }
                };
                Ok(Some(PreparedPipelinePass {
                    bindings,
                    pipeline,
                    shader_id: shader.shader_id,
                    shader_revision: shader.revision,
                    fallback_used: shader.fallback_used,
                }))
            }
            CompiledPassExecutionPlan::Copy(_)
            | CompiledPassExecutionPlan::Present(_)
            | CompiledPassExecutionPlan::BuiltinUiComposite(_) => Ok(None),
        }
    }

    fn resolve_color_target_format_from_plan(
        &self,
        runtime_resources: &FlowRuntimeResources,
        pass_id: RenderPassId,
        targets: &CompiledTargetPlan,
        surface_format: GpuTextureFormat,
    ) -> Result<GpuTextureFormat> {
        if targets.color_outputs.len() != 1 {
            bail!(
                "pass '{}' declares {} color outputs, but runtime realization requires exactly one color output",
                pass_id,
                targets.color_outputs.len()
            );
        }
        let output = targets.color_outputs.first().ok_or_else(|| {
            anyhow::anyhow!(
                "pass '{}' is missing a color output target in execution plan",
                pass_id
            )
        })?;
        let output_key = runtime_resources.resolve_resource_key(pass_id, output, "color_output")?;
        match output_key {
            RuntimeResourceKey::DynamicTexture(key) => self
                .dynamic_texture_targets
                .color_target_format(pass_id, &key),
            _ => runtime_resources.resolve_color_target_format_from_plan(
                pass_id,
                targets,
                surface_format,
            ),
        }
    }

    fn resolve_depth_target_format_from_plan(
        &self,
        runtime_resources: &FlowRuntimeResources,
        pass_id: RenderPassId,
        targets: &CompiledTargetPlan,
    ) -> Result<Option<GpuTextureFormat>> {
        let Some(depth_target) = targets.depth_output.as_ref() else {
            return Ok(None);
        };
        let resource_key =
            runtime_resources.resolve_resource_key(pass_id, depth_target, "depth_output")?;
        match resource_key {
            RuntimeResourceKey::DynamicTexture(key) => self
                .dynamic_texture_targets
                .depth_target_format(pass_id, &key)
                .map(Some),
            _ => runtime_resources.resolve_depth_target_format_from_plan(pass_id, targets),
        }
    }
}

fn admit_resolved_program_source(
    cache: &mut FlowPipelineArtifactCache,
    shader: &super::provenance::ResolvedShaderMaterial<'_>,
    provenance_detail: impl Into<String>,
) -> Result<GpuAdmittedProgramSource> {
    Ok(cache.admit_program_source(
        GpuProgramSourceKey::new(shader.pipeline_identity.as_str())?,
        shader.revision,
        shader.source,
        GpuProgramSourceProvenance::new(
            "render-flow-resolved-program",
            Some(provenance_detail.into()),
        )?,
    )?)
}

fn reject_material_shader_fallback(
    feature_id: Option<crate::plugins::render::RenderFeatureId>,
    shader_reference: Option<&RenderShaderReference>,
    pass_id: crate::plugins::render::RenderPassId,
    shader: &super::provenance::ResolvedShaderMaterial<'_>,
) -> Result<()> {
    if pass_consumes_material_resources(feature_id, shader_reference) && shader.fallback_used {
        bail!(
            "material feature pass '{}' requires the exact generated shader '{}' to be loaded; builtin or scene-bundle fallback is forbidden",
            pass_id,
            shader.shader_id
        );
    }
    Ok(())
}

fn reject_unresident_material_textures(
    packet: &RendererPreparedPacket,
    feature_id: Option<crate::plugins::render::RenderFeatureId>,
    shader: Option<&RenderShaderReference>,
    pass_id: crate::plugins::render::RenderPassId,
) -> Result<()> {
    if !pass_consumes_material_resources(feature_id, shader) {
        return Ok(());
    }
    let Some(material) = &packet.prepared_material else {
        return Ok(());
    };
    let texture_count = material
        .instances
        .iter()
        .map(|instance| instance.texture_bindings.len())
        .sum::<usize>();
    if texture_count == 0 || packet.prepared_material_gpu_resources.is_some() {
        return Ok(());
    }
    bail!(
        "material feature pass '{}' requires {} GPU-resident material texture bindings, but render-flow material resource bind groups are not prepared; refusing shader execution instead of using pseudo texture sampling",
        pass_id,
        texture_count
    );
}

fn empty_specialization_value_set() -> Result<GpuSpecializationValueSet> {
    Ok(GpuSpecializationValueSet::new(
        GpuSpecializationSchema::new([])?,
        [],
    )?)
}

fn compute_specialization_from_constants(
    constants: &[RenderShaderConstant],
) -> Result<GpuSpecializationValueSet> {
    if constants.is_empty() {
        return empty_specialization_value_set();
    }
    let mut declarations = Vec::with_capacity(constants.len());
    let mut entries = Vec::with_capacity(constants.len());
    for constant in constants {
        let key = GpuSpecializationKey::new(constant.name.clone())?;
        declarations.push(GpuSpecializationDeclaration::new(
            key.clone(),
            constant.value.value_type(),
            None,
            GpuCapabilityRequirements::new(),
        )?);
        entries.push(GpuSpecializationEntry::new(key, constant.value));
    }
    Ok(GpuSpecializationValueSet::new(
        GpuSpecializationSchema::new(declarations)?,
        entries,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_compute_specialization_normalizes_order_and_preserves_types() {
        let first = compute_specialization_from_constants(&[
            RenderShaderConstant::u32("COUNT", 4),
            RenderShaderConstant::i32("OFFSET", -1),
        ])
        .unwrap();
        let reordered = compute_specialization_from_constants(&[
            RenderShaderConstant::i32("OFFSET", -1),
            RenderShaderConstant::u32("COUNT", 4),
        ])
        .unwrap();
        let signed_count = compute_specialization_from_constants(&[
            RenderShaderConstant::i32("COUNT", 4),
            RenderShaderConstant::i32("OFFSET", -1),
        ])
        .unwrap();

        assert_eq!(first, reordered);
        assert_ne!(first, signed_count);
    }

    #[test]
    fn typed_compute_specialization_rejects_invalid_or_duplicate_keys() {
        assert!(
            compute_specialization_from_constants(&[RenderShaderConstant::u32("a=1,b", 2)])
                .is_err()
        );
        assert!(
            compute_specialization_from_constants(&[
                RenderShaderConstant::u32("COUNT", 1),
                RenderShaderConstant::u32("COUNT", 2),
            ])
            .is_err()
        );
    }
}
