use engine_sim::SimulationTick;
use runen_ecs::World;
use runen_net::identity::ConnectionHandle;

pub trait ReplicationDriver {
    type Snapshot: serde::Serialize
        + for<'de> serde::Deserialize<'de>
        + Clone
        + Send
        + Sync
        + 'static;
    type Delta: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Sync + 'static;
    type Input: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Sync + 'static;
    type Error: std::error::Error + Send + Sync + 'static;

    fn capture_snapshot(world: &World) -> Result<Option<Self::Snapshot>, Self::Error>;

    fn capture_snapshot_for_connection(
        world: &World,
        connection: ConnectionHandle,
    ) -> Result<Option<Self::Snapshot>, Self::Error> {
        let _ = connection;
        Self::capture_snapshot(world)
    }

    fn build_delta(previous: &Self::Snapshot, current: &Self::Snapshot) -> Self::Delta;

    fn apply_delta_to_snapshot(base: &Self::Snapshot, delta: &Self::Delta) -> Self::Snapshot;

    fn encode_input(input: &[Self::Input]) -> Result<Vec<u8>, Self::Error> {
        postcard::to_allocvec(input).map_err(Self::map_codec_error)
    }

    fn decode_input(bytes: &[u8]) -> Result<Vec<Self::Input>, Self::Error> {
        postcard::from_bytes(bytes).map_err(Self::map_codec_error)
    }

    fn encode_snapshot(snapshot: &Self::Snapshot) -> Result<Vec<u8>, Self::Error> {
        postcard::to_allocvec(snapshot).map_err(Self::map_codec_error)
    }

    fn decode_snapshot(bytes: &[u8]) -> Result<Self::Snapshot, Self::Error> {
        postcard::from_bytes(bytes).map_err(Self::map_codec_error)
    }

    fn encode_delta(delta: &Self::Delta) -> Result<Vec<u8>, Self::Error> {
        postcard::to_allocvec(delta).map_err(Self::map_codec_error)
    }

    fn decode_delta(bytes: &[u8]) -> Result<Self::Delta, Self::Error> {
        postcard::from_bytes(bytes).map_err(Self::map_codec_error)
    }

    fn map_codec_error(error: postcard::Error) -> Self::Error;
}

pub trait SnapshotApplyDriver: ReplicationDriver {
    fn apply_snapshot(
        world: &mut World,
        tick: SimulationTick,
        snapshot: Self::Snapshot,
    ) -> Result<bool, Self::Error>;

    fn apply_delta(
        world: &mut World,
        tick: SimulationTick,
        delta: Self::Delta,
    ) -> Result<bool, Self::Error>;
}

pub trait InputDriver: ReplicationDriver {
    fn receive_remote_input(
        world: &mut World,
        connection: ConnectionHandle,
        tick: SimulationTick,
        input: Vec<Self::Input>,
    ) -> Result<(), Self::Error>;

    fn take_local_input(world: &mut World) -> Result<Vec<Self::Input>, Self::Error>;

    fn apply_input(world: &mut World, input: &[Self::Input]) -> Result<(), Self::Error>;
}
