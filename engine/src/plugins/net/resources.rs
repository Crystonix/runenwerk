use super::*;
use crate::{App, CoreSet, FixedUpdate, FrameEnd, PreUpdate, SystemConfigExt};
use ecs::World;
use engine_net::replication::{InputDriver, ReplicationDriver, SnapshotApplyDriver};
use engine_net::*;
use engine_sim::SimulationTick;
use runen_net::identity::ConnectionHandle;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

// engine/src/plugins/net/resources.rs

const NETWORK_MESSAGE_QUEUE_CAPACITY: usize = 4_096;
const MAX_TRACKED_SENT_BASELINE_CURSORS: usize = 256;

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkPendingEnqueueError<T> {
    Unavailable { endpoint: &'static str, message: T },
    Backpressure { capacity: usize, message: T },
}

impl<T> NetworkPendingEnqueueError<T> {
    pub fn capacity(&self) -> Option<usize> {
        match self {
            Self::Unavailable { .. } => None,
            Self::Backpressure { capacity, .. } => Some(*capacity),
        }
    }

    pub fn into_message(self) -> T {
        match self {
            Self::Unavailable { message, .. } | Self::Backpressure { message, .. } => message,
        }
    }
}

#[derive(Debug, Clone)]
struct PendingNetworkQueue<T> {
    messages: VecDeque<T>,
    capacity: usize,
}

impl<T> Default for PendingNetworkQueue<T> {
    fn default() -> Self {
        Self {
            messages: VecDeque::new(),
            capacity: NETWORK_MESSAGE_QUEUE_CAPACITY,
        }
    }
}

impl<T> PendingNetworkQueue<T> {
    fn enqueue(&mut self, message: T) -> Result<(), NetworkPendingEnqueueError<T>> {
        if self.messages.len() >= self.capacity {
            return Err(NetworkPendingEnqueueError::Backpressure {
                capacity: self.capacity,
                message,
            });
        }
        self.messages.push_back(message);
        Ok(())
    }

    fn len(&self) -> usize {
        self.messages.len()
    }

    fn drain(&mut self) -> Vec<T> {
        self.messages.drain(..).collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutboundServerMessage {
    ToConnection {
        connection: ConnectionHandle,
        message: ServerMessage,
    },
    Broadcast(ServerMessage),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InboundClientMessage {
    pub connection: Option<ConnectionHandle>,
    pub message: ClientMessage,
}

#[derive(Debug, Clone, Default, ecs::Resource)]
pub struct NetworkClientInbox(PendingNetworkQueue<ServerMessage>);

#[derive(Debug, Clone, Default, ecs::Resource)]
pub struct NetworkServerInbox(PendingNetworkQueue<InboundClientMessage>);

#[derive(Debug, Clone, Default, ecs::Resource)]
pub struct NetworkClientOutbox(PendingNetworkQueue<ClientMessage>);

#[derive(Debug, Clone, Default, ecs::Resource)]
pub struct NetworkServerOutbox(PendingNetworkQueue<OutboundServerMessage>);

fn enqueue_pending<T>(
    queue: &mut PendingNetworkQueue<T>,
    queue_name: &'static str,
    message: T,
) -> Result<(), NetworkPendingEnqueueError<T>> {
    let result = queue.enqueue(message);
    if let Err(NetworkPendingEnqueueError::Backpressure { capacity, .. }) = &result {
        tracing::warn!(
            queue = queue_name,
            capacity = *capacity,
            "network queue backpressure; returning rejected message to caller"
        );
    }
    result
}

pub fn enqueue_client_inbox(
    world: &mut World,
    message: ServerMessage,
) -> Result<(), NetworkPendingEnqueueError<ServerMessage>> {
    let inbox = match world.resource_mut::<NetworkClientInbox>() {
        Ok(inbox) => inbox,
        Err(_) => {
            return Err(NetworkPendingEnqueueError::Unavailable {
                endpoint: "NetworkClientInbox",
                message,
            });
        }
    };
    enqueue_pending(&mut inbox.0, "NetworkClientInbox", message)
}

pub fn client_inbox_len(world: &World) -> usize {
    world
        .resource::<NetworkClientInbox>()
        .map(|inbox| inbox.0.len())
        .unwrap_or(0)
}

pub fn client_inbox_is_empty(world: &World) -> bool {
    client_inbox_len(world) == 0
}

pub fn drain_client_inbox(world: &mut World) -> Vec<ServerMessage> {
    world
        .resource_mut::<NetworkClientInbox>()
        .map(|inbox| inbox.0.drain())
        .unwrap_or_default()
}

pub fn enqueue_server_inbox(
    world: &mut World,
    message: ClientMessage,
) -> Result<(), NetworkPendingEnqueueError<InboundClientMessage>> {
    enqueue_server_inbox_from(world, None, message)
}

pub fn enqueue_server_inbox_from(
    world: &mut World,
    connection: Option<ConnectionHandle>,
    message: ClientMessage,
) -> Result<(), NetworkPendingEnqueueError<InboundClientMessage>> {
    let message = InboundClientMessage {
        connection,
        message,
    };
    let inbox = match world.resource_mut::<NetworkServerInbox>() {
        Ok(inbox) => inbox,
        Err(_) => {
            return Err(NetworkPendingEnqueueError::Unavailable {
                endpoint: "NetworkServerInbox",
                message,
            });
        }
    };
    enqueue_pending(&mut inbox.0, "NetworkServerInbox", message)
}

pub fn server_inbox_len(world: &World) -> usize {
    world
        .resource::<NetworkServerInbox>()
        .map(|inbox| inbox.0.len())
        .unwrap_or(0)
}

pub fn server_inbox_is_empty(world: &World) -> bool {
    server_inbox_len(world) == 0
}

pub fn drain_server_inbox(world: &mut World) -> Vec<InboundClientMessage> {
    world
        .resource_mut::<NetworkServerInbox>()
        .map(|inbox| inbox.0.drain())
        .unwrap_or_default()
}

pub fn enqueue_client_outbox(
    world: &mut World,
    message: ClientMessage,
) -> Result<(), NetworkPendingEnqueueError<ClientMessage>> {
    let outbox = match world.resource_mut::<NetworkClientOutbox>() {
        Ok(outbox) => outbox,
        Err(_) => {
            return Err(NetworkPendingEnqueueError::Unavailable {
                endpoint: "NetworkClientOutbox",
                message,
            });
        }
    };
    enqueue_pending(&mut outbox.0, "NetworkClientOutbox", message)
}

pub fn client_outbox_len(world: &World) -> usize {
    world
        .resource::<NetworkClientOutbox>()
        .map(|outbox| outbox.0.len())
        .unwrap_or(0)
}

pub fn client_outbox_is_empty(world: &World) -> bool {
    client_outbox_len(world) == 0
}

pub fn drain_client_outbox(world: &mut World) -> Vec<ClientMessage> {
    world
        .resource_mut::<NetworkClientOutbox>()
        .map(|outbox| outbox.0.drain())
        .unwrap_or_default()
}

pub fn enqueue_server_outbox(
    world: &mut World,
    message: OutboundServerMessage,
) -> Result<(), NetworkPendingEnqueueError<OutboundServerMessage>> {
    let outbox = match world.resource_mut::<NetworkServerOutbox>() {
        Ok(outbox) => outbox,
        Err(_) => {
            return Err(NetworkPendingEnqueueError::Unavailable {
                endpoint: "NetworkServerOutbox",
                message,
            });
        }
    };
    enqueue_pending(&mut outbox.0, "NetworkServerOutbox", message)
}

pub fn enqueue_server_outbox_broadcast(
    world: &mut World,
    message: ServerMessage,
) -> Result<(), NetworkPendingEnqueueError<OutboundServerMessage>> {
    enqueue_server_outbox(world, OutboundServerMessage::Broadcast(message))
}

pub fn enqueue_server_outbox_to(
    world: &mut World,
    connection: ConnectionHandle,
    message: ServerMessage,
) -> Result<(), NetworkPendingEnqueueError<OutboundServerMessage>> {
    enqueue_server_outbox(
        world,
        OutboundServerMessage::ToConnection {
            connection,
            message,
        },
    )
}

pub fn server_outbox_len(world: &World) -> usize {
    world
        .resource::<NetworkServerOutbox>()
        .map(|outbox| outbox.0.len())
        .unwrap_or(0)
}

pub fn server_outbox_is_empty(world: &World) -> bool {
    server_outbox_len(world) == 0
}

pub fn drain_server_outbox(world: &mut World) -> Vec<OutboundServerMessage> {
    world
        .resource_mut::<NetworkServerOutbox>()
        .map(|outbox| outbox.0.drain())
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NetworkInputStageError<T> {
    Backpressure { capacity: usize, input: T },
}

#[derive(Debug, Clone, ecs::Resource)]
pub(crate) struct NetworkInputStaging<TInput>
where
    TInput: Clone + PartialEq + 'static,
{
    by_tick: BTreeMap<SimulationTick, Vec<TInput>>,
    pending: usize,
    capacity: usize,
}

impl<TInput> Default for NetworkInputStaging<TInput>
where
    TInput: Clone + PartialEq + 'static,
{
    fn default() -> Self {
        Self {
            by_tick: BTreeMap::new(),
            pending: 0,
            capacity: NETWORK_MESSAGE_QUEUE_CAPACITY,
        }
    }
}

impl<TInput> NetworkInputStaging<TInput>
where
    TInput: Clone + PartialEq + 'static,
{
    pub(crate) fn stage(
        &mut self,
        tick: SimulationTick,
        input: TInput,
    ) -> Result<(), NetworkInputStageError<TInput>> {
        if self.pending >= self.capacity {
            return Err(NetworkInputStageError::Backpressure {
                capacity: self.capacity,
                input,
            });
        }
        self.by_tick.entry(tick).or_default().push(input);
        self.pending = self.pending.saturating_add(1);
        Ok(())
    }

    pub(crate) fn drain_tick(&mut self, tick: SimulationTick) -> Vec<TInput> {
        let stale_ticks = self
            .by_tick
            .keys()
            .copied()
            .take_while(|staged_tick| *staged_tick < tick)
            .collect::<Vec<_>>();
        for stale_tick in stale_ticks {
            if let Some(stale) = self.by_tick.remove(&stale_tick) {
                self.pending = self.pending.saturating_sub(stale.len());
            }
        }

        let drained = self.by_tick.remove(&tick).unwrap_or_default();
        self.pending = self.pending.saturating_sub(drained.len());
        drained
    }

    #[cfg(test)]
    pub(crate) fn pending_len(&self) -> usize {
        self.pending
    }
}

pub(crate) fn configure_replication_io<TDriver>(app: &mut App)
where
    TDriver: ReplicationDriver + SnapshotApplyDriver + InputDriver + Send + Sync + 'static,
    TDriver::Snapshot: Clone + PartialEq,
    TDriver::Input: Clone + PartialEq,
{
    app.add_systems(
        PreUpdate,
        client_receive_system::<TDriver>.in_set(NetPreUpdateSet::Receive),
    );
    app.add_systems(
        PreUpdate,
        server_receive_system::<TDriver>.in_set(NetPreUpdateSet::Receive),
    );
}

fn configure_session_projection(app: &mut App) {
    app.init_resource::<RunenNetSessionProjection>();
    app.init_resource::<NetworkSessionStatus>();
}

pub(crate) fn configure_client_role(app: &mut App) {
    app.init_resource::<NetworkClientInbox>();
    app.init_resource::<NetworkClientOutbox>();
    app.init_resource::<NetworkInboundQueue>();
    app.init_resource::<NetworkOutboundQueue>();
    configure_session_projection(app);
    app.init_resource::<NetDiagnosticsView>();
    app.init_resource::<ConnectionHealth>();
    app.init_resource::<RoundTripMetrics>();
    app.init_resource::<NetworkReplicationMetadata>();
    app.init_resource::<NetStreamingStateResource>();
    app.init_resource::<NetworkDiagnostics>();
    app.add_systems(FrameEnd, client_flush_system.in_set(CoreSet::FrameEnd));
    app.add_systems(
        FrameEnd,
        sync_net_diagnostics_view_system.in_set(CoreSet::FrameEnd),
    );
}

pub(crate) fn configure_server_role(app: &mut App) {
    app.init_resource::<NetworkServerInbox>();
    app.init_resource::<NetworkServerOutbox>();
    app.init_resource::<NetworkInboundQueue>();
    app.init_resource::<NetworkOutboundQueue>();
    configure_session_projection(app);
    app.init_resource::<NetDiagnosticsView>();
    app.init_resource::<ConnectionHealth>();
    app.init_resource::<RoundTripMetrics>();
    app.init_resource::<NetworkReplicationMetadata>();
    app.init_resource::<NetStreamingStateResource>();
    app.init_resource::<NetworkDiagnostics>();
    app.add_systems(FrameEnd, server_flush_system.in_set(CoreSet::FrameEnd));
    app.add_systems(
        FrameEnd,
        sync_net_diagnostics_view_system.in_set(CoreSet::FrameEnd),
    );
}

pub(crate) fn configure_replication<TDriver>(app: &mut App)
where
    TDriver: ReplicationDriver + Send + Sync + 'static,
    TDriver::Snapshot: Clone + PartialEq + 'static,
{
    app.init_resource::<SnapshotCursor>();
    app.init_resource::<ServerSnapshotReplicationState<TDriver::Snapshot>>();
    app.init_resource::<ClientSnapshotReplicationState<TDriver::Snapshot>>();
    app.init_resource::<ReplicationDiagnostics>();
    app.add_systems(
        FixedUpdate,
        sync_connection_streaming_state_system
            .after(CoreSet::Simulation)
            .before(NetFixedSet::Prediction),
    );
    app.add_systems(
        FixedUpdate,
        replication_step_system::<TDriver>
            .in_set(NetFixedSet::Replication)
            .after(CoreSet::Simulation)
            .after(NetFixedSet::Prediction),
    );
}

pub(crate) fn configure_prediction<TDriver>(app: &mut App)
where
    TDriver: ReplicationDriver + InputDriver + Send + Sync + 'static,
    TDriver::Input: Clone + PartialEq + 'static,
{
    app.init_resource::<NetworkInputStaging<TDriver::Input>>();
    app.init_resource::<PredictionState<TDriver::Input>>();
    app.init_resource::<PredictionDiagnostics>();
    app.add_systems(
        FixedUpdate,
        prediction_step_system::<TDriver>
            .in_set(NetFixedSet::Prediction)
            .after(CoreSet::Simulation),
    );
}

#[derive(Debug, Clone, Default, ecs::Component, ecs::Resource)]
pub struct NetworkInboundQueue {
    client_messages: Vec<InboundClientMessage>,
    server_messages: Vec<ServerMessage>,
}

impl NetworkInboundQueue {
    pub fn clear(&mut self) {
        self.client_messages.clear();
        self.server_messages.clear();
    }

    pub fn push_client(&mut self, connection: Option<ConnectionHandle>, message: ClientMessage) {
        self.client_messages.push(InboundClientMessage {
            connection,
            message,
        });
    }

    pub fn push_server(&mut self, message: ServerMessage) {
        self.server_messages.push(message);
    }

    pub fn client_messages(&self) -> &[InboundClientMessage] {
        &self.client_messages
    }

    pub fn server_messages(&self) -> &[ServerMessage] {
        &self.server_messages
    }
}

#[derive(Debug, Clone, Default, ecs::Component, ecs::Resource)]
pub struct NetworkOutboundQueue {
    client_messages: Vec<ClientMessage>,
    server_messages: Vec<OutboundServerMessage>,
}

impl NetworkOutboundQueue {
    pub fn clear(&mut self) {
        self.client_messages.clear();
        self.server_messages.clear();
    }

    pub fn push_client(&mut self, message: ClientMessage) {
        self.client_messages.push(message);
    }

    pub fn push_server(&mut self, message: OutboundServerMessage) {
        self.server_messages.push(message);
    }

    pub fn client_messages(&self) -> &[ClientMessage] {
        &self.client_messages
    }

    pub fn server_messages(&self) -> &[OutboundServerMessage] {
        &self.server_messages
    }
}

/// Engine-owned status/diagnostic projection.
///
/// This does not authorize membership or connection lifecycle. `connected` and
/// `active_connection_count` are synchronized from [`RunenNetSessionProjection`]; reconnect
/// attempts and errors are host policy/diagnostics only.
#[derive(Debug, Clone, Default, PartialEq, Eq, ecs::Component, ecs::Resource)]
pub struct NetworkSessionStatus {
    pub connected: bool,
    pub active_connection_count: usize,
    pub last_error: Option<String>,
    pub reconnect_attempt: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, ecs::Component, ecs::Resource)]
pub struct ConnectionHealth {
    pub connected: bool,
    pub close_events: u64,
    pub error_events: u64,
    pub reconnect_events: u64,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, ecs::Component, ecs::Resource)]
pub struct RoundTripMetrics {
    pub last_rtt_millis: Option<u32>,
    pub samples: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionBaselineCheckpoint {
    pub last_ack_cursor: SnapshotCursor,
    pub last_sent_cursor: SnapshotCursor,
    pub last_full_snapshot_cursor: SnapshotCursor,
    pub last_full_snapshot_tick: SimulationTick,
    pub needs_full_resync: bool,
    pub sent_cursors: BTreeSet<SnapshotCursor>,
}

impl Default for ConnectionBaselineCheckpoint {
    fn default() -> Self {
        Self {
            last_ack_cursor: SnapshotCursor::default(),
            last_sent_cursor: SnapshotCursor::default(),
            last_full_snapshot_cursor: SnapshotCursor::default(),
            last_full_snapshot_tick: SimulationTick::default(),
            needs_full_resync: true,
            sent_cursors: BTreeSet::new(),
        }
    }
}

impl ConnectionBaselineCheckpoint {
    pub fn mark_snapshot_sent(
        &mut self,
        cursor: SnapshotCursor,
        tick: SimulationTick,
        sent_full_snapshot: bool,
    ) {
        if cursor.0 >= self.last_sent_cursor.0 {
            self.last_sent_cursor = cursor;
        }
        self.sent_cursors.insert(cursor);
        while self.sent_cursors.len() > MAX_TRACKED_SENT_BASELINE_CURSORS {
            let Some(oldest_cursor) = self.sent_cursors.first().copied() else {
                break;
            };
            self.sent_cursors.remove(&oldest_cursor);
        }
        if sent_full_snapshot {
            self.last_full_snapshot_cursor = cursor;
            self.last_full_snapshot_tick = tick;
            self.needs_full_resync = false;
        }
    }

    pub fn mark_snapshot_acknowledged(
        &mut self,
        cursor: SnapshotCursor,
        baseline_available: bool,
    ) -> SnapshotAckOutcome {
        let outcome = self.validate_snapshot_ack(cursor, baseline_available);
        if matches!(outcome, SnapshotAckOutcome::Accepted { .. }) {
            self.last_ack_cursor = cursor;
            self.needs_full_resync = false;
        }
        outcome
    }

    fn validate_snapshot_ack(
        &self,
        cursor: SnapshotCursor,
        baseline_available: bool,
    ) -> SnapshotAckOutcome {
        if self.last_ack_cursor.0 != 0 && cursor <= self.last_ack_cursor {
            return SnapshotAckOutcome::Rejected {
                cursor,
                reason: SnapshotAckRejection::StaleCursor {
                    last_acknowledged: self.last_ack_cursor,
                },
            };
        }
        if self.last_sent_cursor.0 != 0 && cursor > self.last_sent_cursor {
            return SnapshotAckOutcome::Rejected {
                cursor,
                reason: SnapshotAckRejection::FutureCursor {
                    latest_cursor: self.last_sent_cursor,
                },
            };
        }
        if !self.sent_cursors.contains(&cursor) {
            return SnapshotAckOutcome::Rejected {
                cursor,
                reason: SnapshotAckRejection::UnsentCursor,
            };
        }
        if !baseline_available {
            return SnapshotAckOutcome::Rejected {
                cursor,
                reason: SnapshotAckRejection::PrunedCursor,
            };
        }
        SnapshotAckOutcome::Accepted { cursor }
    }
}

#[derive(Debug, Clone, PartialEq, ecs::Component, ecs::Resource)]
pub struct ServerSnapshotReplicationState<TSnapshot>
where
    TSnapshot: Clone + PartialEq + 'static,
{
    pub checkpoints: HashMap<ConnectionHandle, ConnectionBaselineCheckpoint>,
    pub snapshot_history: BTreeMap<SnapshotCursor, TSnapshot>,
    pub snapshot_history_per_connection:
        HashMap<ConnectionHandle, BTreeMap<SnapshotCursor, TSnapshot>>,
    pub latest_snapshot: Option<TSnapshot>,
    pub latest_snapshot_per_connection: HashMap<ConnectionHandle, TSnapshot>,
    pub latest_tick: SimulationTick,
}

impl<TSnapshot> Default for ServerSnapshotReplicationState<TSnapshot>
where
    TSnapshot: Clone + PartialEq + 'static,
{
    fn default() -> Self {
        Self {
            checkpoints: HashMap::new(),
            snapshot_history: BTreeMap::new(),
            snapshot_history_per_connection: HashMap::new(),
            latest_snapshot: None,
            latest_snapshot_per_connection: HashMap::new(),
            latest_tick: SimulationTick::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, ecs::Component, ecs::Resource)]
pub struct ClientSnapshotReplicationState<TSnapshot>
where
    TSnapshot: Clone + PartialEq + 'static,
{
    pub last_acknowledged_cursor: SnapshotCursor,
    pub last_received_tick: SimulationTick,
    pub applied_snapshots: u64,
    pub last_received_snapshot: Option<TSnapshot>,
    pub snapshot_history: BTreeMap<SnapshotCursor, TSnapshot>,
}

impl<TSnapshot> Default for ClientSnapshotReplicationState<TSnapshot>
where
    TSnapshot: Clone + PartialEq + 'static,
{
    fn default() -> Self {
        Self {
            last_acknowledged_cursor: SnapshotCursor::default(),
            last_received_tick: SimulationTick::default(),
            applied_snapshots: 0,
            last_received_snapshot: None,
            snapshot_history: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingInputFrame<TInput>
where
    TInput: Clone + PartialEq + 'static,
{
    pub tick: SimulationTick,
    pub commands: Vec<TInput>,
}

#[derive(Debug, Clone, PartialEq, ecs::Component, ecs::Resource)]
pub struct PredictionState<TInput>
where
    TInput: Clone + PartialEq + 'static,
{
    pub pending_frames: Vec<PendingInputFrame<TInput>>,
}

impl<TInput> Default for PredictionState<TInput>
where
    TInput: Clone + PartialEq + 'static,
{
    fn default() -> Self {
        Self {
            pending_frames: Vec::new(),
        }
    }
}

impl<TInput> PredictionState<TInput>
where
    TInput: Clone + PartialEq + 'static,
{
    pub fn pending_frames_len(&self) -> usize {
        self.pending_frames.len()
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, ecs::Component, ecs::Resource)]
pub struct NetworkDiagnostics {
    pub processed_client_messages_last_frame: usize,
    pub processed_server_messages_last_frame: usize,
    pub flushed_client_messages_last_frame: usize,
    pub flushed_server_messages_last_frame: usize,
    pub flush_count: u64,
    pub accepted_connections: u64,
    pub rejected_connections: u64,
    pub reconnect_attempts: u64,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, ecs::Component, ecs::Resource)]
pub struct ReplicationDiagnostics {
    pub fixed_steps_observed: u64,
    pub last_snapshot_cursor: u64,
    pub emitted_snapshots: u64,
    pub applied_snapshots: u64,
    pub acked: u64,
    pub rejected_acks: u64,
    pub lagged: u64,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, ecs::Component, ecs::Resource)]
pub struct PredictionDiagnostics {
    pub fixed_steps_observed: u64,
    pub commands_applied: u64,
    pub replayed: u64,
    pub corrected: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, ecs::Component, ecs::Resource)]
pub struct NetDiagnosticsView {
    pub connected: bool,
    pub active_connection_count: usize,
    pub accepted_connections: u64,
    pub rejected_connections: u64,
    pub reconnect_attempts: u64,
    pub close_events: u64,
    pub error_events: u64,
    pub reconnect_events: u64,
    pub last_rtt_millis: Option<u32>,
    pub emitted_snapshots: u64,
    pub applied_snapshots: u64,
    pub acked_snapshots: u64,
    pub lagged_inputs: u64,
    pub corrected_predictions: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_input_staging_capacity_is_total_across_ticks_and_recovers_after_stale_cleanup() {
        let mut staging = NetworkInputStaging::<u16>::default();
        for index in 0..NETWORK_MESSAGE_QUEUE_CAPACITY {
            let tick = SimulationTick(1 + (index % 4) as u64);
            staging
                .stage(tick, index as u16)
                .expect("staging should accept inputs through its total capacity");
        }

        assert_eq!(staging.pending_len(), NETWORK_MESSAGE_QUEUE_CAPACITY);
        let rejected = 5_000u16;
        assert_eq!(
            staging.stage(SimulationTick(6), rejected),
            Err(NetworkInputStageError::Backpressure {
                capacity: NETWORK_MESSAGE_QUEUE_CAPACITY,
                input: rejected,
            })
        );

        assert!(staging.drain_tick(SimulationTick(5)).is_empty());
        assert_eq!(staging.pending_len(), 0);
        staging
            .stage(SimulationTick(6), rejected)
            .expect("stale cleanup should recover staging capacity");
        assert_eq!(staging.pending_len(), 1);
    }

    #[test]
    fn network_input_staging_discards_skipped_ticks_and_preserves_current_and_future_order() {
        let mut staging = NetworkInputStaging::<u8>::default();
        staging.stage(SimulationTick(2), 20).unwrap();
        staging.stage(SimulationTick(5), 50).unwrap();
        staging.stage(SimulationTick(5), 51).unwrap();
        staging.stage(SimulationTick(6), 60).unwrap();

        assert_eq!(staging.pending_len(), 4);
        assert_eq!(staging.drain_tick(SimulationTick(5)), vec![50, 51]);
        assert_eq!(staging.pending_len(), 1);
        assert_eq!(staging.drain_tick(SimulationTick(6)), vec![60]);
        assert_eq!(staging.pending_len(), 0);
    }

    #[test]
    fn checkpoint_accepts_only_sent_and_available_baselines() {
        let mut checkpoint = ConnectionBaselineCheckpoint::default();
        checkpoint.mark_snapshot_sent(SnapshotCursor(1), SimulationTick(1), true);

        assert_eq!(
            checkpoint.mark_snapshot_acknowledged(SnapshotCursor(1), true),
            SnapshotAckOutcome::Accepted {
                cursor: SnapshotCursor(1)
            }
        );
        assert_eq!(checkpoint.last_ack_cursor, SnapshotCursor(1));
        assert!(!checkpoint.needs_full_resync);
    }

    #[test]
    fn checkpoint_rejects_stale_future_unsent_and_pruned_acks() {
        let mut checkpoint = ConnectionBaselineCheckpoint::default();
        checkpoint.mark_snapshot_sent(SnapshotCursor(1), SimulationTick(1), true);
        checkpoint.mark_snapshot_sent(SnapshotCursor(3), SimulationTick(3), false);
        assert_eq!(
            checkpoint.mark_snapshot_acknowledged(SnapshotCursor(1), true),
            SnapshotAckOutcome::Accepted {
                cursor: SnapshotCursor(1)
            }
        );

        assert_eq!(
            checkpoint.mark_snapshot_acknowledged(SnapshotCursor(1), true),
            SnapshotAckOutcome::Rejected {
                cursor: SnapshotCursor(1),
                reason: SnapshotAckRejection::StaleCursor {
                    last_acknowledged: SnapshotCursor(1)
                }
            }
        );
        assert_eq!(
            checkpoint.mark_snapshot_acknowledged(SnapshotCursor(99), false),
            SnapshotAckOutcome::Rejected {
                cursor: SnapshotCursor(99),
                reason: SnapshotAckRejection::FutureCursor {
                    latest_cursor: SnapshotCursor(3)
                }
            }
        );
        assert_eq!(
            checkpoint.mark_snapshot_acknowledged(SnapshotCursor(2), false),
            SnapshotAckOutcome::Rejected {
                cursor: SnapshotCursor(2),
                reason: SnapshotAckRejection::UnsentCursor
            }
        );
        checkpoint.mark_snapshot_sent(SnapshotCursor(4), SimulationTick(4), false);
        assert_eq!(
            checkpoint.mark_snapshot_acknowledged(SnapshotCursor(4), false),
            SnapshotAckOutcome::Rejected {
                cursor: SnapshotCursor(4),
                reason: SnapshotAckRejection::PrunedCursor
            }
        );
    }
}
