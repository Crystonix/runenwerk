use crate::plugins::render::backend::{RenderSurfaceId, RenderSurfaceRegistryResource};
use crate::plugins::render::features::{
    FeatureContributionStatus, FeatureFallbackPolicy, PreparedSurfaceFrameSubmission,
    PreparedUiFrameContribution, SurfaceFrameSubmissionRegistryResource,
};
use crate::runtime::{Res, ResMut};
use rusttype::{Font, Scale, point};
use std::collections::HashMap;
use ui_render_data::ViewportSurfaceBindingRegistry;
use ui_text::{FontAtlasSource, FontFaceMetrics, FontId, GlyphMetrics, MsdfFontAtlas};

pub const DEFAULT_EDITOR_FONT_ID: FontId = FontId(1);
const DEFAULT_EDITOR_FONT_BASE_SIZE: f32 = 32.0;
const DEFAULT_ATLAS_COLUMNS: u32 = 16;
const DEFAULT_GLYPH_PADDING: u32 = 3;
const DEFAULT_EDITOR_FONT_BYTES: &[u8] =
    include_bytes!("../../../../../../assets/fonts/JetBrainsMono-Regular.ttf");

#[derive(Debug, Clone)]
pub struct UiFontAtlasImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct UiFontAtlasResource {
    atlases: HashMap<FontId, MsdfFontAtlas>,
    images: HashMap<FontId, UiFontAtlasImage>,
}

impl UiFontAtlasResource {
    pub fn atlas_image(&self, font_id: FontId) -> Option<&UiFontAtlasImage> {
        self.images.get(&font_id)
    }

    pub fn atlas_for_texture_id(
        &self,
        texture_id: u64,
    ) -> Option<(&MsdfFontAtlas, &UiFontAtlasImage)> {
        let font_id = FontId(texture_id);
        let atlas = self.atlases.get(&font_id)?;
        let image = self.images.get(&font_id)?;
        Some((atlas, image))
    }
}

impl FontAtlasSource for UiFontAtlasResource {
    fn atlas(&self, font_id: FontId) -> Option<&MsdfFontAtlas> {
        self.atlases.get(&font_id)
    }
}

impl Default for UiFontAtlasResource {
    fn default() -> Self {
        let (atlas, image) =
            build_default_editor_font_atlas().expect("default editor font atlas should load");
        let mut atlases = HashMap::new();
        let mut images = HashMap::new();
        atlases.insert(DEFAULT_EDITOR_FONT_ID, atlas);
        images.insert(DEFAULT_EDITOR_FONT_ID, image);
        Self { atlases, images }
    }
}

fn build_default_editor_font_atlas() -> anyhow::Result<(MsdfFontAtlas, UiFontAtlasImage)> {
    let font = Font::try_from_bytes(DEFAULT_EDITOR_FONT_BYTES)
        .ok_or_else(|| anyhow::anyhow!("invalid editor font bytes"))?;
    let scale = Scale::uniform(DEFAULT_EDITOR_FONT_BASE_SIZE);
    let v_metrics = font.v_metrics(scale);
    let charset = default_editor_charset();
    let mut glyph_entries = Vec::with_capacity(charset.len());
    let mut max_width = 1_u32;
    let mut max_height = 1_u32;

    for ch in charset {
        let scaled = font.glyph(ch).scaled(scale);
        let h_metrics = scaled.h_metrics();
        let exact_bb = scaled.exact_bounding_box();
        let positioned = scaled.positioned(point(0.0, 0.0));
        let pixel_bb = positioned.pixel_bounding_box();

        let (bitmap_width, bitmap_height, bitmap) = match pixel_bb {
            Some(bb) => {
                let width = bb.width().max(0) as u32;
                let height = bb.height().max(0) as u32;
                if width == 0 || height == 0 {
                    (0, 0, Vec::new())
                } else {
                    let mut data = vec![0_u8; (width * height) as usize];
                    positioned.draw(|x, y, value| {
                        let index = (y * width + x) as usize;
                        data[index] = (value.clamp(0.0, 1.0) * 255.0) as u8;
                    });
                    (width, height, data)
                }
            }
            None => (0, 0, Vec::new()),
        };

        max_width = max_width.max(bitmap_width);
        max_height = max_height.max(bitmap_height);

        let (plane_left, plane_top, plane_right, plane_bottom) = exact_bb
            .map(|bb| (bb.min.x, -bb.min.y, bb.max.x, -bb.max.y))
            .unwrap_or((0.0, 0.0, 0.0, 0.0));

        glyph_entries.push((
            ch,
            h_metrics.advance_width,
            plane_left,
            plane_top,
            plane_right,
            plane_bottom,
            bitmap_width,
            bitmap_height,
            bitmap,
        ));
    }

    let cell_width = max_width + DEFAULT_GLYPH_PADDING * 2;
    let cell_height = max_height + DEFAULT_GLYPH_PADDING * 2;
    let columns = DEFAULT_ATLAS_COLUMNS.max(1);
    let rows = ((glyph_entries.len() as u32).saturating_add(columns - 1)) / columns;
    let atlas_width = (columns * cell_width).max(1);
    let atlas_height = (rows * cell_height).max(1);
    let mut atlas_pixels = vec![0_u8; (atlas_width * atlas_height) as usize];
    let mut glyphs = HashMap::new();

    for (index, entry) in glyph_entries.into_iter().enumerate() {
        let (
            ch,
            advance,
            plane_left,
            plane_top,
            plane_right,
            plane_bottom,
            bitmap_width,
            bitmap_height,
            bitmap,
        ) = entry;
        let col = (index as u32) % columns;
        let row = (index as u32) / columns;
        let x = col * cell_width + DEFAULT_GLYPH_PADDING;
        let y = row * cell_height + DEFAULT_GLYPH_PADDING;

        if bitmap_width > 0 && bitmap_height > 0 {
            for src_y in 0..bitmap_height {
                let dst_y = y + src_y;
                let dst_offset = (dst_y * atlas_width + x) as usize;
                let src_offset = (src_y * bitmap_width) as usize;
                let copy_len = bitmap_width as usize;
                atlas_pixels[dst_offset..dst_offset + copy_len]
                    .copy_from_slice(&bitmap[src_offset..src_offset + copy_len]);
            }
        }

        let atlas_left = x as f32 / atlas_width as f32;
        let atlas_top = y as f32 / atlas_height as f32;
        let atlas_right = (x + bitmap_width.max(1)) as f32 / atlas_width as f32;
        let atlas_bottom = (y + bitmap_height.max(1)) as f32 / atlas_height as f32;

        glyphs.insert(
            ch,
            GlyphMetrics {
                advance,
                plane_left,
                plane_top,
                plane_right,
                plane_bottom,
                atlas_left,
                atlas_top,
                atlas_right,
                atlas_bottom,
            },
        );
    }

    let atlas = MsdfFontAtlas {
        font_id: DEFAULT_EDITOR_FONT_ID,
        texture_width: atlas_width,
        texture_height: atlas_height,
        metrics: FontFaceMetrics {
            ascender: v_metrics.ascent,
            descender: v_metrics.descent,
            line_height: v_metrics.ascent - v_metrics.descent + v_metrics.line_gap,
            base_size: DEFAULT_EDITOR_FONT_BASE_SIZE,
        },
        glyphs,
    };

    let image = UiFontAtlasImage {
        width: atlas_width,
        height: atlas_height,
        pixels: atlas_pixels,
    };

    Ok((atlas, image))
}

fn default_editor_charset() -> Vec<char> {
    let mut chars = (32_u8..=126_u8).map(char::from).collect::<Vec<_>>();
    chars.push('…');
    chars.push('•');
    chars
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct PreparedUiFrameResource {
    pub status: FeatureContributionStatus,
    pub fallback_policy: FeatureFallbackPolicy,
    pub payload: PreparedUiFrameContribution,
    payload_by_surface: std::collections::BTreeMap<RenderSurfaceId, PreparedUiFrameContribution>,
    status_by_surface: std::collections::BTreeMap<RenderSurfaceId, FeatureContributionStatus>,
}

impl Default for PreparedUiFrameResource {
    fn default() -> Self {
        Self {
            status: FeatureContributionStatus::Missing,
            fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            payload: PreparedUiFrameContribution::default(),
            payload_by_surface: std::collections::BTreeMap::new(),
            status_by_surface: std::collections::BTreeMap::new(),
        }
    }
}

impl PreparedUiFrameResource {
    pub fn payload_for_surface(
        &self,
        render_surface_id: RenderSurfaceId,
    ) -> &PreparedUiFrameContribution {
        self.payload_by_surface
            .get(&render_surface_id)
            .unwrap_or(&self.payload)
    }

    pub fn status_for_surface(
        &self,
        render_surface_id: RenderSurfaceId,
    ) -> FeatureContributionStatus {
        self.status_by_surface
            .get(&render_surface_id)
            .copied()
            .unwrap_or(self.status)
    }
}

pub fn prepare_ui_feature_resource_system(
    submissions: Res<SurfaceFrameSubmissionRegistryResource>,
    surfaces: Res<RenderSurfaceRegistryResource>,
    mut prepared: ResMut<PreparedUiFrameResource>,
) {
    let ordered = submissions.ordered_submissions();
    let (status, payload) = prepare_submissions(ordered);
    prepared.status = status;
    prepared.payload = payload;
    prepared.payload_by_surface.clear();
    prepared.status_by_surface.clear();
    for surface in surfaces.records() {
        let (status, payload) = prepare_submissions(
            submissions.ordered_submissions_for_surface(surface.render_surface_id),
        );
        prepared
            .payload_by_surface
            .insert(surface.render_surface_id, payload);
        prepared
            .status_by_surface
            .insert(surface.render_surface_id, status);
    }
}

fn prepare_submissions(
    submissions: Vec<&crate::plugins::render::features::SurfaceFrameSubmission>,
) -> (FeatureContributionStatus, PreparedUiFrameContribution) {
    let status = if submissions.is_empty() {
        FeatureContributionStatus::Missing
    } else {
        FeatureContributionStatus::Ready
    };
    let payload = PreparedUiFrameContribution {
        submissions: submissions
            .into_iter()
            .map(|submission| PreparedSurfaceFrameSubmission {
                producer_id: submission.producer_id,
                route: submission.route.as_str().to_string(),
                layer: submission.order.layer,
                priority: submission.order.priority,
                frame: submission.frame.clone(),
                rect_shader_asset_id: submission.rect_shader_asset_id.clone(),
            })
            .collect(),
    };
    (status, payload)
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource, Default)]
pub struct ViewportSurfaceBindingRegistryResource {
    registry: ViewportSurfaceBindingRegistry,
}

impl ViewportSurfaceBindingRegistryResource {
    pub fn registry(&self) -> &ViewportSurfaceBindingRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut ViewportSurfaceBindingRegistry {
        &mut self.registry
    }

    pub fn replace_registry(&mut self, registry: ViewportSurfaceBindingRegistry) {
        self.registry = registry;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph_metrics(atlas: &MsdfFontAtlas, ch: char) -> &GlyphMetrics {
        atlas
            .glyphs
            .get(&ch)
            .unwrap_or_else(|| panic!("glyph metrics missing for '{ch}'"))
    }

    #[test]
    fn default_editor_font_atlas_uses_baseline_up_orientation_for_representative_glyphs() {
        let (atlas, _image) =
            build_default_editor_font_atlas().expect("default atlas should build for tests");

        for ch in ['N', 'o', 'n', 'g', 'e', 'p', 'y'] {
            let metrics = glyph_metrics(&atlas, ch);
            assert!(
                metrics.plane_top > 0.0,
                "glyph '{ch}' should have positive plane_top with baseline-up metrics",
            );
        }

        for ch in ['g', 'p', 'y'] {
            let metrics = glyph_metrics(&atlas, ch);
            assert!(
                metrics.plane_bottom < -1.0,
                "descender glyph '{ch}' should have clearly negative plane_bottom; got {}",
                metrics.plane_bottom,
            );
        }

        for ch in ['N', 'o', 'n', 'e'] {
            let metrics = glyph_metrics(&atlas, ch);
            assert!(
                metrics.plane_bottom > -1.0,
                "non-descender glyph '{ch}' should stay near baseline for plane_bottom; got {}",
                metrics.plane_bottom,
            );
        }
    }
}
