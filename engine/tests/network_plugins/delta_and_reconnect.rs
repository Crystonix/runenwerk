// Owner: Engine Networking Tests - Delta and Connection-Scoped Replication
#[derive(Debug, Copy, Clone, Default, ecs::Resource)]
struct NetworkTestFrameDelta(f32);

fn apply_network_test_frame_delta(
    frame_delta: Res<NetworkTestFrameDelta>,
    mut time: ResMut<Time>,
) {
    time.delta_seconds = frame_delta.0;
}

fn install_network_test_clock(app: &mut App) {
    app.init_resource::<NetworkTestFrameDelta>();
    app.add_systems(
        PreUpdate,
        apply_network_test_frame_delta.after(CoreSet::Time),
    );
}

fn run_network_protocol_frame(mut app: App, context: &str) -> App {
    app.world_mut()
        .resource_mut::<NetworkTestFrameDelta>()
        .expect("network test clock should be installed")
        .0 = 0.0;
    app.run_for_frames(1)
        .unwrap_or_else(|error| panic!("{context}: {error:#}"))
}

fn run_network_fixed_tick(mut app: App, context: &str) -> App {
    let step_seconds = app
        .world()
        .resource::<FixedTimeConfig>()
        .expect("fixed-time config should be installed")
        .step_seconds;
    app.world_mut()
        .resource_mut::<NetworkTestFrameDelta>()
        .expect("network test clock should be installed")
        .0 = step_seconds;
    app.run_for_frames(1)
        .unwrap_or_else(|error| panic!("{context}: {error:#}"))
}

#[test]
fn server_delta_snapshot_applies_cleanly_on_client() {
    let mut server = App::headless();
    server.add_plugins(default_plugins());
    server.add_plugins((ScenePlugin, NetworkServerPlugin));
    let connection = ConnectionHandle::new(1);
    install_runennet_connections(&mut server, &[(connection, ParticipantId::new(1))]);

    let server = server
        .run_for_ticks(1)
        .expect("first server replication tick should run");
    let full_snapshot = server
        .world()
        .resource::<NetworkOutboundQueue>()
        .unwrap()
        .server_messages()
        .iter()
        .find_map(|message| match message {
            OutboundServerMessage::ToConnection {
                connection: target,
                message: ServerMessage::Snapshot(snapshot),
            } if *target == connection => Some(snapshot.clone()),
            _ => None,
        })
        .expect("server should emit a full snapshot for the admitted connection");

    let mut server = server;
    enqueue_server_inbox_from(
        server.world_mut(),
        Some(connection),
        ClientMessage::Ack(Ack {
            cursor: full_snapshot.cursor,
            last_received_tick: full_snapshot.tick,
        }),
    )
    .expect("server inbox enqueue should succeed");
    server
        .world_mut()
        .resource_mut::<PlayerCommandBuffer>()
        .unwrap()
        .push(ClientCommandEnvelope::Move(MoveCommand { x: -0.5, y: 0.25 }));

    let server = server
        .run_for_frames(1)
        .expect("ack processing frame should run")
        .run_for_ticks(2)
        .expect("second server replication tick should run");
    assert_eq!(
        server
            .world()
            .resource::<ReplicationDiagnostics>()
            .unwrap()
            .acked,
        1
    );

    let outbound = server.world().resource::<NetworkOutboundQueue>().unwrap();
    let delta_snapshot = outbound
        .server_messages()
        .iter()
        .find_map(|message| match message {
            OutboundServerMessage::ToConnection {
                connection: target,
                message: ServerMessage::DeltaSnapshot(snapshot),
            } if *target == connection => Some(snapshot.clone()),
            _ => None,
        })
        .expect("server should emit a delta snapshot for the admitted connection");
    let authoritative_second_snapshot = server
        .world()
        .resource::<ServerSnapshotState>()
        .unwrap()
        .latest_snapshot
        .clone()
        .expect("server should retain the latest authoritative snapshot");
    let decoded_delta: TestDelta =
        postcard::from_bytes(&delta_snapshot.payload).expect("delta payload should decode");
    let delta_tick = delta_snapshot.tick;
    assert_eq!(delta_snapshot.base, SnapshotCursor(1));
    assert_eq!(delta_snapshot.cursor, SnapshotCursor(2));
    assert!(!full_snapshot.payload.is_empty());
    assert!(!decoded_delta.changed);

    let mut client = App::headless();
    client.add_plugins(default_plugins());
    client.add_plugins((ScenePlugin, NetworkClientPlugin));

    enqueue_client_inbox(client.world_mut(), ServerMessage::Snapshot(full_snapshot))
        .expect("client inbox enqueue should succeed");
    let mut client = client
        .run_for_frames(1)
        .expect("client should accept the full snapshot");

    enqueue_client_inbox(
        client.world_mut(),
        ServerMessage::DeltaSnapshot(delta_snapshot),
    )
    .expect("client inbox enqueue should succeed");
    let client = client
        .run_for_frames(1)
        .expect("client should apply the delta snapshot");

    let replication = client.world().resource::<ClientSnapshotState>().unwrap();
    assert_eq!(replication.last_acknowledged_cursor, SnapshotCursor(2));
    assert_eq!(replication.last_received_tick, delta_tick);
    let last_snapshot = replication
        .last_received_snapshot
        .clone()
        .expect("client should retain the latest applied snapshot");
    assert_eq!(last_snapshot, authoritative_second_snapshot);

    let outbound = client.world().resource::<NetworkOutboundQueue>().unwrap();
    assert!(outbound.client_messages().iter().any(
        |message| matches!(message, ClientMessage::Ack(ack) if ack.cursor == SnapshotCursor(2))
    ));
}

#[test]
fn server_rejects_future_snapshot_ack_without_mutating_baseline() {
    let mut server = App::headless();
    server.add_plugins(default_plugins());
    server.add_plugins((ScenePlugin, NetworkServerPlugin));
    let connection = ConnectionHandle::new(1);
    install_runennet_connections(&mut server, &[(connection, ParticipantId::new(1))]);

    let mut server = server
        .run_for_ticks(1)
        .expect("first server replication tick should run");
    enqueue_server_inbox_from(
        server.world_mut(),
        Some(connection),
        ClientMessage::Ack(Ack {
            cursor: SnapshotCursor(99),
            last_received_tick: SimulationTick(99),
        }),
    )
    .expect("server inbox enqueue should succeed");
    let server = server
        .run_for_frames(1)
        .expect("future ack processing frame should run");

    let diagnostics = server.world().resource::<ReplicationDiagnostics>().unwrap();
    assert_eq!(diagnostics.acked, 0);
    assert_eq!(diagnostics.rejected_acks, 1);

    let replication = server.world().resource::<ServerSnapshotState>().unwrap();
    let checkpoint = replication
        .checkpoints
        .get(&connection)
        .expect("connection checkpoint should exist");
    assert_eq!(checkpoint.last_ack_cursor, SnapshotCursor::default());

    let server = server
        .run_for_ticks(2)
        .expect("second server replication tick should run");
    let outbound = server.world().resource::<NetworkOutboundQueue>().unwrap();
    assert!(outbound.server_messages().iter().any(|message| {
        matches!(
            message,
            OutboundServerMessage::ToConnection {
                connection: target,
                message: ServerMessage::Snapshot(snapshot),
            } if *target == connection
                && snapshot.cursor == SnapshotCursor(2)
                && snapshot.last_applied == SnapshotCursor::default()
        )
    }));
    assert!(
        !outbound.server_messages().iter().any(|message| {
            matches!(
                message,
                OutboundServerMessage::ToConnection {
                    connection: target,
                    message: ServerMessage::DeltaSnapshot(snapshot),
                } if *target == connection && snapshot.base == SnapshotCursor(99)
            )
        }),
        "rejected future ACK must not become a delta baseline"
    );
}

#[test]
fn server_tracks_lagged_input_frames_against_simulation_tick() {
    let mut app = App::headless();
    app.add_plugins(default_plugins());
    app.add_plugins((ScenePlugin, NetworkServerPlugin));
    install_network_test_clock(&mut app);
    let connection = ConnectionHandle::new(1);
    install_runennet_connections(&mut app, &[(connection, ParticipantId::new(1))]);
    *app.world_mut().resource_mut::<SimulationTick>().unwrap() = SimulationTick(5);

    let payload =
        TestReplicationDriver::encode_input(&[ClientCommandEnvelope::Move(MoveCommand {
            x: 0.0,
            y: 1.0,
        })])
        .expect("input payload should encode");

    enqueue_server_inbox_from(
        app.world_mut(),
        Some(connection),
        ClientMessage::InputFrame(InputFrame {
            tick: SimulationTick(4),
            payload,
        }),
    )
    .expect("server inbox enqueue should succeed");

    let app = run_network_protocol_frame(app, "server lagged input frame should run");
    assert_eq!(
        *app.world().resource::<SimulationTick>().unwrap(),
        SimulationTick(5),
        "protocol-only frame must not advance fixed time"
    );
    let diagnostics = app.world().resource::<ReplicationDiagnostics>().unwrap();
    assert_eq!(diagnostics.lagged, 1);
    assert!(app.world().resource::<AppliedInputLog>().is_err());
}

#[test]
fn server_rejects_equal_current_remote_input_as_already_simulated() {
    let mut app = App::headless();
    app.add_plugins(default_plugins());
    app.add_plugins((ScenePlugin, NetworkServerPlugin));
    install_network_test_clock(&mut app);
    let connection = ConnectionHandle::new(1);
    install_runennet_connections(&mut app, &[(connection, ParticipantId::new(1))]);
    *app.world_mut().resource_mut::<SimulationTick>().unwrap() = SimulationTick(5);

    let payload = TestReplicationDriver::encode_input(&[ClientCommandEnvelope::Ability(
        AbilityCommand { slot: 5 },
    )])
    .expect("input payload should encode");

    enqueue_server_inbox_from(
        app.world_mut(),
        Some(connection),
        ClientMessage::InputFrame(InputFrame {
            tick: SimulationTick(5),
            payload,
        }),
    )
    .expect("server inbox enqueue should succeed");

    let app = run_network_protocol_frame(app, "equal-current remote input frame should run");
    let diagnostics = app.world().resource::<ReplicationDiagnostics>().unwrap();
    assert_eq!(diagnostics.lagged, 1);
    assert_eq!(
        *app.world().resource::<SimulationTick>().unwrap(),
        SimulationTick(5)
    );
    assert!(app.world().resource::<AppliedInputLog>().is_err());
}

#[test]
fn future_remote_input_precedes_local_input_for_same_fixed_tick() {
    let mut app = App::headless();
    app.add_plugins(default_plugins());
    app.add_plugins((ScenePlugin, NetworkServerPlugin));
    install_network_test_clock(&mut app);
    let connection = ConnectionHandle::new(1);
    install_runennet_connections(&mut app, &[(connection, ParticipantId::new(1))]);

    let remote = ClientCommandEnvelope::Ability(AbilityCommand { slot: 7 });
    let local = ClientCommandEnvelope::Ability(AbilityCommand { slot: 8 });
    let payload = TestReplicationDriver::encode_input(std::slice::from_ref(&remote))
        .expect("input payload should encode");
    enqueue_server_inbox_from(
        app.world_mut(),
        Some(connection),
        ClientMessage::InputFrame(InputFrame {
            tick: SimulationTick(1),
            payload,
        }),
    )
    .expect("future remote input should enqueue");
    app.world_mut()
        .resource_mut::<PlayerCommandBuffer>()
        .unwrap()
        .push(local.clone());

    let app = run_network_fixed_tick(app, "same-tick remote/local input should run");
    assert_eq!(
        *app.world().resource::<SimulationTick>().unwrap(),
        SimulationTick(1)
    );
    assert_eq!(
        app.world().resource::<AppliedInputLog>().unwrap().inputs,
        vec![remote, local]
    );
    assert_eq!(
        app.world()
            .resource::<ReplicationDiagnostics>()
            .unwrap()
            .lagged,
        0
    );
}

#[test]
fn server_tracks_per_connection_baselines_for_runennet_connections() {
    let mut app = App::headless();
    app.add_plugins(default_plugins());
    app.add_plugins((ScenePlugin, NetworkServerPlugin));
    install_network_test_clock(&mut app);
    let connection_a = ConnectionHandle::new(1);
    let connection_b = ConnectionHandle::new(2);
    install_runennet_connections(
        &mut app,
        &[
            (connection_a, ParticipantId::new(1)),
            (connection_b, ParticipantId::new(2)),
        ],
    );

    let app = run_network_protocol_frame(app, "RunenNet projection frame should run");
    assert_eq!(
        *app.world().resource::<SimulationTick>().unwrap(),
        SimulationTick(0),
        "session projection frame must not advance fixed time"
    );
    let mut app = run_network_fixed_tick(app, "first replication tick should run");
    assert_eq!(
        *app.world().resource::<SimulationTick>().unwrap(),
        SimulationTick(1)
    );

    enqueue_server_inbox_from(
        app.world_mut(),
        Some(connection_a),
        ClientMessage::Ack(Ack {
            cursor: SnapshotCursor(1),
            last_received_tick: SimulationTick(1),
        }),
    )
    .expect("server inbox enqueue should succeed");
    let app = run_network_protocol_frame(app, "ack frame should run");
    assert_eq!(
        *app.world().resource::<SimulationTick>().unwrap(),
        SimulationTick(1),
        "protocol-only ACK frame must not advance fixed time"
    );

    let app = run_network_fixed_tick(app, "second replication tick should run");
    assert_eq!(
        *app.world().resource::<SimulationTick>().unwrap(),
        SimulationTick(2)
    );
    let outbound = app.world().resource::<NetworkOutboundQueue>().unwrap();
    assert!(outbound.server_messages().iter().any(|message| {
        matches!(
            message,
            OutboundServerMessage::ToConnection {
                connection,
                message: ServerMessage::DeltaSnapshot(snapshot),
            } if *connection == connection_a
                && snapshot.base == SnapshotCursor(1)
                && snapshot.cursor == SnapshotCursor(2)
        )
    }));
    assert!(outbound.server_messages().iter().any(|message| {
        matches!(
            message,
            OutboundServerMessage::ToConnection {
                connection,
                message: ServerMessage::Snapshot(snapshot),
            } if *connection == connection_b && snapshot.cursor == SnapshotCursor(2)
        )
    }));

    let replication = app.world().resource::<ServerSnapshotState>().unwrap();
    let checkpoint_a = replication
        .checkpoints
        .get(&connection_a)
        .expect("connection 1 checkpoint should exist");
    let checkpoint_b = replication
        .checkpoints
        .get(&connection_b)
        .expect("connection 2 checkpoint should exist");
    assert_eq!(checkpoint_a.last_ack_cursor, SnapshotCursor(1));
    assert_eq!(checkpoint_b.last_full_snapshot_cursor, SnapshotCursor(2));
}
