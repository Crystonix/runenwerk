//! Runtime resources for the drawing app shell.

use std::collections::{BTreeMap, BTreeSet};

use drawing::{CanvasTileId, DrawingInkTileProduct, ProductQualityClass};

use crate::app::{DrawingInkSurfaceKind, RunenwerkDrawApp};

#[derive(Debug, Default, runen_ecs::Resource)]
pub struct DrawingHostResource {
    pub app: RunenwerkDrawApp,
}

#[derive(Debug, Default, runen_ecs::Resource)]
pub struct DrawingInkUploadTrackerResource {
    committed_generations: BTreeMap<DrawingInkUploadKey, u64>,
    preview_generations: BTreeMap<DrawingInkUploadKey, u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct DrawingInkUploadKey {
    quality_class: ProductQualityClass,
    tile_id: CanvasTileId,
}

impl DrawingInkUploadKey {
    fn from_product(product: &DrawingInkTileProduct) -> Self {
        Self {
            quality_class: product.metadata.quality_class,
            tile_id: product.metadata.tile_id,
        }
    }
}

impl DrawingInkUploadTrackerResource {
    pub fn retain_products(
        &mut self,
        surface_kind: DrawingInkSurfaceKind,
        products: &[DrawingInkTileProduct],
    ) {
        let retained_tile_ids = products
            .iter()
            .map(DrawingInkUploadKey::from_product)
            .collect::<BTreeSet<_>>();
        self.generations_mut(surface_kind)
            .retain(|key, _| retained_tile_ids.contains(key));
    }

    pub fn products_requiring_upload<'a>(
        &self,
        surface_kind: DrawingInkSurfaceKind,
        products: &'a [DrawingInkTileProduct],
    ) -> Vec<&'a DrawingInkTileProduct> {
        let generations = self.generations(surface_kind);
        products
            .iter()
            .filter(|product| {
                generations
                    .get(&DrawingInkUploadKey::from_product(product))
                    .copied()
                    != Some(product.descriptor_generation)
            })
            .collect()
    }

    pub fn record_submitted_uploads<'a>(
        &mut self,
        surface_kind: DrawingInkSurfaceKind,
        products: impl IntoIterator<Item = &'a DrawingInkTileProduct>,
    ) {
        let generations = self.generations_mut(surface_kind);
        for product in products {
            generations.insert(
                DrawingInkUploadKey::from_product(product),
                product.descriptor_generation,
            );
        }
    }

    fn generations(
        &self,
        surface_kind: DrawingInkSurfaceKind,
    ) -> &BTreeMap<DrawingInkUploadKey, u64> {
        match surface_kind {
            DrawingInkSurfaceKind::Committed | DrawingInkSurfaceKind::GpuCommitted => {
                &self.committed_generations
            }
            DrawingInkSurfaceKind::Preview | DrawingInkSurfaceKind::GpuPreview => {
                &self.preview_generations
            }
        }
    }

    fn generations_mut(
        &mut self,
        surface_kind: DrawingInkSurfaceKind,
    ) -> &mut BTreeMap<DrawingInkUploadKey, u64> {
        match surface_kind {
            DrawingInkSurfaceKind::Committed | DrawingInkSurfaceKind::GpuCommitted => {
                &mut self.committed_generations
            }
            DrawingInkSurfaceKind::Preview | DrawingInkSurfaceKind::GpuPreview => {
                &mut self.preview_generations
            }
        }
    }
}
