use anyhow::Result;
use engine::plugins::render::{GpuStorage, RenderFlow};
use runen_gpu::GpuBindingKey;

#[derive(Debug, Clone, Copy, GpuStorage)]
struct BloomCell {
    value: u32,
}

fn main() -> Result<()> {
    // This planning example's established ping-pong pair occupies the two
    // explicit group-0 storage slots.
    let bloom_a_binding = binding_key(0);
    let bloom_b_binding = binding_key(1);

    let flow = RenderFlow::new("post.flow")
        .with_surface_color()
        .expect("render flow authoring should succeed")
        .double_buffer_storage_array::<BloomCell>("post.bloom", 64)
        .expect("render flow authoring should succeed")
        .compute_pass("post.bloom_extract")
        .shader_asset("assets/shaders/bloom_extract.wgsl")
        .bind_ping_pong_storage(bloom_a_binding, bloom_b_binding, "post.bloom")
        .dispatch([1, 1, 1])
        .finish()
        .fullscreen_pass("post.compose")
        .shader_asset("assets/shaders/blur_y.wgsl")
        .bind_ping_pong_storage(bloom_a_binding, bloom_b_binding, "post.bloom")
        .write_surface_color()
        .expect("render flow authoring should succeed")
        .finish()
        .validate()?;

    let order = flow.lexical_pass_order()?;
    let order = order
        .into_iter()
        .map(|pass_id| pass_id.to_string())
        .collect::<Vec<_>>();
    println!("postprocess lexical pass order: {}", order.join(" -> "));
    Ok(())
}

fn binding_key(binding: u64) -> GpuBindingKey {
    GpuBindingKey::try_new(0, binding).expect("postprocess flow binding should fit GpuBindingKey")
}
