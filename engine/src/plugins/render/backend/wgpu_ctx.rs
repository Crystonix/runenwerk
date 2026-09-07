use super::{build_surface_config, preferred_surface_format};
use anyhow::Result;
use pollster::block_on;
use runen_gpu::{
    GpuAcquiredSurfaceImage, GpuCapabilityFeature, GpuCapabilityProfile, GpuCapabilityRequirement,
    GpuContext, GpuContextDescriptor, GpuFormatRole, GpuPowerPreference, GpuPreferredFallback,
    GpuSurfaceAcquireErrorCategory, GpuSurfaceConfiguration, GpuSurfaceHandle, GpuTextureFormat,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use winit::window::Window;

use super::RenderSurfaceId;

#[derive(Debug)]
struct WgpuSurfaceState {
    surface: GpuSurfaceHandle,
    config: GpuSurfaceConfiguration,
}

#[derive(Debug)]
pub struct WgpuCtx {
    context: GpuContext,
    surfaces: BTreeMap<RenderSurfaceId, WgpuSurfaceState>,
}

/// Stable renderer-facing acquisition categories across the WGPU 30 surface-result cutover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RenderSurfaceAcquireError {
    #[error("render surface was lost")]
    Lost,
    #[error("render surface configuration is outdated")]
    Outdated,
    #[error("render surface acquisition timed out or was occluded")]
    Timeout,
    #[error("render surface acquisition failed validation")]
    Validation,
}

impl WgpuCtx {
    async fn new_async(window: Arc<Window>) -> Result<Self> {
        let mut requirements = GpuCapabilityProfile::DesktopPresentationBaseline.requirements();
        requirements.insert(GpuCapabilityRequirement::Preferred {
            feature: GpuCapabilityFeature::TimestampQuery,
            fallback: GpuPreferredFallback::DisableInstrumentation,
        })?;
        for feature in [
            GpuCapabilityFeature::Compute,
            GpuCapabilityFeature::IndirectExecution,
            GpuCapabilityFeature::StorageTexture,
            GpuCapabilityFeature::DepthAttachment,
        ] {
            requirements.insert(GpuCapabilityRequirement::Required(feature))?;
        }
        let mut descriptor = GpuContextDescriptor::new(requirements)
            .with_label("Runenwerk renderer")
            .with_provenance("Runenwerk renderer surface execution")
            .with_power_preference(GpuPowerPreference::HighPerformance);
        for (format, roles) in [
            (
                GpuTextureFormat::R8Unorm,
                &[
                    GpuFormatRole::Sampled,
                    GpuFormatRole::Filterable,
                    GpuFormatRole::CopyDestination,
                ][..],
            ),
            (
                GpuTextureFormat::Rgba8Unorm,
                &[
                    GpuFormatRole::Sampled,
                    GpuFormatRole::Filterable,
                    GpuFormatRole::StorageWrite,
                    GpuFormatRole::ColorAttachment,
                    GpuFormatRole::CopySource,
                    GpuFormatRole::CopyDestination,
                ][..],
            ),
            (
                GpuTextureFormat::Rgba8UnormSrgb,
                &[
                    GpuFormatRole::Sampled,
                    GpuFormatRole::Filterable,
                    GpuFormatRole::ColorAttachment,
                    GpuFormatRole::CopySource,
                    GpuFormatRole::CopyDestination,
                ][..],
            ),
            (
                GpuTextureFormat::Bgra8Unorm,
                &[
                    GpuFormatRole::Sampled,
                    GpuFormatRole::Filterable,
                    GpuFormatRole::ColorAttachment,
                    GpuFormatRole::CopySource,
                    GpuFormatRole::CopyDestination,
                ][..],
            ),
            (
                GpuTextureFormat::Bgra8UnormSrgb,
                &[
                    GpuFormatRole::Sampled,
                    GpuFormatRole::Filterable,
                    GpuFormatRole::ColorAttachment,
                    GpuFormatRole::CopySource,
                    GpuFormatRole::CopyDestination,
                ][..],
            ),
            (
                GpuTextureFormat::R32Uint,
                &[
                    GpuFormatRole::Sampled,
                    GpuFormatRole::StorageWrite,
                    GpuFormatRole::ColorAttachment,
                    GpuFormatRole::CopySource,
                    GpuFormatRole::CopyDestination,
                ][..],
            ),
            (
                GpuTextureFormat::Depth32Float,
                &[
                    GpuFormatRole::Sampled,
                    GpuFormatRole::DepthStencil,
                    GpuFormatRole::CopySource,
                    GpuFormatRole::CopyDestination,
                ][..],
            ),
        ] {
            for &role in roles {
                descriptor = descriptor.require_format_role(format, role);
            }
        }
        let (context, surface) =
            GpuContext::request_for_surface(descriptor, Arc::clone(&window)).await?;

        let size = window.inner_size();
        let caps = context.surface_capabilities(surface)?;
        let format = preferred_surface_format(&caps)
            .ok_or_else(|| anyhow::anyhow!("render surface reports no supported format"))?;
        let surface_config = build_surface_config(size.width, size.height, format, &caps)?;
        let surface = context.configure_surface(surface, surface_config.clone())?;

        Ok(Self {
            context,
            surfaces: BTreeMap::from([(
                RenderSurfaceId::primary(),
                WgpuSurfaceState {
                    surface,
                    config: surface_config,
                },
            )]),
        })
    }

    pub fn new(window: Arc<Window>) -> Result<Self> {
        block_on(Self::new_async(window))
    }

    pub fn attach_surface(
        &mut self,
        render_surface_id: RenderSurfaceId,
        window: Arc<Window>,
        target_size_px: (u32, u32),
    ) -> Result<()> {
        let surface = self.context.attach_surface(window)?;
        let caps = self.context.surface_capabilities(surface)?;
        let format = preferred_surface_format(&caps)
            .ok_or_else(|| anyhow::anyhow!("render surface reports no supported format"))?;
        let config = build_surface_config(target_size_px.0, target_size_px.1, format, &caps)?;
        let surface = self.context.configure_surface(surface, config.clone())?;
        self.surfaces
            .insert(render_surface_id, WgpuSurfaceState { surface, config });
        Ok(())
    }

    pub fn detach_surface(&mut self, render_surface_id: RenderSurfaceId) -> bool {
        let Some(state) = self.surfaces.get(&render_surface_id) else {
            return false;
        };
        if self.context.detach_surface(state.surface).is_err() {
            return false;
        }
        self.surfaces.remove(&render_surface_id);
        true
    }

    pub fn has_surface(&self, render_surface_id: RenderSurfaceId) -> bool {
        self.surfaces.contains_key(&render_surface_id)
    }

    pub fn surface_config(
        &self,
        render_surface_id: RenderSurfaceId,
    ) -> Option<&GpuSurfaceConfiguration> {
        self.surfaces
            .get(&render_surface_id)
            .map(|state| &state.config)
    }

    pub fn resize(&mut self, render_surface_id: RenderSurfaceId, width: u32, height: u32) -> bool {
        let Some(state) = self.surfaces.get_mut(&render_surface_id) else {
            return false;
        };
        let Ok(config) = GpuSurfaceConfiguration::new(
            width.max(1),
            height.max(1),
            state.config.format(),
            state.config.usages().iter().copied(),
            state.config.present_mode(),
            state.config.alpha_mode(),
            state.config.desired_maximum_frame_latency(),
            state.config.view_formats().iter().copied(),
        ) else {
            return false;
        };
        let Ok(surface) = self
            .context
            .configure_surface(state.surface, config.clone())
        else {
            return false;
        };
        state.surface = surface;
        state.config = config;
        true
    }

    pub fn acquire_surface_image(
        &self,
        render_surface_id: RenderSurfaceId,
    ) -> Result<GpuAcquiredSurfaceImage, RenderSurfaceAcquireError> {
        let surface = self
            .surfaces
            .get(&render_surface_id)
            .ok_or(RenderSurfaceAcquireError::Lost)?
            .surface;
        self.context
            .acquire_surface_image(surface)
            .map_err(|error| match error.category() {
                GpuSurfaceAcquireErrorCategory::Timeout
                | GpuSurfaceAcquireErrorCategory::Occluded => RenderSurfaceAcquireError::Timeout,
                GpuSurfaceAcquireErrorCategory::Outdated => RenderSurfaceAcquireError::Outdated,
                GpuSurfaceAcquireErrorCategory::Lost
                | GpuSurfaceAcquireErrorCategory::UnknownSurface
                | GpuSurfaceAcquireErrorCategory::StaleGeneration
                | GpuSurfaceAcquireErrorCategory::ContextOrDeviceUnavailableOrLost => {
                    RenderSurfaceAcquireError::Lost
                }
                GpuSurfaceAcquireErrorCategory::NotConfigured
                | GpuSurfaceAcquireErrorCategory::AlreadyAcquired
                | GpuSurfaceAcquireErrorCategory::Validation
                | GpuSurfaceAcquireErrorCategory::ForeignContext
                | GpuSurfaceAcquireErrorCategory::IdentityExhausted
                | GpuSurfaceAcquireErrorCategory::InternalInvariant => {
                    RenderSurfaceAcquireError::Validation
                }
            })
    }

    pub(crate) fn context(&self) -> &GpuContext {
        &self.context
    }
}
