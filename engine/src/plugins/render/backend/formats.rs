use runen_gpu::{GpuSurfaceCapabilities, GpuTextureFormat};

pub fn preferred_surface_format(caps: &GpuSurfaceCapabilities) -> Option<GpuTextureFormat> {
    caps.formats()
        .iter()
        .copied()
        .find(|format| format.is_srgb())
        .or_else(|| caps.formats().first().copied())
}
