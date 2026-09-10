use crate::plugins::render::features::world::sdf_residency::{
    RenderSdfChunkResidencyEntry, RenderSdfResidencyBudgetStatus, RenderSdfResidencyResource,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderSdfRaymarchAccelerationConfig {
    pub screen_tile_count: usize,
    pub depth_slice_count: usize,
    pub max_candidates_per_list: usize,
    pub max_steps_per_ray: u32,
    pub max_empty_space_step: f32,
    pub fullscreen_entity_multiplier: usize,
}

impl Default for RenderSdfRaymarchAccelerationConfig {
    fn default() -> Self {
        Self {
            screen_tile_count: 1,
            depth_slice_count: 1,
            max_candidates_per_list: 64,
            max_steps_per_ray: 128,
            max_empty_space_step: 1.0,
            fullscreen_entity_multiplier: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderSdfRaymarchDiagnosticSeverity {
    Warning,
    Error,
}

impl RenderSdfRaymarchDiagnosticSeverity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderSdfRaymarchDiagnosticKind {
    MissingSdfResidency,
    InvalidStepBudget,
    UnsafeOverstepRisk,
    CandidateExplosion,
    PerEntityFullscreenMultiplication,
    ResidencyPressure,
}

impl RenderSdfRaymarchDiagnosticKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingSdfResidency => "missing_sdf_residency",
            Self::InvalidStepBudget => "invalid_step_budget",
            Self::UnsafeOverstepRisk => "unsafe_overstep_risk",
            Self::CandidateExplosion => "candidate_explosion",
            Self::PerEntityFullscreenMultiplication => "per_entity_fullscreen_multiplication",
            Self::ResidencyPressure => "residency_pressure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSdfRaymarchDiagnostic {
    pub kind: RenderSdfRaymarchDiagnosticKind,
    pub severity: RenderSdfRaymarchDiagnosticSeverity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderSdfDistanceMipLevel {
    pub level: u8,
    pub source_page_count: usize,
    pub source_brick_count: usize,
    pub conservative_min_distance: f32,
    pub max_safe_step: f32,
    pub unsafe_overstep_risk: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSdfRaymarchCandidate {
    pub product_id: u64,
    pub cache_generation: u64,
    pub scale_band: String,
    pub page_count: usize,
    pub brick_count: usize,
    pub resident_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSdfRaymarchCandidateList {
    pub tile_index: usize,
    pub depth_slice: usize,
    pub candidate_count: usize,
    pub rejected_candidate_count: usize,
    pub candidates: Vec<RenderSdfRaymarchCandidate>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderSdfRaymarchAccelerationReport {
    pub resident_product_count: usize,
    pub resident_page_count: usize,
    pub resident_brick_count: usize,
    pub clipmap_window_count: usize,
    pub distance_mips: Vec<RenderSdfDistanceMipLevel>,
    pub candidate_lists: Vec<RenderSdfRaymarchCandidateList>,
    pub total_candidate_count: usize,
    pub rejected_candidate_count: usize,
    pub max_candidates_per_list: usize,
    pub max_steps_per_ray: u32,
    pub fullscreen_entity_multiplier: usize,
    pub diagnostics: Vec<RenderSdfRaymarchDiagnostic>,
}

impl RenderSdfRaymarchAccelerationReport {
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == RenderSdfRaymarchDiagnosticSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.severity == RenderSdfRaymarchDiagnosticSeverity::Warning
            })
            .count()
    }

    pub fn is_acceleration_ready(&self) -> bool {
        self.error_count() == 0 && self.resident_product_count > 0
    }
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderSdfRaymarchAccelerationResource {
    last_report: RenderSdfRaymarchAccelerationReport,
}

impl RenderSdfRaymarchAccelerationResource {
    pub fn last_report(&self) -> &RenderSdfRaymarchAccelerationReport {
        &self.last_report
    }

    pub fn derive_from_residency(
        &mut self,
        residency: &RenderSdfResidencyResource,
        config: RenderSdfRaymarchAccelerationConfig,
    ) -> RenderSdfRaymarchAccelerationReport {
        let report = inspect_sdf_raymarch_acceleration(residency, config);
        self.last_report = report.clone();
        report
    }
}

pub fn inspect_sdf_raymarch_acceleration(
    residency: &RenderSdfResidencyResource,
    config: RenderSdfRaymarchAccelerationConfig,
) -> RenderSdfRaymarchAccelerationReport {
    let residency_summary = residency.last_summary();
    let resident_entries = residency.entries().values().collect::<Vec<_>>();
    let mut diagnostics = Vec::new();

    if resident_entries.is_empty() {
        diagnostics.push(diagnostic(
            RenderSdfRaymarchDiagnosticKind::MissingSdfResidency,
            RenderSdfRaymarchDiagnosticSeverity::Error,
            "SDF raymarch acceleration requires resident SDF page and brick evidence",
        ));
    }
    if config.screen_tile_count == 0 || config.depth_slice_count == 0 {
        diagnostics.push(diagnostic(
            RenderSdfRaymarchDiagnosticKind::InvalidStepBudget,
            RenderSdfRaymarchDiagnosticSeverity::Error,
            "SDF candidate lists require at least one screen tile and one depth slice",
        ));
    }
    if config.max_steps_per_ray == 0 || config.max_candidates_per_list == 0 {
        diagnostics.push(diagnostic(
            RenderSdfRaymarchDiagnosticKind::InvalidStepBudget,
            RenderSdfRaymarchDiagnosticSeverity::Error,
            "SDF raymarch step and candidate budgets must be nonzero",
        ));
    }
    if !(0.0..=1.0).contains(&config.max_empty_space_step) {
        diagnostics.push(diagnostic(
            RenderSdfRaymarchDiagnosticKind::UnsafeOverstepRisk,
            RenderSdfRaymarchDiagnosticSeverity::Error,
            "SDF empty-space step exceeds the conservative safe-step bound",
        ));
    }
    if config.fullscreen_entity_multiplier > 1 {
        diagnostics.push(diagnostic(
            RenderSdfRaymarchDiagnosticKind::PerEntityFullscreenMultiplication,
            RenderSdfRaymarchDiagnosticSeverity::Error,
            "SDF fullscreen raymarching must be one bounded view pass, not multiplied by entity count",
        ));
    }
    if matches!(
        residency_summary.page_budget_status,
        RenderSdfResidencyBudgetStatus::OverBudget | RenderSdfResidencyBudgetStatus::InvalidBudget
    ) || matches!(
        residency_summary.resident_byte_budget_status,
        RenderSdfResidencyBudgetStatus::OverBudget | RenderSdfResidencyBudgetStatus::InvalidBudget
    ) {
        diagnostics.push(diagnostic(
            RenderSdfRaymarchDiagnosticKind::ResidencyPressure,
            RenderSdfRaymarchDiagnosticSeverity::Warning,
            "SDF raymarch acceleration is consuming residency evidence with visible budget pressure",
        ));
    }

    let distance_mips = build_distance_mips(&resident_entries, config.max_empty_space_step);
    if distance_mips.iter().any(|mip| mip.unsafe_overstep_risk) {
        diagnostics.push(diagnostic(
            RenderSdfRaymarchDiagnosticKind::UnsafeOverstepRisk,
            RenderSdfRaymarchDiagnosticSeverity::Error,
            "SDF distance mip evidence reports an unsafe overstep risk",
        ));
    }

    let (candidate_lists, rejected_candidate_count) =
        build_candidate_lists(&resident_entries, config, &mut diagnostics);
    let total_candidate_count = candidate_lists
        .iter()
        .map(|list| list.candidate_count)
        .sum::<usize>();

    RenderSdfRaymarchAccelerationReport {
        resident_product_count: residency_summary.resident_product_count,
        resident_page_count: residency_summary.resident_page_count,
        resident_brick_count: residency_summary.resident_brick_count,
        clipmap_window_count: residency_summary.clipmap_window_count,
        distance_mips,
        candidate_lists,
        total_candidate_count,
        rejected_candidate_count,
        max_candidates_per_list: config.max_candidates_per_list,
        max_steps_per_ray: config.max_steps_per_ray,
        fullscreen_entity_multiplier: config.fullscreen_entity_multiplier,
        diagnostics,
    }
}

fn build_distance_mips(
    entries: &[&RenderSdfChunkResidencyEntry],
    max_empty_space_step: f32,
) -> Vec<RenderSdfDistanceMipLevel> {
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let level = u8::try_from(index).unwrap_or(u8::MAX);
            let denominator = f32::from(level).max(0.0) + 1.0;
            let max_safe_step = (max_empty_space_step / denominator).max(0.0);
            RenderSdfDistanceMipLevel {
                level,
                source_page_count: entry.page_records.len(),
                source_brick_count: entry.brick_atlas_records.len(),
                conservative_min_distance: 0.0,
                max_safe_step,
                unsafe_overstep_risk: !(0.0..=1.0).contains(&max_safe_step),
            }
        })
        .collect()
}

fn build_candidate_lists(
    entries: &[&RenderSdfChunkResidencyEntry],
    config: RenderSdfRaymarchAccelerationConfig,
    diagnostics: &mut Vec<RenderSdfRaymarchDiagnostic>,
) -> (Vec<RenderSdfRaymarchCandidateList>, usize) {
    if config.screen_tile_count == 0 || config.depth_slice_count == 0 {
        return (Vec::new(), 0);
    }

    let candidates = entries
        .iter()
        .map(|entry| candidate_for_entry(entry))
        .collect::<Vec<_>>();
    let mut lists = Vec::new();
    let mut total_rejected = 0_usize;
    for tile_index in 0..config.screen_tile_count {
        for depth_slice in 0..config.depth_slice_count {
            let rejected = candidates
                .len()
                .saturating_sub(config.max_candidates_per_list);
            if rejected > 0 {
                total_rejected = total_rejected.saturating_add(rejected);
            }
            let list_candidates = candidates
                .iter()
                .take(config.max_candidates_per_list)
                .cloned()
                .collect::<Vec<_>>();
            lists.push(RenderSdfRaymarchCandidateList {
                tile_index,
                depth_slice,
                candidate_count: list_candidates.len(),
                rejected_candidate_count: rejected,
                candidates: list_candidates,
            });
        }
    }

    if total_rejected > 0 {
        diagnostics.push(diagnostic(
            RenderSdfRaymarchDiagnosticKind::CandidateExplosion,
            RenderSdfRaymarchDiagnosticSeverity::Warning,
            "SDF candidate lists exceeded the configured per-list budget",
        ));
    }

    (lists, total_rejected)
}

fn candidate_for_entry(entry: &RenderSdfChunkResidencyEntry) -> RenderSdfRaymarchCandidate {
    RenderSdfRaymarchCandidate {
        product_id: entry.product_id.raw(),
        cache_generation: entry.cache_generation,
        scale_band: format!("{:?}", entry.scale_band),
        page_count: entry.page_records.len(),
        brick_count: entry.brick_atlas_records.len(),
        resident_bytes: entry.resident_bytes,
    }
}

fn diagnostic(
    kind: RenderSdfRaymarchDiagnosticKind,
    severity: RenderSdfRaymarchDiagnosticSeverity,
    message: &'static str,
) -> RenderSdfRaymarchDiagnostic {
    RenderSdfRaymarchDiagnostic {
        kind,
        severity,
        message: message.to_string(),
    }
}
