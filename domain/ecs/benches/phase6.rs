use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ecs::prelude::*;
use std::hint::black_box;

#[derive(Debug, Copy, Clone, ecs::Component, ecs::Resource)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, Copy, Clone, ecs::Component, ecs::Resource)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Debug, Copy, Clone, ecs::Component, ecs::Resource)]
struct Simulated;

#[derive(Debug, Copy, Clone, ecs::Component, ecs::Resource)]
struct Disabled;

#[derive(Debug, Copy, Clone, ecs::Component, ecs::Resource)]
struct Health(i32);

#[derive(Debug, Copy, Clone, ecs::Component, ecs::Resource)]
struct ChurnTag;

#[derive(Debug, Default, ecs::Component, ecs::Resource)]
struct MixedStats(u64);

#[derive(Debug, Default, ecs::Component, ecs::Resource)]
struct R0(u64);
#[derive(Debug, Default, ecs::Component, ecs::Resource)]
struct R1(u64);
#[derive(Debug, Default, ecs::Component, ecs::Resource)]
struct R2(u64);
#[derive(Debug, Default, ecs::Component, ecs::Resource)]
struct R3(u64);
#[derive(Debug, Default, ecs::Component, ecs::Resource)]
struct Sink(u64);

#[derive(Copy, Clone)]
struct W2;
impl ScheduleLabel for W2 {
    fn name() -> &'static str {
        "W2"
    }
}

#[derive(Copy, Clone)]
struct W3;
impl ScheduleLabel for W3 {
    fn name() -> &'static str {
        "W3"
    }
}

#[derive(Copy, Clone)]
struct W5;
impl ScheduleLabel for W5 {
    fn name() -> &'static str {
        "W5"
    }
}

#[derive(Copy, Clone)]
struct SetA;
impl SystemSet for SetA {
    fn name() -> &'static str {
        "SetA"
    }
}

#[derive(Copy, Clone)]
struct SetB;
impl SystemSet for SetB {
    fn name() -> &'static str {
        "SetB"
    }
}

#[derive(Copy, Clone)]
struct SetC;
impl SystemSet for SetC {
    fn name() -> &'static str {
        "SetC"
    }
}

fn build_world_for_w1(entity_count: usize) -> World {
    let mut world = World::new();
    for i in 0..entity_count {
        world
            .spawn((
                Position {
                    x: i as f32,
                    y: (i as f32) * 0.5,
                },
                Velocity { x: 1.0, y: -0.5 },
            ))
            .expect("benchmark setup spawn should succeed");
    }
    world
}

fn workload_w1(c: &mut Criterion) {
    let mut group = c.benchmark_group("w1_broad_transform_update");
    for &size in &[10_000_usize, 50_000, 200_000] {
        let mut world = build_world_for_w1(size);
        let query = world.query_state::<(&mut Position, &Velocity), ()>();
        group.bench_with_input(BenchmarkId::new("entities", size), &size, |b, _| {
            b.iter(|| {
                let mut checksum = 0.0_f32;
                for _ in 0..4 {
                    for (position, velocity) in query.iter(&mut world) {
                        position.x += velocity.x;
                        position.y += velocity.y;
                        checksum += position.x + position.y;
                    }
                }
                black_box(checksum)
            });
        });
    }
    group.finish();
}

fn workload_c2_broad_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("c2_broad_read_only");
    for &size in &[10_000_usize, 50_000, 200_000] {
        let world = build_world_for_w1(size);
        let query = world.query_state::<&Position, ()>();
        group.bench_with_input(BenchmarkId::new("entities", size), &size, |b, _| {
            b.iter(|| {
                let mut checksum = 0.0_f32;
                for _ in 0..4 {
                    for position in query.iter(&world) {
                        checksum += position.x + position.y;
                    }
                }
                black_box(checksum)
            });
        });
    }
    group.finish();
}

fn workload_c3_broad_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("c3_broad_write_only");
    for &size in &[10_000_usize, 50_000, 200_000] {
        let mut world = build_world_for_w1(size);
        let query = world.query_state::<&mut Velocity, ()>();
        group.bench_with_input(BenchmarkId::new("entities", size), &size, |b, _| {
            b.iter(|| {
                let mut checksum = 0.0_f32;
                for _ in 0..4 {
                    for velocity in query.iter(&mut world) {
                        velocity.x += 0.01;
                        velocity.y -= 0.01;
                        checksum += velocity.x + velocity.y;
                    }
                }
                black_box(checksum)
            });
        });
    }
    group.finish();
}

fn workload_c4_broad_double_mut(c: &mut Criterion) {
    let mut group = c.benchmark_group("c4_broad_double_mut");
    for &size in &[10_000_usize, 50_000, 200_000] {
        let mut world = build_world_for_w1(size);
        let query = world.query_state::<(&mut Position, &mut Velocity), ()>();
        group.bench_with_input(BenchmarkId::new("entities", size), &size, |b, _| {
            b.iter(|| {
                let mut checksum = 0.0_f32;
                for _ in 0..4 {
                    for (position, velocity) in query.iter(&mut world) {
                        velocity.x += 0.01;
                        velocity.y -= 0.01;
                        position.x += velocity.x;
                        position.y += velocity.y;
                        checksum += position.x + position.y + velocity.x + velocity.y;
                    }
                }
                black_box(checksum)
            });
        });
    }
    group.finish();
}

#[allow(clippy::type_complexity)]
fn w2_move(mut query: Query<(&mut Position, &Velocity), (With<Simulated>, Without<Disabled>)>) {
    for (position, velocity) in query.iter() {
        position.x += velocity.x;
        position.y += velocity.y;
    }
}

fn w2_mutate_health(mut query: Query<&mut Health, With<Simulated>>) {
    for health in query.iter() {
        health.0 = health.0.saturating_sub(1);
    }
}

fn w2_scan_changed(
    mut query: Query<(Entity, &Health), Changed<Health>>,
    mut stats: ResMut<MixedStats>,
) {
    let mut seen = 0_u64;
    for _ in query.iter() {
        seen = seen.saturating_add(1);
    }
    stats.0 = stats.0.wrapping_add(seen);
}

fn w2_scan_added(
    mut query: Query<(Entity, &Health), Added<Health>>,
    mut stats: ResMut<MixedStats>,
) {
    let mut seen = 0_u64;
    for _ in query.iter() {
        seen = seen.saturating_add(1);
    }
    stats.0 = stats.0.wrapping_add(seen);
}

fn build_runtime_for_w2(entity_count: usize) -> (World, Runtime) {
    let mut world = World::new();
    world.insert_resource(MixedStats::default());

    for i in 0..entity_count {
        let disabled = i % 8 == 0;
        if disabled {
            world
                .spawn((
                    Position {
                        x: i as f32,
                        y: 0.0,
                    },
                    Velocity { x: 1.0, y: 0.25 },
                    Health(100),
                    Simulated,
                    Disabled,
                ))
                .expect("benchmark setup spawn should succeed");
        } else {
            world
                .spawn((
                    Position {
                        x: i as f32,
                        y: 0.0,
                    },
                    Velocity { x: 1.0, y: 0.25 },
                    Health(100),
                    Simulated,
                ))
                .expect("benchmark setup spawn should succeed");
        }
    }

    let mut runtime = Runtime::new();
    runtime.add_systems::<W2, _, _>(
        &mut world,
        (w2_move, w2_mutate_health, w2_scan_changed, w2_scan_added),
    );
    (world, runtime)
}

fn workload_w2(c: &mut Criterion) {
    let mut group = c.benchmark_group("w2_gameplay_mixed");
    for &size in &[5_000_usize, 20_000] {
        let (mut world, mut runtime) = build_runtime_for_w2(size);
        group.bench_with_input(BenchmarkId::new("entities", size), &size, |b, _| {
            b.iter(|| {
                runtime
                    .run_schedule::<W2>(&mut world)
                    .expect("w2 schedule should run");
                black_box(world.resource::<MixedStats>().expect("stats resource").0)
            });
        });
    }
    group.finish();
}

fn w3_spawn(mut commands: Commands) {
    for i in 0..128_u32 {
        commands.spawn((
            ChurnTag,
            Position {
                x: i as f32,
                y: i as f32,
            },
            Velocity { x: 0.1, y: -0.1 },
        ));
    }
}

fn w3_despawn(mut commands: Commands, mut query: Query<(Entity, &ChurnTag)>) {
    for (idx, (entity, _)) in query.iter().enumerate() {
        if idx >= 128 {
            break;
        }
        commands.despawn(entity);
    }
}

fn build_runtime_for_w3(entity_count: usize) -> (World, Runtime) {
    let mut world = World::new();
    for i in 0..entity_count {
        world
            .spawn((
                ChurnTag,
                Position {
                    x: i as f32,
                    y: 0.0,
                },
                Velocity { x: 0.0, y: 0.0 },
            ))
            .expect("benchmark setup spawn should succeed");
    }

    let mut runtime = Runtime::new();
    runtime.add_systems::<W3, _, _>(&mut world, (w3_spawn, w3_despawn));
    (world, runtime)
}

fn workload_w3(c: &mut Criterion) {
    let mut group = c.benchmark_group("w3_structural_churn");
    let (mut world, mut runtime) = build_runtime_for_w3(20_000);
    group.bench_function("spawn_insert_remove_despawn_commands", |b| {
        b.iter(|| {
            runtime
                .run_schedule::<W3>(&mut world)
                .expect("w3 schedule should run");
            black_box(world.current_change_tick())
        });
    });
    group.finish();
}

fn w5_write_r0(mut r0: ResMut<R0>) {
    r0.0 = r0.0.wrapping_add(1);
}

fn w5_write_r1(mut r1: ResMut<R1>) {
    r1.0 = r1.0.wrapping_add(1);
}

fn w5_write_r2(mut r2: ResMut<R2>) {
    r2.0 = r2.0.wrapping_add(1);
}

fn w5_write_r3(mut r3: ResMut<R3>) {
    r3.0 = r3.0.wrapping_add(1);
}

fn w5_read_mix(r0: Res<R0>, r1: Res<R1>, r2: Res<R2>, mut sink: ResMut<Sink>) {
    sink.0 = sink
        .0
        .wrapping_add(r0.0)
        .wrapping_add(r1.0)
        .wrapping_add(r2.0);
}

fn w5_read_mix_alt(r1: Res<R1>, r3: Res<R3>, mut sink: ResMut<Sink>) {
    sink.0 = sink.0.wrapping_add(r1.0).wrapping_add(r3.0);
}

fn register_w5_systems(runtime: &mut Runtime, world: &mut World, repeats: usize) {
    for _ in 0..repeats {
        runtime.add_systems::<W5, _, _>(
            world,
            (
                w5_write_r0.in_set(SetA),
                w5_write_r1.in_set(SetA),
                w5_read_mix.after(SetA).in_set(SetB),
                w5_write_r2.after(SetB).in_set(SetC),
                w5_read_mix_alt.after(SetC),
                w5_write_r3.in_set(SetA),
            ),
        );
    }
}

fn build_runtime_for_w5(repeats: usize) -> (World, Runtime) {
    let mut world = World::new();
    world.insert_resource(R0::default());
    world.insert_resource(R1::default());
    world.insert_resource(R2::default());
    world.insert_resource(R3::default());
    world.insert_resource(Sink::default());

    let mut runtime = Runtime::new();
    register_w5_systems(&mut runtime, &mut world, repeats);
    (world, runtime)
}

fn workload_w5(c: &mut Criterion) {
    let mut group = c.benchmark_group("w5_scheduler_stress");

    group.bench_function("scheduler_plan_build_96_systems", |b| {
        b.iter(|| {
            let (world, mut runtime) = build_runtime_for_w5(16);
            let stage_count = runtime
                .plan_for::<W5>()
                .expect("w5 plan should validate")
                .expect("w5 plan should exist")
                .stages
                .len();
            black_box(stage_count);
            black_box(world.resource::<Sink>().expect("sink resource").0)
        });
    });

    let (mut world, mut runtime) = build_runtime_for_w5(16);
    group.bench_function("scheduler_stage_execution_96_systems", |b| {
        b.iter(|| {
            runtime
                .run_schedule::<W5>(&mut world)
                .expect("w5 schedule should run");
            black_box(world.resource::<Sink>().expect("sink resource").0)
        });
    });

    group.finish();
}

fn phase6_benches(c: &mut Criterion) {
    workload_w1(c);
    workload_c2_broad_read(c);
    workload_c3_broad_write(c);
    workload_c4_broad_double_mut(c);
    workload_w2(c);
    workload_w3(c);
    workload_w5(c);
}

criterion_group!(phase6, phase6_benches);
criterion_main!(phase6);
