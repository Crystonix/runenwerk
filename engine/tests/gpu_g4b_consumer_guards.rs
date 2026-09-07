use std::fs;
use std::path::{Path, PathBuf};

const PASS_BINDINGS: &str = "src/plugins/render/api/bindings.rs";
const EXECUTION_PLAN: &str = "src/plugins/render/graph/execution_plan.rs";
const RUNTIME_BINDINGS: &str = "src/plugins/render/renderer/render_flow/bindings.rs";
const RENDERER_SETUP: &str = "src/plugins/render/renderer/setup.rs";
const PRIMITIVE_PLAN: &str = "src/plugins/render/gpu_primitives/plan.rs";
const FRAGMENTS: &str = "src/plugins/render/composition/fragments.rs";
const FRAGMENT_VALIDATION: &str = "src/plugins/render/composition/fragment_validation.rs";
const FRAGMENT_MERGE: &str = "src/plugins/render/graph/merge.rs";
const FLOW_VALIDATION: &str = "src/plugins/render/graph/validation.rs";
const PIPELINE_CACHE: &str = "src/plugins/render/renderer/pipeline_cache.rs";
const PROGRAM_SOURCES: &str = "src/plugins/render/renderer/render_flow/program_sources.rs";

#[test]
fn renderer_shader_binding_identity_is_explicit_and_never_vector_derived() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let authoring = read(&manifest, PASS_BINDINGS);
    let execution = read(&manifest, EXECUTION_PLAN);
    let runtime = read(&manifest, RUNTIME_BINDINGS);
    let primitives = read(&manifest, PRIMITIVE_PLAN);

    assert!(authoring.contains("key: GpuBindingKey"));
    assert!(authoring.contains("pub const fn key(&self) -> GpuBindingKey"));

    let compile_bindings = section(
        &execution,
        "fn compile_pass_bindings(",
        "fn collect_storage_usage(",
        EXECUTION_PLAN,
    );
    assert!(compile_bindings.contains("entries.sort_by_key(CompiledBindingEntry::key)"));
    assert!(compile_bindings.contains("let key = binding.key();"));
    assert!(!compile_bindings.contains(".enumerate()"));

    let runtime_bindings = section(
        &runtime,
        "pub(super) fn resolve_compiled_bind_group",
        "fn resolved_binding_texture_view(",
        RUNTIME_BINDINGS,
    );
    assert!(!runtime_bindings.contains(".enumerate()"));
    assert!(runtime_bindings.contains("runtime_binding_value(value, sampler.as_ref())"));
    assert!(!runtime.contains("BindGroupEntry"));

    let runtime_binding_value = section(
        &runtime,
        "fn runtime_binding_value(",
        "fn gpu_pipeline_descriptor_for_pass(",
        RUNTIME_BINDINGS,
    );
    assert!(runtime_binding_value.contains("GpuRuntimeBindingValue::new(value.key, [resource])"));

    assert!(primitives.contains("shader_bindings: Vec<GpuPrimitiveShaderBinding>"));
    assert!(primitives.contains("GpuBindingKey::try_new(0, binding)"));
    assert!(!primitives.contains("fn stage_binding_resources("));
}

#[test]
fn renderer_program_contract_is_derived_through_public_runengpu_descriptors() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runtime = read(&manifest, RUNTIME_BINDINGS);
    let setup = read(&manifest, RENDERER_SETUP);

    for (path, source) in [(RUNTIME_BINDINGS, &runtime), (RENDERER_SETUP, &setup)] {
        assert!(
            source.contains("GpuProgramDescriptor::new("),
            "{path} must admit programs through the public RunenGPU descriptor"
        );
        assert!(
            source.contains("GpuPipelineConfiguration::"),
            "{path} must construct pipelines through the public RunenGPU configuration contract"
        );
        assert!(source.contains("GpuBindingLayoutRefinement"));
        for forbidden in [
            "GpuProgramInterfaceDescriptor",
            "GpuBindingDeclaration",
            "GpuEntryPointDescriptor",
            "wgpu::",
            "BindGroupEntry",
        ] {
            assert!(
                !source.contains(forbidden),
                "{path} regained caller-authored/private GPU authority through {forbidden}"
            );
        }
    }

    assert!(runtime.contains("pipeline_key.pipeline_descriptor.layout()"));
    assert!(setup.contains("pipeline_descriptor.layout()"));
    assert!(!runtime.contains("context.realize_bind_group("));
}

#[test]
fn fragment_composition_preserves_explicit_shader_binding_identity() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fragments = read(&manifest, FRAGMENTS);
    let validation = read(&manifest, FRAGMENT_VALIDATION);
    let merge = read(&manifest, FRAGMENT_MERGE);

    assert!(fragments.contains("pub shader_bindings: Vec<RenderFragmentShaderBinding>"));
    for required in [
        "texture_binding: GpuBindingKey",
        "sampler_binding: GpuBindingKey",
        "binding: GpuBindingKey",
    ] {
        assert!(fragments.contains(required));
    }
    assert!(validation.contains("fn validate_shader_binding_identity("));
    assert!(validation.contains("if key.group() != 0"));
    assert!(validation.contains("if !keys.insert(key)"));

    let merge_pass = section(
        &merge,
        "fn merge_pass_into_flow(",
        "fn apply_compute_view_scope(",
        FRAGMENT_MERGE,
    );
    assert!(merge_pass.contains("for binding in &pass.shader_bindings"));
    assert!(!merge_pass.contains("for sample in &pass.sample_textures"));
    assert!(!merge_pass.contains("for write in &pass.write_textures"));
}

#[test]
fn render_flow_validates_explicit_shader_binding_identity() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let validation = read(&manifest, FLOW_VALIDATION);

    assert!(validation.contains("fn validate_pass_shader_binding_identity("));
    assert!(validation.contains("if key.group() != 0"));
    assert!(validation.contains("if !keys.insert(key)"));
}

#[test]
fn renderer_program_source_admission_has_one_cache_gateway_without_lifetime_pins() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pipeline = read(
        &manifest,
        "src/plugins/render/renderer/render_flow/pipeline_realization.rs",
    );
    let cache = read(&manifest, PIPELINE_CACHE);
    let authority = read(&manifest, PROGRAM_SOURCES);

    let execution_gateway = section(
        &pipeline,
        "fn admit_resolved_program_source(",
        "fn reject_material_shader_fallback(",
        "pipeline_realization.rs",
    );
    assert!(execution_gateway.contains(") -> Result<GpuAdmittedProgramSource>"));
    assert_eq!(
        execution_gateway
            .matches("cache.admit_program_source(")
            .count(),
        1
    );
    assert!(!pipeline.contains("RendererProgramSourceAuthority::new("));
    assert!(!pipeline.contains(".admit_wgsl("));

    assert_eq!(
        cache.matches("pub(crate) fn admit_program_source(").count(),
        1
    );
    let cache_gateway = section(
        &cache,
        "pub(crate) fn admit_program_source(",
        "pub fn retain_flows(",
        PIPELINE_CACHE,
    );
    assert_eq!(cache_gateway.matches(".admit_wgsl(").count(), 1);
    assert_eq!(
        cache
            .matches("RendererProgramSourceAuthority::new(")
            .count(),
        1
    );
    for forbidden in [
        "admit_and_retain_wgsl",
        "retained_sources",
        "program_source_retentions",
    ] {
        assert!(!cache.contains(forbidden) && !authority.contains(forbidden));
    }
}

#[test]
fn builtin_program_sources_use_the_same_cache_gateway() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cache = read(&manifest, PIPELINE_CACHE);
    let default_impl = section(
        &cache,
        "impl Default for FlowPipelineArtifactCache",
        "impl FlowPipelineArtifactCache",
        PIPELINE_CACHE,
    );
    assert_eq!(
        default_impl
            .matches("admit_builtin_program_source(")
            .count(),
        3
    );

    let builtin_gateway = section(
        &cache,
        "fn admit_builtin_program_source(",
        "#[cfg(test)]",
        PIPELINE_CACHE,
    );
    assert_eq!(builtin_gateway.matches(".admit_program_source(").count(), 1);
}

fn read(manifest: &Path, relative: &str) -> String {
    fs::read_to_string(manifest.join(relative))
        .unwrap_or_else(|error| panic!("cannot read {relative}: {error}"))
}

fn section<'a>(source: &'a str, start: &str, end: &str, path: &str) -> &'a str {
    let start_index = source
        .find(start)
        .unwrap_or_else(|| panic!("{path} no longer contains start marker {start:?}"));
    let tail = &source[start_index..];
    let end_index = tail
        .find(end)
        .unwrap_or_else(|| panic!("{path} no longer contains end marker {end:?} after {start:?}"));
    &tail[..end_index]
}
