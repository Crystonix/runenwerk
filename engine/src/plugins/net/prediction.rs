use super::*;
use crate::WorldMut;
use anyhow::Context;
use ecs::{OwnerRole, World};
use engine_net::replication::{InputDriver, ReplicationDriver, SnapshotApplyDriver};
use engine_net::*;
use engine_sim::{AuthorityRole, SimulationProfileConfig, SimulationTick};
use runen_net::identity::ConnectionHandle;
use world_ops::SyncCursor;

// engine/src/plugins/net/prediction.rs

const FULL_SNAPSHOT_INTERVAL_TICKS: u64 = 30;
const MAX_SERVER_SNAPSHOT_HISTORY: usize = 256;
const MAX_CLIENT_SNAPSHOT_HISTORY: usize = 256;

fn active_connections(world: &World) -> Vec<ConnectionHandle> {
    let mut connections = world
        .resource::<RunenNetSessionProjection>()
        .map(|projection| projection.active_connections().collect::<Vec<_>>())
        .unwrap_or_default();
    connections.sort_by_key(|connection| connection.get());
    connections
}

fn sole_active_connection(world: &World) -> Option<ConnectionHandle> {
    let projection = world.resource::<RunenNetSessionProjection>().ok()?;
    let mut connections = projection.active_connections();
    let first = connections.next()?;
    connections.next().is_none().then_some(first)
}

pub fn replication_step_system<TDriver>(mut world: WorldMut) -> anyhow::Result<()>
where
    TDriver: ReplicationDriver + Send + Sync + 'static,
    TDriver::Snapshot: Clone + PartialEq,
{
    let authority = world
        .resource::<SimulationProfileConfig>()
        .map(|config| config.authority)
        .unwrap_or(AuthorityRole::Local);

    if matches!(authority, AuthorityRole::Server | AuthorityRole::Peer) {
        world
            .resource::<NetworkServerOutbox>()
            .context("NetworkServerOutbox should be installed by the server network role")?;
    }

    if let Ok(diagnostics) = world.resource_mut::<ReplicationDiagnostics>() {
        diagnostics.fixed_steps_observed = diagnostics.fixed_steps_observed.saturating_add(1);
    }

    if !matches!(authority, AuthorityRole::Server | AuthorityRole::Peer) {
        let cursor = world
            .resource::<SnapshotCursor>()
            .map(|cursor| cursor.0)
            .unwrap_or(0);

        if let Ok(diagnostics) = world.resource_mut::<ReplicationDiagnostics>() {
            diagnostics.last_snapshot_cursor = cursor;
        }
        return Ok(());
    }

    let tick = world
        .resource::<SimulationTick>()
        .copied()
        .unwrap_or_default();
    let cursor = {
        let cursor = world.resource_mut::<SnapshotCursor>()?;
        cursor.0 = cursor.0.saturating_add(1);
        *cursor
    };

    let active_connections = active_connections(&world);

    let mut outbound = Vec::<OutboundServerMessage>::new();
    let mut world_streaming_updates = Vec::<(ConnectionHandle, SyncCursor, bool)>::new();
    if !active_connections.is_empty() {
        let mut snapshots_for_connections = Vec::<(ConnectionHandle, TDriver::Snapshot)>::new();
        for connection in &active_connections {
            let captured_snapshot = TDriver::capture_snapshot_for_connection(&world, *connection)
                .map_err(|error| {
                map_driver_error::<TDriver>(error, "capture snapshot for connection")
            })?;
            if let Some(snapshot) = captured_snapshot {
                snapshots_for_connections.push((*connection, snapshot));
            }
        }

        let state = world.resource_mut::<ServerSnapshotReplicationState<TDriver::Snapshot>>()?;
        state.latest_tick = tick;
        state
            .checkpoints
            .retain(|connection, _| active_connections.contains(connection));
        state
            .snapshot_history_per_connection
            .retain(|connection, _| active_connections.contains(connection));
        state
            .latest_snapshot_per_connection
            .retain(|connection, _| active_connections.contains(connection));

        let mut first_snapshot_for_tick: Option<TDriver::Snapshot> = None;

        for (connection, snapshot) in snapshots_for_connections {
            if first_snapshot_for_tick.is_none() {
                first_snapshot_for_tick = Some(snapshot.clone());
            }

            state
                .latest_snapshot_per_connection
                .insert(connection, snapshot.clone());
            state
                .snapshot_history_per_connection
                .entry(connection)
                .or_default()
                .insert(cursor, snapshot.clone());
            prune_snapshot_history_for_connection(state, connection);

            let (last_ack_cursor, needs_full_resync) = {
                let checkpoint = state.checkpoints.entry(connection).or_default();
                (checkpoint.last_ack_cursor, checkpoint.needs_full_resync)
            };

            let scheduled_full = cursor.0 % FULL_SNAPSHOT_INTERVAL_TICKS == 0;
            let mut send_full = needs_full_resync || scheduled_full || last_ack_cursor.0 == 0;

            let message = if send_full {
                let payload = TDriver::encode_snapshot(&snapshot)
                    .map_err(|error| map_driver_error::<TDriver>(error, "encode snapshot"))?;
                ServerMessage::Snapshot(Snapshot {
                    tick,
                    cursor,
                    last_applied: last_ack_cursor,
                    entity_ids: Vec::new(),
                    payload,
                })
            } else if let Some(base_snapshot) = state
                .snapshot_history_per_connection
                .get(&connection)
                .and_then(|history| history.get(&last_ack_cursor))
            {
                let delta = TDriver::build_delta(base_snapshot, &snapshot);
                let payload = TDriver::encode_delta(&delta)
                    .map_err(|error| map_driver_error::<TDriver>(error, "encode delta"))?;
                ServerMessage::DeltaSnapshot(DeltaSnapshot {
                    tick,
                    base: last_ack_cursor,
                    cursor,
                    entity_ids: Vec::new(),
                    payload,
                })
            } else {
                send_full = true;
                let payload = TDriver::encode_snapshot(&snapshot).map_err(|error| {
                    map_driver_error::<TDriver>(error, "encode fallback snapshot")
                })?;
                ServerMessage::Snapshot(Snapshot {
                    tick,
                    cursor,
                    last_applied: last_ack_cursor,
                    entity_ids: Vec::new(),
                    payload,
                })
            };

            state
                .checkpoints
                .entry(connection)
                .or_default()
                .mark_snapshot_sent(cursor, tick, send_full);
            world_streaming_updates.push((connection, SyncCursor(cursor.0), send_full));

            outbound.push(OutboundServerMessage::ToConnection {
                connection,
                message,
            });
        }

        if let Some(snapshot) = first_snapshot_for_tick {
            state.latest_snapshot = Some(snapshot.clone());
            state.snapshot_history.insert(cursor, snapshot);
            prune_snapshot_history(state);
        }
    }

    if !world_streaming_updates.is_empty()
        && let Ok(streaming_state) = world.resource_mut::<NetStreamingStateResource>()
    {
        for (connection, cursor, sent_full_snapshot) in world_streaming_updates {
            streaming_state.mark_snapshot_sent(connection, cursor, sent_full_snapshot);
        }
    }

    for message in &outbound {
        match enqueue_server_outbox(&mut world, message.clone()) {
            Ok(()) => {}
            Err(NetworkPendingEnqueueError::Unavailable { endpoint, .. }) => {
                anyhow::bail!("{endpoint} should be installed by NetPlugin");
            }
            Err(NetworkPendingEnqueueError::Backpressure { capacity, .. }) => {
                tracing::warn!(
                    capacity,
                    "failed to enqueue replication server outbox message"
                );
            }
        }
    }

    if let Ok(diagnostics) = world.resource_mut::<ReplicationDiagnostics>() {
        diagnostics.last_snapshot_cursor = cursor.0;
        diagnostics.emitted_snapshots = diagnostics
            .emitted_snapshots
            .saturating_add(outbound.len() as u64);
    }

    Ok(())
}

pub fn prediction_step_system<TDriver>(mut world: WorldMut) -> anyhow::Result<()>
where
    TDriver: ReplicationDriver + InputDriver + Send + Sync + 'static,
    TDriver::Input: Clone + PartialEq,
{
    world
        .resource::<NetworkInputStaging<TDriver::Input>>()
        .context("NetworkInputStaging should be installed by NetPlugin")?;

    let authority = world
        .resource::<SimulationProfileConfig>()
        .map(|config| config.authority)
        .unwrap_or(AuthorityRole::Local);
    if matches!(authority, AuthorityRole::Client | AuthorityRole::Peer) {
        world
            .resource::<NetworkClientOutbox>()
            .context("NetworkClientOutbox should be installed by the client network role")?;
    }

    if let Ok(diagnostics) = world.resource_mut::<PredictionDiagnostics>() {
        diagnostics.fixed_steps_observed = diagnostics.fixed_steps_observed.saturating_add(1);
    }

    let tick = world
        .resource::<SimulationTick>()
        .copied()
        .unwrap_or_default();

    let commands = TDriver::take_local_input(&mut world)
        .map_err(|error| map_driver_error::<TDriver>(error, "take local input"))?;
    if !commands.is_empty() {
        if matches!(authority, AuthorityRole::Client | AuthorityRole::Peer)
            && let Some(connection) = sole_active_connection(&world)
        {
            let _ = ensure_owner_for_connection(&mut world, connection, OwnerRole::Active);
        }

        {
            let staging = world.resource_mut::<NetworkInputStaging<TDriver::Input>>()?;
            for command in &commands {
                if let Err(NetworkInputStageError::Backpressure { capacity, .. }) =
                    staging.stage(tick, command.clone())
                {
                    tracing::warn!(
                        capacity,
                        tick = tick.0,
                        "network input staging backpressure; rejecting local input"
                    );
                }
            }
        }
    }

    if matches!(authority, AuthorityRole::Client | AuthorityRole::Peer) && !commands.is_empty() {
        let payload = TDriver::encode_input(&commands)
            .map_err(|error| map_driver_error::<TDriver>(error, "encode input"))?;

        match enqueue_client_outbox(
            &mut world,
            ClientMessage::InputFrame(InputFrame { tick, payload }),
        ) {
            Ok(()) => {}
            Err(NetworkPendingEnqueueError::Unavailable { endpoint, .. }) => {
                anyhow::bail!("{endpoint} should be installed by NetPlugin");
            }
            Err(NetworkPendingEnqueueError::Backpressure { capacity, .. }) => {
                tracing::warn!(capacity, "failed to enqueue local input frame");
            }
        }

        if let Ok(prediction) = world.resource_mut::<PredictionState<TDriver::Input>>() {
            prediction.pending_frames.push(PendingInputFrame {
                tick,
                commands: commands.clone(),
            });
        }
    }

    let inputs_to_apply = world
        .resource_mut::<NetworkInputStaging<TDriver::Input>>()?
        .drain_tick(tick);
    if inputs_to_apply.is_empty() {
        return Ok(());
    }

    if let Ok(diagnostics) = world.resource_mut::<PredictionDiagnostics>() {
        diagnostics.commands_applied = diagnostics
            .commands_applied
            .saturating_add(inputs_to_apply.len() as u64);
    }

    TDriver::apply_input(&mut world, &inputs_to_apply)
        .map_err(|error| map_driver_error::<TDriver>(error, "apply streamed input"))?;

    Ok(())
}

pub fn apply_authoritative_snapshot<TDriver>(
    world: &mut World,
    tick: SimulationTick,
    cursor: SnapshotCursor,
    snapshot: Option<TDriver::Snapshot>,
    payload: &[u8],
) -> anyhow::Result<bool>
where
    TDriver: ReplicationDriver + SnapshotApplyDriver + InputDriver + Send + Sync + 'static,
    TDriver::Snapshot: Clone + PartialEq,
    TDriver::Input: Clone + PartialEq,
{
    let snapshot = match snapshot {
        Some(snapshot) => snapshot,
        None => TDriver::decode_snapshot(payload)
            .map_err(|error| map_driver_error::<TDriver>(error, "decode snapshot"))?,
    };

    let corrected = TDriver::apply_snapshot(world, tick, snapshot.clone())
        .map_err(|error| map_driver_error::<TDriver>(error, "apply snapshot"))?;

    if let Ok(tick_resource) = world.resource_mut::<SimulationTick>() {
        *tick_resource = tick;
    }

    if let Ok(state) = world.resource_mut::<ClientSnapshotReplicationState<TDriver::Snapshot>>() {
        state.last_acknowledged_cursor = cursor;
        state.last_received_tick = tick;
        state.applied_snapshots = state.applied_snapshots.saturating_add(1);
        state.last_received_snapshot = Some(snapshot.clone());
        state.snapshot_history.insert(cursor, snapshot);
        prune_client_snapshot_history(state);
    }

    if let Ok(diagnostics) = world.resource_mut::<ReplicationDiagnostics>() {
        diagnostics.applied_snapshots = diagnostics.applied_snapshots.saturating_add(1);
    }

    replay_pending_prediction::<TDriver>(world, tick, "replay predicted input")?;
    Ok(corrected)
}

pub fn apply_authoritative_delta<TDriver>(
    world: &mut World,
    tick: SimulationTick,
    base: SnapshotCursor,
    cursor: SnapshotCursor,
    payload: &[u8],
) -> anyhow::Result<bool>
where
    TDriver: ReplicationDriver + SnapshotApplyDriver + InputDriver + Send + Sync + 'static,
    TDriver::Snapshot: Clone + PartialEq,
    TDriver::Input: Clone + PartialEq,
{
    let delta = TDriver::decode_delta(payload)
        .map_err(|error| map_driver_error::<TDriver>(error, "decode delta"))?;

    let (expected_base, base_snapshot) = {
        let state = world.resource::<ClientSnapshotReplicationState<TDriver::Snapshot>>()?;
        let base_snapshot = state
            .snapshot_history
            .get(&base)
            .cloned()
            .or_else(|| {
                if state.last_acknowledged_cursor == base {
                    state.last_received_snapshot.clone()
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "received delta snapshot with unknown baseline cursor={}",
                    base.0
                )
            })?;
        (state.last_acknowledged_cursor, base_snapshot)
    };

    if base.0 > expected_base.0 {
        anyhow::bail!(
            "delta base cursor mismatch: expected {} got {}",
            expected_base.0,
            base.0
        );
    }

    let rebuilt_snapshot = TDriver::apply_delta_to_snapshot(&base_snapshot, &delta);
    let corrected = if base == expected_base {
        TDriver::apply_delta(world, tick, delta.clone())
            .map_err(|error| map_driver_error::<TDriver>(error, "apply authoritative delta"))?
    } else {
        TDriver::apply_snapshot(world, tick, rebuilt_snapshot.clone()).map_err(|error| {
            map_driver_error::<TDriver>(error, "apply delta via snapshot fallback")
        })?
    };

    if let Ok(tick_resource) = world.resource_mut::<SimulationTick>() {
        *tick_resource = tick;
    }

    if let Ok(state) = world.resource_mut::<ClientSnapshotReplicationState<TDriver::Snapshot>>() {
        state.last_acknowledged_cursor = cursor;
        state.last_received_tick = tick;
        state.applied_snapshots = state.applied_snapshots.saturating_add(1);
        state.last_received_snapshot = Some(rebuilt_snapshot.clone());
        state.snapshot_history.insert(cursor, rebuilt_snapshot);
        prune_client_snapshot_history(state);
    }

    if let Ok(diagnostics) = world.resource_mut::<ReplicationDiagnostics>() {
        diagnostics.applied_snapshots = diagnostics.applied_snapshots.saturating_add(1);
    }

    replay_pending_prediction::<TDriver>(world, tick, "replay predicted input after delta")?;
    Ok(corrected)
}

fn replay_pending_prediction<TDriver>(
    world: &mut World,
    tick: SimulationTick,
    error_context: &'static str,
) -> anyhow::Result<()>
where
    TDriver: ReplicationDriver + InputDriver + Send + Sync + 'static,
    TDriver::Input: Clone + PartialEq,
{
    let pending_frames = {
        let prediction = world.resource_mut::<PredictionState<TDriver::Input>>()?;
        prediction
            .pending_frames
            .retain(|frame| frame.tick.0 > tick.0);
        prediction.pending_frames.clone()
    };

    let mut replayed_commands = 0u64;
    for frame in pending_frames {
        replayed_commands = replayed_commands.saturating_add(frame.commands.len() as u64);
        TDriver::apply_input(world, &frame.commands)
            .map_err(|error| map_driver_error::<TDriver>(error, error_context))?;
    }
    if replayed_commands > 0
        && let Ok(diagnostics) = world.resource_mut::<PredictionDiagnostics>()
    {
        diagnostics.replayed = diagnostics.replayed.saturating_add(replayed_commands);
    }

    Ok(())
}

fn prune_snapshot_history<TSnapshot>(state: &mut ServerSnapshotReplicationState<TSnapshot>)
where
    TSnapshot: Clone + PartialEq,
{
    while state.snapshot_history.len() > MAX_SERVER_SNAPSHOT_HISTORY {
        let Some(oldest_cursor) = state.snapshot_history.keys().next().copied() else {
            break;
        };
        state.snapshot_history.remove(&oldest_cursor);
    }
}

fn prune_snapshot_history_for_connection<TSnapshot>(
    state: &mut ServerSnapshotReplicationState<TSnapshot>,
    connection: ConnectionHandle,
) where
    TSnapshot: Clone + PartialEq,
{
    let Some(history) = state.snapshot_history_per_connection.get_mut(&connection) else {
        return;
    };
    while history.len() > MAX_SERVER_SNAPSHOT_HISTORY {
        let Some(oldest_cursor) = history.keys().next().copied() else {
            break;
        };
        history.remove(&oldest_cursor);
    }
}

fn prune_client_snapshot_history<TSnapshot>(state: &mut ClientSnapshotReplicationState<TSnapshot>)
where
    TSnapshot: Clone + PartialEq,
{
    while state.snapshot_history.len() > MAX_CLIENT_SNAPSHOT_HISTORY {
        let Some(oldest_cursor) = state.snapshot_history.keys().next().copied() else {
            break;
        };
        state.snapshot_history.remove(&oldest_cursor);
    }
}
