//! Temporary G3/G5 bridge from fully resolved render execution operations to backend-neutral
//! RunenGPU work.
//!
//! G5A deliberately requires this adapter to receive execution-complete `GpuWorkOperation`
//! values. It does not allocate logical GPU resources, reconstruct operation accesses, invent
//! pipeline/binding state, or project renderer declarations into a second GPU identity space.
//! The prepared graph remains the only access, initialization, hazard, capability, dependency,
//! and topological-order authority.
//!
//! A compiled render pass is not an execution identity: fixed-step regions may execute one pass
//! repeatedly and feature gates may omit an occurrence. RunenRender therefore supplies distinct
//! occurrence identities plus only render-owned control/non-data requirements. RunenGPU continues
//! to derive every resource dependency and hazard from the canonical operations.

use runen_gpu::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, thiserror::Error)]
pub enum RenderGpuWorkAdapterError {
    #[error(transparent)]
    Descriptor(#[from] GpuResourceDescriptorError),
    #[error(transparent)]
    Operation(#[from] GpuWorkOperationError),
    #[error(transparent)]
    Authoring(#[from] GpuWorkAuthoringError),
    #[error(transparent)]
    Graph(#[from] GpuWorkGraphError),
    #[error(
        "resolved render GPU work reuses logical resource identity '{resource_id}' for incompatible kind-preserving handles"
    )]
    ResourceIdentityConflict { resource_id: GpuWorkResourceId },
    #[error("resolved render GPU work contains duplicate execution occurrence '{occurrence}'")]
    DuplicateOccurrence {
        occurrence: RenderGpuWorkOccurrenceId,
    },
    #[error(
        "resolved render control order references occurrence '{occurrence}' that is absent from this bounded render work"
    )]
    MissingOrderedOccurrence {
        occurrence: RenderGpuWorkOccurrenceId,
    },
    #[error("render GPU work could not map fragment-local node {local_node}")]
    MissingPreparedNodeMapping { local_node: u64 },
}

/// Process-local identity for one actual renderer GPU execution occurrence.
///
/// This is deliberately not `RenderPassId`: one compiled pass can occur more than once in an
/// invocation. It is sidecar/control-flow identity only and must not be persisted, serialized, or
/// treated as a resource/dependency identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RenderGpuWorkOccurrenceId(u64);

impl RenderGpuWorkOccurrenceId {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    pub(crate) const fn raw(self) -> u64 {
        self.0
    }
}

impl core::fmt::Display for RenderGpuWorkOccurrenceId {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// One execution-complete renderer occurrence ready for G3 graph preparation.
///
/// The operation is already the semantic authority for accesses and mechanical capability
/// requirements. `preference` is scheduling preference only; it cannot weaken operation facts.
/// `control_order_after` carries only render-owned non-data/control semantics. It must never be
/// populated from reconstructed resource dependencies.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedRenderGpuWorkNode {
    occurrence: RenderGpuWorkOccurrenceId,
    label: GpuResourceLabel,
    operation: GpuWorkOperation,
    preference: GpuExecutionPreference,
    provenance: GpuResourceProvenance,
    control_order_after: Vec<RenderGpuWorkOccurrenceId>,
}

impl ResolvedRenderGpuWorkNode {
    pub(crate) fn pass(
        occurrence: RenderGpuWorkOccurrenceId,
        label: GpuResourceLabel,
        operation: GpuWorkOperation,
        preference: GpuExecutionPreference,
        control_order_after: impl IntoIterator<Item = RenderGpuWorkOccurrenceId>,
    ) -> Self {
        let provenance = GpuResourceProvenance::new(label.clone(), None, None);
        Self {
            occurrence,
            label,
            operation,
            preference,
            provenance,
            control_order_after: control_order_after.into_iter().collect(),
        }
    }

    pub(crate) fn upload(
        occurrence: RenderGpuWorkOccurrenceId,
        label: GpuResourceLabel,
        operation: GpuUploadOperation,
        control_order_after: impl IntoIterator<Item = RenderGpuWorkOccurrenceId>,
    ) -> Self {
        let provenance = GpuResourceProvenance::new(label.clone(), None, None);
        Self {
            occurrence,
            label,
            operation: GpuWorkOperation::Upload(operation),
            preference: GpuExecutionPreference::TransferPreferred,
            provenance,
            control_order_after: control_order_after.into_iter().collect(),
        }
    }

    pub(crate) fn timing_resolve(
        occurrence: RenderGpuWorkOccurrenceId,
        label: GpuResourceLabel,
        operation: GpuQueryResolveOperation,
        control_order_after: impl IntoIterator<Item = RenderGpuWorkOccurrenceId>,
    ) -> Self {
        let provenance = GpuResourceProvenance::new(label.clone(), None, None);
        Self {
            occurrence,
            label,
            operation: GpuWorkOperation::Resolve(operation),
            preference: GpuExecutionPreference::TransferPreferred,
            provenance,
            control_order_after: control_order_after.into_iter().collect(),
        }
    }

    pub(crate) fn timing_readback(
        occurrence: RenderGpuWorkOccurrenceId,
        label: GpuResourceLabel,
        operation: GpuReadbackOperation,
        control_order_after: impl IntoIterator<Item = RenderGpuWorkOccurrenceId>,
    ) -> Self {
        let provenance = GpuResourceProvenance::new(label.clone(), None, None);
        Self {
            occurrence,
            label,
            operation: GpuWorkOperation::Readback(operation),
            preference: GpuExecutionPreference::TransferPreferred,
            provenance,
            control_order_after: control_order_after.into_iter().collect(),
        }
    }

    pub(crate) fn capture_readback(
        occurrence: RenderGpuWorkOccurrenceId,
        label: GpuResourceLabel,
        operation: GpuReadbackOperation,
        control_order_after: impl IntoIterator<Item = RenderGpuWorkOccurrenceId>,
    ) -> Self {
        let provenance = GpuResourceProvenance::new(label.clone(), None, None);
        Self {
            occurrence,
            label,
            operation: GpuWorkOperation::Readback(operation),
            preference: GpuExecutionPreference::TransferPreferred,
            provenance,
            control_order_after: control_order_after.into_iter().collect(),
        }
    }

    /// Frame-terminal presentation enters only the canonical GPU work graph. There is
    /// intentionally no renderer executor payload or alternate presentation authority.
    pub(crate) fn present(
        occurrence: RenderGpuWorkOccurrenceId,
        label: GpuResourceLabel,
        operation: GpuPresentOperation,
        control_order_after: impl IntoIterator<Item = RenderGpuWorkOccurrenceId>,
    ) -> Self {
        let provenance = GpuResourceProvenance::new(label.clone(), None, None);
        Self {
            occurrence,
            label,
            operation: GpuWorkOperation::Present(operation),
            preference: GpuExecutionPreference::Automatic,
            provenance,
            control_order_after: control_order_after.into_iter().collect(),
        }
    }
}

struct AuthoredRenderFragment {
    fragment: GpuWorkFragment,
    occurrence_nodes: BTreeMap<RenderGpuWorkOccurrenceId, GpuWorkNodeId>,
}

/// Prepares one bounded frame/surface render submission from execution-complete logical GPU
/// occurrences.
///
/// Every canonical occurrence that currently shares one physical renderer submission must enter
/// this one fragment in deterministic frame execution sequence. That gives G3 direct authority over
/// cross-invocation RAW/WAR/WAW hazards and initialization without using fragment collection order
/// or reconstructing resource dependencies in RunenRender.
pub(crate) fn prepare_render_gpu_frame_work(
    graph_label: GpuResourceLabel,
    nodes: impl IntoIterator<Item = ResolvedRenderGpuWorkNode>,
) -> Result<GpuPreparedWorkGraph, RenderGpuWorkAdapterError> {
    prepare_resolved_render_gpu_work(graph_label, nodes)
}

/// Prepares one bounded render work set from execution-complete logical GPU occurrences.
///
/// All kind-preserving resources are discovered from operation-derived accesses. Every non-query
/// resource is supplied to G3R with descriptor initialization coverage, so an uninitialized
/// descriptor remains uninitialized and a Prepared/Zeroed descriptor contributes only the
/// coverage already owned by RunenGPU. Caller-declared duplicate access truth is intentionally
/// absent.
///
/// G3 intentionally rejects explicit order already guaranteed by typed data dependencies. The
/// adapter therefore performs one provisional preparation without control edges, consumes G3's
/// own dependency result, and retains only unsatisfied render-control requirements for the final
/// preparation. No access intersection or hazard rule is duplicated in RunenRender.
fn prepare_resolved_render_gpu_work(
    graph_label: GpuResourceLabel,
    nodes: impl IntoIterator<Item = ResolvedRenderGpuWorkNode>,
) -> Result<GpuPreparedWorkGraph, RenderGpuWorkAdapterError> {
    let nodes = nodes.into_iter().collect::<Vec<_>>();
    validate_occurrences(&nodes)?;

    let graph_provenance = GpuResourceProvenance::new(graph_label.clone(), None, None);
    let resources = collect_operation_resources(&nodes)?;
    let inputs = resources
        .values()
        .filter(|resource| !matches!(resource, GpuResourceRef::QuerySet(_)))
        .map(|resource| {
            Ok(GpuWorkResourceInput::new(
                resource.clone(),
                GpuInitialCoverage::descriptor_initialization(resource.clone())?,
                resource.common().provenance().clone(),
            )?)
        })
        .collect::<Result<Vec<_>, RenderGpuWorkAdapterError>>()?;
    let desired_control_orders = collect_desired_control_orders(&nodes);

    let provisional = author_render_fragment(
        &nodes,
        &resources,
        &inputs,
        &graph_label,
        &graph_provenance,
        &BTreeSet::new(),
    )?;
    let provisional_graph =
        GpuPreparedWorkGraph::prepare(graph_label.clone(), [provisional.fragment])?;
    let provisional_occurrences =
        map_prepared_occurrences(&provisional_graph, &provisional.occurrence_nodes)?;
    let required_explicit_orders = normalize_control_orders(
        &provisional_graph,
        &provisional_occurrences,
        &desired_control_orders,
    );

    let graph = if required_explicit_orders.is_empty() {
        provisional_graph
    } else {
        let final_fragment = author_render_fragment(
            &nodes,
            &resources,
            &inputs,
            &graph_label,
            &graph_provenance,
            &required_explicit_orders,
        )?;
        GpuPreparedWorkGraph::prepare(graph_label, [final_fragment.fragment])?
    };

    Ok(graph)
}

fn validate_occurrences(
    nodes: &[ResolvedRenderGpuWorkNode],
) -> Result<(), RenderGpuWorkAdapterError> {
    let mut occurrences = BTreeSet::new();
    for node in nodes {
        if !occurrences.insert(node.occurrence) {
            return Err(RenderGpuWorkAdapterError::DuplicateOccurrence {
                occurrence: node.occurrence,
            });
        }
    }
    for node in nodes {
        for occurrence in &node.control_order_after {
            if !occurrences.contains(occurrence) {
                return Err(RenderGpuWorkAdapterError::MissingOrderedOccurrence {
                    occurrence: *occurrence,
                });
            }
        }
    }
    Ok(())
}

fn collect_desired_control_orders(
    nodes: &[ResolvedRenderGpuWorkNode],
) -> Vec<(RenderGpuWorkOccurrenceId, RenderGpuWorkOccurrenceId)> {
    nodes
        .iter()
        .flat_map(|node| {
            node.control_order_after
                .iter()
                .copied()
                .map(move |before| (before, node.occurrence))
        })
        .collect()
}

fn author_render_fragment(
    nodes: &[ResolvedRenderGpuWorkNode],
    resources: &BTreeMap<GpuWorkResourceId, GpuResourceRef>,
    inputs: &[GpuWorkResourceInput],
    graph_label: &GpuResourceLabel,
    graph_provenance: &GpuResourceProvenance,
    explicit_orders: &BTreeSet<(RenderGpuWorkOccurrenceId, RenderGpuWorkOccurrenceId)>,
) -> Result<AuthoredRenderFragment, RenderGpuWorkAdapterError> {
    for occurrence in explicit_orders
        .iter()
        .flat_map(|(before, after)| [before, after])
    {
        if !nodes.iter().any(|node| node.occurrence == *occurrence) {
            return Err(RenderGpuWorkAdapterError::MissingOrderedOccurrence {
                occurrence: *occurrence,
            });
        }
    }

    let mut occurrence_nodes = BTreeMap::<RenderGpuWorkOccurrenceId, GpuWorkNodeId>::new();
    let fragment = GpuWorkFragment::build_with_provenance(
        graph_label.clone(),
        graph_provenance.clone(),
        |builder| {
            for resource in resources.values() {
                builder.declare_resource(resource.clone())?;
            }
            for input in inputs {
                builder.add_input(input.clone())?;
            }

            for node in nodes {
                let node_id = builder.add_node(
                    node.label.clone(),
                    node.operation.clone(),
                    [],
                    GpuCapabilityRequirements::new(),
                    node.preference,
                    node.provenance.clone(),
                )?;
                occurrence_nodes.insert(node.occurrence, node_id.clone());
            }

            for (before_occurrence, after_occurrence) in explicit_orders {
                let before = occurrence_nodes
                    .get(before_occurrence)
                    .expect("validated render occurrence predecessor must be authored");
                let after = occurrence_nodes
                    .get(after_occurrence)
                    .expect("validated render occurrence successor must be authored");
                builder.add_explicit_order(GpuExplicitOrder::new(
                    before,
                    after,
                    "render-owned occurrence control order",
                )?)?;
            }
            Ok(())
        },
    )?;

    Ok(AuthoredRenderFragment {
        fragment,
        occurrence_nodes,
    })
}

fn map_prepared_occurrences(
    graph: &GpuPreparedWorkGraph,
    occurrence_nodes: &BTreeMap<RenderGpuWorkOccurrenceId, GpuWorkNodeId>,
) -> Result<BTreeMap<RenderGpuWorkOccurrenceId, GpuPreparedWorkNodeId>, RenderGpuWorkAdapterError> {
    let prepared_by_local = graph
        .nodes()
        .iter()
        .map(|node| (node.id().local_node(), node.id()))
        .collect::<BTreeMap<_, _>>();
    occurrence_nodes
        .iter()
        .map(|(occurrence, node_id)| {
            let local = node_id.diagnostic_local();
            let prepared = prepared_by_local.get(&local).copied().ok_or(
                RenderGpuWorkAdapterError::MissingPreparedNodeMapping { local_node: local },
            )?;
            Ok((*occurrence, prepared))
        })
        .collect()
}

fn normalize_control_orders(
    provisional_graph: &GpuPreparedWorkGraph,
    occurrence_nodes: &BTreeMap<RenderGpuWorkOccurrenceId, GpuPreparedWorkNodeId>,
    desired: &[(RenderGpuWorkOccurrenceId, RenderGpuWorkOccurrenceId)],
) -> BTreeSet<(RenderGpuWorkOccurrenceId, RenderGpuWorkOccurrenceId)> {
    // This graph comes only from a G3 preparation with zero explicit orders. These edges are
    // therefore G3-derived data dependencies, not renderer-reconstructed access semantics.
    let mut satisfied_edges = provisional_graph
        .dependencies()
        .iter()
        .map(|dependency| (dependency.before(), dependency.after()))
        .collect::<BTreeSet<_>>();
    let mut retained = BTreeSet::new();

    for &(before_occurrence, after_occurrence) in desired {
        let before = occurrence_nodes[&before_occurrence];
        let after = occurrence_nodes[&after_occurrence];
        if dependency_path_exists(&satisfied_edges, before, after) {
            continue;
        }
        retained.insert((before_occurrence, after_occurrence));
        satisfied_edges.insert((before, after));
    }

    retained
}

fn dependency_path_exists(
    edges: &BTreeSet<(GpuPreparedWorkNodeId, GpuPreparedWorkNodeId)>,
    start: GpuPreparedWorkNodeId,
    target: GpuPreparedWorkNodeId,
) -> bool {
    let mut ready = vec![start];
    let mut visited = BTreeSet::new();
    while let Some(node) = ready.pop() {
        if !visited.insert(node) {
            continue;
        }
        for &(before, after) in edges {
            if before != node {
                continue;
            }
            if after == target {
                return true;
            }
            ready.push(after);
        }
    }
    false
}

fn collect_operation_resources(
    nodes: &[ResolvedRenderGpuWorkNode],
) -> Result<BTreeMap<GpuWorkResourceId, GpuResourceRef>, RenderGpuWorkAdapterError> {
    let mut resources = BTreeMap::new();
    for node in nodes {
        for access in node.operation.derived_accesses()? {
            let resource = declared_resource_for_access(&access);
            let identity = resource.diagnostic_identity();
            match resources.get(&identity) {
                Some(existing) if existing != &resource => {
                    return Err(RenderGpuWorkAdapterError::ResourceIdentityConflict {
                        resource_id: identity,
                    });
                }
                Some(_) => {}
                None => {
                    resources.insert(identity, resource);
                }
            }
        }
    }
    Ok(resources)
}

fn declared_resource_for_access(access: &GpuResourceAccess) -> GpuResourceRef {
    match access {
        GpuResourceAccess::Buffer(access) => GpuResourceRef::Buffer(access.buffer().clone()),
        GpuResourceAccess::Texture(access) => match access.resource() {
            GpuTextureAccessResource::Texture(texture) => GpuResourceRef::Texture(texture.clone()),
            GpuTextureAccessResource::TextureView(view) => {
                GpuResourceRef::TextureView(view.clone())
            }
        },
        GpuResourceAccess::Query(access) => GpuResourceRef::QuerySet(access.query_set().clone()),
        GpuResourceAccess::Sampler(access) => GpuResourceRef::Sampler(access.sampler().clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(value: &str) -> GpuResourceLabel {
        GpuResourceLabel::new(value).expect("test label should be valid")
    }

    fn common(value: &str) -> GpuResourceCommon {
        let resource_label = label(value);
        GpuResourceCommon::owned(
            resource_label.clone(),
            GpuResourceLifetime::Transient,
            GpuMemoryIntent::Device,
            GpuReconstruction::SourceBacked,
            GpuResourceProvenance::new(resource_label, None, None),
        )
        .expect("test resource common should be valid")
    }

    fn transfer_payload(name: &str, byte_len: usize) -> PreparedGpuData<TransferData> {
        PreparedGpuData::from_pod_transfer(
            name,
            &vec![0_u8; byte_len],
            GpuResourceProvenance::new(label(name), None, None),
        )
        .expect("test transfer payload should be valid")
    }

    fn buffer(
        allocator: &mut GpuWorkResourceIdAllocator,
        name: &str,
        byte_len: u64,
    ) -> GpuBufferHandle {
        let resource_label = label(name);
        allocator
            .allocate_buffer_handle(
                GpuBufferDescriptor::new(
                    common(name),
                    byte_len,
                    GpuBufferUsages::new(
                        &resource_label,
                        [GpuBufferUsage::CopySource, GpuBufferUsage::CopyDestination],
                    )
                    .expect("test buffer usage should be valid"),
                    GpuBufferInitialization::Uninitialized,
                )
                .expect("test buffer descriptor should be valid"),
            )
            .expect("test buffer handle should allocate")
    }

    fn whole_region(buffer: &GpuBufferHandle, byte_len: u64) -> GpuBufferRegion {
        GpuBufferRegion::new(
            buffer,
            GpuBufferRange::new(buffer, 0, byte_len).expect("test range should be valid"),
        )
        .expect("test region should be valid")
    }

    fn color_target_view(allocator: &mut GpuWorkResourceIdAllocator) -> GpuTextureViewHandle {
        let texture_label = label("frame surface color");
        let texture = allocator
            .allocate_texture_handle(
                GpuTextureDescriptor::new(
                    common("frame surface color"),
                    GpuTextureDimension::D2,
                    GpuTextureExtent::new(&texture_label, GpuTextureDimension::D2, 4, 4, 1)
                        .expect("test surface extent should be valid"),
                    1,
                    1,
                    GpuTextureFormat::Rgba8Unorm,
                    GpuTextureUsages::new(&texture_label, [GpuTextureUsage::ColorAttachment])
                        .expect("test surface usage should be valid"),
                    GpuTextureInitialization::Uninitialized,
                )
                .expect("test surface texture descriptor should be valid"),
            )
            .expect("test surface texture handle should allocate");
        let view_label = label("frame surface color view");
        allocator
            .allocate_texture_view_handle(
                GpuTextureViewDescriptor::new(
                    common("frame surface color view"),
                    &texture,
                    None,
                    GpuTextureDimension::D2,
                    GpuTextureSubresourceRange::new(
                        &view_label,
                        0,
                        1,
                        0,
                        1,
                        GpuTextureAspect::Color,
                    )
                    .expect("test surface subresources should be valid"),
                )
                .expect("test surface view descriptor should be valid"),
            )
            .expect("test surface view handle should allocate")
    }

    #[test]
    fn frame_work_preparation_owns_cross_invocation_raw_and_initialization() {
        let mut allocator = GpuWorkResourceIdAllocator::new();
        let shared = buffer(&mut allocator, "frame shared", 16);
        let copied = buffer(&mut allocator, "frame copied", 16);
        let independent = buffer(&mut allocator, "frame independent", 16);

        let producer = RenderGpuWorkOccurrenceId::new(1);
        let consumer = RenderGpuWorkOccurrenceId::new(2);
        let unrelated = RenderGpuWorkOccurrenceId::new(3);
        let nodes = [
            ResolvedRenderGpuWorkNode::upload(
                producer,
                label("invocation a upload"),
                GpuUploadOperation::new(
                    whole_region(&shared, 16).into(),
                    transfer_payload("invocation a payload", 16),
                )
                .expect("producer upload should be valid"),
                [],
            ),
            ResolvedRenderGpuWorkNode::pass(
                consumer,
                label("invocation b read"),
                GpuWorkOperation::Copy(
                    GpuCopyOperation::buffer_to_buffer(
                        whole_region(&shared, 16),
                        whole_region(&copied, 16),
                    )
                    .expect("consumer copy should be valid"),
                ),
                GpuExecutionPreference::TransferPreferred,
                [],
            ),
            ResolvedRenderGpuWorkNode::upload(
                unrelated,
                label("invocation c independent"),
                GpuUploadOperation::new(
                    whole_region(&independent, 16).into(),
                    transfer_payload("invocation c payload", 16),
                )
                .expect("independent upload should be valid"),
                [],
            ),
        ];

        let prepared = prepare_render_gpu_frame_work(label("render frame test work"), nodes)
            .expect("bounded frame work should prepare");
        let node_id = |label: &str| {
            prepared
                .nodes()
                .iter()
                .find(|node| node.node().label().as_str() == label)
                .expect("prepared node label should exist")
                .id()
        };
        let producer_node = node_id("invocation a upload");
        let consumer_node = node_id("invocation b read");
        let unrelated_node = node_id("invocation c independent");

        assert!(prepared.dependencies().iter().any(|dependency| {
            dependency.before() == producer_node && dependency.after() == consumer_node
        }));
        assert!(prepared.dependencies().iter().all(|dependency| {
            dependency.before() != unrelated_node && dependency.after() != unrelated_node
        }));

        for buffer in [&shared, &copied, &independent] {
            let initialization = prepared
                .initialization()
                .iter()
                .find(|entry| {
                    entry.resource().diagnostic_identity() == buffer.diagnostic_identity()
                })
                .expect("frame graph should retain initialization evidence for every buffer");
            assert!(
                initialization.final_coverage().is_some(),
                "frame graph should own final initialization coverage for '{}'",
                buffer.descriptor().common().label().as_str()
            );
        }
    }

    #[test]
    fn capture_readback_control_order_survives_without_data_hazards() {
        let mut allocator = GpuWorkResourceIdAllocator::new();
        let captured = buffer(&mut allocator, "capture source", 16);
        let pass_source = buffer(&mut allocator, "pass source", 16);
        let pass_destination = buffer(&mut allocator, "pass destination", 16);

        let capture_init = RenderGpuWorkOccurrenceId::new(1);
        let pass_init = RenderGpuWorkOccurrenceId::new(2);
        let before_capture = RenderGpuWorkOccurrenceId::new(3);
        let pass = RenderGpuWorkOccurrenceId::new(4);
        let after_capture = RenderGpuWorkOccurrenceId::new(5);
        let nodes = [
            ResolvedRenderGpuWorkNode::upload(
                capture_init,
                label("capture init"),
                GpuUploadOperation::new(
                    whole_region(&captured, 16).into(),
                    transfer_payload("capture init payload", 16),
                )
                .expect("capture source upload should be valid"),
                [],
            ),
            ResolvedRenderGpuWorkNode::upload(
                pass_init,
                label("pass init"),
                GpuUploadOperation::new(
                    whole_region(&pass_source, 16).into(),
                    transfer_payload("pass init payload", 16),
                )
                .expect("pass source upload should be valid"),
                [],
            ),
            ResolvedRenderGpuWorkNode::capture_readback(
                before_capture,
                label("capture before"),
                GpuReadbackOperation::new(
                    whole_region(&captured, 16).into(),
                    GpuReadbackId::allocate().expect("readback id should allocate"),
                )
                .expect("before capture readback should be valid"),
                [],
            ),
            ResolvedRenderGpuWorkNode::pass(
                pass,
                label("independent pass"),
                GpuWorkOperation::Copy(
                    GpuCopyOperation::buffer_to_buffer(
                        whole_region(&pass_source, 16),
                        whole_region(&pass_destination, 16),
                    )
                    .expect("independent pass copy should be valid"),
                ),
                GpuExecutionPreference::TransferPreferred,
                [before_capture],
            ),
            ResolvedRenderGpuWorkNode::capture_readback(
                after_capture,
                label("capture after"),
                GpuReadbackOperation::new(
                    whole_region(&captured, 16).into(),
                    GpuReadbackId::allocate().expect("readback id should allocate"),
                )
                .expect("after capture readback should be valid"),
                [pass],
            ),
        ];

        let prepared = prepare_render_gpu_frame_work(label("capture stage order test"), nodes)
            .expect("capture stage work should prepare");
        let prepared_node = |label: &str| {
            prepared
                .nodes()
                .iter()
                .find(|node| node.node().label().as_str() == label)
                .expect("prepared node label should exist")
        };
        let before_node = prepared_node("capture before").id();
        let pass_node = prepared_node("independent pass").id();
        let after_node = prepared_node("capture after").id();

        assert!(prepared.dependencies().iter().any(|dependency| {
            dependency.before() == before_node && dependency.after() == pass_node
        }));
        assert!(prepared.dependencies().iter().any(|dependency| {
            dependency.before() == pass_node && dependency.after() == after_node
        }));
        assert_eq!(
            prepared_node("capture before").node().kind(),
            GpuWorkNodeKind::Readback
        );
        assert_eq!(
            prepared_node("capture after").node().kind(),
            GpuWorkNodeKind::Readback
        );
    }

    #[test]
    fn presenting_frame_without_explicit_present_predecessors_ends_at_present() {
        let mut allocator = GpuWorkResourceIdAllocator::new();
        let surface_view = color_target_view(&mut allocator);
        let independent = buffer(&mut allocator, "independent frame upload", 16);
        let render_occurrence = RenderGpuWorkOccurrenceId::new(1);
        let independent_occurrence = RenderGpuWorkOccurrenceId::new(2);
        let present_occurrence = RenderGpuWorkOccurrenceId::new(3);
        let render = GpuRenderOperation::new(
            [GpuRenderColorAttachment::new(
                surface_view.clone(),
                GpuColorAttachmentLoad::Clear(
                    GpuColorClearValue::new(0.0, 0.0, 0.0, 1.0)
                        .expect("test clear value should be valid"),
                ),
                GpuAttachmentStore::Store,
                None,
            )
            .expect("test surface attachment should be valid")],
            None,
            std::iter::empty::<GpuRenderDraw>(),
            None,
        )
        .expect("test surface render should be valid");
        let present = GpuPresentOperation::new(
            surface_view.clone().into(),
            surface_view.descriptor().subresources(),
        )
        .expect("test Present should be valid");
        let nodes = [
            ResolvedRenderGpuWorkNode::pass(
                render_occurrence,
                label("surface render"),
                GpuWorkOperation::Render(render),
                GpuExecutionPreference::GraphicsRequired,
                [],
            ),
            ResolvedRenderGpuWorkNode::upload(
                independent_occurrence,
                label("independent upload"),
                GpuUploadOperation::new(
                    whole_region(&independent, 16).into(),
                    transfer_payload("independent upload payload", 16),
                )
                .expect("independent upload should be valid"),
                [],
            ),
            ResolvedRenderGpuWorkNode::present(
                present_occurrence,
                label("frame terminal Present"),
                present,
                [],
            ),
        ];

        let prepared = prepare_render_gpu_frame_work(label("terminal Present test"), nodes)
            .expect("G3 should order a zero-control Present from resource hazards");
        let prepared_node = |label: &str| {
            prepared
                .nodes()
                .iter()
                .find(|node| node.node().label().as_str() == label)
                .expect("prepared node label should exist")
        };
        let render_node = prepared_node("surface render").id();
        let present_node = prepared_node("frame terminal Present").id();

        assert_eq!(
            prepared
                .nodes()
                .iter()
                .filter(|node| node.node().kind() == GpuWorkNodeKind::Present)
                .count(),
            1
        );
        assert_eq!(prepared.topological_order().last(), Some(&present_node));
        let surface_hazard = prepared
            .dependencies()
            .iter()
            .find(|dependency| {
                dependency.before() == render_node && dependency.after() == present_node
            })
            .expect("G3 must order the surface write before Present");
        assert!(
            surface_hazard
                .reasons()
                .iter()
                .any(|reason| matches!(reason, GpuDependencyReason::ReadAfterWrite { .. }))
        );
        assert!(
            surface_hazard
                .reasons()
                .iter()
                .all(|reason| !matches!(reason, GpuDependencyReason::ExplicitNonData { .. }))
        );
    }
}
