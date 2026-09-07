use super::*;
use runen_gpu::{
    GpuContext, GpuCopyExtent, GpuReadbackId, GpuReadbackOperation, GpuTextureAspect,
    GpuTextureCopyRegion, GpuTextureHandle, GpuTextureOrigin,
};

/// Capture identity and the canonical readback submitted by G5.
#[derive(Debug)]
pub struct PreparedCaptureReadback {
    pub selector_index: usize,
    pub selector: RenderCaptureSelector,
    pub identity: RenderCaptureIdentity,
    operation: GpuReadbackOperation,
    pub width: u32,
    pub height: u32,
    pub source_format: GpuTextureFormat,
}

impl PreparedCaptureReadback {
    pub(super) fn canonical_operation(&self) -> &GpuReadbackOperation {
        &self.operation
    }
}

#[derive(Debug, Clone)]
struct SelectorRuntimeState {
    selector: RenderCaptureSelector,
    capture_point: Option<RenderCapturePointIdentity>,
    frame_identity: Option<RenderCaptureIdentity>,
    readback_pending: bool,
    terminal: Option<RenderCaptureTerminal>,
}

impl SelectorRuntimeState {
    fn new(selector: RenderCaptureSelector) -> Self {
        Self {
            selector,
            capture_point: None,
            frame_identity: None,
            readback_pending: false,
            terminal: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameCaptureRuntime {
    pub frame_index: u64,
    selectors: Vec<SelectorRuntimeState>,
}

impl FrameCaptureRuntime {
    pub fn new(
        frame_index: u64,
        debug_control: &RenderDebugControlResource,
        selectors: &[RenderCaptureSelector],
    ) -> Self {
        let mut states = selectors
            .iter()
            .cloned()
            .map(SelectorRuntimeState::new)
            .collect::<Vec<_>>();

        if !debug_control.capture_enabled {
            for state in &mut states {
                state.terminal = Some(RenderCaptureTerminal::with_reason(
                    RenderCaptureTerminalCode::Disabled,
                    "capture_disabled",
                    "capture stage is disabled by RenderDebugControlResource",
                ));
            }
        } else if !debug_control.readback_enabled {
            for state in &mut states {
                state.terminal = Some(RenderCaptureTerminal::with_reason(
                    RenderCaptureTerminalCode::Disabled,
                    "readback_disabled",
                    "readback stage is disabled by RenderDebugControlResource",
                ));
            }
        }

        Self {
            frame_index,
            selectors: states,
        }
    }

    pub fn selectors_len(&self) -> usize {
        self.selectors.len()
    }

    pub fn selector_snapshot(
        &self,
        selector_index: usize,
    ) -> Option<(
        RenderCaptureSelector,
        bool,
        Option<RenderCapturePointIdentity>,
    )> {
        self.selectors.get(selector_index).map(|state| {
            (
                state.selector.clone(),
                state.terminal.is_some(),
                state.capture_point.clone(),
            )
        })
    }

    pub fn should_attempt_stage(&self, stage: CaptureStage) -> bool {
        self.selectors.iter().any(|state| {
            state.selector.stage == stage
                && state.terminal.is_none()
                && state.frame_identity.is_none()
        })
    }

    pub fn set_terminal_with_reason(
        &mut self,
        selector_index: usize,
        code: RenderCaptureTerminalCode,
        reason_code: &str,
        detail: String,
    ) {
        if let Some(state) = self.selectors.get_mut(selector_index)
            && state.terminal.is_none()
        {
            state.readback_pending = false;
            state.terminal = Some(RenderCaptureTerminal::with_reason(
                code,
                reason_code,
                detail,
            ));
        }
    }

    pub fn set_terminal(&mut self, selector_index: usize, terminal: RenderCaptureTerminal) {
        if let Some(state) = self.selectors.get_mut(selector_index) {
            state.readback_pending = false;
            state.terminal = Some(terminal);
        }
    }

    pub fn set_matched_identity(
        &mut self,
        selector_index: usize,
        capture_point: RenderCapturePointIdentity,
        frame_identity: RenderCaptureIdentity,
    ) {
        if let Some(state) = self.selectors.get_mut(selector_index) {
            state.capture_point = Some(capture_point);
            state.frame_identity = Some(frame_identity);
        }
    }

    pub fn set_readback_pending(&mut self, selector_index: usize) {
        if let Some(state) = self.selectors.get_mut(selector_index)
            && state.frame_identity.is_some()
            && state.terminal.is_none()
        {
            state.readback_pending = true;
        }
    }

    pub fn finalize_unresolved(&mut self) {
        for state in &mut self.selectors {
            if state.terminal.is_some() {
                continue;
            }
            if state.readback_pending {
                continue;
            }
            if state.frame_identity.is_some() {
                state.terminal = Some(RenderCaptureTerminal::with_reason(
                    RenderCaptureTerminalCode::Skipped,
                    "missing_terminal_capture_result",
                    "selector matched a capture point but no terminal capture result was produced",
                ));
                continue;
            }
            state.terminal = Some(RenderCaptureTerminal::with_reason(
                RenderCaptureTerminalCode::Unmatched,
                "selector_unmatched",
                "selector matched no capture point in this frame",
            ));
        }
    }

    pub fn into_plan_and_results(
        self,
    ) -> (ResolvedRenderCapturePlan, Vec<RenderCaptureSelectorResult>) {
        let mut plan = ResolvedRenderCapturePlan {
            frame_index: self.frame_index,
            selectors: Vec::with_capacity(self.selectors.len()),
        };
        let mut results = Vec::<RenderCaptureSelectorResult>::with_capacity(self.selectors.len());

        for (selector_index, state) in self.selectors.into_iter().enumerate() {
            let terminal = state.terminal.or_else(|| {
                (!state.readback_pending).then(|| {
                    RenderCaptureTerminal::with_reason(
                        RenderCaptureTerminalCode::Unmatched,
                        "selector_unmatched",
                        "selector matched no capture point in this frame",
                    )
                })
            });
            let capture_point = state
                .capture_point
                .clone()
                .unwrap_or_else(|| state.selector.stable_point_fallback());
            let resolution = match terminal.as_ref().map(|terminal| terminal.code) {
                Some(RenderCaptureTerminalCode::Unmatched) => RenderSelectorResolution::Unmatched {
                    reason: terminal
                        .as_ref()
                        .and_then(|value| value.reason.clone())
                        .unwrap_or_else(|| {
                            crate::plugins::render::inspect::RenderCaptureTerminalReason::new(
                                "selector_unmatched",
                                "selector matched no capture point in this frame",
                            )
                        }),
                },
                Some(RenderCaptureTerminalCode::Disabled) => RenderSelectorResolution::Disabled {
                    reason: terminal
                        .as_ref()
                        .and_then(|value| value.reason.clone())
                        .unwrap_or_else(|| {
                            crate::plugins::render::inspect::RenderCaptureTerminalReason::new(
                                "capture_disabled",
                                "capture is disabled",
                            )
                        }),
                },
                Some(RenderCaptureTerminalCode::Unsupported) => {
                    RenderSelectorResolution::Unsupported {
                        reason: terminal
                            .as_ref()
                            .and_then(|value| value.reason.clone())
                            .unwrap_or_else(|| {
                                crate::plugins::render::inspect::RenderCaptureTerminalReason::new(
                                    "capture_unsupported",
                                    "selector resolved to an unsupported capture path",
                                )
                            }),
                    }
                }
                Some(RenderCaptureTerminalCode::Skipped) => RenderSelectorResolution::Skipped {
                    reason: terminal
                        .as_ref()
                        .and_then(|value| value.reason.clone())
                        .unwrap_or_else(|| {
                            crate::plugins::render::inspect::RenderCaptureTerminalReason::new(
                                "capture_skipped",
                                "capture matched a point but did not produce a completed readback",
                            )
                        }),
                },
                Some(RenderCaptureTerminalCode::ReadbackFailed)
                | Some(RenderCaptureTerminalCode::ExportFailed)
                | Some(RenderCaptureTerminalCode::Completed) => {
                    if let Some(frame_identity) = state.frame_identity.clone() {
                        RenderSelectorResolution::Matched {
                            capture_point: capture_point.clone(),
                            frame_identity,
                        }
                    } else {
                        RenderSelectorResolution::Skipped {
                            reason: terminal.as_ref().and_then(|value| value.reason.clone()).unwrap_or_else(|| {
                                crate::plugins::render::inspect::RenderCaptureTerminalReason::new(
                                    "capture_missing_match",
                                    "selector terminal state did not include a matched frame id",
                                )
                            }),
                        }
                    }
                }
                None => {
                    if let Some(frame_identity) = state.frame_identity.clone() {
                        RenderSelectorResolution::Pending {
                            capture_point: capture_point.clone(),
                            frame_identity,
                        }
                    } else {
                        RenderSelectorResolution::Skipped {
                            reason:
                                crate::plugins::render::inspect::RenderCaptureTerminalReason::new(
                                    "capture_missing_match",
                                    "pending capture state did not include a matched frame id",
                                ),
                        }
                    }
                }
            };

            plan.selectors.push(
                crate::plugins::render::inspect::ResolvedRenderCaptureSelector {
                    selector_index,
                    selector: state.selector.clone(),
                    resolution,
                },
            );
            if let Some(terminal) = terminal {
                results.push(RenderCaptureSelectorResult {
                    selector_index,
                    selector: state.selector,
                    capture_point,
                    frame_identity: state.frame_identity,
                    terminal,
                    artifact_path: None,
                });
            }
        }

        (plan, results)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureReadbackMode {
    Rgba8,
    Bgra8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureReadbackFormat {
    pub mode: TextureReadbackMode,
}

#[derive(Debug, Clone, Copy)]
pub enum CaptureTextureSource<'a> {
    Surface { handle: &'a GpuTextureHandle },
    Logical { handle: &'a GpuTextureHandle },
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_texture_capture_readback(
    _context: &GpuContext,
    selector_index: usize,
    selector: RenderCaptureSelector,
    identity: RenderCaptureIdentity,
    texture: CaptureTextureSource<'_>,
    size: (u32, u32),
    source_format: GpuTextureFormat,
    _readback_format: TextureReadbackFormat,
) -> Result<PreparedCaptureReadback> {
    let width = size.0.max(1);
    let height = size.1.max(1);
    let handle = match texture {
        CaptureTextureSource::Surface { handle } | CaptureTextureSource::Logical { handle } => {
            handle
        }
    };
    let operation = canonical_capture_readback_operation(handle, width, height)?;

    Ok(PreparedCaptureReadback {
        selector_index,
        selector,
        identity,
        operation,
        width,
        height,
        source_format,
    })
}

fn canonical_capture_readback_operation(
    texture: &GpuTextureHandle,
    width: u32,
    height: u32,
) -> Result<GpuReadbackOperation> {
    let region = GpuTextureCopyRegion::new(
        texture,
        0,
        GpuTextureOrigin::new(0, 0, 0),
        GpuTextureAspect::Color,
        GpuCopyExtent::new(width, height, 1)?,
    )?;
    Ok(GpuReadbackOperation::new(
        region.into(),
        GpuReadbackId::allocate()?,
    )?)
}

pub fn texture_readback_format(format: GpuTextureFormat) -> Option<TextureReadbackFormat> {
    let mode = match format {
        GpuTextureFormat::Rgba8Unorm | GpuTextureFormat::Rgba8UnormSrgb => {
            TextureReadbackMode::Rgba8
        }
        GpuTextureFormat::Bgra8Unorm | GpuTextureFormat::Bgra8UnormSrgb => {
            TextureReadbackMode::Bgra8
        }
        _ => return None,
    };
    Some(TextureReadbackFormat { mode })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::render::renderer::resource_descriptors::texture_descriptor;
    use runen_gpu::{
        GpuResourceLifetime, GpuTextureFormat, GpuTextureUsage, GpuTransferRegion,
        GpuWorkResourceIdAllocator,
    };

    #[test]
    fn accepted_pending_capture_is_not_finalized_as_skipped() {
        let selector = RenderCaptureSelector::named_pass_surface_color("flow", "pass");
        let control = RenderDebugControlResource {
            capture_enabled: true,
            readback_enabled: true,
            ..RenderDebugControlResource::default()
        };
        let mut runtime = FrameCaptureRuntime::new(37, &control, std::slice::from_ref(&selector));
        let capture_point = selector.stable_point_fallback();
        runtime.set_matched_identity(
            0,
            capture_point.clone(),
            RenderCaptureIdentity {
                frame_index: 37,
                pass_label: "pass".to_string(),
                capture_point,
            },
        );
        runtime.set_readback_pending(0);
        runtime.finalize_unresolved();

        let (plan, results) = runtime.into_plan_and_results();
        assert!(results.is_empty());
        assert!(matches!(
            plan.selectors[0].resolution,
            RenderSelectorResolution::Pending { .. }
        ));
    }

    #[test]
    fn canonical_capture_readback_uses_exact_logical_texture_region() {
        let mut allocator = GpuWorkResourceIdAllocator::new();
        let texture = allocator
            .allocate_texture_handle(
                texture_descriptor(
                    "capture semantic source",
                    (7, 5),
                    GpuTextureFormat::Rgba8Unorm,
                    [GpuTextureUsage::CopySource],
                    GpuResourceLifetime::Transient,
                )
                .expect("capture source descriptor should be valid"),
            )
            .expect("capture source handle should allocate");

        let operation = canonical_capture_readback_operation(&texture, 7, 5)
            .expect("copy-source texture should admit canonical capture readback");
        let GpuTransferRegion::Texture(region) = operation.source() else {
            panic!("canonical capture readback should have a texture source");
        };
        assert_eq!(
            region.texture().diagnostic_identity(),
            texture.diagnostic_identity()
        );
        assert_eq!(region.extent(), GpuCopyExtent::new(7, 5, 1).unwrap());
    }

    #[test]
    fn canonical_capture_readback_requires_copy_source_usage() {
        let mut allocator = GpuWorkResourceIdAllocator::new();
        let texture = allocator
            .allocate_texture_handle(
                texture_descriptor(
                    "capture non-copy source",
                    (4, 4),
                    GpuTextureFormat::Rgba8Unorm,
                    [GpuTextureUsage::ColorAttachment],
                    GpuResourceLifetime::Transient,
                )
                .expect("capture source descriptor should be valid"),
            )
            .expect("capture source handle should allocate");

        assert!(canonical_capture_readback_operation(&texture, 4, 4).is_err());
    }
}
