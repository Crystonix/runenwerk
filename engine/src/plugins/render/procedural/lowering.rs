use super::authoring::ProceduralDrawSource;
use super::descriptors::{
    ProceduralPassDescriptor, ProceduralShader, ProceduralTargetDescriptor,
    ProceduralVisualDescriptor,
};
use super::validation::validate_procedural_pass;
use crate::plugins::render::api::{PassParamBinding, RenderFlowAuthoringError};
use crate::plugins::render::{RenderFlow, RenderIndirectDrawResource};
use runen_gpu::GpuBindingKey;

#[derive(Debug)]
pub(crate) struct ProceduralUniformBinding {
    pub(crate) key: GpuBindingKey,
    pub(crate) projection: PassParamBinding,
}

#[derive(Debug)]
pub(crate) struct ProceduralPassLowering {
    pub(crate) uniform_bindings: Vec<ProceduralUniformBinding>,
    pub(crate) draw_source: ProceduralDrawSource,
}

impl Default for ProceduralPassLowering {
    fn default() -> Self {
        Self {
            uniform_bindings: Vec::new(),
            draw_source: ProceduralDrawSource::Direct,
        }
    }
}

pub fn build_procedural_pass(
    flow: RenderFlow,
    descriptor: ProceduralPassDescriptor,
) -> Result<RenderFlow, RenderFlowAuthoringError> {
    lower_procedural_pass(flow, descriptor, ProceduralPassLowering::default())
}

pub(crate) fn lower_procedural_pass(
    flow: RenderFlow,
    descriptor: ProceduralPassDescriptor,
    lowering: ProceduralPassLowering,
) -> Result<RenderFlow, RenderFlowAuthoringError> {
    validate_procedural_pass(&descriptor)?;

    let ProceduralPassDescriptor {
        label,
        shader,
        visual,
        instance_buffer,
        instance_count,
        index_buffer,
        target,
        policy,
        non_data_order_after,
    } = descriptor;

    let mut builder = flow.graphics_pass(label);
    builder = match shader.expect("validated procedural pass has shader") {
        ProceduralShader::AssetPath(path) => builder.shader_asset(path),
        ProceduralShader::RegistryHandle(handle) => builder.shader(handle),
    };
    builder = builder.raster_state(policy.into());

    if let ProceduralVisualDescriptor::MeshSprite { vertex_buffer, .. } = &visual {
        builder = builder
            .push_vertex_buffer_resource(vertex_buffer.resource_id, vertex_buffer.layout.clone());
    }

    builder =
        builder.push_instance_buffer_resource(instance_buffer.resource_id, instance_buffer.layout);

    if let Some(index_buffer) = index_buffer {
        builder = builder.push_index_buffer_resource(index_buffer.resource_id);
    }

    let target = target.expect("validated procedural pass has target");
    builder = apply_target(builder, target);

    for uniform_binding in lowering.uniform_bindings {
        builder = builder.push_uniform_binding(uniform_binding.key, uniform_binding.projection);
    }

    builder = match lowering.draw_source {
        ProceduralDrawSource::Direct => builder.draw(visual.vertex_count(), instance_count),
        ProceduralDrawSource::Indirect {
            args_buffer,
            args_kind,
            args_element_count,
            args_element_size,
            byte_offset,
        } => builder.draw_indirect_resource(
            RenderIndirectDrawResource::new(
                args_buffer,
                args_kind,
                args_element_count,
                args_element_size,
                byte_offset,
            ),
            visual.vertex_count(),
            instance_count,
        ),
    };
    for dependency in non_data_order_after {
        builder = builder.order_after(dependency);
    }

    Ok(builder.finish())
}

fn apply_target(
    mut builder: crate::plugins::render::api::GraphicsPassBuilder,
    target: ProceduralTargetDescriptor,
) -> crate::plugins::render::api::GraphicsPassBuilder {
    builder = builder.write_color_target(target.color_target);
    if let Some(depth_target) = target.depth_target {
        builder = builder.depth_target(depth_target);
    }
    if let Some(clear_color) = target.clear_color {
        builder = builder.clear_color(clear_color);
    }
    builder
}
