use ecs::prelude::*;
use ecs::telemetry::{self, EcsTelemetrySnapshot};
use scheduler::{ScheduleLabel, SystemSet};
use std::time::Instant;

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

fn print_workload_report(name: &str, elapsed: std::time::Duration, delta: &EcsTelemetrySnapshot) {
    println!("\n=== {} ===", name);
    println!("wall_time_ms: {:.3}", elapsed.as_secs_f64() * 1000.0);
    println!("telemetry: {:#?}", delta);
}

fn main() {
    telemetry::reset();

    let before_w1 = telemetry::snapshot();
    let start_w1 = Instant::now();
    {
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
        for _ in 0..8 {
            for (position, velocity) in query.iter(&mut world) {
                position.x += velocity.x;
                position.y += velocity.y;
            }
        }
    }
    let after_w1 = telemetry::snapshot();
    print_workload_report(
        "W1 broad transform update",
        start_w1.elapsed(),
        &telemetry::snapshot_delta(&before_w1, &after_w1),
    );

    let before_c2 = telemetry::snapshot();
    let start_c2 = Instant::now();
    {
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
        let mut checksum = 0.0_f32;
        for _ in 0..8 {
            for position in query.iter(&world) {
                checksum += position.x + position.y;
            }
        }
        std::hint::black_box(checksum);
    }
    let after_c2 = telemetry::snapshot();
    print_workload_report(
        "C2 broad no-filter read query",
        start_c2.elapsed(),
        &telemetry::snapshot_delta(&before_c2, &after_c2),
    );

    let before_c3 = telemetry::snapshot();
    let start_c3 = Instant::now();
    {
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
        let mut checksum = 0.0_f32;
        for _ in 0..8 {
            for velocity in query.iter(&mut world) {
                velocity.x += 0.01;
                velocity.y -= 0.01;
                checksum += velocity.x + velocity.y;
            }
        }
        std::hint::black_box(checksum);
    }
    let after_c3 = telemetry::snapshot();
    print_workload_report(
        "C3 broad simple write query",
        start_c3.elapsed(),
        &telemetry::snapshot_delta(&before_c3, &after_c3),
    );

    let before_c4 = telemetry::snapshot();
    let start_c4 = Instant::now();
    {
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
    }
    let after_c4 = telemetry::snapshot();
    print_workload_report(
        "C4 broad double mutable query",
        start_c4.elapsed(),
        &telemetry::snapshot_delta(&before_c4, &after_c4),
    );

    let before_w2 = telemetry::snapshot();
    let start_w2 = Instant::now();
    {
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
        for _ in 0..20 {
            runtime.run_schedule::<W2>(&mut world).expect("w2 run");
        }
    }
    let after_w2 = telemetry::snapshot();
    print_workload_report(
        "W2 gameplay mixed",
        start_w2.elapsed(),
        &telemetry::snapshot_delta(&before_w2, &after_w2),
    );

    let before_w3 = telemetry::snapshot();
    let start_w3 = Instant::now();
    {
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
        for _ in 0..20 {
            runtime.run_schedule::<W3>(&mut world).expect("w3 run");
        }
    }
    let after_w3 = telemetry::snapshot();
    print_workload_report(
        "W3 structural churn",
        start_w3.elapsed(),
        &telemetry::snapshot_delta(&before_w3, &after_w3),
    );

    let before_w5 = telemetry::snapshot();
    let start_w5 = Instant::now();
    {
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

        for _ in 0..40 {
            runtime.run_schedule::<W5>(&mut world).expect("w5 run");
        }
    }
    let after_w5 = telemetry::snapshot();
    print_workload_report(
        "W5 scheduler stress",
        start_w5.elapsed(),
        &telemetry::snapshot_delta(&before_w5, &after_w5),
    );

    println!("\n=== cumulative snapshot ===");
    println!("{:#?}", telemetry::snapshot());
}
