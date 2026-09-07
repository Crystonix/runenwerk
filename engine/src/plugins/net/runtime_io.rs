use super::*;
use crate::WorldMut;
use anyhow::Context;
use ecs::{OwnerRole, World};
use engine_net::replication::{InputDriver, ReplicationDriver, SnapshotApplyDriver};
use engine_net::*;
use engine_sim::SimulationTick;
use runen_net::identity::ConnectionHandle;
use std::collections::HashSet;
use world_ops::SyncCursor;

// engine/src/plugins/net/runtime_io.rs

pub fn map_driver_error<TDriver>(error: TDriver::Error, context: &'static str) -> anyhow::Error
where
    TDriver: ReplicationDriver,
{
    anyhow::Error::new(error).context(context)
}

pub fn client_receive_system<TDriver>(mut world: WorldMut) -> anyhow::Result<()>
where
    TDriver: ReplicationDriver + SnapshotApplyDriver + InputDriver + Send + Sync + 'static,
    TDriver::Snapshot: Clone + PartialEq,
    TDriver::Input: Clone + PartialEq,
{
    let messages = drain_client_inbox(&mut world);
    if messages.is_empty() {
        return Ok(());
    }

    world
        .resource::<NetworkClientOutbox>()
        .context("NetworkClientOutbox should be installed by the client network role")?;

    if let Ok(diagnostics) = world.resource_mut::<NetworkDiagnostics>() {
        diagnostics.processed_server_messages_last_frame = messages.len();
    }

    if let Ok(inbound) = world.resource_mut::<NetworkInboundQueue>() {
        inbound.clear();
        for message in &messages {
            inbound.push_server(message.clone());
        }
    }

    for message in messages {
        match message {
            ServerMessage::Snapshot(snapshot) => {
                let result = apply_authoritative_snapshot::<TDriver>(
                    &mut world,
                    snapshot.tick,
                    snapshot.cursor,
                    None,
                    &snapshot.payload,
                )
                .with_context(|| {
                    format!(
                        "failed applying snapshot tick={} cursor={} payload_len={}",
                        snapshot.tick.0,
                        snapshot.cursor.0,
                        snapshot.payload.len()
                    )
                });

                match result {
                    Ok(corrected) => {
                        match enqueue_client_outbox(
                            &mut world,
                            ClientMessage::Ack(Ack {
                                cursor: snapshot.cursor,
                                last_received_tick: snapshot.tick,
                            }),
                        ) {
                            Ok(()) => {}
                            Err(NetworkPendingEnqueueError::Unavailable { endpoint, .. }) => {
                                anyhow::bail!("{endpoint} should be installed by NetPlugin");
                            }
                            Err(NetworkPendingEnqueueError::Backpressure { capacity, .. }) => {
                                tracing::warn!(capacity, "failed to enqueue snapshot ack");
                            }
                        }
                        if corrected
                            && let Ok(diagnostics) = world.resource_mut::<PredictionDiagnostics>()
                        {
                            diagnostics.corrected = diagnostics.corrected.saturating_add(1);
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            error = %format!("{error:#}"),
                            "network snapshot apply failed"
                        );
                    }
                }
            }
            ServerMessage::DeltaSnapshot(snapshot) => {
                let result = apply_authoritative_delta::<TDriver>(
                    &mut world,
                    snapshot.tick,
                    snapshot.base,
                    snapshot.cursor,
                    &snapshot.payload,
                )
                .with_context(|| {
                    format!(
                        "failed applying delta snapshot tick={} cursor={} payload_len={}",
                        snapshot.tick.0,
                        snapshot.cursor.0,
                        snapshot.payload.len()
                    )
                });

                match result {
                    Ok(corrected) => {
                        match enqueue_client_outbox(
                            &mut world,
                            ClientMessage::Ack(Ack {
                                cursor: snapshot.cursor,
                                last_received_tick: snapshot.tick,
                            }),
                        ) {
                            Ok(()) => {}
                            Err(NetworkPendingEnqueueError::Unavailable { endpoint, .. }) => {
                                anyhow::bail!("{endpoint} should be installed by NetPlugin");
                            }
                            Err(NetworkPendingEnqueueError::Backpressure { capacity, .. }) => {
                                tracing::warn!(capacity, "failed to enqueue delta snapshot ack");
                            }
                        }
                        if corrected
                            && let Ok(diagnostics) = world.resource_mut::<PredictionDiagnostics>()
                        {
                            diagnostics.corrected = diagnostics.corrected.saturating_add(1);
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            error = %format!("{error:#}"),
                            "network delta snapshot apply failed"
                        );
                    }
                }
            }
            ServerMessage::RunEvent(_)
            | ServerMessage::RunResult(_)
            | ServerMessage::TypedPayload(_) => {}
        }
    }

    sync_net_diagnostics_view(&mut world);
    Ok(())
}

fn connection_is_admitted(world: &World, connection: ConnectionHandle) -> bool {
    world
        .resource::<RunenNetSessionProjection>()
        .ok()
        .and_then(|projection| projection.participant_for_connection(connection))
        .is_some()
}

pub fn server_receive_system<TDriver>(mut world: WorldMut) -> anyhow::Result<()>
where
    TDriver: ReplicationDriver + InputDriver + Send + Sync + 'static,
    TDriver::Snapshot: Clone + PartialEq,
    TDriver::Input: Clone + PartialEq,
{
    let messages = drain_server_inbox(&mut world);
    if messages.is_empty() {
        return Ok(());
    }

    if let Ok(diagnostics) = world.resource_mut::<NetworkDiagnostics>() {
        diagnostics.processed_client_messages_last_frame = messages.len();
    }

    if let Ok(inbound) = world.resource_mut::<NetworkInboundQueue>() {
        inbound.clear();
        for incoming in &messages {
            inbound.push_client(incoming.connection, incoming.message.clone());
        }
    }

    for incoming in messages {
        let connection = incoming.connection;
        let message = incoming.message;

        if matches!(
            message,
            ClientMessage::Ack(_) | ClientMessage::InputFrame(_)
        ) {
            let Some(connection) = connection else {
                tracing::warn!("ignoring replication input without a RunenNet connection handle");
                continue;
            };
            if !connection_is_admitted(&world, connection) {
                tracing::warn!(
                    connection = connection.get(),
                    "ignoring replication input from a connection not admitted by RunenNet session"
                );
                continue;
            }
        }

        if let ClientMessage::Ack(ack) = &message
            && let Some(connection) = connection
        {
            let ack_outcome = if let Ok(state) =
                world.resource_mut::<ServerSnapshotReplicationState<TDriver::Snapshot>>()
            {
                let baseline_available = state
                    .snapshot_history_per_connection
                    .get(&connection)
                    .is_some_and(|history| history.contains_key(&ack.cursor));
                let checkpoint = state.checkpoints.entry(connection).or_default();
                checkpoint.mark_snapshot_acknowledged(ack.cursor, baseline_available)
            } else {
                SnapshotAckOutcome::Rejected {
                    cursor: ack.cursor,
                    reason: SnapshotAckRejection::UnsentCursor,
                }
            };

            match ack_outcome {
                SnapshotAckOutcome::Accepted { .. } => {
                    if let Ok(streaming_state) = world.resource_mut::<NetStreamingStateResource>() {
                        streaming_state
                            .mark_snapshot_acknowledged(connection, SyncCursor(ack.cursor.0));
                    }
                    if let Ok(diagnostics) = world.resource_mut::<ReplicationDiagnostics>() {
                        diagnostics.acked = diagnostics.acked.saturating_add(1);
                    }
                }
                SnapshotAckOutcome::Rejected { .. } => {
                    if let Ok(diagnostics) = world.resource_mut::<ReplicationDiagnostics>() {
                        diagnostics.rejected_acks = diagnostics.rejected_acks.saturating_add(1);
                    }
                }
            }
        }

        if let ClientMessage::InputFrame(frame) = &message
            && let Some(connection) = connection
        {
            world
                .resource::<NetworkInputStaging<TDriver::Input>>()
                .context("NetworkInputStaging should be installed by NetPlugin")?;

            let decoded = TDriver::decode_input(&frame.payload)
                .map_err(|error| map_driver_error::<TDriver>(error, "decode remote input"))?;
            let _ = ensure_owner_for_connection(&mut world, connection, OwnerRole::Active);

            let current_tick = world
                .resource::<SimulationTick>()
                .copied()
                .unwrap_or_default();
            let mut lagged = 0u64;
            {
                let staging = world
                    .resource_mut::<NetworkInputStaging<TDriver::Input>>()
                    .context("NetworkInputStaging should be installed by NetPlugin")?;
                for command in decoded {
                    if frame.tick <= current_tick {
                        lagged = lagged.saturating_add(1);
                        continue;
                    }

                    if let Err(NetworkInputStageError::Backpressure { capacity, .. }) =
                        staging.stage(frame.tick, command)
                    {
                        tracing::warn!(
                            capacity,
                            tick = frame.tick.0,
                            "network input staging backpressure; rejecting remote input"
                        );
                    }
                }
            }
            if lagged > 0
                && let Ok(diagnostics) = world.resource_mut::<ReplicationDiagnostics>()
            {
                diagnostics.lagged = diagnostics.lagged.saturating_add(lagged);
            }
        }
    }

    sync_net_diagnostics_view(&mut world);
    Ok(())
}

pub fn record_reconnect_attempt(world: &mut World, attempt: u32) {
    if let Ok(status) = world.resource_mut::<NetworkSessionStatus>() {
        status.reconnect_attempt = Some(attempt);
    }
    if let Ok(health) = world.resource_mut::<ConnectionHealth>() {
        health.reconnect_events = health.reconnect_events.saturating_add(1);
    }
    if let Ok(diagnostics) = world.resource_mut::<NetworkDiagnostics>() {
        diagnostics.reconnect_attempts = diagnostics.reconnect_attempts.saturating_add(1);
    }
}

pub fn record_network_error(world: &mut World, message: impl Into<String>) {
    if let Ok(status) = world.resource_mut::<NetworkSessionStatus>() {
        status.last_error = Some(message.into());
    }
    if let Ok(health) = world.resource_mut::<ConnectionHealth>() {
        health.error_events = health.error_events.saturating_add(1);
    }
}

/// Reconcile engine-owned routing/status projections from RunenNet-authorized session bindings.
///
/// This function never decides admission or loss. The authoritative input is
/// [`RunenNetSessionProjection`], which is itself updated only after accepted RunenNet Core
/// lifecycle operations. Engine owner routing and diagnostics are derived outputs.
pub fn sync_runennet_session_projection(world: &mut World) {
    let mut active_connections = world
        .resource::<RunenNetSessionProjection>()
        .map(|projection| projection.active_connections().collect::<Vec<_>>())
        .unwrap_or_default();
    active_connections.sort_by_key(|connection| connection.get());
    let active_set = active_connections.iter().copied().collect::<HashSet<_>>();

    let existing_connections = world
        .resource::<NetworkOwnerRouting>()
        .map(|routing| routing.by_connection.keys().copied().collect::<Vec<_>>())
        .unwrap_or_default();
    let existing_set = existing_connections.iter().copied().collect::<HashSet<_>>();

    let newly_active = active_connections
        .iter()
        .filter(|connection| !existing_set.contains(connection))
        .count();
    let mut stale_connections = existing_connections
        .into_iter()
        .filter(|connection| !active_set.contains(connection))
        .collect::<Vec<_>>();
    stale_connections.sort_by_key(|connection| connection.get());

    for connection in stale_connections.iter().copied() {
        if let Some(owner) = remove_owner_for_connection(world, connection) {
            let _ = world.transfer_owned_targets_to_world(owner);
        }
    }
    for connection in active_connections.iter().copied() {
        let _ = ensure_owner_for_connection(world, connection, OwnerRole::Active);
    }

    let active_connection_count = active_connections.len();
    let connected = active_connection_count > 0;
    if let Ok(status) = world.resource_mut::<NetworkSessionStatus>() {
        status.connected = connected;
        status.active_connection_count = active_connection_count;
    }
    if let Ok(health) = world.resource_mut::<ConnectionHealth>() {
        health.connected = connected;
        health.close_events = health
            .close_events
            .saturating_add(stale_connections.len() as u64);
    }
    if newly_active > 0
        && let Ok(diagnostics) = world.resource_mut::<NetworkDiagnostics>()
    {
        diagnostics.accepted_connections = diagnostics
            .accepted_connections
            .saturating_add(newly_active as u64);
    }
}

pub fn sync_runennet_session_projection_system(mut world: WorldMut) {
    sync_runennet_session_projection(&mut world);
}

fn sync_net_diagnostics_view(world: &mut World) {
    sync_runennet_session_projection(world);

    let status = world.resource::<NetworkSessionStatus>().ok().cloned();
    let health = world.resource::<ConnectionHealth>().ok().cloned();
    let round_trip = world.resource::<RoundTripMetrics>().ok().copied();
    let network = world.resource::<NetworkDiagnostics>().ok().copied();
    let replication = world.resource::<ReplicationDiagnostics>().ok().copied();
    let prediction = world.resource::<PredictionDiagnostics>().ok().copied();

    if let Ok(view) = world.resource_mut::<NetDiagnosticsView>() {
        if let Some(status) = status {
            view.connected = status.connected;
            view.active_connection_count = status.active_connection_count;
        }
        if let Some(health) = health {
            view.close_events = health.close_events;
            view.error_events = health.error_events;
            view.reconnect_events = health.reconnect_events;
        }
        if let Some(round_trip) = round_trip {
            view.last_rtt_millis = round_trip.last_rtt_millis;
        }
        if let Some(network) = network {
            view.accepted_connections = network.accepted_connections;
            view.rejected_connections = network.rejected_connections;
            view.reconnect_attempts = network.reconnect_attempts;
        }
        if let Some(replication) = replication {
            view.emitted_snapshots = replication.emitted_snapshots;
            view.applied_snapshots = replication.applied_snapshots;
            view.acked_snapshots = replication.acked;
            view.lagged_inputs = replication.lagged;
        }
        if let Some(prediction) = prediction {
            view.corrected_predictions = prediction.corrected;
        }
    }
}

pub fn sync_net_diagnostics_view_system(mut world: WorldMut) {
    sync_net_diagnostics_view(&mut world);
}

pub fn client_flush_system(mut world: WorldMut) -> anyhow::Result<()> {
    world
        .resource::<NetworkClientOutbox>()
        .context("NetworkClientOutbox should be installed by the client network role")?;
    let messages = drain_client_outbox(&mut world);
    if messages.is_empty() {
        return Ok(());
    }

    if let Ok(diagnostics) = world.resource_mut::<NetworkDiagnostics>() {
        diagnostics.flushed_client_messages_last_frame = messages.len();
        diagnostics.flush_count = diagnostics.flush_count.saturating_add(1);
    }

    if let Ok(queue) = world.resource_mut::<NetworkOutboundQueue>() {
        queue.clear();
        for message in messages {
            queue.push_client(message);
        }
    }

    Ok(())
}

pub fn server_flush_system(mut world: WorldMut) -> anyhow::Result<()> {
    world
        .resource::<NetworkServerOutbox>()
        .context("NetworkServerOutbox should be installed by the server network role")?;
    let messages = drain_server_outbox(&mut world);
    if messages.is_empty() {
        return Ok(());
    }

    if let Ok(diagnostics) = world.resource_mut::<NetworkDiagnostics>() {
        diagnostics.flushed_server_messages_last_frame = messages.len();
        diagnostics.flush_count = diagnostics.flush_count.saturating_add(1);
    }

    if let Ok(queue) = world.resource_mut::<NetworkOutboundQueue>() {
        queue.clear();
        for message in messages {
            queue.push_server(message);
        }
    }

    Ok(())
}