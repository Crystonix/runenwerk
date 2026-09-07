use std::fs;
use std::path::Path;

fn read(path: &str) -> String {
    fs::read_to_string(Path::new(path)).unwrap_or_else(|err| panic!("failed to read {path}: {err}"))
}

#[test]
fn canonical_copy_format_compatibility_is_owned_only_by_runengpu() {
    assert!(
        !Path::new("src/plugins/render/renderer/render_flow/execute_passes.rs").exists(),
        "G5C1 must delete the residual raw copy executor"
    );
    let projection = read("src/plugins/render/renderer/render_flow/logical_copy.rs");
    assert!(
        projection.contains("GpuCopyOperation::")
            && !projection.contains("gpu_texture_formats_copy_compatible")
            && !projection.contains("remove_srgb_suffix"),
        "renderer copy projection must construct canonical operations and leave format compatibility to RunenGPU"
    );
}

#[test]
fn fixed_step_iteration_uniforms_are_not_generic_gpu_uploads() {
    let source = read("src/plugins/render/renderer/render_flow/execute.rs");
    let generic_start = source
        .find("fn realize_projected_uniform_uploads(")
        .expect("generic projected-uniform realization should exist");
    let occurrence_offset = source[generic_start..]
        .find("fn realize_fixed_step_iteration_upload(")
        .expect("fixed-step occurrence-local upload realization should exist");
    let occurrence_start = generic_start + occurrence_offset;
    let generic = &source[generic_start..occurrence_start];

    let filter = generic
        .find("region.iteration_uniform == *buffer_id")
        .expect("generic uniform realization must identify fixed-step iteration uniforms");
    let skip = generic[filter..]
        .find("continue;")
        .map(|offset| filter + offset)
        .expect("fixed-step iteration uniforms must be skipped by generic realization");
    let prepare = generic
        .find("prepare_uniform_upload")
        .expect("generic uniform realization should still prepare ordinary uniforms");

    assert!(
        filter < skip && skip < prepare,
        "fixed-step iteration uniforms must be rejected from the generic GPU-upload path before physical upload preparation"
    );

    let occurrence_local = &source[occurrence_start..];
    assert!(
        occurrence_local.contains("region.iteration_uniform")
            && occurrence_local.contains("project_buffer_upload("),
        "actual fixed-step occurrences must retain their occurrence-local canonical upload path"
    );
}
