use super::{CaptureStage, CaptureTextureClass, RenderCapturePointIdentity, RenderPixelCoordinate};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderUvCoordinate {
    pub u: f32,
    pub v: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderCaptureSelector {
    pub flow_id: Option<String>,
    pub pass_id: Option<String>,
    pub stage: CaptureStage,
    pub resource_id: String,
    pub texture_class: CaptureTextureClass,
}

impl RenderCaptureSelector {
    pub fn named_pass_surface_color(
        flow_id: impl Into<String>,
        pass_id: impl Into<String>,
    ) -> Self {
        Self {
            flow_id: Some(flow_id.into()),
            pass_id: Some(pass_id.into()),
            stage: CaptureStage::After,
            resource_id: crate::plugins::render::api::SURFACE_COLOR_RESOURCE_LABEL.to_string(),
            texture_class: CaptureTextureClass::ImportedTexture,
        }
    }

    pub fn matches_point(&self, point: &RenderCapturePointIdentity) -> bool {
        if self.stage != point.stage {
            return false;
        }
        if self.resource_id != point.resource_id {
            return false;
        }
        if self.texture_class != point.texture_class {
            return false;
        }
        if let Some(flow_id) = self.flow_id.as_deref()
            && flow_id != point.flow_id
        {
            return false;
        }
        if let Some(pass_id) = self.pass_id.as_deref()
            && pass_id != point.pass_id
        {
            return false;
        }
        true
    }

    pub fn describe(&self) -> String {
        format!(
            "flow={} pass={} stage={} resource={} class={}",
            self.flow_id.as_deref().unwrap_or("*"),
            self.pass_id.as_deref().unwrap_or("*"),
            self.stage.as_str(),
            self.resource_id,
            self.texture_class.as_str(),
        )
    }

    pub fn stable_point_fallback(&self) -> RenderCapturePointIdentity {
        RenderCapturePointIdentity {
            flow_id: self.flow_id.clone().unwrap_or_else(|| "*".to_string()),
            pass_id: self.pass_id.clone().unwrap_or_else(|| "*".to_string()),
            stage: self.stage,
            resource_id: self.resource_id.clone(),
            texture_class: self.texture_class,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderPixelSampleMode {
    Center,
    Pixel(RenderPixelCoordinate),
    Uv(RenderUvCoordinate),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderPixelProbeAssertionMode {
    None,
    Exact([u8; 4]),
    Tolerance {
        expected: [u8; 4],
        tolerance: u8,
    },
    CompareToCapture {
        other_selector: RenderCaptureSelector,
        tolerance: u8,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderPixelProbeRequest {
    pub id: String,
    pub selector: RenderCaptureSelector,
    pub sample_mode: RenderPixelSampleMode,
    pub assertion: RenderPixelProbeAssertionMode,
}

impl RenderPixelProbeRequest {
    pub fn center(id: impl Into<String>, selector: RenderCaptureSelector) -> Self {
        Self {
            id: id.into(),
            selector,
            sample_mode: RenderPixelSampleMode::Center,
            assertion: RenderPixelProbeAssertionMode::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTextureDiffRequest {
    pub id: String,
    pub left_selector: RenderCaptureSelector,
    pub right_selector: RenderCaptureSelector,
    pub mismatch_sample_limit: usize,
    pub max_channel_delta: Option<u8>,
    pub max_changed_pixels_per_million: Option<u32>,
}

impl RenderTextureDiffRequest {
    pub fn new(
        id: impl Into<String>,
        left_selector: RenderCaptureSelector,
        right_selector: RenderCaptureSelector,
    ) -> Self {
        Self {
            id: id.into(),
            left_selector,
            right_selector,
            mismatch_sample_limit: 16,
            max_channel_delta: None,
            max_changed_pixels_per_million: None,
        }
    }

    pub fn with_thresholds(
        mut self,
        max_channel_delta: u8,
        max_changed_pixels_per_million: u32,
    ) -> Self {
        self.max_channel_delta = Some(max_channel_delta);
        self.max_changed_pixels_per_million = Some(max_changed_pixels_per_million);
        self
    }
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderDebugConfigResource {
    pub capture_selectors: Vec<RenderCaptureSelector>,
    pub pixel_probes: Vec<RenderPixelProbeRequest>,
    pub texture_diffs: Vec<RenderTextureDiffRequest>,
}

impl RenderDebugConfigResource {
    pub fn clear(&mut self) {
        self.capture_selectors.clear();
        self.pixel_probes.clear();
        self.texture_diffs.clear();
    }
}

pub const DEFAULT_RENDER_DIAGNOSTICS_SLOW_FRAME_THRESHOLD_MS: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderFrameDiagnosticsMode {
    Tiered,
    FullEveryFrame,
}

impl RenderFrameDiagnosticsMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tiered => "tiered",
            Self::FullEveryFrame => "full_every_frame",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderFrameDiagnosticsPolicyResource {
    pub mode: RenderFrameDiagnosticsMode,
    pub slow_frame_threshold_ms: f32,
    pub force_next_full_report: bool,
}

impl Default for RenderFrameDiagnosticsPolicyResource {
    fn default() -> Self {
        Self {
            mode: RenderFrameDiagnosticsMode::Tiered,
            slow_frame_threshold_ms: DEFAULT_RENDER_DIAGNOSTICS_SLOW_FRAME_THRESHOLD_MS,
            force_next_full_report: false,
        }
    }
}

impl RenderFrameDiagnosticsPolicyResource {
    pub fn tiered() -> Self {
        Self::default()
    }

    pub fn full_every_frame() -> Self {
        Self {
            mode: RenderFrameDiagnosticsMode::FullEveryFrame,
            ..Self::default()
        }
    }

    pub fn request_full_report(&mut self) {
        self.force_next_full_report = true;
    }
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderDebugControlResource {
    pub provenance_enabled: bool,
    pub capture_enabled: bool,
    pub readback_enabled: bool,
    pub artifact_export_enabled: bool,
    pub artifact_output_dir: PathBuf,
}

impl Default for RenderDebugControlResource {
    fn default() -> Self {
        Self {
            provenance_enabled: env_flag("RUNENWERK_RENDER_PROVENANCE"),
            capture_enabled: env_flag("RUNENWERK_RENDER_CAPTURE"),
            readback_enabled: env_flag("RUNENWERK_RENDER_READBACK"),
            artifact_export_enabled: env_flag("RUNENWERK_RENDER_EXPORT"),
            artifact_output_dir: std::env::var("RUNENWERK_RENDER_ARTIFACT_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("target/render-debug")),
        }
    }
}

impl RenderDebugControlResource {
    pub fn capture_readback_enabled(&self) -> bool {
        self.capture_enabled && self.readback_enabled
    }
}

fn env_flag(key: &str) -> bool {
    std::env::var(key)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_matches_full_identity() {
        let selector = RenderCaptureSelector {
            flow_id: Some("flow.main".to_string()),
            pass_id: Some("pass.compose".to_string()),
            stage: CaptureStage::After,
            resource_id: "surface.color".to_string(),
            texture_class: CaptureTextureClass::ImportedTexture,
        };
        let point = RenderCapturePointIdentity {
            flow_id: "flow.main".to_string(),
            pass_id: "pass.compose".to_string(),
            stage: CaptureStage::After,
            resource_id: "surface.color".to_string(),
            texture_class: CaptureTextureClass::ImportedTexture,
        };
        assert!(selector.matches_point(&point));
    }
}
