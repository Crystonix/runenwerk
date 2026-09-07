use crate::plugins::render::{PreparedMaterialTextureBinding, PreparedMaterialTextureKind};
use anyhow::{Context, bail};
use runen_gpu::{GpuResourceLabel, GpuTextureDimension, GpuTextureExtent, GpuTextureFormat};

#[derive(Debug)]
pub(crate) struct MaterialKtx2Upload {
    pub size: GpuTextureExtent,
    pub dimension: GpuTextureDimension,
    pub format: GpuTextureFormat,
    pub bytes: Vec<u8>,
}

pub(crate) fn load_material_ktx2_upload(
    binding: &PreparedMaterialTextureBinding,
) -> anyhow::Result<MaterialKtx2Upload> {
    if binding.descriptor_hash.trim().is_empty() {
        bail!(
            "material texture binding '{}' has no descriptor hash",
            binding.artifact_id
        );
    }
    let bytes = std::fs::read(&binding.artifact_path).with_context(|| {
        format!(
            "failed to read KTX2 material texture artifact '{}'",
            binding.artifact_path
        )
    })?;
    if let Some(expected) = binding.container_byte_length
        && bytes.len() as u64 != expected
    {
        bail!(
            "KTX2 material texture artifact '{}' byte length {} did not match descriptor {}",
            binding.artifact_id,
            bytes.len(),
            expected
        );
    }
    let reader = ktx2::Reader::new(bytes.as_slice()).with_context(|| {
        format!(
            "failed to parse KTX2 material texture artifact '{}'",
            binding.artifact_path
        )
    })?;
    let header = reader.header();
    if header.supercompression_scheme.is_some() {
        bail!(
            "KTX2 material texture artifact '{}' uses unsupported supercompression {:?}",
            binding.artifact_id,
            header.supercompression_scheme
        );
    }
    if header.level_count.max(1) != 1 {
        bail!(
            "KTX2 material texture artifact '{}' exposes {} mip levels; WR-021 residency currently requires a single validated base level",
            binding.artifact_id,
            header.level_count.max(1)
        );
    }
    if header.layer_count.max(1) != 1 || header.face_count != 1 {
        bail!(
            "KTX2 material texture artifact '{}' must be a single-layer non-cubemap texture",
            binding.artifact_id
        );
    }
    let format = match header.format {
        Some(ktx2::Format::R8G8B8A8_UNORM) => GpuTextureFormat::Rgba8Unorm,
        Some(ktx2::Format::R8G8B8A8_SRGB) => GpuTextureFormat::Rgba8UnormSrgb,
        other => {
            bail!(
                "KTX2 material texture artifact '{}' uses unsupported runtime format {:?}",
                binding.artifact_id,
                other
            );
        }
    };
    let width = header.pixel_width;
    let height = header.pixel_height.max(1);
    let depth = header.pixel_depth.max(1);
    let expected_dimension = match binding.texture_kind {
        PreparedMaterialTextureKind::Texture2D => {
            if depth != 1 {
                bail!(
                    "KTX2 material texture artifact '{}' is bound as Texture2D but has depth {}",
                    binding.artifact_id,
                    depth
                );
            }
            GpuTextureDimension::D2
        }
        PreparedMaterialTextureKind::Texture3D => {
            if depth <= 1 {
                bail!(
                    "KTX2 material texture artifact '{}' is bound as Texture3D but has depth {}",
                    binding.artifact_id,
                    depth
                );
            }
            GpuTextureDimension::D3
        }
    };
    if width != binding.extent_width.max(1)
        || height != binding.extent_height.max(1)
        || depth != binding.extent_depth.max(1)
    {
        bail!(
            "KTX2 material texture artifact '{}' extent {}x{}x{} did not match descriptor {}x{}x{}",
            binding.artifact_id,
            width,
            height,
            depth,
            binding.extent_width,
            binding.extent_height,
            binding.extent_depth
        );
    }
    let mut levels = reader.levels();
    let level = levels
        .next()
        .ok_or_else(|| anyhow::anyhow!("KTX2 material texture has no base level"))?;
    let expected_len = width as usize * height as usize * depth as usize * 4;
    if level.data.len() != expected_len {
        bail!(
            "KTX2 material texture artifact '{}' base level has {} bytes, expected {} for RGBA8 {}x{}x{}",
            binding.artifact_id,
            level.data.len(),
            expected_len,
            width,
            height,
            depth
        );
    }
    let extent_label = GpuResourceLabel::new(format!("material_texture_{}", binding.artifact_id))?;
    let size = GpuTextureExtent::new(&extent_label, expected_dimension, width, height, depth)?;
    Ok(MaterialKtx2Upload {
        size,
        dimension: expected_dimension,
        format,
        bytes: level.data.to_vec(),
    })
}
