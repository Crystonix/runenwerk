use ecs::prelude::*;
use ecs::telemetry::{self, EcsTelemetrySnapshot};
use std::time::{Duration, Instant};

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

#[derive(Copy, Clone)]
struct WorkloadMeta {
    entity_count: usize,
    repetition_count: u32,
    schedule_run_count: u32,
}

fn nanos_to_ms(nanos: u64) -> f64 {
    nanos as f64 / 1_000_000.0
}

fn print_workload_report(
    name: &str,
    setup_elapsed: Duration,
    run_elapsed: Duration,
    meta: WorkloadMeta,
    delta: &EcsTelemetrySnapshot,
) {
    let query_total_ms = nanos_to_ms(
        delta
            .query_matching_nanos
            .saturating_add(delta.query_iter_nanos)
            .saturating_add(delta.query_get_nanos)
            .saturating_add(delta.query_single_nanos),
    );
    let filter_total_ms = nanos_to_ms(
        delta
            .changed_check_nanos
            .saturating_add(delta.added_check_nanos),
    );
    let runtime_total_ms = nanos_to_ms(
        delta
            .runtime_plan_nanos
            .saturating_add(delta.runtime_stage_nanos)
            .saturating_add(delta.runtime_flush_nanos),
    );

    println!("\n=== {} ===", name);
    println!("setup_time_ms: {:.3}", setup_elapsed.as_secs_f64() * 1000.0);
    println!("run_time_ms: {:.3}", run_elapsed.as_secs_f64() * 1000.0);
    println!(
        "metadata: entities={} repetitions={} schedule_runs={}",
        meta.entity_count, meta.repetition_count, meta.schedule_run_count
    );

    println!(
        "derived_ms: query_total={:.3} filter_total={:.3} runtime_total={:.3}",
        query_total_ms, filter_total_ms, runtime_total_ms
    );

    println!(
        "query: matching_calls={} iter_calls={} get_calls={} single_calls={}",
        delta.query_matching_calls,
        delta.query_iter_calls,
        delta.query_get_calls,
        delta.query_single_calls
    );
    println!(
        "runtime: plan_calls={} stage_calls={} flush_calls={} flush_queues={}",
        delta.runtime_plan_calls,
        delta.runtime_stage_calls,
        delta.runtime_flush_calls,
        delta.runtime_flush_command_queues
    );

    println!("schedule_summary:");
    println!(
        "  plan_build_calls={} plan_build_ms={:.3} conflict_checks={} stage_count={}",
        delta.schedule_plan_build_calls,
        nanos_to_ms(delta.schedule_plan_build_nanos),
        delta.schedule_plan_conflict_checks,
        delta.schedule_plan_stage_count
    );

    if delta.query_get_calls == 0 {
        println!("note: query_get_nanos is zero because Query::get was not exercised.");
    }
    if delta.query_single_calls == 0 {
        println!("note: query_single_nanos is zero because Query::single was not exercised.");
    }
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

fn main() {
    telemetry::reset();

    {
        let setup_start = Instant::now();
        let mut world = World::new();
        for i in 0..50_000 {
            world
                .spawn((
                    Position {
                        x: i as f32,
                        y: 0.0,
                    },
                    Velocity { x: 1.0, y: -0.25 },
                ))
                .expect("profile setup spawn should succeed");
        }
        let query = world.query_state::<(&mut Position, &Velocity), ()>();
        let setup_elapsed = setup_start.elapsed();

        // Warmup iteration (untimed).
        for (position, velocity) in query.iter(&mut world) {
            position.x += velocity.x;
            position.y += velocity.y;
        }

        let before = telemetry::snapshot();
        let run_start = Instant::now();
        for _ in 0..8 {
            for (position, velocity) in query.iter(&mut world) {
                position.x += velocity.x;
                position.y += velocity.y;
            }
        }
        let run_elapsed = run_start.elapsed();
        let after = telemetry::snapshot();

        print_workload_report(
            "W1 broad transform update (50k x 8)",
            setup_elapsed,
            run_elapsed,
            WorkloadMeta {
                entity_count: 50_000,
                repetition_count: 8,
                schedule_run_count: 0,
            },
            &telemetry::snapshot_delta(&before, &after),
        );
    }

    {
        let setup_start = Instant::now();
        let mut world = World::new();
        for i in 0..50_000 {
            world
                .spawn((
                    Position {
                        x: i as f32,
                        y: 0.0,
                    },
                    Velocity { x: 1.0, y: -0.25 },
                ))
                .expect("profile setup spawn should succeed");
        }
        let query = world.query_state::<&Position, ()>();
        let setup_elapsed = setup_start.elapsed();

        // Warmup iteration (untimed).
        let mut warmup_checksum = 0.0_f32;
        for position in query.iter(&world) {
            warmup_checksum += position.x + position.y;
        }
        std::hint::black_box(warmup_checksum);

        let before = telemetry::snapshot();
        let run_start = Instant::now();
        let mut checksum = 0.0_f32;
        for _ in 0..8 {
            for position in query.iter(&world) {
                checksum += position.x + position.y;
            }
        }
        std::hint::black_box(checksum);
        let run_elapsed = run_start.elapsed();
        let after = telemetry::snapshot();

        print_workload_report(
            "C2 broad no-filter read query (50k x 8)",
            setup_elapsed,
            run_elapsed,
            WorkloadMeta {
                entity_count: 50_000,
                repetition_count: 8,
                schedule_run_count: 0,
            },
            &telemetry::snapshot_delta(&before, &after),
        );
    }

    {
        let setup_start = Instant::now();
        let mut world = World::new();
        for i in 0..50_000 {
            world
                .spawn((
                    Position {
                        x: i as f32,
                        y: 0.0,
                    },
                    Velocity { x: 1.0, y: -0.25 },
                ))
                .expect("profile setup spawn should succeed");
        }
        let query = world.query_state::<&mut Velocity, ()>();
        let setup_elapsed = setup_start.elapsed();

        // Warmup iteration (untimed).
        let mut warmup_checksum = 0.0_f32;
        for velocity in query.iter(&mut world) {
            velocity.x += 0.01;
            velocity.y -= 0.01;
            warmup_checksum += velocity.x + velocity.y;
        }
        std::hint::black_box(warmup_checksum);

        let before = telemetry::snapshot();
        let run_start = Instant::now();
        let mut checksum = 0.0_f32;
        for _ in 0..8 {
            for velocity in query.iter(&mut world) {
                velocity.x += 0.01;
                velocity.y -= 0.01;
                checksum += velocity.x + velocity.y;
            }
        }
        std::hint::black_box(checksum);
        let run_elapsed = run_start.elapsed();
        let after = telemetry::snapshot();

        print_workload_report(
            "C3 broad simple write query (50k x 8)",
            setup_elapsed,
            run_elapsed,
            WorkloadMeta {
                entity_count: 50_000,
                repetition_count: 8,
                schedule_run_count: 0,
            },
            &telemetry::snapshot_delta(&before, &after),
        );
    }

    {
        let setup_start = Instant::now();
        let mut world = World::new();
        for i in 0..50_000 {
            world
                .spawn((
                    Position {
                        x: i as f32,
                        y: 0.0,
                    },
                    Velocity { x: 1.0, y: -0.25 },
                ))
                .expect("profile setup spawn should succeed");
        }
        let query = world.query_state::<(&mut Position, &mut Velocity), ()>();
        let setup_elapsed = setup_start.elapsed();

        // Warmup iteration (untimed).
        let mut warmup_checksum = 0.0_f32;
        for (position, velocity) in query.iter(&mut world) {
            velocity.x += 0.01;
            velocity.y -= 0.01;
            position.x += velocity.x;
            position.y += velocity.y;
            warmup_checksum += position.x + position.y + velocity.x + velocity.y;
        }
        std::hint::black_box(warmup_checksum);

        let before = telemetry::snapshot();
        let run_start = Instant::now();
        let mut checksum = 0.0_f32;
        for _ in 0..8 {
            for (position, velocity) in query.iter(&mut world) {
                velocity.x += 0.01;
                velocity.y -= 0.01;
                position.x += velocity.x;
                position.y += velocity.y;
                checksum += position.x + position.y + velocity.x + velocity.y;
            }
        }
        std::hint::black_box(checksum);
        let run_elapsed = run_start.elapsed();
        let after = telemetry::snapshot();

        print_workload_report(
            "C4 broad double mutable query (50k x 8)",
            setup_elapsed,
            run_elapsed,
            WorkloadMeta {
                entity_count: 50_000,
                repetition_count: 8,
                schedule_run_count: 0,
            },
            &telemetry::snapshot_delta(&before, &after),
        );
    }

    {
        let setup_start = Instant::now();
        let mut world = World::new();
        world.insert_resource(MixedStats::default());
        for i in 0..20_000 {
            if i % 8 == 0 {
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
                    .expect("profile setup spawn should succeed");
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
                    .expect("profile setup spawn should succeed");
            }
        }
        let mut runtime = Runtime::new();
        runtime.add_systems::<W2, _, _>(
            &mut world,
            (w2_move, w2_mutate_health, w2_scan_changed, w2_scan_added),
        );
        runtime
            .plan_for::<W2>()
            .expect("w2 plan should validate")
            .expect("w2 plan should exist");
        let setup_elapsed = setup_start.elapsed();

        // Warmup iteration (untimed).
        runtime
            .run_schedule::<W2>(&mut world)
            .expect("w2 warmup run");

        let before = telemetry::snapshot();
        let run_start = Instant::now();
        for _ in 0..20 {
            runtime.run_schedule::<W2>(&mut world).expect("w2 run");
        }
        let run_elapsed = run_start.elapsed();
        let after = telemetry::snapshot();

        print_workload_report(
            "W2 mixed/composite gameplay schedule (20k x 20 runs)",
            setup_elapsed,
            run_elapsed,
            WorkloadMeta {
                entity_count: 20_000,
                repetition_count: 20,
                schedule_run_count: 20,
            },
            &telemetry::snapshot_delta(&before, &after),
        );
    }

    {
        let setup_start = Instant::now();
        let mut world = World::new();
        for i in 0..20_000 {
            world
                .spawn((
                    ChurnTag,
                    Position {
                        x: i as f32,
                        y: 0.0,
                    },
                    Velocity { x: 0.0, y: 0.0 },
                ))
                .expect("profile setup spawn should succeed");
        }
        let mut runtime = Runtime::new();
        runtime.add_systems::<W3, _, _>(&mut world, (w3_spawn, w3_despawn));
        runtime
            .plan_for::<W3>()
            .expect("w3 plan should validate")
            .expect("w3 plan should exist");
        let setup_elapsed = setup_start.elapsed();

        // Warmup iteration (untimed).
        runtime
            .run_schedule::<W3>(&mut world)
            .expect("w3 warmup run");

        let before = telemetry::snapshot();
        let run_start = Instant::now();
        for _ in 0..20 {
            runtime.run_schedule::<W3>(&mut world).expect("w3 run");
        }
        let run_elapsed = run_start.elapsed();
        let after = telemetry::snapshot();

        print_workload_report(
            "W3 structural churn schedule (20k base, 128 churn x 20 runs)",
            setup_elapsed,
            run_elapsed,
            WorkloadMeta {
                entity_count: 20_000,
                repetition_count: 20,
                schedule_run_count: 20,
            },
            &telemetry::snapshot_delta(&before, &after),
        );
    }

    {
        let setup_start = Instant::now();
        let mut world = World::new();
        world.insert_resource(R0::default());
        world.insert_resource(R1::default());
        world.insert_resource(R2::default());
        world.insert_resource(R3::default());
        world.insert_resource(Sink::default());

        let mut runtime = Runtime::new();
        for _ in 0..16 {
            runtime.add_systems::<W5, _, _>(
                &mut world,
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
        // Build the execution plan before timed runs so setup/planning cost is separated.
        runtime
            .plan_for::<W5>()
            .expect("w5 plan should validate")
            .expect("w5 plan should exist");
        let setup_elapsed = setup_start.elapsed();

        // Warmup iteration (untimed).
        runtime
            .run_schedule::<W5>(&mut world)
            .expect("w5 warmup run");

        let before = telemetry::snapshot();
        let run_start = Instant::now();
        for _ in 0..40 {
            runtime.run_schedule::<W5>(&mut world).expect("w5 run");
        }
        let run_elapsed = run_start.elapsed();
        let after = telemetry::snapshot();

        print_workload_report(
            "W5 scheduler stress schedule (16 registrations x 40 runs)",
            setup_elapsed,
            run_elapsed,
            WorkloadMeta {
                entity_count: 0,
                repetition_count: 40,
                schedule_run_count: 40,
            },
            &telemetry::snapshot_delta(&before, &after),
        );
    }

    println!("\n=== cumulative snapshot ===");
    println!("{:#?}", telemetry::snapshot());
}
