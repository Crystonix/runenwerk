use crate::app::App;
use crate::plugin::Plugin;
use crate::plugins::scene::{
    SceneReplayArchive, SceneReplayInputFrameV2, SceneResource, SceneSimulationCodec,
    SceneSimulationSnapshotV2, capture_scene_replay_command_frame,
    capture_scene_simulation_snapshot, republish_scene_resources, validate_scene_replay,
};
use crate::runtime::{
    CoreSet, FixedTimeConfig, FixedUpdate, FrameEnd, PreUpdate, Res, ResMut, SimulationTick,
    SystemConfigExt,
};
use anyhow::{Result, anyhow};
use engine_replay::{
    CheckpointPolicy, ReplayController, ReplayHeader, ReplayRecorder, ReplayStoragePolicy,
    ReplayValidationReport,
};
use engine_sim::{SimulationCodec, SimulationProfileConfig, SimulationSeed, SimulationSessionId};

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub enum ReplayMode {
    #[default]
    Disabled,
    Recording,
    Playback,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, runen_ecs::Component, runen_ecs::Resource)]
pub struct ReplaySessionInfo {
    pub session_id: SimulationSessionId,
    pub seed: SimulationSeed,
    pub tick_rate_hz: u16,
}

impl Default for ReplaySessionInfo {
    fn default() -> Self {
        Self {
            session_id: SimulationSessionId::default(),
            seed: SimulationSeed::default(),
            tick_rate_hz: 60,
        }
    }
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct ReplayState {
    pub mode: ReplayMode,
    pub initial_checkpoint_captured: bool,
    pub last_loaded_tick: Option<SimulationTick>,
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource, Default)]
pub struct ReplayRecorderResource {
    pub recorder: Option<ReplayRecorder<SceneSimulationSnapshotV2, SceneReplayInputFrameV2>>,
    pub checkpoint_policy: CheckpointPolicy,
    pub storage_policy: ReplayStoragePolicy,
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct ReplayControllerResource {
    pub controller: ReplayController<SceneSimulationSnapshotV2, SceneReplayInputFrameV2>,
    pub last_validation: ReplayValidationReport,
}

pub struct ReplayPlugin;

impl Plugin for ReplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneResource>();
        app.init_resource::<ReplayState>();
        app.init_resource::<ReplaySessionInfo>();
        app.init_resource::<ReplayRecorderResource>();
        app.init_resource::<ReplayControllerResource>();
        app.add_systems(
            PreUpdate,
            replay_capture_initial_checkpoint_system.before(CoreSet::Scene),
        );
        app.add_systems(
            FixedUpdate,
            replay_record_command_frame_system
                .before(CoreSet::Scene)
                .before(CoreSet::Simulation),
        );
        app.add_systems(
            FixedUpdate,
            replay_capture_checkpoint_system
                .after(CoreSet::Scene)
                .after(CoreSet::Simulation),
        );
        app.add_systems(FrameEnd, replay_frame_end_system.in_set(CoreSet::FrameEnd));
    }
}

pub(crate) fn start_recording(world: &mut runen_ecs::World) -> Result<()> {
    if !world.has_resource::<ReplayRecorderResource>() || !world.has_resource::<ReplayState>() {
        return Err(anyhow!("ReplayPlugin is not installed"));
    }
    let profile = world
        .resource::<SimulationProfileConfig>()
        .ok()
        .copied()
        .unwrap_or_default();
    let seed = world
        .resource::<SimulationSeed>()
        .ok()
        .copied()
        .unwrap_or_default();
    let session_id = world
        .resource::<SimulationSessionId>()
        .ok()
        .copied()
        .unwrap_or_default();
    let tick_rate_hz = fixed_tick_rate(world);
    let (checkpoint_policy, storage_policy) = {
        let recorder = world
            .resource::<ReplayRecorderResource>()
            .map_err(|_| anyhow!("ReplayPlugin recorder resource is not available"))?;
        (recorder.checkpoint_policy, recorder.storage_policy)
    };

    let header = ReplayHeader {
        format_version: ReplayHeader::FORMAT_VERSION,
        profile: profile.profile,
        determinism: profile.determinism,
        session_id,
        seed,
        tick_rate_hz,
        codec_id: SceneSimulationCodec::codec_id().to_string(),
        codec_version: SceneSimulationCodec::codec_version(),
    };

    if let Ok(info) = world.resource_mut::<ReplaySessionInfo>() {
        *info = ReplaySessionInfo {
            session_id,
            seed,
            tick_rate_hz,
        };
    }
    if let Ok(controller) = world.resource_mut::<ReplayControllerResource>() {
        controller.controller.clear();
        controller.last_validation = ReplayValidationReport::default();
    }
    if let Ok(recorder) = world.resource_mut::<ReplayRecorderResource>() {
        recorder.recorder = Some(ReplayRecorder::new(
            header,
            checkpoint_policy,
            storage_policy,
        ));
    }
    if let Ok(state) = world.resource_mut::<ReplayState>() {
        state.mode = ReplayMode::Recording;
        state.initial_checkpoint_captured = false;
        state.last_loaded_tick = None;
    }
    Ok(())
}

pub(crate) fn stop_recording(world: &mut runen_ecs::World) -> Result<SceneReplayArchive> {
    if !world.has_resource::<ReplayRecorderResource>() || !world.has_resource::<ReplayState>() {
        return Err(anyhow!("ReplayPlugin is not installed"));
    }
    let archive = world
        .resource_mut::<ReplayRecorderResource>()
        .map_err(|_| anyhow!("ReplayPlugin recorder resource is not available"))?
        .recorder
        .take()
        .ok_or_else(|| anyhow!("replay recording is not active"))?
        .into_archive();
    if let Ok(state) = world.resource_mut::<ReplayState>() {
        state.mode = ReplayMode::Disabled;
        state.initial_checkpoint_captured = false;
        state.last_loaded_tick = None;
    }
    Ok(archive)
}

pub(crate) fn load_replay(world: &mut runen_ecs::World, archive: SceneReplayArchive) -> Result<()> {
    if !world.has_resource::<ReplayControllerResource>() || !world.has_resource::<ReplayState>() {
        return Err(anyhow!("ReplayPlugin is not installed"));
    }
    if let Ok(controller) = world.resource_mut::<ReplayControllerResource>() {
        controller.controller.load(archive);
        controller.last_validation = ReplayValidationReport::default();
    }
    if let Ok(state) = world.resource_mut::<ReplayState>() {
        state.mode = ReplayMode::Playback;
        state.initial_checkpoint_captured = false;
        state.last_loaded_tick = None;
    }
    Ok(())
}

pub(crate) fn seek_loaded_replay(
    world: &mut runen_ecs::World,
    target_tick: SimulationTick,
) -> Result<ReplayValidationReport> {
    let archive = world
        .resource::<ReplayControllerResource>()
        .map_err(|_| anyhow!("ReplayPlugin controller resource is not available"))?
        .controller
        .archive_cloned()
        .ok_or_else(|| anyhow!("no replay archive is loaded"))?;
    let report = validate_scene_replay(world, &archive, target_tick)?;
    if let Ok(controller) = world.resource_mut::<ReplayControllerResource>() {
        controller.last_validation = report.clone();
    }
    if let Ok(state) = world.resource_mut::<ReplayState>() {
        state.mode = ReplayMode::Playback;
        state.last_loaded_tick = Some(target_tick);
    }
    republish_scene_resources(world)?;
    Ok(report)
}

fn replay_capture_initial_checkpoint_system(
    tick: Res<SimulationTick>,
    mut state: ResMut<ReplayState>,
    mut recorder: ResMut<ReplayRecorderResource>,
    scene_resource: ResMut<SceneResource>,
) -> Result<()> {
    if state.mode != ReplayMode::Recording || state.initial_checkpoint_captured {
        return Ok(());
    }
    let Some(recorder) = recorder.recorder.as_mut() else {
        return Ok(());
    };
    let Some(manager) = scene_resource.manager.as_ref() else {
        return Ok(());
    };
    let snapshot = capture_scene_simulation_snapshot(manager)?;
    let hash = SceneSimulationCodec::hash(&snapshot)?;
    recorder.record_checkpoint(engine_replay::ReplayCheckpoint {
        meta: engine_replay::ReplayCheckpointMeta { tick: *tick, hash },
        snapshot,
    });
    state.initial_checkpoint_captured = true;
    Ok(())
}

fn replay_record_command_frame_system(
    tick: Res<SimulationTick>,
    state: Res<ReplayState>,
    mut recorder: ResMut<ReplayRecorderResource>,
    scene_resource: Res<SceneResource>,
) {
    if state.mode != ReplayMode::Recording {
        return;
    }
    let Some(recorder) = recorder.recorder.as_mut() else {
        return;
    };
    let Some(manager) = scene_resource.manager.as_ref() else {
        return;
    };
    let frame = capture_scene_replay_command_frame(manager, SimulationTick(tick.0 + 1));
    recorder.record_journal_frame(engine_replay::ReplayJournalFrame {
        tick: frame.tick,
        commands: vec![frame],
        post_hash: None,
    });
}

fn replay_capture_checkpoint_system(
    tick: Res<SimulationTick>,
    state: Res<ReplayState>,
    mut recorder: ResMut<ReplayRecorderResource>,
    scene_resource: Res<SceneResource>,
) -> Result<()> {
    if state.mode != ReplayMode::Recording {
        return Ok(());
    }
    let Some(recorder) = recorder.recorder.as_mut() else {
        return Ok(());
    };
    let Some(manager) = scene_resource.manager.as_ref() else {
        return Ok(());
    };
    let next_tick = SimulationTick(tick.0 + 1);
    let snapshot = capture_scene_simulation_snapshot(manager)?;
    let hash = SceneSimulationCodec::hash(&snapshot)?;
    if let Some(last_frame) = recorder
        .last_journal_frame_mut()
        .filter(|frame| frame.tick == next_tick)
    {
        last_frame.post_hash = Some(hash);
    }
    if recorder.should_checkpoint(next_tick) {
        recorder.record_checkpoint(engine_replay::ReplayCheckpoint {
            meta: engine_replay::ReplayCheckpointMeta {
                tick: next_tick,
                hash,
            },
            snapshot,
        });
    }
    Ok(())
}

fn replay_frame_end_system() {}

fn fixed_tick_rate(world: &runen_ecs::World) -> u16 {
    let step = world
        .resource::<FixedTimeConfig>()
        .map(|config| config.step_seconds)
        .unwrap_or(1.0 / 60.0)
        .max(1.0 / 480.0);
    (1.0 / step).round().clamp(1.0, u16::MAX as f32) as u16
}
