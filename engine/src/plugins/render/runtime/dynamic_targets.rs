use crate::plugins::render::{
    RenderDynamicTextureTargetDescriptor, RenderDynamicTextureTargetDescriptorError,
    RenderDynamicTextureTargetKey, RenderFrameProducerId,
};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderDynamicTextureTargetRequestDiagnostic {
    pub producer_id: RenderFrameProducerId,
    pub key: RenderDynamicTextureTargetKey,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderDynamicTextureTargetRequestRegistryError {
    InvalidDescriptor(RenderDynamicTextureTargetDescriptorError),
    DuplicateTargetWithinProducer {
        producer_id: RenderFrameProducerId,
        key: RenderDynamicTextureTargetKey,
    },
    DuplicateTargetAcrossProducers {
        existing_producer_id: RenderFrameProducerId,
        replacement_producer_id: RenderFrameProducerId,
        key: RenderDynamicTextureTargetKey,
    },
}

impl fmt::Display for RenderDynamicTextureTargetRequestRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescriptor(err) => write!(f, "{err}"),
            Self::DuplicateTargetWithinProducer { producer_id, key } => write!(
                f,
                "render target producer '{producer_id:?}' requested duplicate dynamic target '{key}'"
            ),
            Self::DuplicateTargetAcrossProducers {
                existing_producer_id,
                replacement_producer_id,
                key,
            } => write!(
                f,
                "render target producer '{replacement_producer_id:?}' requested dynamic target '{key}' already owned by producer '{existing_producer_id:?}'"
            ),
        }
    }
}

impl std::error::Error for RenderDynamicTextureTargetRequestRegistryError {}

impl From<RenderDynamicTextureTargetDescriptorError>
    for RenderDynamicTextureTargetRequestRegistryError
{
    fn from(value: RenderDynamicTextureTargetDescriptorError) -> Self {
        Self::InvalidDescriptor(value)
    }
}

#[derive(Debug, Default, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct RenderDynamicTextureTargetRequestRegistryResource {
    contributions: BTreeMap<
        RenderFrameProducerId,
        BTreeMap<RenderDynamicTextureTargetKey, RenderDynamicTextureTargetDescriptor>,
    >,
    diagnostics: Vec<RenderDynamicTextureTargetRequestDiagnostic>,
}

impl RenderDynamicTextureTargetRequestRegistryResource {
    pub fn clear(&mut self) {
        self.contributions.clear();
        self.diagnostics.clear();
    }

    pub fn remove_contribution(
        &mut self,
        producer_id: impl Into<RenderFrameProducerId>,
    ) -> Option<BTreeMap<RenderDynamicTextureTargetKey, RenderDynamicTextureTargetDescriptor>> {
        self.contributions.remove(&producer_id.into())
    }

    pub fn replace_contribution(
        &mut self,
        producer_id: impl Into<RenderFrameProducerId>,
        descriptors: impl IntoIterator<Item = RenderDynamicTextureTargetDescriptor>,
    ) -> Result<(), RenderDynamicTextureTargetRequestRegistryError> {
        let producer_id = producer_id.into();
        self.diagnostics
            .retain(|diagnostic| diagnostic.producer_id != producer_id);
        let mut contribution =
            BTreeMap::<RenderDynamicTextureTargetKey, RenderDynamicTextureTargetDescriptor>::new();

        for descriptor in descriptors {
            if let Err(err) = descriptor.validate() {
                self.diagnostics
                    .push(RenderDynamicTextureTargetRequestDiagnostic {
                        producer_id,
                        key: descriptor.key.clone(),
                        message: err.to_string(),
                    });
                return Err(err.into());
            }
            let key = descriptor.key.clone();
            if contribution.insert(key.clone(), descriptor).is_some() {
                let err =
                    RenderDynamicTextureTargetRequestRegistryError::DuplicateTargetWithinProducer {
                        producer_id,
                        key: key.clone(),
                    };
                self.diagnostics
                    .push(RenderDynamicTextureTargetRequestDiagnostic {
                        producer_id,
                        key,
                        message: err.to_string(),
                    });
                return Err(err);
            }
        }

        for (existing_producer_id, existing_contribution) in &self.contributions {
            if *existing_producer_id == producer_id {
                continue;
            }
            for key in contribution.keys() {
                if existing_contribution.contains_key(key) {
                    let err =
                        RenderDynamicTextureTargetRequestRegistryError::DuplicateTargetAcrossProducers {
                            existing_producer_id: *existing_producer_id,
                            replacement_producer_id: producer_id,
                            key: key.clone(),
                        };
                    self.diagnostics
                        .push(RenderDynamicTextureTargetRequestDiagnostic {
                            producer_id,
                            key: key.clone(),
                            message: err.to_string(),
                        });
                    return Err(err);
                }
            }
        }
        self.contributions.insert(producer_id, contribution);
        Ok(())
    }

    pub fn snapshot(&self) -> Vec<RenderDynamicTextureTargetDescriptor> {
        self.contributions
            .values()
            .flat_map(|contribution| contribution.values().cloned())
            .collect()
    }

    pub fn diagnostics(&self) -> &[RenderDynamicTextureTargetRequestDiagnostic] {
        &self.diagnostics
    }

    pub fn is_empty(&self) -> bool {
        self.contributions
            .values()
            .all(|contribution| contribution.is_empty())
    }
}
