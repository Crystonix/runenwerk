// Owner: Engine Networking Tests - Basic Flow

fn client_probe(value: u8) -> ClientMessage {
    ClientMessage::TypedPayload(TypedPayloadMessage::new(
        "test/client",
        "ClientProbe",
        1,
        vec![value],
    ))
}

fn server_probe(value: u8) -> ServerMessage {
    ServerMessage::TypedPayload(TypedPayloadMessage::new(
        "test/server",
        "ServerProbe",
        1,
        vec![value],
    ))
}

fn install_pending_endpoint_resources(world: &mut World) {
    world.insert_resource(NetworkClientInbox::default());
    world.insert_resource(NetworkServerInbox::default());
    world.insert_resource(NetworkClientOutbox::default());
    world.insert_resource(NetworkServerOutbox::default());
}

#[test]
fn pending_endpoint_resources_preserve_fifo_order() {
    let mut world = World::new();
    install_pending_endpoint_resources(&mut world);
    let client_messages = vec![client_probe(1), client_probe(2), client_probe(3)];
    let server_messages = vec![server_probe(4), server_probe(5), server_probe(6)];

    for message in server_messages.iter().cloned() {
        enqueue_client_inbox(&mut world, message).expect("client inbox enqueue should succeed");
    }
    for message in client_messages.iter().cloned() {
        enqueue_server_inbox(&mut world, message).expect("server inbox enqueue should succeed");
    }
    for message in client_messages.iter().cloned() {
        enqueue_client_outbox(&mut world, message).expect("client outbox enqueue should succeed");
    }
    for message in server_messages.iter().cloned() {
        enqueue_server_outbox_broadcast(&mut world, message)
            .expect("server outbox enqueue should succeed");
    }

    assert_eq!(
        engine::plugins::net::drain_client_inbox(&mut world),
        server_messages
    );
    let drained_server_inbox = engine::plugins::net::drain_server_inbox(&mut world);
    assert_eq!(
        drained_server_inbox
            .into_iter()
            .map(|incoming| incoming.message)
            .collect::<Vec<_>>(),
        client_messages
    );
    assert_eq!(
        engine::plugins::net::drain_client_outbox(&mut world),
        client_messages
    );
    assert_eq!(
        engine::plugins::net::drain_server_outbox(&mut world),
        server_messages
            .into_iter()
            .map(OutboundServerMessage::Broadcast)
            .collect::<Vec<_>>()
    );
}

#[test]
fn pending_endpoint_enqueue_requires_installed_owner_resource_and_recovers_payload() {
    let mut world = World::new();
    let rejected = client_probe(254);

    let error = enqueue_client_outbox(&mut world, rejected.clone())
        .expect_err("enqueue without the endpoint owner resource should fail");

    assert!(world.resource::<NetworkClientOutbox>().is_err());
    assert_eq!(error.capacity(), None);
    assert!(matches!(
        &error,
        engine::plugins::net::NetworkPendingEnqueueError::Unavailable {
            endpoint: "NetworkClientOutbox",
            ..
        }
    ));
    assert_eq!(error.into_message(), rejected);
}

#[test]
fn pending_endpoint_backpressure_recovers_rejected_payload() {
    let mut world = World::new();
    world.insert_resource(NetworkClientOutbox::default());
    for index in 0..4_096usize {
        enqueue_client_outbox(&mut world, client_probe((index % 251) as u8))
            .expect("queue should accept messages through its configured capacity");
    }

    let rejected = client_probe(255);
    let error = enqueue_client_outbox(&mut world, rejected.clone())
        .expect_err("message beyond bounded capacity should be rejected");

    assert_eq!(error.capacity(), Some(4_096));
    assert_eq!(error.into_message(), rejected);
    assert_eq!(client_outbox_len(&world), 4_096);
}

#[test]
fn client_pending_resources_are_distinct_from_processed_and_flushed_projections() {
    let mut app = App::headless();
    app.add_plugin(NetworkClientPlugin);
    let inbound = server_probe(10);
    let outbound = client_probe(11);

    enqueue_client_inbox(app.world_mut(), inbound.clone())
        .expect("client inbox enqueue should succeed");
    enqueue_client_outbox(app.world_mut(), outbound.clone())
        .expect("client outbox enqueue should succeed");

    assert!(!client_inbox_is_empty(app.world()));
    assert_eq!(client_outbox_len(app.world()), 1);
    assert!(
        app.world()
            .resource::<engine::plugins::net::NetworkInboundQueue>()
            .unwrap()
            .server_messages()
            .is_empty()
    );
    assert!(
        app.world()
            .resource::<NetworkOutboundQueue>()
            .unwrap()
            .client_messages()
            .is_empty()
    );

    let app = app
        .run_for_frames(1)
        .expect("client network frame should process pending endpoint state");

    assert!(client_inbox_is_empty(app.world()));
    assert_eq!(client_outbox_len(app.world()), 0);
    assert_eq!(
        app.world()
            .resource::<engine::plugins::net::NetworkInboundQueue>()
            .unwrap()
            .server_messages(),
        &[inbound]
    );
    assert_eq!(
        app.world()
            .resource::<NetworkOutboundQueue>()
            .unwrap()
            .client_messages(),
        &[outbound]
    );
}

#[test]
fn server_pending_resources_are_distinct_from_processed_and_flushed_projections() {
    let mut app = App::headless();
    app.add_plugin(NetworkServerPlugin);
    let inbound = client_probe(12);
    let outbound = server_probe(13);

    enqueue_server_inbox(app.world_mut(), inbound.clone())
        .expect("server inbox enqueue should succeed");
    enqueue_server_outbox_broadcast(app.world_mut(), outbound.clone())
        .expect("server outbox enqueue should succeed");

    assert!(!server_inbox_is_empty(app.world()));
    assert_eq!(server_outbox_len(app.world()), 1);
    assert!(
        app.world()
            .resource::<engine::plugins::net::NetworkInboundQueue>()
            .unwrap()
            .client_messages()
            .is_empty()
    );
    assert!(
        app.world()
            .resource::<NetworkOutboundQueue>()
            .unwrap()
            .server_messages()
            .is_empty()
    );

    let app = app
        .run_for_frames(1)
        .expect("server network frame should process pending endpoint state");

    assert!(server_inbox_is_empty(app.world()));
    assert_eq!(server_outbox_len(app.world()), 0);
    let inbound_projection = app
        .world()
        .resource::<engine::plugins::net::NetworkInboundQueue>()
        .unwrap();
    assert_eq!(inbound_projection.client_messages().len(), 1);
    assert_eq!(inbound_projection.client_messages()[0].message, inbound);
    assert_eq!(
        app.world()
            .resource::<NetworkOutboundQueue>()
            .unwrap()
            .server_messages(),
        &[OutboundServerMessage::Broadcast(outbound)]
    );
}

#[test]
fn network_client_plugin_drains_server_messages_and_flushes_client_messages() {
    let mut app = App::headless();
    app.add_plugin(NetworkClientPlugin);
    enqueue_client_inbox(
        app.world_mut(),
        ServerMessage::TypedPayload(TypedPayloadMessage::new(
            "test/server",
            "ServerProbe",
            1,
            vec![1],
        )),
    )
    .expect("client inbox enqueue should succeed");
    enqueue_client_outbox(
        app.world_mut(),
        ClientMessage::TypedPayload(TypedPayloadMessage::new(
            "test/client",
            "ClientProbe",
            1,
            vec![2],
        )),
    )
    .expect("client outbox enqueue should succeed");

    let app = app
        .run_for_frames(1)
        .expect("client network frame should run");

    let diagnostics = app.world().resource::<NetworkDiagnostics>().unwrap();
    assert_eq!(diagnostics.processed_server_messages_last_frame, 1);
    assert_eq!(diagnostics.flushed_client_messages_last_frame, 1);
    assert_eq!(diagnostics.flush_count, 1);
    assert!(client_inbox_is_empty(app.world()));
    assert_eq!(client_outbox_len(app.world()), 0);
}

#[test]
fn network_server_plugin_drains_client_messages_and_flushes_server_messages() {
    let mut app = App::headless();
    app.add_plugin(NetworkServerPlugin);
    enqueue_server_inbox(
        app.world_mut(),
        ClientMessage::TypedPayload(TypedPayloadMessage::new(
            "test/client",
            "ClientProbe",
            1,
            vec![3],
        )),
    )
    .expect("server inbox enqueue should succeed");
    enqueue_server_outbox_broadcast(
        app.world_mut(),
        ServerMessage::TypedPayload(TypedPayloadMessage::new(
            "test/server",
            "ServerProbe",
            1,
            vec![4],
        )),
    )
    .expect("server outbox enqueue should succeed");

    let app = app
        .run_for_frames(1)
        .expect("server network frame should run");

    let diagnostics = app.world().resource::<NetworkDiagnostics>().unwrap();
    assert_eq!(diagnostics.processed_client_messages_last_frame, 1);
    assert_eq!(diagnostics.flushed_server_messages_last_frame, 1);
    assert_eq!(diagnostics.flush_count, 1);
    assert!(server_inbox_is_empty(app.world()));
    assert_eq!(server_outbox_len(app.world()), 0);
}

#[test]
fn replication_and_prediction_plugins_run_on_fixed_update() {
    let mut app = App::headless();
    app.add_plugin(NetworkServerPlugin);
    app.world_mut()
        .resource_mut::<PlayerCommandBuffer>()
        .unwrap()
        .push(ClientCommandEnvelope::Ability(AbilityCommand { slot: 2 }));

    let app = app.run_for_ticks(2).expect("fixed ticks should run");

    let replication = app.world().resource::<ReplicationDiagnostics>().unwrap();
    assert_eq!(replication.fixed_steps_observed, 2);
    assert_eq!(replication.last_snapshot_cursor, 2);
    assert_eq!(app.world().resource::<SnapshotCursor>().unwrap().0, 2);

    let prediction = app.world().resource::<PredictionDiagnostics>().unwrap();
    assert_eq!(prediction.fixed_steps_observed, 2);
    assert_eq!(prediction.commands_applied, 1);
    assert!(
        app.world()
            .resource::<PlayerCommandBuffer>()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn runennet_admission_drives_engine_session_projection_and_owner_routing() {
    let mut app = App::headless();
    app.add_plugin(NetworkServerPlugin);
    let connection = ConnectionHandle::new(7);
    let participant = ParticipantId::new(3);

    install_runennet_connections(&mut app, &[(connection, participant)]);

    let projection = app.world().resource::<RunenNetSessionProjection>().unwrap();
    assert_eq!(
        projection.participant_for_connection(connection),
        Some(participant)
    );
    let status = app.world().resource::<NetworkSessionStatus>().unwrap();
    assert!(status.connected);
    assert_eq!(status.active_connection_count, 1);
    let routing = app.world().resource::<NetworkOwnerRouting>().unwrap();
    assert!(routing.by_connection.contains_key(&connection));
    let diagnostics = app.world().resource::<NetworkDiagnostics>().unwrap();
    assert_eq!(diagnostics.accepted_connections, 1);
}

#[test]
fn reconnect_attempt_is_host_policy_not_session_authority() {
    let mut app = App::headless();
    app.add_plugin(NetworkClientPlugin);

    record_reconnect_attempt(app.world_mut(), 2);

    let status = app.world().resource::<NetworkSessionStatus>().unwrap();
    assert!(!status.connected);
    assert_eq!(status.active_connection_count, 0);
    assert_eq!(status.reconnect_attempt, Some(2));
    let projection = app.world().resource::<RunenNetSessionProjection>().unwrap();
    assert_eq!(projection.active_connection_count(), 0);
    let diagnostics = app.world().resource::<NetworkDiagnostics>().unwrap();
    assert_eq!(diagnostics.reconnect_attempts, 1);
}

#[test]
fn host_plugin_composes_client_and_server_runtime_roles() {
    let mut app = App::headless();
    app.add_plugin(NetworkHostPlugin);

    let profile = app.world().resource::<SimulationProfileConfig>().unwrap();
    assert_eq!(profile.authority, AuthorityRole::Peer);
    assert_eq!(profile.profile, SimulationProfile::DedicatedAuthority);

    assert!(app.world().resource::<NetworkClientInbox>().is_ok());
    assert!(app.world().resource::<NetworkClientOutbox>().is_ok());
    assert!(app.world().resource::<NetworkServerInbox>().is_ok());
    assert!(app.world().resource::<NetworkServerOutbox>().is_ok());
}
