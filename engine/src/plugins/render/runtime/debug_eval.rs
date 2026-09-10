use crate::plugins::inspect::{
    RenderCaptureSelector, RenderCaptureTerminalCode, RenderCaptureTerminalReason,
    RenderCapturedTexture, RenderPixelCoordinate, RenderPixelProbeAssertionMode,
    RenderPixelProbeRequest, RenderPixelProbeResult, RenderPixelProbeStatus, RenderPixelSampleMode,
    RenderTextureDiffMetrics, RenderTextureDiffMismatchSample, RenderTextureDiffRequest,
    RenderTextureDiffResult, RenderTextureDiffStatus,
};

pub(crate) fn evaluate_pixel_probes(
    probes: &[RenderPixelProbeRequest],
    captures: &[RenderCapturedTexture],
) -> Vec<RenderPixelProbeResult> {
    probes
        .iter()
        .map(|probe| {
            let capture = find_capture_for_selector(captures, &probe.selector);
            let capture_point_identity = capture
                .map(|value| value.identity.capture_point.clone())
                .unwrap_or_else(|| probe.selector.stable_point_fallback());
            let frame_identity = capture.map(|value| value.identity.clone());
            let comparison_mode = probe.assertion.clone();

            let mut result = RenderPixelProbeResult {
                probe_id: probe.id.clone(),
                capture_point_identity,
                frame_identity,
                sample_mode: probe.sample_mode,
                resolved_coordinate: None,
                comparison_mode,
                sampled_rgba8: None,
                compared_rgba8: None,
                status: RenderPixelProbeStatus::Skipped,
                message: None,
            };

            let Some(capture) = capture else {
                result.message = Some(RenderCaptureTerminalReason::new(
                    "capture_missing_for_probe",
                    "no completed capture matched the probe selector",
                ));
                return result;
            };

            if capture.terminal.code != RenderCaptureTerminalCode::Completed {
                result.message = Some(RenderCaptureTerminalReason::new(
                    "capture_not_completed",
                    format!(
                        "probe target capture terminal state is '{}'",
                        capture.terminal.code.as_str()
                    ),
                ));
                return result;
            }

            let Some(coordinate) = resolve_probe_coordinate(probe.sample_mode, capture) else {
                result.message = Some(RenderCaptureTerminalReason::new(
                    "invalid_probe_coordinate",
                    "probe sample coordinate is outside capture bounds",
                ));
                return result;
            };

            result.resolved_coordinate = Some(coordinate);
            let sampled = capture.sample_pixel_rgba8(coordinate);
            result.sampled_rgba8 = sampled;

            match &probe.assertion {
                RenderPixelProbeAssertionMode::None => {
                    result.status = if sampled.is_some() {
                        RenderPixelProbeStatus::Sampled
                    } else {
                        RenderPixelProbeStatus::Skipped
                    };
                    if sampled.is_none() {
                        result.message = Some(RenderCaptureTerminalReason::new(
                            "probe_sample_missing",
                            "probe target did not expose sampled rgba8 pixel",
                        ));
                    }
                }
                RenderPixelProbeAssertionMode::Exact(expected) => match sampled {
                    Some(sampled) => {
                        if sampled == *expected {
                            result.status = RenderPixelProbeStatus::Passed;
                        } else {
                            result.status = RenderPixelProbeStatus::Failed;
                            result.message = Some(RenderCaptureTerminalReason::new(
                                "probe_exact_mismatch",
                                format!(
                                    "expected {:?} but sampled {:?} at ({}, {})",
                                    expected, sampled, coordinate.x, coordinate.y
                                ),
                            ));
                        }
                    }
                    None => {
                        result.status = RenderPixelProbeStatus::Skipped;
                        result.message = Some(RenderCaptureTerminalReason::new(
                            "probe_sample_missing",
                            "probe target did not expose sampled rgba8 pixel",
                        ));
                    }
                },
                RenderPixelProbeAssertionMode::Tolerance {
                    expected,
                    tolerance,
                } => match sampled {
                    Some(sampled) => {
                        let max_distance = sampled
                            .into_iter()
                            .zip(expected.iter().copied())
                            .map(|(actual, wanted)| actual.abs_diff(wanted))
                            .max()
                            .unwrap_or(0);
                        if max_distance <= *tolerance {
                            result.status = RenderPixelProbeStatus::Passed;
                        } else {
                            result.status = RenderPixelProbeStatus::Failed;
                            result.message = Some(RenderCaptureTerminalReason::new(
                                "probe_tolerance_mismatch",
                                format!(
                                    "max channel delta {} exceeds tolerance {}",
                                    max_distance, tolerance
                                ),
                            ));
                        }
                    }
                    None => {
                        result.status = RenderPixelProbeStatus::Skipped;
                        result.message = Some(RenderCaptureTerminalReason::new(
                            "probe_sample_missing",
                            "probe target did not expose sampled rgba8 pixel",
                        ));
                    }
                },
                RenderPixelProbeAssertionMode::CompareToCapture {
                    other_selector,
                    tolerance,
                } => {
                    let Some(sampled) = sampled else {
                        result.status = RenderPixelProbeStatus::Skipped;
                        result.message = Some(RenderCaptureTerminalReason::new(
                            "probe_sample_missing",
                            "probe target did not expose sampled rgba8 pixel",
                        ));
                        return result;
                    };

                    let Some(other_capture) = find_capture_for_selector(captures, other_selector)
                    else {
                        result.status = RenderPixelProbeStatus::Skipped;
                        result.message = Some(RenderCaptureTerminalReason::new(
                            "probe_compare_capture_missing",
                            "comparison capture selector did not resolve to a completed capture",
                        ));
                        return result;
                    };

                    if other_capture.terminal.code != RenderCaptureTerminalCode::Completed {
                        result.status = RenderPixelProbeStatus::Skipped;
                        result.message = Some(RenderCaptureTerminalReason::new(
                            "probe_compare_capture_not_completed",
                            format!(
                                "comparison capture terminal state is '{}'",
                                other_capture.terminal.code.as_str()
                            ),
                        ));
                        return result;
                    }

                    let Some(other_coordinate) =
                        resolve_probe_coordinate(probe.sample_mode, other_capture)
                    else {
                        result.status = RenderPixelProbeStatus::Skipped;
                        result.message = Some(RenderCaptureTerminalReason::new(
                            "probe_compare_coordinate_invalid",
                            "comparison capture coordinate is outside capture bounds",
                        ));
                        return result;
                    };

                    let Some(other_sampled) = other_capture.sample_pixel_rgba8(other_coordinate)
                    else {
                        result.status = RenderPixelProbeStatus::Skipped;
                        result.message = Some(RenderCaptureTerminalReason::new(
                            "probe_compare_sample_missing",
                            "comparison capture did not expose sampled rgba8 pixel",
                        ));
                        return result;
                    };

                    result.compared_rgba8 = Some(other_sampled);
                    let max_distance = sampled
                        .into_iter()
                        .zip(other_sampled)
                        .map(|(left, right)| left.abs_diff(right))
                        .max()
                        .unwrap_or(0);

                    if max_distance <= *tolerance {
                        result.status = RenderPixelProbeStatus::Passed;
                    } else {
                        result.status = RenderPixelProbeStatus::Failed;
                        result.message = Some(RenderCaptureTerminalReason::new(
                            "probe_compare_mismatch",
                            format!(
                                "max channel delta {} exceeds tolerance {}",
                                max_distance, tolerance
                            ),
                        ));
                    }
                }
            }

            result
        })
        .collect()
}

pub(crate) fn evaluate_texture_diffs(
    diffs: &[RenderTextureDiffRequest],
    captures: &[RenderCapturedTexture],
) -> Vec<RenderTextureDiffResult> {
    diffs
        .iter()
        .map(|request| {
            let left_capture = find_capture_for_selector(captures, &request.left_selector);
            let right_capture = find_capture_for_selector(captures, &request.right_selector);

            let mut result = RenderTextureDiffResult {
                diff_id: request.id.clone(),
                request: request.clone(),
                left_capture_point: left_capture
                    .map(|value| value.identity.capture_point.clone())
                    .unwrap_or_else(|| request.left_selector.stable_point_fallback()),
                right_capture_point: right_capture
                    .map(|value| value.identity.capture_point.clone())
                    .unwrap_or_else(|| request.right_selector.stable_point_fallback()),
                left_frame_identity: left_capture.map(|value| value.identity.clone()),
                right_frame_identity: right_capture.map(|value| value.identity.clone()),
                status: RenderTextureDiffStatus::Skipped,
                metrics: None,
                mismatch_samples: Vec::new(),
                diff_image_path: None,
                message: None,
            };

            let Some(left) = left_capture else {
                result.message = Some(RenderCaptureTerminalReason::new(
                    "diff_left_capture_missing",
                    "left selector did not resolve to a completed capture",
                ));
                return result;
            };

            let Some(right) = right_capture else {
                result.message = Some(RenderCaptureTerminalReason::new(
                    "diff_right_capture_missing",
                    "right selector did not resolve to a completed capture",
                ));
                return result;
            };

            if left.terminal.code != RenderCaptureTerminalCode::Completed
                || right.terminal.code != RenderCaptureTerminalCode::Completed
            {
                result.message = Some(RenderCaptureTerminalReason::new(
                    "diff_capture_not_completed",
                    "both captures must be completed before running texture diff",
                ));
                return result;
            }

            let (Some(left_pixels), Some(right_pixels)) =
                (left.bytes_rgba8.as_ref(), right.bytes_rgba8.as_ref())
            else {
                result.message = Some(RenderCaptureTerminalReason::new(
                    "diff_pixels_missing",
                    "both captures must include rgba8 bytes before running texture diff",
                ));
                return result;
            };

            if left.width != right.width || left.height != right.height {
                result.status = RenderTextureDiffStatus::Failed;
                result.message = Some(RenderCaptureTerminalReason::new(
                    "diff_dimensions_mismatch",
                    format!(
                        "left is {}x{} but right is {}x{}",
                        left.width, left.height, right.width, right.height
                    ),
                ));
                return result;
            }

            let pixel_count = (left.width as usize).saturating_mul(left.height as usize);
            if pixel_count == 0 {
                result.status = RenderTextureDiffStatus::Compared;
                result.metrics = Some(RenderTextureDiffMetrics {
                    total_pixel_count: 0,
                    changed_pixel_count: 0,
                    changed_pixel_ratio: 0.0,
                    max_delta: 0,
                    mean_delta: 0.0,
                });
                return result;
            }

            let mut changed_pixel_count = 0u64;
            let mut max_delta = 0u8;
            let mut total_delta = 0u64;

            for pixel_index in 0..pixel_count {
                let byte_index = pixel_index * 4;
                let left_rgba = [
                    left_pixels[byte_index],
                    left_pixels[byte_index + 1],
                    left_pixels[byte_index + 2],
                    left_pixels[byte_index + 3],
                ];
                let right_rgba = [
                    right_pixels[byte_index],
                    right_pixels[byte_index + 1],
                    right_pixels[byte_index + 2],
                    right_pixels[byte_index + 3],
                ];

                let pixel_delta = left_rgba
                    .into_iter()
                    .zip(right_rgba)
                    .map(|(left_value, right_value)| left_value.abs_diff(right_value))
                    .max()
                    .unwrap_or(0);

                max_delta = max_delta.max(pixel_delta);
                total_delta = total_delta.saturating_add(pixel_delta as u64);

                if pixel_delta == 0 {
                    continue;
                }

                changed_pixel_count = changed_pixel_count.saturating_add(1);
                if result.mismatch_samples.len() < request.mismatch_sample_limit {
                    let x = (pixel_index as u32) % left.width.max(1);
                    let y = (pixel_index as u32) / left.width.max(1);
                    result
                        .mismatch_samples
                        .push(RenderTextureDiffMismatchSample {
                            coordinate: RenderPixelCoordinate { x, y },
                            left_rgba8: left_rgba,
                            right_rgba8: right_rgba,
                            max_channel_delta: pixel_delta,
                        });
                }
            }

            let changed_pixel_ratio = changed_pixel_count as f32 / pixel_count as f32;
            let metrics = RenderTextureDiffMetrics {
                total_pixel_count: pixel_count as u64,
                changed_pixel_count,
                changed_pixel_ratio,
                max_delta,
                mean_delta: total_delta as f32 / pixel_count as f32,
            };
            result.status = texture_diff_threshold_status(request, &metrics);
            if matches!(result.status, RenderTextureDiffStatus::Failed) {
                result.message = Some(texture_diff_threshold_message(request, &metrics));
            }
            result.metrics = Some(metrics);

            result
        })
        .collect()
}

fn texture_diff_threshold_status(
    request: &RenderTextureDiffRequest,
    metrics: &RenderTextureDiffMetrics,
) -> RenderTextureDiffStatus {
    if let Some(max_channel_delta) = request.max_channel_delta
        && metrics.max_delta > max_channel_delta
    {
        return RenderTextureDiffStatus::Failed;
    }

    if let Some(max_changed_pixels_per_million) = request.max_changed_pixels_per_million {
        let allowed_changed_pixels =
            allowed_changed_pixels(metrics.total_pixel_count, max_changed_pixels_per_million);
        if metrics.changed_pixel_count > allowed_changed_pixels {
            return RenderTextureDiffStatus::Failed;
        }
    }

    RenderTextureDiffStatus::Compared
}

fn texture_diff_threshold_message(
    request: &RenderTextureDiffRequest,
    metrics: &RenderTextureDiffMetrics,
) -> RenderCaptureTerminalReason {
    if let Some(max_channel_delta) = request.max_channel_delta
        && metrics.max_delta > max_channel_delta
    {
        return RenderCaptureTerminalReason::new(
            "diff_max_delta_exceeded",
            format!(
                "max channel delta {} exceeds threshold {}",
                metrics.max_delta, max_channel_delta
            ),
        );
    }

    let max_changed_pixels_per_million = request.max_changed_pixels_per_million.unwrap_or(0);
    let allowed_changed_pixels =
        allowed_changed_pixels(metrics.total_pixel_count, max_changed_pixels_per_million);
    RenderCaptureTerminalReason::new(
        "diff_changed_pixel_ratio_exceeded",
        format!(
            "changed pixels {} exceeds allowed {} ({}/1_000_000)",
            metrics.changed_pixel_count, allowed_changed_pixels, max_changed_pixels_per_million
        ),
    )
}

fn allowed_changed_pixels(total_pixel_count: u64, max_changed_pixels_per_million: u32) -> u64 {
    total_pixel_count
        .saturating_mul(max_changed_pixels_per_million as u64)
        .saturating_add(999_999)
        / 1_000_000
}

fn find_capture_for_selector<'a>(
    captures: &'a [RenderCapturedTexture],
    selector: &RenderCaptureSelector,
) -> Option<&'a RenderCapturedTexture> {
    captures
        .iter()
        .find(|capture| selector.matches_point(&capture.identity.capture_point))
}

fn resolve_probe_coordinate(
    mode: RenderPixelSampleMode,
    capture: &RenderCapturedTexture,
) -> Option<RenderPixelCoordinate> {
    if capture.width == 0 || capture.height == 0 {
        return None;
    }

    match mode {
        RenderPixelSampleMode::Center => Some(RenderPixelCoordinate {
            x: capture.width / 2,
            y: capture.height / 2,
        }),
        RenderPixelSampleMode::Pixel(coordinate) => {
            if coordinate.x >= capture.width || coordinate.y >= capture.height {
                return None;
            }
            Some(coordinate)
        }
        RenderPixelSampleMode::Uv(uv) => {
            let clamped_u = uv.u.clamp(0.0, 1.0);
            let clamped_v = uv.v.clamp(0.0, 1.0);
            let x = ((capture.width - 1) as f32 * clamped_u).round() as u32;
            let y = ((capture.height - 1) as f32 * clamped_v).round() as u32;
            Some(RenderPixelCoordinate { x, y })
        }
    }
}

pub(crate) fn clamp_lines(lines: &mut Vec<String>, max_lines: usize) {
    if max_lines == 0 {
        lines.clear();
        return;
    }
    let overflow = lines.len().saturating_sub(max_lines);
    if overflow > 0 {
        lines.drain(..overflow);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::inspect::{
        CaptureStage, CaptureTextureClass, RenderCaptureIdentity, RenderCaptureTerminal,
    };
    use crate::plugins::render::frame::{
        PreparedDeformationFeatureContribution, PreparedDeformationStream, PreparedDrawBatch,
        PreparedDrawFeatureContribution, PreparedFeaturePayload,
        PreparedMaterialFeatureContribution, PreparedMaterialInstanceInput,
        PreparedMaterialOutputTarget, PreparedMaterialParameterInput,
        PreparedMaterialParameterKind, PreparedMaterialParameterPayloadV1,
        PreparedMaterialParameterProfile,
    };
    use crate::plugins::render::*;

    const CUSTOM_FEATURE_ID: RenderFeatureId = render_feature_id(999);

    const fn render_feature_id(raw: u64) -> RenderFeatureId {
        match RenderFeatureId::try_from_raw(raw) {
            Ok(id) => id,
            Err(_) => panic!("render feature id constants must be non-zero"),
        }
    }

    fn test_world() -> runen_ecs::World {
        let mut world = runen_ecs::World::default();
        let mut registry = RenderFeatureRegistryResource::default();
        registry.sync_order();
        world.insert_resource(registry);
        world
    }

    #[test]
    fn frame_prepare_ingests_draw_material_deformation_feature_resources() {
        let mut world = test_world();
        world.insert_resource(PreparedDrawFeatureResource {
            status: FeatureContributionStatus::Ready,
            fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            payload: PreparedDrawFeatureContribution {
                batches: vec![PreparedDrawBatch {
                    batch_id: "batch.0".to_string(),
                    mesh_ref: "mesh.0".to_string(),
                    material_ref: "material.0".to_string(),
                    instance_count: 2,
                }],
            },
        });
        world.insert_resource(PreparedMaterialFeatureResource {
            status: FeatureContributionStatus::Ready,
            fallback_policy: FeatureFallbackPolicy::SkipFeaturePasses,
            payload: PreparedMaterialFeatureContribution {
                instances: vec![PreparedMaterialInstanceInput {
                    material_instance_id: "mat.instance".to_string(),
                    specialization_key_fragment: "opaque".to_string(),
                    parameter_payload: PreparedMaterialParameterPayloadV1::new(
                        PreparedMaterialParameterProfile::PbrPreview,
                        PreparedMaterialOutputTarget::PbrPreview,
                        [PreparedMaterialParameterInput::new(
                            "roughness",
                            PreparedMaterialParameterKind::Scalar,
                        )],
                    ),
                    texture_bindings: Vec::new(),
                }],
                binding_table: PreparedMaterialBindingTable::default(),
                scene_bundle: None,
                model_mesh_material_selections: Vec::new(),
            },
        });
        world.insert_resource(PreparedDeformationFeatureResource {
            status: FeatureContributionStatus::Stale,
            fallback_policy: FeatureFallbackPolicy::ReuseLastGood,
            payload: PreparedDeformationFeatureContribution {
                streams: vec![PreparedDeformationStream {
                    stream_id: "skin.stream".to_string(),
                    input_pose_ref: "pose.current".to_string(),
                    output_buffer_ref: "buffer.skinning".to_string(),
                }],
            },
        });

        let contributions = frame_prepare::build_frame_feature_contributions(
            &world,
            "world".to_string(),
            "overlay".to_string(),
            &[],
        );

        let draw = contributions
            .feature(&WORLD_DRAW_RENDER_FEATURE_ID)
            .expect("draw contribution should be published");
        assert_eq!(draw.status, FeatureContributionStatus::Ready);
        assert!(matches!(draw.payload, PreparedFeaturePayload::Draw(_)));

        let material = contributions
            .feature(&MATERIAL_RENDER_FEATURE_ID)
            .expect("material contribution should be published");
        assert_eq!(material.status, FeatureContributionStatus::Ready);
        assert!(matches!(
            material.payload,
            PreparedFeaturePayload::Material(_)
        ));

        let deformation = contributions
            .feature(&DEFORMATION_RENDER_FEATURE_ID)
            .expect("deformation contribution should be published");
        assert_eq!(deformation.status, FeatureContributionStatus::Stale);
        assert_eq!(
            deformation.fallback_policy,
            FeatureFallbackPolicy::SkipFeaturePasses
        );
        assert!(matches!(
            deformation.payload,
            PreparedFeaturePayload::Deformation(_)
        ));
    }

    #[test]
    fn material_handoff_feature_resource_reaches_material_render_feature_contribution() {
        let mut world = test_world();
        world.insert_resource(PreparedMaterialFeatureResource {
            status: FeatureContributionStatus::Ready,
            fallback_policy: FeatureFallbackPolicy::ReuseLastGood,
            payload: PreparedMaterialFeatureContribution {
                instances: vec![PreparedMaterialInstanceInput {
                    material_instance_id: "material.product.42".to_string(),
                    specialization_key_fragment: "material.first_slice.render_material".to_string(),
                    parameter_payload: PreparedMaterialParameterPayloadV1::new(
                        PreparedMaterialParameterProfile::RenderMaterial,
                        PreparedMaterialOutputTarget::RenderMaterial,
                        [PreparedMaterialParameterInput::new(
                            "roughness",
                            PreparedMaterialParameterKind::Scalar,
                        )],
                    ),
                    texture_bindings: Vec::new(),
                }],
                binding_table: PreparedMaterialBindingTable::default(),
                scene_bundle: None,
                model_mesh_material_selections: Vec::new(),
            },
        });

        let contributions = frame_prepare::build_frame_feature_contributions(
            &world,
            "world".to_string(),
            "overlay".to_string(),
            &[],
        );

        let material = contributions
            .feature(&MATERIAL_RENDER_FEATURE_ID)
            .expect("material contribution should be published");
        assert_eq!(material.status, FeatureContributionStatus::Ready);
        assert_eq!(
            material.fallback_policy,
            FeatureFallbackPolicy::SkipFeaturePasses
        );
        let PreparedFeaturePayload::Material(payload) = &material.payload else {
            panic!("material feature should carry material payload");
        };
        assert_eq!(payload.instances.len(), 1);
        assert_eq!(
            payload.instances[0].specialization_key_fragment,
            "material.first_slice.render_material"
        );
        let encoded = payload.instances[0].parameter_payload.encode_v1();
        let decoded =
            PreparedMaterialParameterPayloadV1::decode_v1(&encoded).expect("payload should decode");
        assert_eq!(
            decoded.profile,
            PreparedMaterialParameterProfile::RenderMaterial
        );
        assert_eq!(
            decoded.output_target,
            PreparedMaterialOutputTarget::RenderMaterial
        );
        assert!(!String::from_utf8_lossy(&encoded).contains("Scalar"));
    }

    #[test]
    fn prepare_inserts_missing_gate_for_execution_referenced_feature_without_payload() {
        let world = test_world();
        let execution_feature_ids = vec![CUSTOM_FEATURE_ID];
        let contributions =
            crate::plugins::render::runtime::frame_prepare::build_frame_feature_contributions(
                &world,
                "world".to_string(),
                "overlay".to_string(),
                &execution_feature_ids,
            );

        let missing = contributions
            .feature(&CUSTOM_FEATURE_ID)
            .expect("execution-referenced feature should still publish gate");
        assert_eq!(missing.status, FeatureContributionStatus::Missing);
        assert_eq!(
            missing.fallback_policy,
            FeatureFallbackPolicy::SkipFeaturePasses
        );
    }

    fn test_selector(pass_id: &str) -> RenderCaptureSelector {
        RenderCaptureSelector {
            flow_id: Some("flow.main".to_string()),
            pass_id: Some(pass_id.to_string()),
            stage: CaptureStage::After,
            resource_id: "surface.color".to_string(),
            texture_class: CaptureTextureClass::ImportedTexture,
        }
    }

    fn completed_capture(
        frame_index: u64,
        selector: &RenderCaptureSelector,
        pixels: [u8; 4],
    ) -> RenderCapturedTexture {
        RenderCapturedTexture {
            identity: RenderCaptureIdentity {
                frame_index,
                pass_label: selector
                    .pass_id
                    .clone()
                    .unwrap_or_else(|| "pass".to_string()),
                capture_point: selector.stable_point_fallback(),
            },
            width: 1,
            height: 1,
            format: "Rgba8Unorm".to_string(),
            bytes_rgba8: Some(pixels.to_vec()),
            terminal: RenderCaptureTerminal::completed(),
        }
    }

    #[test]
    fn pixel_probe_results_include_identity_sampling_and_assertion_metadata() {
        let selector = test_selector("pass.viewport");
        let captures = vec![completed_capture(3, &selector, [10, 20, 30, 255])];
        let probes = vec![RenderPixelProbeRequest {
            id: "center-probe".to_string(),
            selector: selector.clone(),
            sample_mode: RenderPixelSampleMode::Center,
            assertion: RenderPixelProbeAssertionMode::Exact([10, 20, 30, 255]),
        }];

        let results = evaluate_pixel_probes(&probes, &captures);
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(
            result.capture_point_identity,
            selector.stable_point_fallback()
        );
        assert!(result.frame_identity.is_some());
        assert_eq!(result.sample_mode, RenderPixelSampleMode::Center);
        assert_eq!(
            result.comparison_mode,
            RenderPixelProbeAssertionMode::Exact([10, 20, 30, 255])
        );
        assert_eq!(
            result.resolved_coordinate,
            Some(RenderPixelCoordinate { x: 0, y: 0 })
        );
        assert_eq!(result.status, RenderPixelProbeStatus::Passed);
    }

    #[test]
    fn texture_diffs_emit_structured_metrics_even_without_diff_image() {
        let left_selector = test_selector("pass.left");
        let right_selector = test_selector("pass.right");
        let captures = vec![
            completed_capture(9, &left_selector, [1, 2, 3, 255]),
            completed_capture(9, &right_selector, [1, 2, 9, 255]),
        ];
        let diffs = vec![RenderTextureDiffRequest::new(
            "left-vs-right",
            left_selector,
            right_selector,
        )];

        let results = evaluate_texture_diffs(&diffs, &captures);
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.status, RenderTextureDiffStatus::Compared);
        assert!(result.metrics.is_some());
        let metrics = result
            .metrics
            .as_ref()
            .expect("diff metrics should be present");
        assert_eq!(metrics.changed_pixel_count, 1);
        assert_eq!(metrics.total_pixel_count, 1);
        assert_eq!(metrics.changed_pixel_ratio, 1.0);
        assert_eq!(metrics.max_delta, 6);
        assert!(metrics.mean_delta > 0.0);
        assert!(result.diff_image_path.is_none());
    }

    #[test]
    fn texture_diff_thresholds_fail_when_metrics_exceed_limits() {
        let left_selector = test_selector("pass.left");
        let right_selector = test_selector("pass.right");
        let captures = vec![
            completed_capture(9, &left_selector, [1, 2, 3, 255]),
            completed_capture(9, &right_selector, [1, 2, 9, 255]),
        ];
        let diffs = vec![
            RenderTextureDiffRequest::new("left-vs-right", left_selector, right_selector)
                .with_thresholds(2, 10_000),
        ];

        let results = evaluate_texture_diffs(&diffs, &captures);
        let result = &results[0];
        assert_eq!(result.status, RenderTextureDiffStatus::Failed);
        assert_eq!(
            result
                .message
                .as_ref()
                .expect("threshold failure should report a message")
                .code,
            "diff_max_delta_exceeded"
        );
    }
}
