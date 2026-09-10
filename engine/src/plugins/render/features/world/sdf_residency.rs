use std::collections::{BTreeMap, BTreeSet};

use product::{
    FieldProductDiagnostic, FieldProductDiagnosticCode, FieldProductDiagnosticSeverity,
    ProductAuthorityClass, ProductFreshness, ProductIdentity, ProductQueryPolicy, ProductResidency,
    ProductScaleBand, RenderProductSelection, RenderResidencyRequest, RenderSelectedProduct,
};
use runen_spatial::ChunkId;
use world_sdf::{SdfBrickRecord, SdfChunkPayload, SdfPageCoord3};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RenderSdfResidencyStatus {
    #[default]
    Resident,
    Preserved,
}

impl RenderSdfResidencyStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Resident => "resident",
            Self::Preserved => "preserved",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RenderSdfResidencyBudgetStatus {
    #[default]
    NotMeasured,
    WithinBudget,
    OverBudget,
    InvalidBudget,
}

impl RenderSdfResidencyBudgetStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotMeasured => "not_measured",
            Self::WithinBudget => "within_budget",
            Self::OverBudget => "over_budget",
            Self::InvalidBudget => "invalid_budget",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderSdfResidencyBudgetResource {
    pub max_resident_pages: usize,
    pub max_resident_bricks: usize,
    pub max_resident_bytes: u64,
    pub max_upload_bytes_per_frame: u64,
    pub max_clipmap_pages_per_window: usize,
    pub bytes_per_page_table_entry: u64,
    pub bytes_per_brick_metadata: u64,
    pub bytes_per_sdf_sample: u64,
}

impl Default for RenderSdfResidencyBudgetResource {
    fn default() -> Self {
        Self {
            max_resident_pages: 16_384,
            max_resident_bricks: 262_144,
            max_resident_bytes: 512 * 1024 * 1024,
            max_upload_bytes_per_frame: 32 * 1024 * 1024,
            max_clipmap_pages_per_window: 4_096,
            bytes_per_page_table_entry: 64,
            bytes_per_brick_metadata: 48,
            bytes_per_sdf_sample: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSdfResidencySourceProduct {
    pub product_id: ProductIdentity,
    pub product_generation: u64,
    pub payload: SdfChunkPayload,
}

impl RenderSdfResidencySourceProduct {
    pub fn new(
        product_id: ProductIdentity,
        product_generation: u64,
        payload: SdfChunkPayload,
    ) -> Self {
        Self {
            product_id,
            product_generation,
            payload,
        }
    }
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderSdfResidencySourceResource {
    products: BTreeMap<ProductIdentity, RenderSdfResidencySourceProduct>,
}

impl RenderSdfResidencySourceResource {
    pub fn products(&self) -> &BTreeMap<ProductIdentity, RenderSdfResidencySourceProduct> {
        &self.products
    }

    pub fn product(&self, product_id: ProductIdentity) -> Option<&RenderSdfResidencySourceProduct> {
        self.products.get(&product_id)
    }

    pub fn insert_product(&mut self, product: RenderSdfResidencySourceProduct) {
        self.products.insert(product.product_id, product);
    }

    pub fn upsert_payload(
        &mut self,
        product_id: ProductIdentity,
        product_generation: u64,
        payload: SdfChunkPayload,
    ) {
        self.insert_product(RenderSdfResidencySourceProduct::new(
            product_id,
            product_generation,
            payload,
        ));
    }

    pub fn remove_product(
        &mut self,
        product_id: ProductIdentity,
    ) -> Option<RenderSdfResidencySourceProduct> {
        self.products.remove(&product_id)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderSdfResidencySummary {
    pub addressable_product_count: usize,
    pub selected_product_count: usize,
    pub requested_product_count: usize,
    pub resident_product_count: usize,
    pub resident_page_count: usize,
    pub resident_brick_count: usize,
    pub clipmap_window_count: usize,
    pub invalidated_product_count: usize,
    pub rejected_product_count: usize,
    pub resident_bytes: u64,
    pub upload_bytes: u64,
    pub page_budget_status: RenderSdfResidencyBudgetStatus,
    pub brick_budget_status: RenderSdfResidencyBudgetStatus,
    pub resident_byte_budget_status: RenderSdfResidencyBudgetStatus,
    pub upload_byte_budget_status: RenderSdfResidencyBudgetStatus,
    pub clipmap_page_budget_status: RenderSdfResidencyBudgetStatus,
    pub diagnostic_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSdfPageResidencyRecord {
    pub page_coord: SdfPageCoord3,
    pub page_generation: u64,
    pub brick_count: usize,
    pub resident_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSdfBrickAtlasRecord {
    pub page_coord: SdfPageCoord3,
    pub brick_coord: [u8; 3],
    pub occupancy_mask: u8,
    pub material_channel_mask: u16,
    pub surface_band_present: bool,
    pub resident_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSdfClipmapWindowRecord {
    pub scale_band: ProductScaleBand,
    pub chunk_count: usize,
    pub page_count: usize,
    pub brick_count: usize,
    pub resident_bytes: u64,
    pub page_budget_status: RenderSdfResidencyBudgetStatus,
    pub brick_budget_status: RenderSdfResidencyBudgetStatus,
    pub resident_byte_budget_status: RenderSdfResidencyBudgetStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSdfChunkResidencyEntry {
    pub product_id: ProductIdentity,
    pub product_generation: u64,
    pub scale_band: ProductScaleBand,
    pub freshness: ProductFreshness,
    pub source_residency: ProductResidency,
    pub authority_class: ProductAuthorityClass,
    pub query_policy: ProductQueryPolicy,
    pub requested_residency: ProductResidency,
    pub priority: i32,
    pub hard_pin: bool,
    pub status: RenderSdfResidencyStatus,
    pub chunk_id: ChunkId,
    pub chunk_revision: u64,
    pub chunk_generation: u64,
    pub checksum: u64,
    pub cache_generation: u64,
    pub invalidated: bool,
    pub page_records: Vec<RenderSdfPageResidencyRecord>,
    pub brick_atlas_records: Vec<RenderSdfBrickAtlasRecord>,
    pub clipmap_window: RenderSdfClipmapWindowRecord,
    pub resident_bytes: u64,
    pub upload_bytes: u64,
    pub diagnostics: Vec<FieldProductDiagnostic>,
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderSdfResidencyResource {
    entries: BTreeMap<ProductIdentity, RenderSdfChunkResidencyEntry>,
    clipmap_windows: Vec<RenderSdfClipmapWindowRecord>,
    diagnostics: Vec<FieldProductDiagnostic>,
    next_cache_generation: u64,
    last_summary: RenderSdfResidencySummary,
}

impl Default for RenderSdfResidencyResource {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            clipmap_windows: Vec::new(),
            diagnostics: Vec::new(),
            next_cache_generation: 1,
            last_summary: RenderSdfResidencySummary::default(),
        }
    }
}

impl RenderSdfResidencyResource {
    pub fn entries(&self) -> &BTreeMap<ProductIdentity, RenderSdfChunkResidencyEntry> {
        &self.entries
    }

    pub fn entry(&self, product_id: ProductIdentity) -> Option<&RenderSdfChunkResidencyEntry> {
        self.entries.get(&product_id)
    }

    pub fn clipmap_windows(&self) -> &[RenderSdfClipmapWindowRecord] {
        &self.clipmap_windows
    }

    pub fn diagnostics(&self) -> &[FieldProductDiagnostic] {
        &self.diagnostics
    }

    pub fn last_summary(&self) -> RenderSdfResidencySummary {
        self.last_summary
    }

    pub fn derive_from_sources(
        &mut self,
        selections: &[RenderProductSelection],
        sources: &RenderSdfResidencySourceResource,
        budget: &RenderSdfResidencyBudgetResource,
    ) -> RenderSdfResidencySummary {
        let previous_entries = self.entries.clone();
        self.entries.clear();
        self.clipmap_windows.clear();
        self.diagnostics.clear();

        let mut summary = RenderSdfResidencySummary::default();
        let mut selected_products = BTreeMap::<ProductIdentity, RenderSelectedProduct>::new();
        let mut residency_requests = BTreeMap::<ProductIdentity, RenderResidencyRequest>::new();

        for selection in selections {
            summary.addressable_product_count = summary
                .addressable_product_count
                .saturating_add(selection.selected_products.len());
            for product in &selection.selected_products {
                selected_products.insert(product.product_id, product.clone());
            }
            for request in &selection.residency_requests {
                residency_requests.insert(request.product_id, request.clone());
            }
        }

        let mut candidate_ids = BTreeSet::<ProductIdentity>::new();
        candidate_ids.extend(residency_requests.keys().copied());
        candidate_ids.extend(
            selected_products
                .keys()
                .filter(|product_id| sources.product(**product_id).is_some())
                .copied(),
        );

        summary.selected_product_count = candidate_ids.len();
        summary.requested_product_count = residency_requests.len();

        for product_id in candidate_ids {
            let Some(selected) = selected_products.get(&product_id) else {
                summary.rejected_product_count = summary.rejected_product_count.saturating_add(1);
                self.push_global_diagnostic(diagnostic(
                    FieldProductDiagnosticCode::MissingProduct,
                    FieldProductDiagnosticSeverity::Blocking,
                    product_id,
                    "SDF residency request has no selected product",
                    "renderer received a residency request without matching selected product state",
                    "publish the SDF product selection and residency request together",
                ));
                continue;
            };

            let Some(request) = residency_requests.get(&product_id) else {
                summary.rejected_product_count = summary.rejected_product_count.saturating_add(1);
                self.push_global_diagnostic(diagnostic(
                    FieldProductDiagnosticCode::MissingDependency,
                    FieldProductDiagnosticSeverity::Blocking,
                    product_id,
                    "SDF selected product has no residency request",
                    "renderer cannot derive GPU SDF residency without explicit residency intent",
                    "publish a RenderResidencyRequest for the selected SDF product",
                ));
                continue;
            };

            let Some(source) = sources.product(product_id) else {
                summary.rejected_product_count = summary.rejected_product_count.saturating_add(1);
                self.push_global_diagnostic(diagnostic(
                    FieldProductDiagnosticCode::MissingProduct,
                    FieldProductDiagnosticSeverity::Blocking,
                    product_id,
                    "SDF selected product has no payload source",
                    "renderer cannot derive brick, page, or clipmap residency without a domain-owned SDF payload",
                    "publish the SDF payload reference before deriving renderer residency",
                ));
                continue;
            };

            let validation = validate_selected_product(selected, request, source);
            if has_blocking_diagnostic(&validation) {
                summary.rejected_product_count = summary.rejected_product_count.saturating_add(1);
                self.diagnostics.extend(validation);
                continue;
            }

            let previous = previous_entries.get(&product_id);
            let entry = build_entry(
                selected,
                request,
                source,
                previous,
                budget,
                validation,
                self.next_cache_generation,
            );
            self.next_cache_generation = self.next_cache_generation.saturating_add(1);

            if entry.invalidated {
                summary.invalidated_product_count =
                    summary.invalidated_product_count.saturating_add(1);
            }
            summary.resident_product_count = summary.resident_product_count.saturating_add(1);
            summary.resident_page_count = summary
                .resident_page_count
                .saturating_add(entry.page_records.len());
            summary.resident_brick_count = summary
                .resident_brick_count
                .saturating_add(entry.brick_atlas_records.len());
            summary.resident_bytes = summary.resident_bytes.saturating_add(entry.resident_bytes);
            summary.upload_bytes = summary.upload_bytes.saturating_add(entry.upload_bytes);
            summary.diagnostic_count = summary
                .diagnostic_count
                .saturating_add(entry.diagnostics.len());

            self.clipmap_windows.push(entry.clipmap_window.clone());
            self.entries.insert(product_id, entry);
        }

        for product_id in previous_entries.keys() {
            if !self.entries.contains_key(product_id) {
                summary.invalidated_product_count =
                    summary.invalidated_product_count.saturating_add(1);
            }
        }

        summary.clipmap_window_count = self.clipmap_windows.len();
        summary.page_budget_status =
            count_budget_status(summary.resident_page_count, budget.max_resident_pages);
        summary.brick_budget_status =
            count_budget_status(summary.resident_brick_count, budget.max_resident_bricks);
        summary.resident_byte_budget_status =
            byte_budget_status(summary.resident_bytes, budget.max_resident_bytes);
        summary.upload_byte_budget_status =
            byte_budget_status(summary.upload_bytes, budget.max_upload_bytes_per_frame);
        summary.clipmap_page_budget_status = self
            .clipmap_windows
            .iter()
            .map(|window| window.page_budget_status)
            .find(|status| *status == RenderSdfResidencyBudgetStatus::OverBudget)
            .unwrap_or(RenderSdfResidencyBudgetStatus::WithinBudget);
        summary.diagnostic_count = summary
            .diagnostic_count
            .saturating_add(self.diagnostics.len());

        push_budget_diagnostics(&mut self.diagnostics, &summary);
        summary.diagnostic_count = self.diagnostics.len().saturating_add(
            self.entries
                .values()
                .map(|entry| entry.diagnostics.len())
                .sum::<usize>(),
        );

        self.last_summary = summary;
        summary
    }

    fn push_global_diagnostic(&mut self, diagnostic: FieldProductDiagnostic) {
        self.diagnostics.push(diagnostic);
    }
}

fn validate_selected_product(
    selected: &RenderSelectedProduct,
    request: &RenderResidencyRequest,
    source: &RenderSdfResidencySourceProduct,
) -> Vec<FieldProductDiagnostic> {
    let mut diagnostics = Vec::new();

    if selected.generation != source.product_generation {
        diagnostics.push(diagnostic(
            FieldProductDiagnosticCode::GenerationMismatch,
            FieldProductDiagnosticSeverity::Blocking,
            selected.product_id,
            "SDF product generation does not match payload source generation",
            "selected product and SDF payload source were produced from different generations",
            "republish the product selection from the same SDF payload generation",
        ));
    }

    if !selected.freshness.is_strict_current() {
        diagnostics.push(diagnostic(
            freshness_diagnostic_code(selected.freshness),
            FieldProductDiagnosticSeverity::Blocking,
            selected.product_id,
            "SDF product freshness is not current",
            "renderer SDF residency cannot treat stale, retired, fallback, or rebuilding products as resident brick/page truth",
            "wait for a current SDF product or publish an explicit degraded product selection",
        ));
    }

    if selected.residency != ProductResidency::Resident {
        diagnostics.push(diagnostic(
            residency_diagnostic_code(selected.residency),
            FieldProductDiagnosticSeverity::Blocking,
            selected.product_id,
            "SDF product is not resident in product space",
            "renderer cannot derive GPU brick/page residency from a nonresident or fallback product",
            "publish a resident SDF product before deriving renderer residency",
        ));
    }

    if request.residency != ProductResidency::Resident {
        diagnostics.push(diagnostic(
            FieldProductDiagnosticCode::PendingResidency,
            FieldProductDiagnosticSeverity::Blocking,
            selected.product_id,
            "SDF residency request does not request resident data",
            "renderer SDF residency requires explicit resident intent for brick/page payloads",
            "request resident SDF data for this product",
        ));
    }

    if !selected.query_policy.allows(
        selected.freshness,
        selected.residency,
        selected.authority_class,
    ) {
        diagnostics.push(diagnostic(
            FieldProductDiagnosticCode::UnsupportedConsumerRequest,
            FieldProductDiagnosticSeverity::Blocking,
            selected.product_id,
            "SDF product query policy rejects the selected product state",
            "renderer cannot override product-owned freshness, residency, or authority policy",
            "publish a product state that satisfies its query policy",
        ));
    }

    diagnostics
}

fn build_entry(
    selected: &RenderSelectedProduct,
    request: &RenderResidencyRequest,
    source: &RenderSdfResidencySourceProduct,
    previous: Option<&RenderSdfChunkResidencyEntry>,
    budget: &RenderSdfResidencyBudgetResource,
    diagnostics: Vec<FieldProductDiagnostic>,
    next_cache_generation: u64,
) -> RenderSdfChunkResidencyEntry {
    let page_records = build_page_records(&source.payload, budget);
    let brick_atlas_records = build_brick_atlas_records(&source.payload, budget);
    let resident_bytes = page_records
        .iter()
        .map(|record| record.resident_bytes)
        .chain(
            brick_atlas_records
                .iter()
                .map(|record| record.resident_bytes),
        )
        .fold(0_u64, u64::saturating_add);
    let invalidated = previous
        .map(|entry| {
            entry.product_generation != selected.generation
                || entry.chunk_revision != source.payload.chunk_revision.0
                || entry.chunk_generation != source.payload.chunk_generation.0
                || entry.checksum != source.payload.checksum
        })
        .unwrap_or(false);
    let status = if previous.is_some() && !invalidated {
        RenderSdfResidencyStatus::Preserved
    } else {
        RenderSdfResidencyStatus::Resident
    };
    let upload_bytes = if status == RenderSdfResidencyStatus::Preserved {
        0
    } else {
        resident_bytes
    };
    let cache_generation = if status == RenderSdfResidencyStatus::Preserved {
        previous
            .map(|entry| entry.cache_generation)
            .unwrap_or(next_cache_generation)
    } else {
        next_cache_generation
    };
    let clipmap_window = RenderSdfClipmapWindowRecord {
        scale_band: selected.scale_band,
        chunk_count: 1,
        page_count: page_records.len(),
        brick_count: brick_atlas_records.len(),
        resident_bytes,
        page_budget_status: count_budget_status(
            page_records.len(),
            budget.max_clipmap_pages_per_window,
        ),
        brick_budget_status: count_budget_status(
            brick_atlas_records.len(),
            budget.max_resident_bricks,
        ),
        resident_byte_budget_status: byte_budget_status(resident_bytes, budget.max_resident_bytes),
    };

    RenderSdfChunkResidencyEntry {
        product_id: selected.product_id,
        product_generation: selected.generation,
        scale_band: selected.scale_band,
        freshness: selected.freshness,
        source_residency: selected.residency,
        authority_class: selected.authority_class,
        query_policy: selected.query_policy,
        requested_residency: request.residency,
        priority: request.priority,
        hard_pin: request.hard_pin,
        status,
        chunk_id: source.payload.chunk_id,
        chunk_revision: source.payload.chunk_revision.0,
        chunk_generation: source.payload.chunk_generation.0,
        checksum: source.payload.checksum,
        cache_generation,
        invalidated,
        page_records,
        brick_atlas_records,
        clipmap_window,
        resident_bytes,
        upload_bytes,
        diagnostics,
    }
}

fn build_page_records(
    payload: &SdfChunkPayload,
    budget: &RenderSdfResidencyBudgetResource,
) -> Vec<RenderSdfPageResidencyRecord> {
    payload
        .page_table
        .iter()
        .map(|(page_coord, page)| RenderSdfPageResidencyRecord {
            page_coord: *page_coord,
            page_generation: page.page_generation,
            brick_count: page.bricks.len(),
            resident_bytes: budget.bytes_per_page_table_entry,
        })
        .collect()
}

fn build_brick_atlas_records(
    payload: &SdfChunkPayload,
    budget: &RenderSdfResidencyBudgetResource,
) -> Vec<RenderSdfBrickAtlasRecord> {
    let mut records = Vec::new();
    for (page_coord, page) in &payload.page_table {
        for (brick_coord, brick) in &page.bricks {
            records.push(RenderSdfBrickAtlasRecord {
                page_coord: *page_coord,
                brick_coord: *brick_coord,
                occupancy_mask: brick.metadata.occupancy_mask,
                material_channel_mask: brick.metadata.material_channel_mask,
                surface_band_present: brick.metadata.surface_band_present,
                resident_bytes: brick_resident_bytes(brick, budget),
            });
        }
    }
    records
}

fn brick_resident_bytes(brick: &SdfBrickRecord, budget: &RenderSdfResidencyBudgetResource) -> u64 {
    let sample_count = u64::try_from(brick.samples.distances.len()).unwrap_or(u64::MAX);
    budget
        .bytes_per_brick_metadata
        .saturating_add(sample_count.saturating_mul(budget.bytes_per_sdf_sample))
}

fn count_budget_status(observed: usize, limit: usize) -> RenderSdfResidencyBudgetStatus {
    if limit == 0 {
        RenderSdfResidencyBudgetStatus::InvalidBudget
    } else if observed > limit {
        RenderSdfResidencyBudgetStatus::OverBudget
    } else {
        RenderSdfResidencyBudgetStatus::WithinBudget
    }
}

fn byte_budget_status(observed: u64, limit: u64) -> RenderSdfResidencyBudgetStatus {
    if limit == 0 {
        RenderSdfResidencyBudgetStatus::InvalidBudget
    } else if observed > limit {
        RenderSdfResidencyBudgetStatus::OverBudget
    } else {
        RenderSdfResidencyBudgetStatus::WithinBudget
    }
}

fn push_budget_diagnostics(
    diagnostics: &mut Vec<FieldProductDiagnostic>,
    summary: &RenderSdfResidencySummary,
) {
    push_budget_diagnostic(
        diagnostics,
        summary.page_budget_status,
        "SDF resident page count exceeds renderer budget",
    );
    push_budget_diagnostic(
        diagnostics,
        summary.brick_budget_status,
        "SDF resident brick count exceeds renderer budget",
    );
    push_budget_diagnostic(
        diagnostics,
        summary.resident_byte_budget_status,
        "SDF resident bytes exceed renderer budget",
    );
    push_budget_diagnostic(
        diagnostics,
        summary.upload_byte_budget_status,
        "SDF upload bytes exceed per-frame renderer budget",
    );
    push_budget_diagnostic(
        diagnostics,
        summary.clipmap_page_budget_status,
        "SDF clipmap window page count exceeds renderer budget",
    );
}

fn push_budget_diagnostic(
    diagnostics: &mut Vec<FieldProductDiagnostic>,
    status: RenderSdfResidencyBudgetStatus,
    message: &'static str,
) {
    if matches!(
        status,
        RenderSdfResidencyBudgetStatus::OverBudget | RenderSdfResidencyBudgetStatus::InvalidBudget
    ) {
        diagnostics.push(FieldProductDiagnostic {
            code: FieldProductDiagnosticCode::RebuildBudgetExhausted,
            severity: FieldProductDiagnosticSeverity::Warning,
            product_id: None,
            family: None,
            scale_band: None,
            consumer_class: None,
            generation: None,
            message: message.to_string(),
            cause: "renderer SDF residency budget pressure is visible".to_string(),
            suggested_action:
                "tune renderer SDF residency budgets or reduce selected SDF working set"
                    .to_string(),
            related_products: Vec::new(),
        });
    }
}

fn diagnostic(
    code: FieldProductDiagnosticCode,
    severity: FieldProductDiagnosticSeverity,
    product_id: ProductIdentity,
    message: &'static str,
    cause: &'static str,
    suggested_action: &'static str,
) -> FieldProductDiagnostic {
    FieldProductDiagnostic {
        code,
        severity,
        product_id: Some(product_id),
        family: None,
        scale_band: None,
        consumer_class: None,
        generation: None,
        message: message.to_string(),
        cause: cause.to_string(),
        suggested_action: suggested_action.to_string(),
        related_products: Vec::new(),
    }
}

fn has_blocking_diagnostic(diagnostics: &[FieldProductDiagnostic]) -> bool {
    diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.severity,
            FieldProductDiagnosticSeverity::Blocking | FieldProductDiagnosticSeverity::Error
        )
    })
}

fn freshness_diagnostic_code(freshness: ProductFreshness) -> FieldProductDiagnosticCode {
    match freshness {
        ProductFreshness::PotentiallyStale => FieldProductDiagnosticCode::PotentiallyStale,
        ProductFreshness::Stale => FieldProductDiagnosticCode::StaleProduct,
        ProductFreshness::Retired => FieldProductDiagnosticCode::RetiredSelected,
        ProductFreshness::Missing => FieldProductDiagnosticCode::MissingProduct,
        ProductFreshness::FailedPreserved => FieldProductDiagnosticCode::FailedPreservedOutput,
        _ => FieldProductDiagnosticCode::DeclaredNotFormed,
    }
}

fn residency_diagnostic_code(residency: ProductResidency) -> FieldProductDiagnosticCode {
    match residency {
        ProductResidency::NonResident => FieldProductDiagnosticCode::NonResident,
        ProductResidency::PendingLoad | ProductResidency::PendingUnload => {
            FieldProductDiagnosticCode::PendingResidency
        }
        ProductResidency::FallbackResident => FieldProductDiagnosticCode::FallbackUsed,
        ProductResidency::GhostSummary => FieldProductDiagnosticCode::GhostSummaryUsed,
        ProductResidency::Missing => FieldProductDiagnosticCode::MissingProduct,
        ProductResidency::FailedPreserved => FieldProductDiagnosticCode::FailedPreservedOutput,
        _ => FieldProductDiagnosticCode::DeclaredNotFormed,
    }
}
