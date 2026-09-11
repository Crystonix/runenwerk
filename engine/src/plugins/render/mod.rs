pub mod adapters;
pub mod admission;
pub mod api;
pub mod appearance;
pub mod backend;
pub mod composition;
pub mod derived_state;
#[cfg_attr(not(test), allow(dead_code))]
mod derived_transform;
pub mod features;
pub mod frame;
pub mod gpu_primitives;
pub mod graph;
pub mod inspect;
pub mod lowering;
pub mod material_compiler;
pub mod method;
pub mod output_result;
pub mod params;
pub mod participation;
pub mod pipelines;
pub mod procedural;
#[cfg_attr(not(test), allow(dead_code))]
mod render_result;
pub mod renderer;
pub mod representation;
pub mod request;
pub mod residency;
pub mod resource;
pub mod scene;
pub mod semantic_plan;
pub mod shader;
pub mod space_time;
pub mod surface_result;
mod texture_upload;

#[cfg(test)]
mod derived_transform_r7_proof;
#[cfg(test)]
mod r6_proof;
#[cfg(test)]
mod r6_reference_proof;
#[cfg(test)]
mod r6_spine_proof;

mod plugin;
pub mod runtime;

pub use adapters::*;
pub use api::*;
pub use bytemuck;
pub use composition::*;
pub use engine_render_macros::{GpuStorage, GpuUniform};
pub use features::*;
pub use frame::*;
pub use gpu_primitives::*;
pub use graph::*;
pub use material_compiler::*;
pub use params::*;
pub use plugin::RenderPlugin;
pub use procedural::*;
pub use renderer::{Gfx, GfxFrameTimings, RenderFrameDataRegistry, Renderer, RendererFrameTimings};
pub use residency::*;
pub use resource::*;
pub use runtime::*;
pub use shader::{
    ShaderHandle, ShaderRegistryEvent, ShaderRegistryEventKind, ShaderRegistryResource,
    ShaderReloadPollReport, ShaderReloadPollStatus,
};
