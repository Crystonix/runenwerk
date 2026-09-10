//! Runtime product cache metadata and inspection state.
//!
//! The engine owns cache decisions and diagnostics, but not product-family
//! payload bytes or package-specific sidecar paths.

use std::collections::{BTreeMap, VecDeque};

use product::{
    FieldProductDiagnostic, FieldProductDiagnosticCode, FieldProductDiagnosticSeverity,
    ProductCacheDecision, ProductCacheDecisionKind, ProductCacheIdentity, ProductCacheKey,
    ProductDescriptorCore, ProductIdentity,
};

const PRODUCT_CACHE_DECISION_LIMIT: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProductCacheEntry {
    pub key: ProductCacheKey,
    pub identity: ProductCacheIdentity,
    pub descriptor: ProductDescriptorCore,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeProductCacheSnapshot {
    pub entry_count: usize,
    pub decisions: Vec<ProductCacheDecision>,
    pub diagnostics: Vec<FieldProductDiagnostic>,
}

#[derive(Debug, Clone, Default, runen_ecs::Resource)]
pub struct RuntimeProductCacheResource {
    entries: BTreeMap<ProductCacheKey, RuntimeProductCacheEntry>,
    last_good_by_product: BTreeMap<ProductIdentity, ProductCacheKey>,
    decisions: VecDeque<ProductCacheDecision>,
    diagnostics: Vec<FieldProductDiagnostic>,
}

impl RuntimeProductCacheResource {
    pub fn lookup(&mut self, identity: &ProductCacheIdentity) -> ProductCacheDecision {
        let key = identity.cache_key();
        let decision = match self.entries.get(&key) {
            Some(entry) if entry.identity == *identity => {
                ProductCacheDecision::new(ProductCacheDecisionKind::Hit, key.clone())
                    .for_identity(identity)
                    .with_stored_generation(entry.identity.descriptor_generation)
            }
            Some(entry) => {
                ProductCacheDecision::new(ProductCacheDecisionKind::Rejected, key.clone())
                    .for_identity(identity)
                    .with_stored_generation(entry.identity.descriptor_generation)
                    .with_diagnostics([cache_identity_mismatch_diagnostic(identity)])
            }
            None => self.lookup_missing(identity, key.clone()),
        };
        self.record_decision(decision.clone());
        decision
    }

    pub fn entry(&self, key: &ProductCacheKey) -> Option<&RuntimeProductCacheEntry> {
        self.entries.get(key)
    }

    pub fn record_accepted_descriptor(
        &mut self,
        descriptor: ProductDescriptorCore,
    ) -> ProductCacheDecision {
        let identity = descriptor.cache_identity();
        let key = identity.cache_key();
        let entry = RuntimeProductCacheEntry {
            key: key.clone(),
            identity: identity.clone(),
            descriptor,
        };
        self.entries.insert(key.clone(), entry);
        self.last_good_by_product
            .insert(identity.product_id, key.clone());
        let decision = ProductCacheDecision::new(ProductCacheDecisionKind::Hit, key)
            .for_identity(&identity)
            .with_stored_generation(identity.descriptor_generation);
        self.record_decision(decision.clone());
        decision
    }

    pub fn record_accepted_descriptors(
        &mut self,
        descriptors: impl IntoIterator<Item = ProductDescriptorCore>,
    ) -> Vec<ProductCacheDecision> {
        descriptors
            .into_iter()
            .map(|descriptor| self.record_accepted_descriptor(descriptor))
            .collect()
    }

    pub fn record_write_failed(
        &mut self,
        identity: &ProductCacheIdentity,
        diagnostics: impl IntoIterator<Item = FieldProductDiagnostic>,
    ) -> ProductCacheDecision {
        let diagnostics = diagnostics.into_iter().collect::<Vec<_>>();
        let decision =
            ProductCacheDecision::new(ProductCacheDecisionKind::WriteFailed, identity.cache_key())
                .for_identity(identity)
                .with_diagnostics(diagnostics);
        self.record_decision(decision.clone());
        decision
    }

    pub fn record_preserved_last_good(
        &mut self,
        identity: &ProductCacheIdentity,
        diagnostics: impl IntoIterator<Item = FieldProductDiagnostic>,
    ) -> ProductCacheDecision {
        let diagnostics = diagnostics.into_iter().collect::<Vec<_>>();
        let stored_generation = self
            .last_good_by_product
            .get(&identity.product_id)
            .and_then(|key| self.entries.get(key))
            .map(|entry| entry.identity.descriptor_generation);
        let mut decision = ProductCacheDecision::new(
            ProductCacheDecisionKind::PreservedLastGood,
            identity.cache_key(),
        )
        .for_identity(identity)
        .with_diagnostics(diagnostics);
        if let Some(generation) = stored_generation {
            decision = decision.with_stored_generation(generation);
        }
        self.record_decision(decision.clone());
        decision
    }

    pub fn snapshot(&self) -> RuntimeProductCacheSnapshot {
        RuntimeProductCacheSnapshot {
            entry_count: self.entries.len(),
            decisions: self.decisions.iter().cloned().collect(),
            diagnostics: self.diagnostics.clone(),
        }
    }

    fn lookup_missing(
        &self,
        identity: &ProductCacheIdentity,
        key: ProductCacheKey,
    ) -> ProductCacheDecision {
        if let Some(last_good_key) = self.last_good_by_product.get(&identity.product_id)
            && let Some(entry) = self.entries.get(last_good_key)
        {
            return ProductCacheDecision::new(ProductCacheDecisionKind::Stale, key)
                .for_identity(identity)
                .with_stored_generation(entry.identity.descriptor_generation)
                .with_diagnostics([cache_stale_diagnostic(identity, entry)]);
        }
        ProductCacheDecision::new(ProductCacheDecisionKind::Miss, key).for_identity(identity)
    }

    fn record_decision(&mut self, decision: ProductCacheDecision) {
        self.diagnostics
            .extend(decision.diagnostics.iter().cloned());
        self.decisions.push_back(decision);
        if self.decisions.len() > PRODUCT_CACHE_DECISION_LIMIT {
            let drain = self.decisions.len() - PRODUCT_CACHE_DECISION_LIMIT;
            self.decisions.drain(0..drain);
        }
    }
}

fn cache_identity_mismatch_diagnostic(identity: &ProductCacheIdentity) -> FieldProductDiagnostic {
    FieldProductDiagnostic::blocking(
        FieldProductDiagnosticCode::GenerationMismatch,
        "product cache entry key matched but metadata identity differed",
    )
    .for_product(identity.product_id)
}

fn cache_stale_diagnostic(
    identity: &ProductCacheIdentity,
    entry: &RuntimeProductCacheEntry,
) -> FieldProductDiagnostic {
    FieldProductDiagnostic::new(
        FieldProductDiagnosticCode::StaleProduct,
        FieldProductDiagnosticSeverity::Warning,
        format!(
            "product cache has last-good generation {} but requested generation {}",
            entry.identity.descriptor_generation, identity.descriptor_generation
        ),
    )
    .for_product(identity.product_id)
}

#[cfg(test)]
mod tests {
    use product::{
        ProductDescriptorCore, ProductFamily, ProductIdentity, ProductKind, ProductLineage,
        ProductScaleBand, ProductScope,
    };

    use super::*;

    #[test]
    fn runtime_product_cache_records_hit_miss_stale_and_preserved_last_good() {
        let mut cache = RuntimeProductCacheResource::default();
        let accepted_descriptor = descriptor(7, 10);
        let identity = accepted_descriptor.cache_identity();

        let miss = cache.lookup(&identity);
        assert_eq!(miss.kind, ProductCacheDecisionKind::Miss);

        cache.record_accepted_descriptor(accepted_descriptor);
        let hit = cache.lookup(&identity);
        assert_eq!(hit.kind, ProductCacheDecisionKind::Hit);
        assert_eq!(hit.stored_generation, Some(10));

        let stale_identity = descriptor(7, 11).cache_identity();
        let stale = cache.lookup(&stale_identity);
        assert_eq!(stale.kind, ProductCacheDecisionKind::Stale);
        assert_eq!(stale.stored_generation, Some(10));
        assert!(!stale.diagnostics.is_empty());

        let preserved = cache.record_preserved_last_good(&stale_identity, stale.diagnostics);
        assert_eq!(preserved.kind, ProductCacheDecisionKind::PreservedLastGood);
        assert_eq!(preserved.stored_generation, Some(10));
        assert_eq!(cache.snapshot().entry_count, 1);
    }

    #[test]
    fn runtime_product_cache_records_write_failed_diagnostics() {
        let mut cache = RuntimeProductCacheResource::default();
        let identity = descriptor(3, 1).cache_identity();

        let decision = cache.record_write_failed(
            &identity,
            [FieldProductDiagnostic::blocking(
                FieldProductDiagnosticCode::FormationFailure,
                "cache write failed",
            )
            .for_product(identity.product_id)],
        );

        assert_eq!(decision.kind, ProductCacheDecisionKind::WriteFailed);
        assert_eq!(cache.snapshot().diagnostics.len(), 1);
    }

    fn descriptor(id: u64, generation: u64) -> ProductDescriptorCore {
        ProductDescriptorCore::new(
            ProductIdentity::new(id),
            ProductFamily::Texture,
            ProductKind::new("test.texture"),
            ProductScope::non_spatial("test"),
            ProductScaleBand::Preview,
            ProductLineage::new("engine.runtime.cache.test", generation)
                .with_source_revision(generation.to_string())
                .with_source_key(format!("source:{id}")),
        )
    }
}
