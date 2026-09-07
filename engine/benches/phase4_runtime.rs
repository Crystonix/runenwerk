use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ecs::{With, Without};
use engine::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Component, ecs::Resource)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, Copy, Clone, PartialEq, Component, ecs::Resource)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Debug, Copy, Clone, PartialEq, Component, ecs::Resource)]
struct Simulated;

#[derive(Debug, Copy, Clone, PartialEq, Component, ecs::Resource)]
struct Disabled;

struct Phase4RuntimeBenchPlugin {
    entity_count: usize,
}

impl Plugin for Phase4RuntimeBenchPlugin {
    fn build(&self, app: &mut App) {
        for i in 0..self.entity_count {
            if i % 10 == 0 {
                app.world_mut()
                    .spawn((
                        Position {
                            x: i as f32,
                            y: i as f32,
                        },
                        Velocity { x: 1.0, y: -0.5 },
                        Simulated,
                        Disabled,
                    ))
                    .expect("benchmark setup spawn should succeed");
            } else {
                app.world_mut()
                    .spawn((
                        Position {
                            x: i as f32,
                            y: i as f32,
                        },
                        Velocity { x: 1.0, y: -0.5 },
                        Simulated,
                    ))
                    .expect("benchmark setup spawn should succeed");
            }
        }

        app.add_systems(Update, movement);
    }
}

#[allow(clippy::type_complexity)]
fn movement(mut query: Query<(&mut Position, &Velocity), (With<Simulated>, Without<Disabled>)>) {
    for (position, velocity) in query.iter() {
        position.x += velocity.x;
        position.y += velocity.y;
    }
}

fn workload_engine_runtime(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine_runtime_phase4");

    for &entity_count in &[5_000_usize, 20_000] {
        let mut app = App::headless();
        app.add_plugin(Phase4RuntimeBenchPlugin { entity_count });
        let mut app_slot = Some(app.run_for_frames(1).expect("startup frame should run"));

        group.bench_with_input(
            BenchmarkId::new("headless_mixed_frame", entity_count),
            &entity_count,
            |b, _| {
                b.iter(|| {
                    let mut app = app_slot
                        .take()
                        .expect("bench app state should always be available");
                    app = app.run_for_frames(1).expect("frame should run");
                    app_slot = Some(app);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(phase4_runtime, workload_engine_runtime);
criterion_main!(phase4_runtime);
