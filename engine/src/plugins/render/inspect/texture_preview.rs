use crate::plugins::render::texture_upload::load_material_ktx2_upload;
use crate::plugins::render::{PreparedMaterialTextureBinding, RenderTextureTargetFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TexturePreviewChannelMode {
    All,
    R,
    G,
    B,
    A,
}

impl TexturePreviewChannelMode {
    pub const fn as_label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::R => "r",
            Self::G => "g",
            Self::B => "b",
            Self::A => "a",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TexturePreviewUploadRequest {
    pub binding: PreparedMaterialTextureBinding,
    pub selected_mip: u32,
    pub selected_slice: u32,
    pub selected_channel: TexturePreviewChannelMode,
    pub sampler_identity: String,
    pub bind_group_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TexturePreviewUploadProof {
    pub width: u32,
    pub height: u32,
    pub source_depth: u32,
    pub upload_format: RenderTextureTargetFormat,
    pub selected_mip: u32,
    pub selected_slice: u32,
    pub selected_channel: TexturePreviewChannelMode,
    pub sampler_identity: String,
    pub bind_group_identity: String,
    pub residency_state: TexturePreviewResidencyState,
    pub residency_class: String,
    pub rgba8: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TexturePreviewResidencyState {
    Ready,
}

pub fn prepare_texture_preview_upload_proof(
    request: &TexturePreviewUploadRequest,
) -> anyhow::Result<TexturePreviewUploadProof> {
    if request.selected_mip != 0 {
        anyhow::bail!(
            "texture preview selected mip {} is unsupported by the shared material KTX2 base-level residency path",
            request.selected_mip
        );
    }
    let upload = load_material_ktx2_upload(&request.binding)?;
    let depth = upload.size.depth_or_layers().max(1);
    if request.selected_slice >= depth {
        anyhow::bail!(
            "texture preview selected slice {} is outside uploaded depth {}",
            request.selected_slice,
            depth
        );
    }
    let upload_format = match upload.format {
        runen_gpu::GpuTextureFormat::Rgba8Unorm => RenderTextureTargetFormat::Rgba8Unorm,
        runen_gpu::GpuTextureFormat::Rgba8UnormSrgb => RenderTextureTargetFormat::Rgba8UnormSrgb,
        other => anyhow::bail!("texture preview unsupported upload format {other:?}"),
    };
    let rgba8 = slice_rgba8_preview(
        &upload.bytes,
        upload.size.width(),
        upload.size.height(),
        request.selected_slice,
        request.selected_channel,
    )?;

    Ok(TexturePreviewUploadProof {
        width: upload.size.width(),
        height: upload.size.height(),
        source_depth: depth,
        upload_format,
        selected_mip: request.selected_mip,
        selected_slice: request.selected_slice,
        selected_channel: request.selected_channel,
        sampler_identity: request.sampler_identity.clone(),
        bind_group_identity: request.bind_group_identity.clone(),
        residency_state: TexturePreviewResidencyState::Ready,
        residency_class: "engine.material_ktx2_upload".to_string(),
        rgba8,
    })
}

fn slice_rgba8_preview(
    bytes: &[u8],
    width: u32,
    height: u32,
    slice: u32,
    channel: TexturePreviewChannelMode,
) -> anyhow::Result<Vec<u8>> {
    let texels_per_slice = width as usize * height as usize;
    let slice_start = slice as usize * texels_per_slice * 4;
    let slice_end = slice_start
        .checked_add(texels_per_slice * 4)
        .ok_or_else(|| anyhow::anyhow!("texture preview slice byte range overflowed"))?;
    if slice_end > bytes.len() {
        anyhow::bail!(
            "texture preview slice {} byte range {}..{} exceeds upload length {}",
            slice,
            slice_start,
            slice_end,
            bytes.len()
        );
    }
    let mut out = bytes[slice_start..slice_end].to_vec();
    match channel {
        TexturePreviewChannelMode::All => {}
        TexturePreviewChannelMode::R
        | TexturePreviewChannelMode::G
        | TexturePreviewChannelMode::B
        | TexturePreviewChannelMode::A => {
            let index = match channel {
                TexturePreviewChannelMode::R => 0,
                TexturePreviewChannelMode::G => 1,
                TexturePreviewChannelMode::B => 2,
                TexturePreviewChannelMode::A => 3,
                TexturePreviewChannelMode::All => unreachable!(),
            };
            for pixel in out.as_chunks_mut::<4>().0 {
                let value = pixel[index];
                pixel[0] = value;
                pixel[1] = value;
                pixel[2] = value;
                pixel[3] = 255;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::{
        PreparedMaterialTextureBindingLocation, PreparedMaterialTextureKind,
        RenderTextureTargetFormat,
    };

    #[test]
    fn texture_preview_upload_residency_path() {
        let bytes = build_rgba8_ktx2(2, 2, 2, [10, 20, 30, 255], [90, 80, 70, 255]);
        let path = std::env::temp_dir().join(format!(
            "runenwerk-texture-preview-upload-{}.ktx2",
            std::process::id()
        ));
        std::fs::write(&path, &bytes).expect("test ktx2 should write");
        let binding = PreparedMaterialTextureBinding::new(
            0,
            "preview",
            PreparedMaterialTextureBindingLocation::new(0, 1, 0, 1),
            "artifact.7",
            path.to_string_lossy(),
            PreparedMaterialTextureKind::Texture3D,
            "cache",
        )
        .with_extent(2, 2, 2)
        .with_descriptor_hash("descriptor-hash")
        .with_residency_identity("ktx2:7:1:cache:descriptor-hash")
        .with_ktx2_contract("Rgba8Unorm", "None", Some(bytes.len() as u64));
        let request = TexturePreviewUploadRequest {
            binding,
            selected_mip: 0,
            selected_slice: 1,
            selected_channel: TexturePreviewChannelMode::G,
            sampler_identity: "linear-repeat".to_string(),
            bind_group_identity: "preview-bind-group".to_string(),
        };

        let proof =
            prepare_texture_preview_upload_proof(&request).expect("preview proof should prepare");

        assert_eq!(proof.width, 2);
        assert_eq!(proof.height, 2);
        assert_eq!(proof.source_depth, 2);
        assert_eq!(proof.upload_format, RenderTextureTargetFormat::Rgba8Unorm);
        assert_eq!(proof.selected_slice, 1);
        assert_eq!(proof.selected_channel, TexturePreviewChannelMode::G);
        assert_eq!(proof.residency_class, "engine.material_ktx2_upload");
        assert_eq!(&proof.rgba8[0..4], &[80, 80, 80, 255]);
        let _ = std::fs::remove_file(path);
    }

    fn build_rgba8_ktx2(
        width: u32,
        height: u32,
        depth: u32,
        slice0_texel: [u8; 4],
        slice1_texel: [u8; 4],
    ) -> Vec<u8> {
        let format = ktx2::Format::R8G8B8A8_UNORM;
        let (basic, type_size) =
            ktx2::dfd::Basic::from_format(format).expect("rgba8 dfd should build");
        let dfd_block = ktx2::dfd::Block::Basic(basic);
        let dfd_block_bytes = dfd_block.to_vec();
        let dfd_total_size = 4 + dfd_block_bytes.len();
        let level_index_offset = ktx2::Header::LENGTH;
        let dfd_offset = level_index_offset + ktx2::LevelIndex::LENGTH;
        let after_dfd = dfd_offset + dfd_total_size;
        let level_data_offset = after_dfd.div_ceil(4) * 4;
        let texel_count = width as usize * height as usize * depth.max(1) as usize;
        let level_data_size = texel_count * 4;
        let mut bytes = vec![0u8; level_data_offset + level_data_size];

        let header = ktx2::Header {
            format: Some(format),
            type_size,
            pixel_width: width,
            pixel_height: height,
            pixel_depth: if depth > 1 { depth } else { 0 },
            layer_count: 0,
            face_count: 1,
            level_count: 1,
            supercompression_scheme: None,
            index: ktx2::Index {
                dfd_byte_offset: dfd_offset as u32,
                dfd_byte_length: dfd_total_size as u32,
                kvd_byte_offset: 0,
                kvd_byte_length: 0,
                sgd_byte_offset: 0,
                sgd_byte_length: 0,
            },
        };
        bytes[..ktx2::Header::LENGTH].copy_from_slice(&header.as_bytes());
        let index = ktx2::LevelIndex {
            byte_offset: level_data_offset as u64,
            byte_length: level_data_size as u64,
            uncompressed_byte_length: level_data_size as u64,
        };
        bytes[level_index_offset..level_index_offset + ktx2::LevelIndex::LENGTH]
            .copy_from_slice(&index.as_bytes());
        bytes[dfd_offset..dfd_offset + 4].copy_from_slice(&(dfd_total_size as u32).to_le_bytes());
        bytes[dfd_offset + 4..dfd_offset + 4 + dfd_block_bytes.len()]
            .copy_from_slice(&dfd_block_bytes);
        let data = &mut bytes[level_data_offset..level_data_offset + level_data_size];
        let texels_per_slice = width as usize * height as usize;
        for (index, pixel) in data.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let texel = if index / texels_per_slice == 0 {
                slice0_texel
            } else {
                slice1_texel
            };
            pixel.copy_from_slice(&texel);
        }
        bytes
    }
}
