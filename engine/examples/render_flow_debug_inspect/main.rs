use anyhow::Result;
use engine::plugins::render::RenderFlow;
use engine::plugins::render::inspect::{
    PassTimingSample, dump_flow_graph, inspect_resources, inspect_texture_resources,
    summarize_pass_timings,
};
use runen_gpu::GpuBindingKey;

const FLOW_ID: &str = "inspect.flow";

#[derive(Debug, Clone, Copy, engine::plugins::render::GpuStorage)]
struct InspectCell {
    value: u32,
}

fn main() -> Result<()> {
    // This inspection flow retains its established two-slot ping-pong storage
    // contract without deriving shader slots from access-vector order.
    let cells_a_binding = binding_key(0);
    let cells_b_binding = binding_key(1);

    let flow = RenderFlow::new(FLOW_ID)
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .with_builtin_ui()
        .double_buffer_storage_array::<InspectCell>("inspect.cells", 16)
        .expect("render flow authoring should succeed")
        .compute_pass("inspect.sim")
        .shader_asset("assets/shaders/world_compute_basic.wgsl")
        .bind_ping_pong_storage(cells_a_binding, cells_b_binding, "inspect.cells")
        .dispatch([1, 1, 1])
        .finish()
        .fullscreen_pass("inspect.compose")
        .shader_asset("assets/shaders/tonemap.wgsl")
        .bind_ping_pong_storage(cells_a_binding, cells_b_binding, "inspect.cells")
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .finish()
        .builtin_ui_composite_pass("inspect.ui")
        .expect("render flow authoring should succeed")
        .finish()
        .validate()?;

    let dump = dump_flow_graph(&flow)?;
    println!("graph dump:");
    for line in &dump.lines {
        println!("  {line}");
    }

    let resources = inspect_resources(&flow);
    println!("resource count: {}", resources.len());
    let textures = inspect_texture_resources(&flow);
    println!("texture count: {}", textures.len());

    let timings = summarize_pass_timings(&[
        PassTimingSample {
            flow_id: FLOW_ID.to_string(),
            pass_id: "inspect.sim".to_string(),
            pass_kind: "compute".to_string(),
            millis: 0.5,
            dispatch_workgroups: Some([1, 1, 1]),
        },
        PassTimingSample {
            flow_id: FLOW_ID.to_string(),
            pass_id: "inspect.compose".to_string(),
            pass_kind: "fullscreen".to_string(),
            millis: 0.2,
            dispatch_workgroups: None,
        },
    ]);
    println!(
        "frame {:.2}ms, slowest {:?}",
        timings.total_millis, timings.slowest_pass_id
    );

    Ok(())
}

fn binding_key(binding: u64) -> GpuBindingKey {
    GpuBindingKey::try_new(0, binding).expect("inspection flow binding should fit GpuBindingKey")
}
