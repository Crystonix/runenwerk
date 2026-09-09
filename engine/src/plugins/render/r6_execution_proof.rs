// Permanent public-RunenGPU execution/readback proof for the bounded R6 founding renderer.
//
// This is a child of `r6_spine_proof` so the founding scene, method, object identities, and
// representation identities have one test-only authority. The execution request intentionally uses
// a finite numeric tolerance because the proof lowers numeric results to f32; object identity stays
// exact. Concrete surface behavior comes from `r6_proof`, and CPU shading/depth meaning comes from
// `r6_reference_proof`.

use super::super::lowering::RenderWorkSet;
use super::super::r6_proof::FoundingRepresentationRealization;
use super::super::r6_reference_proof::{direct_lighting_radiance, observation_forward_depth};
use super::super::representation::RenderSurfaceQuery;
use super::super::surface_result::RenderOrientedSurfaceHit;
use super::*;
use runen_gpu::{
    GpuBufferRange, GpuBufferRegion, GpuBufferTextureLayout, GpuBufferUsage,
    GpuCapabilityFeature, GpuComputeOperation, GpuComputePipelineDescriptor, GpuCopyOperation,
    GpuDispatchIntent, GpuDispatchSize, GpuFormatRole, GpuReadbackBytes, GpuReadbackId,
    GpuReadbackOperation, GpuReadbackStatus, GpuResourceScope, GpuRuntimeBindingValue,
    GpuSubmission, GpuSubmissionStatus, GpuTextureCopyRegion, GpuTextureFormat, GpuTextureUsage,
    GpuTransferRegion, GpuUploadOperation, GpuWorkFragment, PreparedGpuData, TransferData,
    admit_static_wgsl_sources,
};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

const NUMERIC_TOLERANCE: f64 = 1.0e-4;
const WORKGROUP_SIZE: u32 = 64;
const INPUT_HEADER_WORDS: usize = 40;
const GEOMETRY_RECORD_WORDS: usize = 32;
const OUTPUT_WORD_BYTES: u64 = 4;

const FOUNDING_WGSL: &str = r#"
struct Hit {
    found: bool,
    t: f32,
    code: u32,
    normal_scene: vec3<f32>,
    reflectance: f32,
};

@group(0) @binding(0)
var<storage, read> input_words: array<u32>;

@group(0) @binding(1)
var<storage, read_write> output_words: array<u32>;

@group(0) @binding(2)
var<storage, read_write> status_words: array<u32>;

fn load_f32(index: u32) -> f32 {
    return bitcast<f32>(input_words[index]);
}

fn mul3(base: u32, value: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        dot(vec3<f32>(load_f32(base), load_f32(base + 1u), load_f32(base + 2u)), value),
        dot(vec3<f32>(load_f32(base + 3u), load_f32(base + 4u), load_f32(base + 5u)), value),
        dot(vec3<f32>(load_f32(base + 6u), load_f32(base + 7u), load_f32(base + 8u)), value),
    );
}

fn record_base(index: u32) -> u32 {
    return 40u + index * 32u;
}

fn to_local_point(base: u32, point_scene: vec3<f32>) -> vec3<f32> {
    let translation = vec3<f32>(
        load_f32(base + 13u),
        load_f32(base + 14u),
        load_f32(base + 15u),
    );
    return mul3(base + 4u, point_scene - translation);
}

fn to_local_direction(base: u32, direction_scene: vec3<f32>) -> vec3<f32> {
    return mul3(base + 4u, direction_scene);
}

fn normal_to_scene(base: u32, normal_local: vec3<f32>) -> vec3<f32> {
    return normalize(mul3(base + 16u, normal_local));
}

fn intersect_sphere(base: u32, origin_scene: vec3<f32>, direction_scene: vec3<f32>) -> Hit {
    let origin = to_local_point(base, origin_scene);
    let direction = to_local_direction(base, direction_scene);
    let center = vec3<f32>(
        load_f32(base + 25u),
        load_f32(base + 26u),
        load_f32(base + 27u),
    );
    let radius = load_f32(base + 28u);
    let relative = origin - center;
    let a = dot(direction, direction);
    let b = 2.0 * dot(relative, direction);
    let c = dot(relative, relative) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return Hit(false, 0.0, 0u, vec3<f32>(0.0), 0.0);
    }
    let root = sqrt(discriminant);
    let denominator = 2.0 * a;
    let first = (-b - root) / denominator;
    let second = (-b + root) / denominator;
    var t = 1.0e30;
    if first >= 0.0 {
        t = first;
    }
    if second >= 0.0 && second < t {
        t = second;
    }
    if t == 1.0e30 {
        return Hit(false, 0.0, 0u, vec3<f32>(0.0), 0.0);
    }
    let hit_local = origin + direction * t;
    return Hit(
        true,
        t,
        input_words[base + 1u],
        normal_to_scene(base, normalize(hit_local - center)),
        load_f32(base + 2u),
    );
}

fn intersect_plane(base: u32, origin_scene: vec3<f32>, direction_scene: vec3<f32>) -> Hit {
    let origin = to_local_point(base, origin_scene);
    let direction = to_local_direction(base, direction_scene);
    let point = vec3<f32>(
        load_f32(base + 25u),
        load_f32(base + 26u),
        load_f32(base + 27u),
    );
    let normal = normalize(vec3<f32>(
        load_f32(base + 28u),
        load_f32(base + 29u),
        load_f32(base + 30u),
    ));
    let denominator = dot(normal, direction);
    if abs(denominator) <= 1.0e-7 {
        return Hit(false, 0.0, 0u, vec3<f32>(0.0), 0.0);
    }
    let t = dot(point - origin, normal) / denominator;
    if t < 0.0 {
        return Hit(false, 0.0, 0u, vec3<f32>(0.0), 0.0);
    }
    return Hit(
        true,
        t,
        input_words[base + 1u],
        normal_to_scene(base, normal),
        load_f32(base + 2u),
    );
}

fn intersect_geometry(base: u32, origin_scene: vec3<f32>, direction_scene: vec3<f32>) -> Hit {
    if input_words[base] == 2u {
        return intersect_plane(base, origin_scene, direction_scene);
    }
    return intersect_sphere(base, origin_scene, direction_scene);
}

fn nearest_hit(origin_scene: vec3<f32>, direction_scene: vec3<f32>, ignored_code: u32) -> Hit {
    let geometry_count = input_words[3u];
    var nearest = Hit(false, 1.0e30, 0u, vec3<f32>(0.0), 0.0);
    var index = 0u;
    loop {
        if index >= geometry_count {
            break;
        }
        let base = record_base(index);
        let code = input_words[base + 1u];
        if code != ignored_code {
            let candidate = intersect_geometry(base, origin_scene, direction_scene);
            if candidate.found && candidate.t < nearest.t {
                nearest = candidate;
            }
        }
        index = index + 1u;
    }
    return nearest;
}

fn direct_radiance(hit_position: vec3<f32>, hit: Hit) -> f32 {
    let direction_to_source = normalize(vec3<f32>(
        load_f32(28u),
        load_f32(29u),
        load_f32(30u),
    ));
    let cosine = max(dot(hit.normal_scene, direction_to_source), 0.0);
    if cosine == 0.0 {
        return 0.0;
    }
    if nearest_hit(hit_position, direction_to_source, hit.code).found {
        return 0.0;
    }
    return hit.reflectance * load_f32(31u) * cosine / 3.14159265358979323846;
}

fn observation_origin(base: u32) -> vec3<f32> {
    return vec3<f32>(load_f32(base), load_f32(base + 1u), load_f32(base + 2u));
}

fn perspective_direction(x: u32, y: u32) -> vec3<f32> {
    let width = f32(input_words[0u]);
    let height = f32(input_words[1u]);
    let u = (f32(x) + 0.5) / width;
    let v = (f32(y) + 0.5) / height;
    let local = vec3<f32>(
        (2.0 * u - 1.0) * load_f32(20u) * load_f32(21u),
        (1.0 - 2.0 * v) * load_f32(20u),
        -1.0,
    );
    return normalize(mul3(11u, local));
}

fn perspective_forward() -> vec3<f32> {
    return normalize(mul3(11u, vec3<f32>(0.0, 0.0, -1.0)));
}

fn probe_direction() -> vec3<f32> {
    return normalize(vec3<f32>(load_f32(25u), load_f32(26u), load_f32(27u)));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let index = invocation.x;
    let sample_count = input_words[2u];
    if index < sample_count {
        let width = input_words[0u];
        let origin = observation_origin(8u);
        let direction = perspective_direction(index % width, index / width);
        let hit = nearest_hit(origin, direction, 0u);
        if !hit.found {
            status_words[index] = 1u;
            return;
        }
        let position = origin + direction * hit.t;
        output_words[input_words[4u] + index] = bitcast<u32>(direct_radiance(position, hit));
        output_words[input_words[5u] + index] = bitcast<u32>(dot(position - origin, perspective_forward()));
        output_words[input_words[6u] + index] = hit.code;
        return;
    }

    if index == sample_count {
        let origin = observation_origin(22u);
        let direction = probe_direction();
        let hit = nearest_hit(origin, direction, 0u);
        if !hit.found {
            status_words[index] = 1u;
            return;
        }
        let position = origin + direction * hit.t;
        output_words[input_words[7u]] = bitcast<u32>(direct_radiance(position, hit));
    }
}
"#;

#[derive(Debug, Clone, Copy, PartialEq)]
enum FoundingShape {
    AnalyticSphere { center: [f64; 3], radius: f64 },
    AnalyticPlane { point: [f64; 3], normal: [f64; 3] },
    FieldSphere { center: [f64; 3], radius: f64 },
}

impl FoundingShape {
    fn realization(
        self,
        representation_id: RenderRepresentationId,
    ) -> Result<FoundingRepresentationRealization, String> {
        match self {
            Self::AnalyticSphere { center, radius } => {
                FoundingRepresentationRealization::analytic_sphere(
                    representation_id,
                    center,
                    radius,
                )
            }
            Self::AnalyticPlane { point, normal } => {
                FoundingRepresentationRealization::analytic_plane(
                    representation_id,
                    point,
                    normal,
                )
            }
            Self::FieldSphere { center, radius } => {
                FoundingRepresentationRealization::field_sphere(
                    representation_id,
                    center,
                    radius,
                )
            }
        }
        .map_err(|error| format!("R6 proof realization rejected: {error:?}"))
    }

    const fn kind_code(self) -> u32 {
        match self {
            Self::AnalyticSphere { .. } => 1,
            Self::AnalyticPlane { .. } => 2,
            Self::FieldSphere { .. } => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FoundingRealizationInput {
    object_id: RenderObjectId,
    representation_id: RenderRepresentationId,
    shape: FoundingShape,
}

struct ExecutionFixture {
    plan: RenderPlan,
    representation_ids: [RenderRepresentationId; 3],
    geometry_object_ids: [RenderObjectId; 3],
}

struct LoweredExecution {
    work_set: RenderWorkSet,
    output_readbacks: [GpuReadbackId; 4],
    status_readback: GpuReadbackId,
    object_codes: BTreeMap<RenderObjectId, u32>,
}

#[derive(Debug, Clone, Copy)]
struct ReferenceHit {
    object_id: RenderObjectId,
    hit: RenderOrientedSurfaceHit,
}

#[derive(Debug, Clone, Copy)]
struct ReferencePixel {
    radiance: f64,
    depth: f64,
    identity_code: u32,
}

type WorkNodeSignature = (String, String);
type WorkFragmentSignature = (String, usize, Vec<WorkNodeSignature>);

fn execution_tolerance() -> RenderSemanticTolerance {
    RenderSemanticTolerance::absolute(NUMERIC_TOLERANCE).expect("positive R6 execution tolerance")
}

fn execution_request() -> RenderRequest {
    let shutter = instant();
    let perspective = RenderObservationSpec::Perspective(
        RenderPerspectiveObservation::new(
            RenderAffineTransform3::identity(),
            std::f64::consts::FRAC_PI_4,
            1.0,
            shutter,
            RenderSamplingSupport::ideal_ray(),
        )
        .expect("R6 execution perspective"),
    );
    let probe = RenderObservationSpec::Probe(
        RenderProbeObservation::new(
            RenderAffineTransform3::identity(),
            shutter,
            RenderSamplingSupport::ideal_ray(),
        )
        .expect("R6 execution probe"),
    );
    let radiometric =
        RenderRadiometricRepresentation::spectral_at_wavelength_meters(PROOF_WAVELENGTH_METERS)
            .expect("R6 execution spectral domain");
    let lattice = || RenderResultTopology::sample_lattice_2d(PROOF_WIDTH, PROOF_HEIGHT).unwrap();
    let radiance = |topology| {
        RenderOutputSpec::new(
            RenderOutputValue::Radiance {
                representation: radiometric,
            },
            topology,
            execution_tolerance(),
        )
        .unwrap()
    };
    RenderRequest::new(
        shutter,
        vec![perspective, probe],
        vec![
            RenderRequestedOutput::new(0, radiance(lattice())),
            RenderRequestedOutput::new(
                0,
                RenderOutputSpec::new(
                    RenderOutputValue::Distance {
                        convention: RenderDistanceConvention::ObservationForwardDepth,
                    },
                    lattice(),
                    execution_tolerance(),
                )
                .unwrap(),
            ),
            RenderRequestedOutput::new(
                0,
                RenderOutputSpec::new(
                    RenderOutputValue::ObjectIdentity,
                    lattice(),
                    RenderSemanticTolerance::exact(),
                )
                .unwrap(),
            ),
            RenderRequestedOutput::new(1, radiance(RenderResultTopology::scalar())),
        ],
    )
    .expect("R6 execution request")
}

fn execution_fixture() -> ExecutionFixture {
    let spine = founding_fixture();
    let method = founding_method();
    let request = execution_request();
    let plan = plan_render(spine.plan.scene(), &request, std::slice::from_ref(&method))
        .expect("R6 execution must use the ordinary R4 planner over the founding scene");
    ExecutionFixture {
        plan,
        representation_ids: spine.representation_ids,
        geometry_object_ids: spine.geometry_object_ids,
    }
}

fn founding_realizations(fixture: &ExecutionFixture) -> [FoundingRealizationInput; 3] {
    [
        FoundingRealizationInput {
            object_id: fixture.geometry_object_ids[0],
            representation_id: fixture.representation_ids[0],
            shape: FoundingShape::AnalyticSphere {
                center: [0.0; 3],
                radius: 1.25,
            },
        },
        FoundingRealizationInput {
            object_id: fixture.geometry_object_ids[1],
            representation_id: fixture.representation_ids[1],
            shape: FoundingShape::AnalyticPlane {
                point: [0.0, 1.0, -6.0],
                normal: [0.0, 0.0, 1.0],
            },
        },
        FoundingRealizationInput {
            object_id: fixture.geometry_object_ids[2],
            representation_id: fixture.representation_ids[2],
            shape: FoundingShape::FieldSphere {
                center: [0.0; 3],
                radius: 1.25,
            },
        },
    ]
}

fn founding_realizations_for_spine(
    fixture: &FoundingSpineFixture,
) -> [FoundingRealizationInput; 3] {
    [
        FoundingRealizationInput {
            object_id: fixture.geometry_object_ids[0],
            representation_id: fixture.representation_ids[0],
            shape: FoundingShape::AnalyticSphere {
                center: [0.0; 3],
                radius: 1.25,
            },
        },
        FoundingRealizationInput {
            object_id: fixture.geometry_object_ids[1],
            representation_id: fixture.representation_ids[1],
            shape: FoundingShape::AnalyticPlane {
                point: [0.0, 1.0, -6.0],
                normal: [0.0, 0.0, 1.0],
            },
        },
        FoundingRealizationInput {
            object_id: fixture.geometry_object_ids[2],
            representation_id: fixture.representation_ids[2],
            shape: FoundingShape::FieldSphere {
                center: [0.0; 3],
                radius: 1.25,
            },
        },
    ]
}

fn execution_output_bindings() -> Vec<RenderOutputBinding> {
    let mut allocator = GpuWorkResourceIdAllocator::new();
    let mut bindings = Vec::with_capacity(4);
    for (output_index, label) in [
        "R6 executed perspective radiance",
        "R6 executed perspective depth",
        "R6 executed perspective identity",
    ]
    .into_iter()
    .enumerate()
    {
        let descriptor = GpuTextureDescriptor::ordinary_owned_2d(
            label,
            GpuResourceLifetime::Transient,
            GpuReconstruction::SourceBacked,
            PROOF_WIDTH,
            PROOF_HEIGHT,
            GpuTextureFormat::R32Uint,
            [GpuTextureUsage::CopyDestination, GpuTextureUsage::CopySource],
            GpuTextureInitialization::Uninitialized,
        )
        .unwrap();
        let texture = allocator.allocate_texture_handle(descriptor).unwrap();
        bindings.push(RenderOutputBinding::new(
            output_index,
            RenderOutputDestination::SampleLatticeTexture(texture),
        ));
    }
    let descriptor = GpuBufferDescriptor::ordinary_owned(
        "R6 executed scalar radiance probe",
        GpuResourceLifetime::Transient,
        GpuReconstruction::SourceBacked,
        4,
        [GpuBufferUsage::CopyDestination, GpuBufferUsage::CopySource],
        GpuBufferInitialization::Uninitialized,
    )
    .unwrap();
    let buffer = allocator.allocate_buffer_handle(descriptor).unwrap();
    bindings.push(RenderOutputBinding::new(
        3,
        RenderOutputDestination::ScalarBuffer(buffer),
    ));
    bindings
}

fn request_execution_context() -> Option<GpuContext> {
    let descriptor = GpuContextDescriptor::new(GpuCapabilityProfile::ComputeBaseline.requirements())
        .require_format_role(GpuTextureFormat::R32Uint, GpuFormatRole::CopyDestination)
        .require_format_role(GpuTextureFormat::R32Uint, GpuFormatRole::CopySource)
        .with_label("RunenRender R6 execution proof");
    match pollster::block_on(GpuContext::request(descriptor)) {
        Ok(context) => Some(context),
        Err(error) if error.category() == GpuContextRequestErrorCategory::NoAdapterAvailable => {
            assert_ne!(
                std::env::var("RUNENRENDER_R6_REQUIRE_GPU")
                    .ok()
                    .as_deref(),
                Some("1"),
                "permanent R6 execution CI requires a public RunenGPU adapter"
            );
            None
        }
        Err(error) => panic!("unexpected R6 RunenGPU context failure: {error}"),
    }
}

fn admitted_texture_row_alignment(context: &GpuContext) -> Result<u64, String> {
    context
        .device_facts()
        .device_limits()
        .alignments()
        .bytes_per_row
        .ok_or_else(|| "RunenGPU did not expose an admitted bytes-per-row alignment".to_string())
}

fn align_up(value: u64, alignment: u64) -> Result<u64, String> {
    if alignment == 0 {
        return Err("RunenGPU exposed a zero bytes-per-row alignment".to_string());
    }
    let remainder = value % alignment;
    if remainder == 0 {
        Ok(value)
    } else {
        value
            .checked_add(alignment - remainder)
            .ok_or_else(|| "R6 aligned row size overflow".to_string())
    }
}

fn admit_execution(
    fixture: &ExecutionFixture,
    context: &GpuContext,
) -> (
    super::super::admission::AdmittedRenderPlan,
    Vec<RenderOutputBinding>,
) {
    let availability = fixture
        .representation_ids
        .iter()
        .copied()
        .map(|representation_id| {
            RenderRepresentationAvailabilityFact::new(
                representation_id,
                RenderRepresentationAvailabilityState::Available,
            )
        })
        .collect::<Vec<_>>();
    let bindings = execution_output_bindings();
    let admitted = admit_render_plan(&fixture.plan, &availability, &bindings, context)
        .expect("R6 execution plan must reach ordinary R5 admission");
    (admitted, bindings)
}

fn normalize_realizations(
    admitted: &super::super::admission::AdmittedRenderPlan,
    inputs: &[FoundingRealizationInput],
) -> Result<Vec<FoundingRealizationInput>, String> {
    let required = admitted
        .outputs()
        .iter()
        .flat_map(|output| output.object_representations())
        .map(|object| {
            (
                object.object_id(),
                object.representation().representation_id(),
            )
        })
        .collect::<BTreeSet<_>>();
    let mut by_representation = BTreeMap::new();
    for input in inputs.iter().copied() {
        if !required.contains(&(input.object_id, input.representation_id)) {
            return Err(format!(
                "foreign R6 realization {:?}",
                input.representation_id
            ));
        }
        if by_representation
            .insert(input.representation_id, input)
            .is_some()
        {
            return Err(format!(
                "duplicate R6 realization {:?}",
                input.representation_id
            ));
        }
    }
    for (_, representation_id) in &required {
        if !by_representation.contains_key(representation_id) {
            return Err(format!("missing R6 realization {representation_id:?}"));
        }
    }
    Ok(by_representation.into_values().collect())
}

fn canonical_object_codes(
    admitted: &super::super::admission::AdmittedRenderPlan,
) -> Result<BTreeMap<RenderObjectId, u32>, String> {
    admitted
        .outputs()
        .iter()
        .flat_map(|output| output.object_representations())
        .map(|object| object.object_id())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .enumerate()
        .map(|(index, object_id)| {
            let code =
                u32::try_from(index + 1).map_err(|_| "R6 object-code overflow".to_string())?;
            Ok((object_id, code))
        })
        .collect()
}

fn founding_emitter(
    admitted: &super::super::admission::AdmittedRenderPlan,
) -> RenderDirectionalEmitter {
    let emitters = admitted
        .plan()
        .scene()
        .object_ids()
        .into_iter()
        .filter_map(|object_id| {
            admitted
                .plan()
                .scene()
                .object_participation(object_id)
                .and_then(RenderObjectParticipation::emitter)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        emitters.len(),
        1,
        "R6 founding scene has exactly one emitter"
    );
    emitters[0]
}

fn validate_founding_admission(
    admitted: &super::super::admission::AdmittedRenderPlan,
) -> Result<(), String> {
    if admitted.plan().request() != &execution_request() {
        return Err("R6 lowering accepts only the bounded founding execution request".to_string());
    }
    if admitted.selected_candidate().method_id() != RenderMethodId::new(1).unwrap() {
        return Err("R6 lowering cannot substitute a non-founding method".to_string());
    }
    if admitted.outputs().len() != 4 {
        return Err("R6 lowering requires exactly four admitted founding outputs".to_string());
    }

    let expected_observations = [0, 0, 0, 1];
    let expected_protocols = [
        RenderRepresentationProtocol::OrientedSurfaceQuery,
        RenderRepresentationProtocol::SurfaceQuery,
        RenderRepresentationProtocol::SurfaceQuery,
        RenderRepresentationProtocol::OrientedSurfaceQuery,
    ];
    for (output_index, output) in admitted.outputs().iter().enumerate() {
        if output.output_index() != output_index
            || output.observation_index() != expected_observations[output_index]
        {
            return Err(format!(
                "R6 admitted output correlation changed at output {output_index}"
            ));
        }
        for object in output.object_representations() {
            if object
                .representation()
                .requirement()
                .protocol()
                .protocol()
                != expected_protocols[output_index]
            {
                return Err(format!(
                    "R6 admitted representation protocol changed at output {output_index}"
                ));
            }
        }
        let destination_matches = matches!(
            (output_index, output.binding().destination()),
            (0..=2, RenderOutputDestination::SampleLatticeTexture(_))
                | (3, RenderOutputDestination::ScalarBuffer(_))
        );
        if !destination_matches {
            return Err(format!(
                "R6 admitted physical destination kind changed at output {output_index}"
            ));
        }
    }
    Ok(())
}

fn lower_execution(
    admitted: &super::super::admission::AdmittedRenderPlan,
    inputs: &[FoundingRealizationInput],
    texture_row_alignment: u64,
) -> Result<LoweredExecution, String> {
    validate_founding_admission(admitted)?;
    let realizations = normalize_realizations(admitted, inputs)?;
    let object_codes = canonical_object_codes(admitted)?;
    let emitter = founding_emitter(admitted);
    let words = pack_inputs(admitted, &realizations, &object_codes, emitter)?;
    let proof_width = usize::try_from(PROOF_WIDTH).map_err(|_| "R6 width overflow".to_string())?;
    let proof_height =
        usize::try_from(PROOF_HEIGHT).map_err(|_| "R6 height overflow".to_string())?;
    let sample_count = proof_width
        .checked_mul(proof_height)
        .ok_or_else(|| "R6 sample-count overflow".to_string())?;
    let output_word_count = sample_count
        .checked_mul(3)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| "R6 output-size overflow".to_string())?;
    let status_word_count = sample_count
        .checked_add(1)
        .ok_or_else(|| "R6 status-size overflow".to_string())?;
    let logical_row_bytes = u64::from(PROOF_WIDTH)
        .checked_mul(OUTPUT_WORD_BYTES)
        .ok_or_else(|| "R6 logical row size overflow".to_string())?;
    let texture_bytes_per_row = align_up(logical_row_bytes, texture_row_alignment)?;
    let texture_bytes_per_row_u32 = u32::try_from(texture_bytes_per_row)
        .map_err(|_| "R6 aligned texture row does not fit public copy layout".to_string())?;
    let texture_transfer_size = texture_bytes_per_row
        .checked_mul(u64::from(PROOF_HEIGHT))
        .and_then(|bytes| bytes.checked_mul(3))
        .ok_or_else(|| "R6 texture-transfer staging size overflow".to_string())?;

    let mut resources = GpuResourceScope::new();
    let input_payload = PreparedGpuData::<TransferData>::ordinary_pod_transfer(
        "R6 packed input payload",
        &words,
    )
    .map_err(|error| error.to_string())?;
    let input = resources
        .buffer(
            GpuBufferDescriptor::ordinary_owned(
                "R6 packed inputs",
                GpuResourceLifetime::Transient,
                GpuReconstruction::SourceBacked,
                input_payload.layout().byte_len(),
                [GpuBufferUsage::Storage, GpuBufferUsage::CopyDestination],
                GpuBufferInitialization::Uninitialized,
            )
            .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    let input_upload = GpuUploadOperation::whole_buffer(&input, input_payload)
        .map_err(|error| error.to_string())?;

    let output_zeroes = vec![0_u32; output_word_count];
    let output_payload = PreparedGpuData::<TransferData>::ordinary_pod_transfer(
        "R6 packed output initialization",
        &output_zeroes,
    )
    .map_err(|error| error.to_string())?;
    let output = resources
        .buffer(
            GpuBufferDescriptor::ordinary_owned(
                "R6 packed outputs",
                GpuResourceLifetime::Transient,
                GpuReconstruction::SourceBacked,
                output_payload.layout().byte_len(),
                [
                    GpuBufferUsage::Storage,
                    GpuBufferUsage::CopySource,
                    GpuBufferUsage::CopyDestination,
                ],
                GpuBufferInitialization::Uninitialized,
            )
            .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    let output_upload = GpuUploadOperation::whole_buffer(&output, output_payload)
        .map_err(|error| error.to_string())?;

    let status_zeroes = vec![0_u32; status_word_count];
    let status_payload = PreparedGpuData::<TransferData>::ordinary_pod_transfer(
        "R6 proof status initialization",
        &status_zeroes,
    )
    .map_err(|error| error.to_string())?;
    let status = resources
        .buffer(
            GpuBufferDescriptor::ordinary_owned(
                "R6 proof status",
                GpuResourceLifetime::Transient,
                GpuReconstruction::SourceBacked,
                status_payload.layout().byte_len(),
                [
                    GpuBufferUsage::Storage,
                    GpuBufferUsage::CopySource,
                    GpuBufferUsage::CopyDestination,
                ],
                GpuBufferInitialization::Uninitialized,
            )
            .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    let status_upload = GpuUploadOperation::whole_buffer(&status, status_payload)
        .map_err(|error| error.to_string())?;

    let texture_transfer_zeroes = vec![
        0_u8;
        usize::try_from(texture_transfer_size)
            .map_err(|_| "R6 texture-transfer staging size does not fit host memory".to_string())?
    ];
    let texture_transfer_payload = PreparedGpuData::<TransferData>::ordinary_pod_transfer(
        "R6 texture-transfer staging initialization",
        &texture_transfer_zeroes,
    )
    .map_err(|error| error.to_string())?;
    let texture_transfer = resources
        .buffer(
            GpuBufferDescriptor::ordinary_owned(
                "R6 texture-transfer staging",
                GpuResourceLifetime::Transient,
                GpuReconstruction::SourceBacked,
                texture_transfer_size,
                [GpuBufferUsage::CopySource, GpuBufferUsage::CopyDestination],
                GpuBufferInitialization::Uninitialized,
            )
            .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    let texture_transfer_upload =
        GpuUploadOperation::whole_buffer(&texture_transfer, texture_transfer_payload)
            .map_err(|error| error.to_string())?;

    let [source] =
        admit_static_wgsl_sources([("runenrender.r6.execution", 1, FOUNDING_WGSL)])
            .map_err(|error| error.to_string())?;
    let pipeline = GpuComputePipelineDescriptor::ordinary(source, "main")
        .map_err(|error| error.to_string())?;
    let runtime_bindings = pipeline
        .runtime_bindings([
            GpuRuntimeBindingValue::whole_buffer(0, 0, &input),
            GpuRuntimeBindingValue::whole_buffer(0, 1, &output),
            GpuRuntimeBindingValue::whole_buffer(0, 2, &status),
        ])
        .map_err(|error| error.to_string())?;
    let invocation_count =
        u32::try_from(status_word_count).map_err(|_| "R6 invocation overflow".to_string())?;
    let compute = GpuComputeOperation::new(
        pipeline,
        runtime_bindings,
        GpuDispatchIntent::direct(GpuDispatchSize::new(
            invocation_count.div_ceil(WORKGROUP_SIZE),
            1,
            1,
        )),
    )
    .map_err(|error| error.to_string())?;

    let mut row_repack_copies = Vec::new();
    for output_index in 0..3_usize {
        for row in 0..proof_height {
            let source_word_offset = output_index
                .checked_mul(sample_count)
                .and_then(|base| {
                    row.checked_mul(proof_width)
                        .and_then(|row_offset| base.checked_add(row_offset))
                })
                .ok_or_else(|| "R6 texture row source offset overflow".to_string())?;
            let source_byte_offset = u64::try_from(source_word_offset)
                .map_err(|_| "R6 texture row source offset conversion overflow".to_string())?
                .checked_mul(OUTPUT_WORD_BYTES)
                .ok_or_else(|| "R6 texture row source byte offset overflow".to_string())?;
            let destination_row = output_index
                .checked_mul(proof_height)
                .and_then(|base| base.checked_add(row))
                .ok_or_else(|| "R6 texture row destination index overflow".to_string())?;
            let destination_byte_offset = u64::try_from(destination_row)
                .map_err(|_| "R6 texture row destination conversion overflow".to_string())?
                .checked_mul(texture_bytes_per_row)
                .ok_or_else(|| "R6 texture row destination byte offset overflow".to_string())?;
            let source_region = GpuBufferRegion::new(
                &output,
                GpuBufferRange::new(&output, source_byte_offset, logical_row_bytes)
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            let destination_region = GpuBufferRegion::new(
                &texture_transfer,
                GpuBufferRange::new(
                    &texture_transfer,
                    destination_byte_offset,
                    logical_row_bytes,
                )
                .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            row_repack_copies.push(
                GpuCopyOperation::buffer_to_buffer(source_region, destination_region)
                    .map_err(|error| error.to_string())?,
            );
        }
    }

    let mut texture_copies = Vec::new();
    for output_index in 0..3_usize {
        let RenderOutputDestination::SampleLatticeTexture(texture) =
            admitted.outputs()[output_index].binding().destination()
        else {
            return Err(format!(
                "R6 output {output_index} lost its admitted lattice destination"
            ));
        };
        let segment_offset = u64::try_from(output_index)
            .map_err(|_| "R6 texture segment offset conversion overflow".to_string())?
            .checked_mul(u64::from(PROOF_HEIGHT))
            .and_then(|rows| rows.checked_mul(texture_bytes_per_row))
            .ok_or_else(|| "R6 texture segment byte offset overflow".to_string())?;
        let source_layout = GpuBufferTextureLayout::new(
            &texture_transfer,
            segment_offset,
            texture_bytes_per_row_u32,
            0,
        )
        .map_err(|error| error.to_string())?;
        let destination = GpuTextureCopyRegion::whole_base_mip(texture)
            .map_err(|error| error.to_string())?;
        texture_copies.push(
            GpuCopyOperation::buffer_to_texture(source_layout, destination)
                .map_err(|error| error.to_string())?,
        );
    }
    let scalar_offset = u64::try_from(
        sample_count
            .checked_mul(3)
            .ok_or_else(|| "R6 scalar offset overflow".to_string())?,
    )
    .map_err(|_| "R6 scalar offset overflow".to_string())?
    .checked_mul(OUTPUT_WORD_BYTES)
    .ok_or_else(|| "R6 scalar byte offset overflow".to_string())?;
    let scalar_source = GpuBufferRegion::new(
        &output,
        GpuBufferRange::new(&output, scalar_offset, OUTPUT_WORD_BYTES)
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let RenderOutputDestination::ScalarBuffer(scalar_destination) =
        admitted.outputs()[3].binding().destination()
    else {
        return Err("R6 scalar output lost its admitted destination".to_string());
    };
    let scalar_copy = GpuCopyOperation::buffer_to_buffer(
        scalar_source,
        GpuBufferRegion::whole(scalar_destination).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;

    let mut readback_operations = Vec::new();
    let mut readback_ids = Vec::new();
    for output_index in 0..3 {
        let RenderOutputDestination::SampleLatticeTexture(texture) =
            admitted.outputs()[output_index].binding().destination()
        else {
            unreachable!()
        };
        let operation = GpuReadbackOperation::ordinary(GpuTransferRegion::from(
            GpuTextureCopyRegion::whole_base_mip(texture).map_err(|error| error.to_string())?,
        ))
        .map_err(|error| error.to_string())?;
        readback_ids.push(operation.id());
        readback_operations.push(operation);
    }
    let scalar_readback = GpuReadbackOperation::ordinary(GpuTransferRegion::from(
        GpuBufferRegion::whole(scalar_destination).map_err(|error| error.to_string())?,
    ))
    .map_err(|error| error.to_string())?;
    readback_ids.push(scalar_readback.id());
    readback_operations.push(scalar_readback);
    let status_readback = GpuReadbackOperation::ordinary(GpuTransferRegion::from(
        GpuBufferRegion::whole(&status).map_err(|error| error.to_string())?,
    ))
    .map_err(|error| error.to_string())?;
    let status_readback_id = status_readback.id();

    let fragment = GpuWorkFragment::build("RunenRender R6 founding execution", |work| {
        work.operation("upload founding inputs", input_upload)?;
        work.operation("initialize renderer scratch outputs", output_upload)?;
        work.operation("initialize proof status", status_upload)?;
        work.operation(
            "initialize texture-transfer staging",
            texture_transfer_upload,
        )?;
        work.compute("evaluate founding direct lighting", compute)?;
        for (row_index, copy) in row_repack_copies.into_iter().enumerate() {
            work.operation(format!("pack founding lattice row {row_index}"), copy)?;
        }
        for (output_index, copy) in texture_copies.into_iter().enumerate() {
            work.operation(format!("copy founding lattice output {output_index}"), copy)?;
        }
        work.operation("copy founding scalar output", scalar_copy)?;
        for (output_index, readback) in readback_operations.into_iter().enumerate() {
            work.operation(
                format!("read back admitted founding output {output_index}"),
                readback,
            )?;
        }
        work.operation("read back R6 proof status", status_readback)?;
        Ok(())
    })
    .map_err(|error| error.to_string())?;

    let output_readbacks: [GpuReadbackId; 4] = readback_ids
        .try_into()
        .map_err(|_| "R6 output-readback cardinality changed".to_string())?;
    Ok(LoweredExecution {
        work_set: RenderWorkSet::from_lowering(admitted, vec![fragment]),
        output_readbacks,
        status_readback: status_readback_id,
        object_codes,
    })
}

type PackedObjectTransform = ([f64; 9], [f64; 3], [f64; 9]);

fn pack_inputs(
    admitted: &super::super::admission::AdmittedRenderPlan,
    realizations: &[FoundingRealizationInput],
    object_codes: &BTreeMap<RenderObjectId, u32>,
    emitter: RenderDirectionalEmitter,
) -> Result<Vec<u32>, String> {
    let sample_count = PROOF_WIDTH
        .checked_mul(PROOF_HEIGHT)
        .ok_or_else(|| "R6 sample-count overflow".to_string())?;
    let mut words =
        vec![0_u32; INPUT_HEADER_WORDS + GEOMETRY_RECORD_WORDS * realizations.len()];
    words[0] = PROOF_WIDTH;
    words[1] = PROOF_HEIGHT;
    words[2] = sample_count;
    words[3] =
        u32::try_from(realizations.len()).map_err(|_| "R6 geometry-count overflow".to_string())?;
    words[4] = 0;
    words[5] = sample_count;
    words[6] = sample_count
        .checked_mul(2)
        .ok_or_else(|| "R6 identity offset overflow".to_string())?;
    words[7] = sample_count
        .checked_mul(3)
        .ok_or_else(|| "R6 probe offset overflow".to_string())?;

    let observations = admitted.plan().request().observations();
    let RenderObservationSpec::Perspective(perspective) = observations[0] else {
        return Err("R6 perspective observation changed".to_string());
    };
    let RenderObservationSpec::Probe(probe) = observations[1] else {
        return Err("R6 probe observation changed".to_string());
    };
    pack_observation_transform(&mut words, 8, perspective.observation_to_scene())?;
    words[20] = f32_bits((perspective.vertical_field_of_view_radians() * 0.5).tan())?;
    words[21] = f32_bits(perspective.aspect_ratio())?;
    pack_observation_origin(&mut words, 22, probe.observation_to_scene())?;
    pack_vec3(
        &mut words,
        25,
        observation_forward(probe.observation_to_scene())?,
    )?;
    pack_vec3(&mut words, 28, emitter.direction_to_source_scene())?;
    words[31] = f32_bits(emitter.spectral_irradiance_w_m3())?;

    for (index, realization) in realizations.iter().enumerate() {
        let base = INPUT_HEADER_WORDS + index * GEOMETRY_RECORD_WORDS;
        words[base] = realization.shape.kind_code();
        words[base + 1] = *object_codes
            .get(&realization.object_id)
            .ok_or_else(|| "R6 object code missing".to_string())?;
        let scene = admitted.plan().scene();
        let participation = scene
            .object_participation(realization.object_id)
            .ok_or_else(|| "R6 geometry participation missing".to_string())?;
        words[base + 2] = f32_bits(
            participation
                .material_assignment()
                .ok_or_else(|| "R6 material missing".to_string())?
                .material()
                .reflectance(),
        )?;
        let state = scene
            .object_state(realization.object_id)
            .ok_or_else(|| "R6 object state missing".to_string())?;
        let (scene_to_local, translation, normal_to_scene) = object_transform(state)?;
        pack_matrix3(&mut words, base + 4, scene_to_local)?;
        pack_vec3(&mut words, base + 13, translation)?;
        pack_matrix3(&mut words, base + 16, normal_to_scene)?;
        match realization.shape {
            FoundingShape::AnalyticSphere { center, radius }
            | FoundingShape::FieldSphere { center, radius } => {
                pack_vec3(&mut words, base + 25, center)?;
                words[base + 28] = f32_bits(radius)?;
            }
            FoundingShape::AnalyticPlane { point, normal } => {
                pack_vec3(&mut words, base + 25, point)?;
                pack_vec3(&mut words, base + 28, normalize3(normal)?)?;
            }
        }
    }
    Ok(words)
}

fn pack_observation_transform(
    words: &mut [u32],
    base: usize,
    transform: RenderAffineTransform3,
) -> Result<(), String> {
    pack_observation_origin(words, base, transform)?;
    let matrix = transform.row_major_3x4();
    pack_matrix3(
        words,
        base + 3,
        [
            matrix[0], matrix[1], matrix[2], matrix[4], matrix[5], matrix[6], matrix[8], matrix[9],
            matrix[10],
        ],
    )
}

fn pack_observation_origin(
    words: &mut [u32],
    base: usize,
    transform: RenderAffineTransform3,
) -> Result<(), String> {
    let matrix = transform.row_major_3x4();
    pack_vec3(words, base, [matrix[3], matrix[7], matrix[11]])
}

fn observation_forward(transform: RenderAffineTransform3) -> Result<[f64; 3], String> {
    let matrix = transform.row_major_3x4();
    normalize3([-matrix[2], -matrix[6], -matrix[10]])
}

fn object_transform(state: &RenderObjectState) -> Result<PackedObjectTransform, String> {
    let spatial = state.spatial();
    let matrix = spatial.local_to_scene().row_major_3x4();
    let units = spatial.local_space().meters_per_unit();
    let local_to_scene = [
        matrix[0] * units,
        matrix[1] * units,
        matrix[2] * units,
        matrix[4] * units,
        matrix[5] * units,
        matrix[6] * units,
        matrix[8] * units,
        matrix[9] * units,
        matrix[10] * units,
    ];
    let scene_to_local = inverse3(local_to_scene)
        .ok_or_else(|| "R6 object transform is non-invertible".to_string())?;
    Ok((
        scene_to_local,
        [matrix[3], matrix[7], matrix[11]],
        transpose3(scene_to_local),
    ))
}

fn inverse3(matrix: [f64; 9]) -> Option<[f64; 9]> {
    let determinant = matrix[0] * (matrix[4] * matrix[8] - matrix[5] * matrix[7])
        - matrix[1] * (matrix[3] * matrix[8] - matrix[5] * matrix[6])
        + matrix[2] * (matrix[3] * matrix[7] - matrix[4] * matrix[6]);
    if determinant == 0.0 || !determinant.is_finite() {
        return None;
    }
    let inverse = determinant.recip();
    Some([
        (matrix[4] * matrix[8] - matrix[5] * matrix[7]) * inverse,
        (matrix[2] * matrix[7] - matrix[1] * matrix[8]) * inverse,
        (matrix[1] * matrix[5] - matrix[2] * matrix[4]) * inverse,
        (matrix[5] * matrix[6] - matrix[3] * matrix[8]) * inverse,
        (matrix[0] * matrix[8] - matrix[2] * matrix[6]) * inverse,
        (matrix[2] * matrix[3] - matrix[0] * matrix[5]) * inverse,
        (matrix[3] * matrix[7] - matrix[4] * matrix[6]) * inverse,
        (matrix[1] * matrix[6] - matrix[0] * matrix[7]) * inverse,
        (matrix[0] * matrix[4] - matrix[1] * matrix[3]) * inverse,
    ])
}

fn transpose3(matrix: [f64; 9]) -> [f64; 9] {
    [
        matrix[0], matrix[3], matrix[6], matrix[1], matrix[4], matrix[7], matrix[2], matrix[5],
        matrix[8],
    ]
}

fn normalize3(value: [f64; 3]) -> Result<[f64; 3], String> {
    let length = (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt();
    if length == 0.0 || !length.is_finite() {
        return Err("R6 zero/non-finite direction".to_string());
    }
    Ok(value.map(|component| component / length))
}

fn f32_bits(value: f64) -> Result<u32, String> {
    let converted = value as f32;
    if !value.is_finite() || !converted.is_finite() {
        return Err("R6 value is not representable as f32".to_string());
    }
    Ok(converted.to_bits())
}

fn pack_vec3(words: &mut [u32], base: usize, value: [f64; 3]) -> Result<(), String> {
    words[base] = f32_bits(value[0])?;
    words[base + 1] = f32_bits(value[1])?;
    words[base + 2] = f32_bits(value[2])?;
    Ok(())
}

fn pack_matrix3(words: &mut [u32], base: usize, value: [f64; 9]) -> Result<(), String> {
    for (offset, component) in value.into_iter().enumerate() {
        words[base + offset] = f32_bits(component)?;
    }
    Ok(())
}

fn nearest_reference_hit(
    admitted: &super::super::admission::AdmittedRenderPlan,
    realizations: &[FoundingRealizationInput],
    origin: [f64; 3],
    direction: [f64; 3],
    ignored: Option<RenderObjectId>,
) -> Result<Option<ReferenceHit>, String> {
    let mut nearest: Option<ReferenceHit> = None;
    for input in realizations {
        if ignored == Some(input.object_id) {
            continue;
        }
        let state = admitted
            .plan()
            .scene()
            .object_state(input.object_id)
            .ok_or_else(|| "R6 reference object state missing".to_string())?;
        let query = RenderSurfaceQuery::new(
            origin,
            direction,
            RenderTimePoint::from_seconds(0.0).expect("finite R6 reference time"),
        )
        .map_err(|error| format!("R6 reference query rejected: {error:?}"))?;
        let result = input
            .shape
            .realization(input.representation_id)?
            .oriented_surface_query(state, query)
            .map_err(|error| format!("R6 reference realization query failed: {error:?}"))?;
        let Some(hit) = result.hit() else {
            continue;
        };
        if nearest.is_none_or(|current| {
            hit.surface_hit().distance_meters() < current.hit.surface_hit().distance_meters()
        }) {
            nearest = Some(ReferenceHit {
                object_id: input.object_id,
                hit,
            });
        }
    }
    Ok(nearest)
}

fn perspective_ray(
    observation: RenderPerspectiveObservation,
    x: u32,
    y: u32,
) -> Result<([f64; 3], [f64; 3]), String> {
    let transform = observation.observation_to_scene().row_major_3x4();
    let origin = [transform[3], transform[7], transform[11]];
    let u = (f64::from(x) + 0.5) / f64::from(PROOF_WIDTH);
    let v = (f64::from(y) + 0.5) / f64::from(PROOF_HEIGHT);
    let tan_half = (observation.vertical_field_of_view_radians() * 0.5).tan();
    let local = [
        (2.0 * u - 1.0) * tan_half * observation.aspect_ratio(),
        (1.0 - 2.0 * v) * tan_half,
        -1.0,
    ];
    let scene = [
        transform[0] * local[0] + transform[1] * local[1] + transform[2] * local[2],
        transform[4] * local[0] + transform[5] * local[1] + transform[6] * local[2],
        transform[8] * local[0] + transform[9] * local[1] + transform[10] * local[2],
    ];
    Ok((origin, normalize3(scene)?))
}

fn reference_outputs(
    admitted: &super::super::admission::AdmittedRenderPlan,
    realizations: &[FoundingRealizationInput],
    object_codes: &BTreeMap<RenderObjectId, u32>,
) -> Result<(Vec<ReferencePixel>, f64), String> {
    let observations = admitted.plan().request().observations();
    let RenderObservationSpec::Perspective(perspective) = observations[0] else {
        return Err("R6 reference perspective changed".to_string());
    };
    let RenderObservationSpec::Probe(probe) = observations[1] else {
        return Err("R6 reference probe changed".to_string());
    };
    let requested =
        RenderRadiometricRepresentation::spectral_at_wavelength_meters(PROOF_WAVELENGTH_METERS)
            .unwrap();
    let emitter = founding_emitter(admitted);
    let mut pixels = Vec::new();
    for y in 0..PROOF_HEIGHT {
        for x in 0..PROOF_WIDTH {
            let (origin, direction) = perspective_ray(perspective, x, y)?;
            let hit = nearest_reference_hit(admitted, realizations, origin, direction, None)?
                .ok_or_else(|| format!("unexpected R6 CPU primary miss at ({x}, {y})"))?;
            let participation = admitted
                .plan()
                .scene()
                .object_participation(hit.object_id)
                .unwrap();
            let material = participation.material_assignment().unwrap().material();
            let shadow_origin = hit.hit.surface_hit().position_scene_meters();
            let occluded = nearest_reference_hit(
                admitted,
                realizations,
                shadow_origin,
                emitter.direction_to_source_scene(),
                Some(hit.object_id),
            )?
            .is_some();
            let radiance = direct_lighting_radiance(
                material,
                emitter,
                requested,
                hit.hit,
                occluded,
            )
            .map_err(|error| format!("R6 CPU radiance oracle failed: {error:?}"))?;
            let depth = observation_forward_depth(perspective, hit.hit);
            let identity_code = *object_codes.get(&hit.object_id).unwrap();
            pixels.push(ReferencePixel {
                radiance,
                depth,
                identity_code,
            });
        }
    }

    let probe_transform = probe.observation_to_scene().row_major_3x4();
    let probe_origin = [
        probe_transform[3],
        probe_transform[7],
        probe_transform[11],
    ];
    let probe_direction = observation_forward(probe.observation_to_scene())?;
    let hit = nearest_reference_hit(
        admitted,
        realizations,
        probe_origin,
        probe_direction,
        None,
    )?
    .ok_or_else(|| "unexpected R6 CPU probe miss".to_string())?;
    let material = admitted
        .plan()
        .scene()
        .object_participation(hit.object_id)
        .unwrap()
        .material_assignment()
        .unwrap()
        .material();
    let occluded = nearest_reference_hit(
        admitted,
        realizations,
        hit.hit.surface_hit().position_scene_meters(),
        emitter.direction_to_source_scene(),
        Some(hit.object_id),
    )?
    .is_some();
    let probe_radiance =
        direct_lighting_radiance(material, emitter, requested, hit.hit, occluded)
            .map_err(|error| format!("R6 CPU probe oracle failed: {error:?}"))?;
    Ok((pixels, probe_radiance))
}

fn wait_readback(
    context: &GpuContext,
    submission: &GpuSubmission,
    id: GpuReadbackId,
) -> GpuReadbackBytes {
    let readback = submission
        .readback(id)
        .expect("R6 submitted readback identity must remain observable")
        .clone();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        context.progress();
        match readback.status() {
            GpuReadbackStatus::Ready(bytes) => return bytes,
            GpuReadbackStatus::Failed(failure) => panic!("R6 readback failed: {failure:?}"),
            GpuReadbackStatus::Pending => {}
        }
        if let GpuSubmissionStatus::Failed(failure) = submission.status() {
            panic!("R6 submission failed before readback: {failure:?}");
        }
        assert!(Instant::now() < deadline, "R6 readback timed out");
        std::thread::yield_now();
    }
}

fn wait_submission(context: &GpuContext, submission: &GpuSubmission) {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        context.progress();
        match submission.status() {
            GpuSubmissionStatus::Completed => return,
            GpuSubmissionStatus::Failed(failure) => panic!("R6 submission failed: {failure:?}"),
            GpuSubmissionStatus::Accepted => {}
        }
        assert!(
            Instant::now() < deadline,
            "R6 submission did not terminalize"
        );
        std::thread::yield_now();
    }
}

fn decode_words(bytes: &GpuReadbackBytes) -> Vec<u32> {
    let (chunks, remainder) = bytes.as_bytes().as_chunks::<4>();
    assert!(
        remainder.is_empty(),
        "R6 readback must preserve whole u32 words"
    );
    chunks
        .iter()
        .map(|chunk| u32::from_ne_bytes(*chunk))
        .collect()
}

fn work_structure_signature(work_set: &RenderWorkSet) -> Vec<WorkFragmentSignature> {
    work_set
        .fragments()
        .iter()
        .map(|fragment| {
            (
                fragment.label().as_str().to_string(),
                fragment.resources().len(),
                fragment
                    .nodes()
                    .iter()
                    .map(|node| {
                        (
                            node.label().as_str().to_string(),
                            format!("{:?}", node.kind()),
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}

fn assert_close(label: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= NUMERIC_TOLERANCE,
        "{label}: actual={actual}, expected={expected}, tolerance={NUMERIC_TOLERANCE}"
    );
}

#[test]
fn founding_execution_rejects_duplicate_foreign_and_missing_realizations() {
    let Some(context) = request_execution_context() else {
        return;
    };
    let fixture = execution_fixture();
    let (admitted, _) = admit_execution(&fixture, &context);
    let inputs = founding_realizations(&fixture);

    let mut duplicate = inputs.to_vec();
    duplicate.push(inputs[0]);
    assert!(normalize_realizations(&admitted, &duplicate).is_err());

    assert!(normalize_realizations(&admitted, &inputs[..2]).is_err());

    let mut foreign = inputs;
    foreign[0].representation_id = RenderRepresentationId::from_raw(999_999).unwrap();
    assert!(normalize_realizations(&admitted, &foreign).is_err());
}

#[test]
fn founding_execution_rejects_non_founding_admitted_request() {
    let Some(context) = request_execution_context() else {
        return;
    };
    let fixture = founding_fixture();
    let availability = fixture
        .representation_ids
        .iter()
        .copied()
        .map(|representation_id| {
            RenderRepresentationAvailabilityFact::new(
                representation_id,
                RenderRepresentationAvailabilityState::Available,
            )
        })
        .collect::<Vec<_>>();
    let bindings = founding_output_bindings();
    let admitted = admit_render_plan(&fixture.plan, &availability, &bindings, &context)
        .expect("R1-R5 spine should still admit independently of R6 lowering");
    let row_alignment = admitted_texture_row_alignment(&context).unwrap();
    let error = lower_execution(
        &admitted,
        &founding_realizations_for_spine(&fixture),
        row_alignment,
    )
    .err()
    .expect("R6 lowerer must reject a different admitted request rather than reinterpret it");
    assert!(error.contains("bounded founding execution request"));
}

#[test]
fn founding_lowering_is_deterministic_across_realization_insertion_order() {
    let Some(context) = request_execution_context() else {
        return;
    };
    let fixture = execution_fixture();
    let (admitted, _) = admit_execution(&fixture, &context);
    let inputs = founding_realizations(&fixture);
    let mut reversed = inputs;
    reversed.reverse();

    let first_normalized = normalize_realizations(&admitted, &inputs).unwrap();
    let second_normalized = normalize_realizations(&admitted, &reversed).unwrap();
    assert_eq!(first_normalized, second_normalized);

    let object_codes = canonical_object_codes(&admitted).unwrap();
    let emitter = founding_emitter(&admitted);
    assert_eq!(
        pack_inputs(&admitted, &first_normalized, &object_codes, emitter).unwrap(),
        pack_inputs(&admitted, &second_normalized, &object_codes, emitter).unwrap(),
        "non-semantic realization insertion order must not change packed renderer semantics"
    );

    let row_alignment = admitted_texture_row_alignment(&context).unwrap();
    let first = lower_execution(&admitted, &inputs, row_alignment).unwrap();
    let second = lower_execution(&admitted, &reversed, row_alignment).unwrap();
    assert_eq!(first.object_codes, second.object_codes);
    assert_eq!(
        work_structure_signature(&first.work_set),
        work_structure_signature(&second.work_set),
        "equivalent admitted semantics must produce the same public RunenGPU work structure"
    );
}

#[test]
fn founding_renderer_executes_and_matches_cpu_reference_through_public_runengpu() {
    let Some(context) = request_execution_context() else {
        return;
    };
    assert!(
        context
            .device_facts()
            .is_enabled(GpuCapabilityFeature::Compute)
    );
    assert!(
        context
            .device_facts()
            .is_enabled(GpuCapabilityFeature::Copy)
    );

    let fixture = execution_fixture();
    let (admitted, bindings) = admit_execution(&fixture, &context);
    let realizations = founding_realizations(&fixture);
    let row_alignment = admitted_texture_row_alignment(&context).unwrap();
    let lowered = lower_execution(&admitted, &realizations, row_alignment)
        .expect("R6 public-RunenGPU lowering");

    assert_eq!(lowered.work_set.admitted_plan(), &admitted);
    assert_eq!(lowered.work_set.fragments().len(), 1);
    for (output_index, output) in admitted.outputs().iter().enumerate() {
        assert_eq!(output.output_index(), output_index);
        assert_eq!(output.binding(), &bindings[output_index]);
    }

    let (expected_pixels, expected_probe) =
        reference_outputs(&admitted, &realizations, &lowered.object_codes)
            .expect("R6 CPU semantic reference");
    let field_code = *lowered
        .object_codes
        .get(&fixture.geometry_object_ids[2])
        .expect("field-backed founding object must retain renderer identity");
    assert!(
        expected_pixels
            .iter()
            .any(|sample| sample.identity_code == field_code),
        "the bounded CPU reference must actually exercise the field-backed surface"
    );

    let submission = pollster::block_on(context.submit_work(
        "R6 founding execution graph",
        lowered.work_set.fragments().iter().cloned(),
    ))
    .expect("public RunenGPU must execute the R6 founding work set");

    let status = decode_words(&wait_readback(
        &context,
        &submission,
        lowered.status_readback,
    ));
    assert_eq!(
        status,
        vec![0; usize::try_from(PROOF_WIDTH * PROOF_HEIGHT + 1).unwrap()],
        "any primary/probe miss is a structural R6 proof failure, never a semantic zero sentinel"
    );

    let radiance = decode_words(&wait_readback(
        &context,
        &submission,
        lowered.output_readbacks[0],
    ));
    let depth = decode_words(&wait_readback(
        &context,
        &submission,
        lowered.output_readbacks[1],
    ));
    let identity = decode_words(&wait_readback(
        &context,
        &submission,
        lowered.output_readbacks[2],
    ));
    let probe = decode_words(&wait_readback(
        &context,
        &submission,
        lowered.output_readbacks[3],
    ));
    wait_submission(&context, &submission);

    assert_eq!(radiance.len(), expected_pixels.len());
    assert_eq!(depth.len(), expected_pixels.len());
    assert_eq!(identity.len(), expected_pixels.len());
    assert!(
        identity.contains(&field_code),
        "public RunenGPU execution must actually render the field-backed founding surface"
    );
    for (index, expected) in expected_pixels.iter().enumerate() {
        assert_close(
            "R6 perspective radiance",
            f64::from(f32::from_bits(radiance[index])),
            expected.radiance,
        );
        assert_close(
            "R6 perspective depth",
            f64::from(f32::from_bits(depth[index])),
            expected.depth,
        );
        assert_eq!(
            identity[index], expected.identity_code,
            "R6 object identity must remain exact"
        );
    }
    assert_eq!(probe.len(), 1);
    assert_close(
        "R6 scalar radiance probe",
        f64::from(f32::from_bits(probe[0])),
        expected_probe,
    );
}
