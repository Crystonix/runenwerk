use crate::app::WindowedAppState;
use crate::plugins::InputState;
use crate::plugins::render::backend::RenderSurfaceRegistryResource;
use crate::plugins::render::renderer::Gfx;
use crate::runtime::frame_lifecycle::{
    prepare_world_for_run, run_frame as run_runtime_frame, run_startup_if_needed,
};
use crate::runtime::frame_pacing::{
    FramePacingPolicyResource, FramePacingRuntimeStateResource, decide_frame_pacing,
};
use crate::runtime::native_window_hooks::with_native_window_hooks;
use crate::runtime::platform::{
    PlatformEvent, PlatformWindowEvent, PlatformWindowEventQueueResource, apply_platform_event,
};
use crate::runtime::window::{
    NativeWindowCreationRequest, NativeWindowId, WindowCursorIcon, WindowState,
    WindowStateRegistryResource,
};
use anyhow::{Context, Result, anyhow};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{CursorIcon, Window, WindowAttributes, WindowId};

pub(crate) fn run(mut state: WindowedAppState) -> Result<()> {
    let mut event_loop_builder = EventLoop::builder();
    with_native_window_hooks(&mut state.world, |registry, _world| {
        registry.configure_event_loop(&mut event_loop_builder);
    });
    let event_loop = event_loop_builder.build()?;
    event_loop.set_control_flow(state.control_flow);
    let mut runner = WinitRunner {
        state,
        window: None,
        windows: BTreeMap::new(),
        native_windows_by_winit: BTreeMap::new(),
        last_primary_redraw_at: None,
        fatal_error: None,
    };
    event_loop
        .run_app(&mut runner)
        .map_err(anyhow::Error::from)?;
    if let Some(err) = runner.fatal_error.take() {
        Err(err)
    } else {
        Ok(())
    }
}

struct WinitRunner {
    state: WindowedAppState,
    window: Option<Arc<Window>>,
    windows: BTreeMap<WindowId, Arc<Window>>,
    native_windows_by_winit: BTreeMap<WindowId, NativeWindowId>,
    last_primary_redraw_at: Option<Instant>,
    fatal_error: Option<anyhow::Error>,
}

impl WinitRunner {
    fn sync_window_state(&mut self, window: &Window) -> Result<()> {
        let size = window.inner_size();
        let window_state = {
            let window_state = self
                .state
                .world
                .resource_mut::<WindowState>()
                .context("missing WindowState resource")?;
            window_state.set_headless(false);
            window_state.size_px = (size.width, size.height);
            window_state.scale_factor = window.scale_factor();
            window_state.title = window.title().to_string();
            window_state.clone()
        };
        self.sync_primary_window_registries(&window_state);
        Ok(())
    }

    fn apply_event(&mut self, event: PlatformEvent) -> Result<()> {
        self.apply_event_for_native_window(NativeWindowId::primary(), event)
    }

    fn apply_event_for_native_window(
        &mut self,
        native_window_id: NativeWindowId,
        event: PlatformEvent,
    ) -> Result<()> {
        if let Ok(queue) = self
            .state
            .world
            .resource_mut::<PlatformWindowEventQueueResource>()
        {
            queue.publish(PlatformWindowEvent::new(native_window_id, event.clone()));
        }
        if native_window_id != NativeWindowId::primary() {
            self.apply_secondary_window_event(native_window_id, event);
            return Ok(());
        }
        match &event {
            PlatformEvent::Resumed
            | PlatformEvent::CloseRequested
            | PlatformEvent::Focused { .. }
            | PlatformEvent::Resized { .. }
            | PlatformEvent::ScaleFactorChanged { .. }
            | PlatformEvent::RedrawRequested => {
                let window_state = {
                    let window_state = self
                        .state
                        .world
                        .resource_mut::<WindowState>()
                        .context("missing WindowState resource")?;
                    let mut input = InputState::new();
                    apply_platform_event(window_state, &mut input, &event);
                    window_state.clone()
                };
                self.sync_primary_window_registries(&window_state);
            }
            PlatformEvent::KeyboardInput { .. }
            | PlatformEvent::MouseWheel { .. }
            | PlatformEvent::CursorMoved { .. }
            | PlatformEvent::MouseInput { .. }
            | PlatformEvent::MouseMotion { .. }
            | PlatformEvent::Touch { .. } => {
                let mut window_state = WindowState::headless("");
                let input = self
                    .state
                    .world
                    .resource_mut::<InputState>()
                    .context("missing InputState resource")?;
                apply_platform_event(&mut window_state, input, &event);
            }
        }
        Ok(())
    }

    fn apply_secondary_window_event(
        &mut self,
        native_window_id: NativeWindowId,
        event: PlatformEvent,
    ) {
        match &event {
            PlatformEvent::KeyboardInput { .. }
            | PlatformEvent::MouseWheel { .. }
            | PlatformEvent::CursorMoved { .. }
            | PlatformEvent::MouseInput { .. }
            | PlatformEvent::MouseMotion { .. }
            | PlatformEvent::Touch { .. } => {
                if let Ok(input) = self.state.world.resource_mut::<InputState>() {
                    let mut shadow_window = WindowState::headless("");
                    apply_platform_event(&mut shadow_window, input, &event);
                }
            }
            PlatformEvent::Resumed
            | PlatformEvent::CloseRequested
            | PlatformEvent::Focused { .. }
            | PlatformEvent::Resized { .. }
            | PlatformEvent::ScaleFactorChanged { .. }
            | PlatformEvent::RedrawRequested => {
                if let Ok(registry) = self
                    .state
                    .world
                    .resource_mut::<WindowStateRegistryResource>()
                    && let Some(record) = registry.record_mut(native_window_id)
                {
                    match event {
                        PlatformEvent::Resumed => record.redraw_requested = true,
                        PlatformEvent::CloseRequested => {
                            record.receive_close_intent();
                            record.request_redraw();
                        }
                        PlatformEvent::Focused { focused } => {
                            record.focused = focused;
                            record.request_redraw();
                        }
                        PlatformEvent::Resized { width, height } => {
                            record.size_px = (width, height);
                            record.request_redraw();
                        }
                        PlatformEvent::ScaleFactorChanged {
                            scale_factor,
                            width,
                            height,
                        } => {
                            record.scale_factor = scale_factor;
                            record.size_px = (width, height);
                            record.request_redraw();
                        }
                        PlatformEvent::RedrawRequested => record.redraw_requested = false,
                        PlatformEvent::KeyboardInput { .. }
                        | PlatformEvent::MouseWheel { .. }
                        | PlatformEvent::CursorMoved { .. }
                        | PlatformEvent::MouseInput { .. }
                        | PlatformEvent::MouseMotion { .. }
                        | PlatformEvent::Touch { .. } => {}
                    }
                }
            }
        }
    }

    fn sync_primary_window_registries(&mut self, window_state: &WindowState) {
        if let Ok(registry) = self
            .state
            .world
            .resource_mut::<WindowStateRegistryResource>()
        {
            registry.ensure_primary_from_legacy(window_state);
        }
        if let Ok(surface_registry) = self
            .state
            .world
            .resource_mut::<RenderSurfaceRegistryResource>()
        {
            surface_registry
                .ensure_surface_for_native_window(NativeWindowId::primary(), window_state.size_px);
        }
    }

    fn run_startup_if_needed(&mut self) -> Result<()> {
        // Windowed flow uses the same startup contract as headless.
        prepare_world_for_run(&mut self.state.world, &self.state.title, false);
        run_startup_if_needed(
            &mut self.state.world,
            &mut self.state.scheduler,
            &mut self.state.startup_ran,
        )
    }

    fn run_frame(&mut self) -> Result<()> {
        // Windowed flow uses the same per-frame schedule order as headless.
        if let Some(window) = self.window.clone() {
            with_native_window_hooks(&mut self.state.world, |registry, world| {
                registry.dispatch_frame(&window, world);
            });
        }
        run_runtime_frame(&mut self.state.world, &mut self.state.scheduler)
    }

    fn attach_native_window_hooks(&mut self, window: &Window) {
        with_native_window_hooks(&mut self.state.world, |registry, world| {
            registry.attach_hooks(window, world);
        });
    }

    fn register_runtime_window(&mut self, native_window_id: NativeWindowId, window: Arc<Window>) {
        self.native_windows_by_winit
            .insert(window.id(), native_window_id);
        self.windows.insert(window.id(), window);
    }

    fn drain_pending_window_requests(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let requests = self
            .state
            .world
            .resource_mut::<WindowStateRegistryResource>()
            .ok()
            .map(|registry| registry.take_pending_creation_requests())
            .unwrap_or_default();
        for request in requests {
            self.create_window_for_request(event_loop, request)?;
        }
        Ok(())
    }

    fn create_window_for_request(
        &mut self,
        event_loop: &ActiveEventLoop,
        request: NativeWindowCreationRequest,
    ) -> Result<()> {
        let attrs: WindowAttributes = Window::default_attributes()
            .with_title(request.title.clone())
            .with_inner_size(winit::dpi::PhysicalSize::new(
                request.size_px.0,
                request.size_px.1,
            ));
        let window = match event_loop.create_window(attrs) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                self.mark_window_creation_failed(
                    request.native_window_id,
                    format!("native window creation failed: {err}"),
                );
                return Ok(());
            }
        };

        let mut snapshot = WindowState::windowed(request.title);
        snapshot.size_px = request.size_px;
        snapshot.scale_factor = window.scale_factor();
        snapshot.set_headless(false);
        let render_surface_id = self
            .state
            .world
            .resource_mut::<RenderSurfaceRegistryResource>()
            .ok()
            .map(|registry| {
                registry.ensure_surface_for_native_window(request.native_window_id, request.size_px)
            });
        let Some(render_surface_id) = render_surface_id else {
            self.mark_window_creation_failed(
                request.native_window_id,
                "render surface registry is unavailable",
            );
            return Ok(());
        };
        let attach_result = self
            .state
            .world
            .resource_mut::<Gfx>()
            .context("runtime gfx is unavailable")
            .and_then(|gfx| {
                gfx.attach_surface(render_surface_id, Arc::clone(&window), request.size_px)
            });
        if let Err(err) = attach_result {
            if let Ok(surface_registry) = self
                .state
                .world
                .resource_mut::<RenderSurfaceRegistryResource>()
            {
                surface_registry.retire_surface_for_native_window(request.native_window_id);
            }
            self.mark_window_creation_failed(
                request.native_window_id,
                format!("GPU surface attachment failed: {err:#}"),
            );
            return Ok(());
        }
        if let Ok(registry) = self
            .state
            .world
            .resource_mut::<WindowStateRegistryResource>()
        {
            registry.register_created_window(request.native_window_id, &snapshot);
        }

        self.attach_native_window_hooks(&window);
        window.request_redraw();
        self.register_runtime_window(request.native_window_id, window);
        Ok(())
    }

    fn mark_window_creation_failed(
        &mut self,
        native_window_id: NativeWindowId,
        reason: impl Into<String>,
    ) {
        if let Ok(registry) = self
            .state
            .world
            .resource_mut::<WindowStateRegistryResource>()
            && let Some(record) = registry.record_mut(native_window_id)
        {
            record.mark_creation_failed(reason);
        }
    }

    fn dispatch_native_window_event(&mut self, window: &Window, event: &WindowEvent) {
        with_native_window_hooks(&mut self.state.world, |registry, world| {
            registry.dispatch_window_event(window, event, world);
        });
    }

    fn dispatch_native_device_event(&mut self, event: &DeviceEvent) {
        with_native_window_hooks(&mut self.state.world, |registry, world| {
            registry.dispatch_device_event(event, world);
        });
    }

    fn apply_window_effects(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        self.drain_pending_window_requests(event_loop)?;

        let windows = self.windows.values().cloned().collect::<Vec<_>>();
        for window in windows {
            let Some(native_window_id) =
                native_window_id_for_winit_event(&self.native_windows_by_winit, window.id())
            else {
                tracing::debug!(
                    target = "engine.runtime.window",
                    ?window,
                    "skipping effects for unregistered native window"
                );
                continue;
            };
            let record = self
                .state
                .world
                .resource::<WindowStateRegistryResource>()
                .ok()
                .and_then(|registry| registry.record(native_window_id).cloned());
            let Some(record) = record else {
                continue;
            };

            if window.title() != record.title {
                window.set_title(&record.title);
            }
            window.set_cursor(winit_cursor_icon(record.cursor_icon));

            if record.close_requested {
                if native_window_id == NativeWindowId::primary() {
                    event_loop.exit();
                    return Ok(());
                }
                let render_surface_id = self
                    .state
                    .world
                    .resource::<RenderSurfaceRegistryResource>()
                    .ok()
                    .and_then(|registry| registry.surface_for_native_window(native_window_id));
                if let Some(render_surface_id) = render_surface_id
                    && let Ok(gfx) = self.state.world.resource_mut::<Gfx>()
                {
                    gfx.detach_surface(render_surface_id);
                }
                if let Ok(surface_registry) = self
                    .state
                    .world
                    .resource_mut::<RenderSurfaceRegistryResource>()
                {
                    surface_registry.retire_surface_for_native_window(native_window_id);
                }
                if let Ok(window_registry) = self
                    .state
                    .world
                    .resource_mut::<WindowStateRegistryResource>()
                {
                    window_registry.remove_window(native_window_id);
                }
                self.windows.remove(&window.id());
                self.native_windows_by_winit.remove(&window.id());
                continue;
            }

            if record.redraw_requested {
                window.request_redraw();
                if let Ok(registry) = self
                    .state
                    .world
                    .resource_mut::<WindowStateRegistryResource>()
                    && let Some(record) = registry.record_mut(native_window_id)
                {
                    record.redraw_requested = false;
                }
            }
        }

        Ok(())
    }

    fn frame_pacing_policy(&self) -> FramePacingPolicyResource {
        self.state
            .world
            .resource::<FramePacingPolicyResource>()
            .ok()
            .copied()
            .unwrap_or_default()
    }

    fn observe_frame_pacing_decision(
        &mut self,
        policy: FramePacingPolicyResource,
        now: Instant,
        next_deadline: Option<Instant>,
        redraw_requested: bool,
    ) {
        if let Ok(state) = self
            .state
            .world
            .resource_mut::<FramePacingRuntimeStateResource>()
        {
            state.observe_policy(policy);
            state.observe_next_deadline(now, next_deadline);
            state.observe_redraw_requested(redraw_requested);
        }
    }

    fn request_redraw_for_native_window(&mut self, native_window_id: NativeWindowId) {
        if native_window_id == NativeWindowId::primary()
            && let Ok(window_state) = self.state.world.resource_mut::<WindowState>()
        {
            window_state.request_redraw();
        }
        if let Ok(registry) = self
            .state
            .world
            .resource_mut::<WindowStateRegistryResource>()
            && let Some(record) = registry.record_mut(native_window_id)
        {
            record.request_redraw();
        }
        if let Ok(state) = self
            .state
            .world
            .resource_mut::<FramePacingRuntimeStateResource>()
        {
            state.observe_redraw_requested(true);
        }
    }

    fn observe_primary_frame_started(&mut self) {
        let now = Instant::now();
        if let Some(previous) = self.last_primary_redraw_at
            && let Ok(state) = self
                .state
                .world
                .resource_mut::<FramePacingRuntimeStateResource>()
        {
            state.observe_frame_interval(now.saturating_duration_since(previous));
        }
        self.last_primary_redraw_at = Some(now);
    }

    fn apply_frame_pacing(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let policy = self.frame_pacing_policy();
        let decision = decide_frame_pacing(policy, self.last_primary_redraw_at, now);
        if decision.request_redraw
            && let Some(window) = self.window.as_ref()
        {
            window.request_redraw();
        }
        match decision.next_deadline {
            Some(deadline) => event_loop.set_control_flow(ControlFlow::wait_duration(
                deadline.saturating_duration_since(Instant::now()),
            )),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
        self.observe_frame_pacing_decision(
            policy,
            now,
            decision.next_deadline,
            decision.request_redraw,
        );
    }

    fn exit_with_error(&mut self, event_loop: &ActiveEventLoop, err: anyhow::Error) {
        tracing::error!(error = %format!("{err:#}"), "runtime windowed execution failed");
        self.fatal_error = Some(err);
        event_loop.exit();
    }
}

impl Drop for WinitRunner {
    fn drop(&mut self) {
        with_native_window_hooks(&mut self.state.world, |registry, world| {
            registry.detach_hooks(world);
        });
    }
}

fn winit_cursor_icon(cursor_icon: WindowCursorIcon) -> CursorIcon {
    match cursor_icon {
        WindowCursorIcon::Default => CursorIcon::Default,
        WindowCursorIcon::ColResize => CursorIcon::ColResize,
        WindowCursorIcon::RowResize => CursorIcon::RowResize,
        WindowCursorIcon::NwseResize => CursorIcon::NwseResize,
        WindowCursorIcon::NeswResize => CursorIcon::NeswResize,
        WindowCursorIcon::Grab => CursorIcon::Grab,
        WindowCursorIcon::Grabbing => CursorIcon::Grabbing,
    }
}

impl ApplicationHandler for WinitRunner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs: WindowAttributes =
            Window::default_attributes().with_title(self.state.title.clone());
        let window = match event_loop.create_window(attrs) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                self.exit_with_error(
                    event_loop,
                    anyhow!("failed to create runtime window: {err}"),
                );
                return;
            }
        };

        if let Err(err) = self.sync_window_state(&window) {
            self.exit_with_error(
                event_loop,
                anyhow!("failed to sync initial window state: {err:#}"),
            );
            return;
        }

        if self.state.world.resource::<Gfx>().is_err() {
            match Gfx::new(window.clone()) {
                Ok(gfx) => self.state.world.insert_resource(gfx),
                Err(err) => {
                    self.exit_with_error(
                        event_loop,
                        anyhow!("failed to initialize runtime gfx: {err:#}"),
                    );
                    return;
                }
            }
        }

        if let Err(err) = self.apply_event(PlatformEvent::Resumed) {
            self.exit_with_error(event_loop, anyhow!("failed to apply resume event: {err:#}"));
            return;
        }

        self.attach_native_window_hooks(&window);

        if let Err(err) = self.run_startup_if_needed() {
            self.exit_with_error(event_loop, anyhow!("runtime startup failed: {err:#}"));
            return;
        }

        window.request_redraw();
        self.register_runtime_window(NativeWindowId::primary(), window.clone());
        self.window = Some(window);
        self.apply_frame_pacing(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.windows.get(&window_id).cloned() else {
            tracing::debug!(
                target = "engine.runtime.window",
                ?window_id,
                "ignoring event for unknown native window"
            );
            return;
        };
        let Some(native_window_id) =
            native_window_id_for_winit_event(&self.native_windows_by_winit, window_id)
        else {
            tracing::debug!(
                target = "engine.runtime.window",
                ?window_id,
                "ignoring event for unregistered native window"
            );
            return;
        };

        self.dispatch_native_window_event(&window, &event);

        let result = match event {
            WindowEvent::CloseRequested => {
                self.apply_event_for_native_window(native_window_id, PlatformEvent::CloseRequested)
            }
            WindowEvent::Focused(focused) => self.apply_event_for_native_window(
                native_window_id,
                PlatformEvent::Focused { focused },
            ),
            WindowEvent::Resized(size) => self.apply_event_for_native_window(
                native_window_id,
                PlatformEvent::Resized {
                    width: size.width,
                    height: size.height,
                },
            ),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let size = window.inner_size();
                self.apply_event_for_native_window(
                    native_window_id,
                    PlatformEvent::ScaleFactorChanged {
                        scale_factor,
                        width: size.width,
                        height: size.height,
                    },
                )
            }
            WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                winit::keyboard::PhysicalKey::Code(code) => {
                    let result = self.apply_event_for_native_window(
                        native_window_id,
                        PlatformEvent::KeyboardInput {
                            key: code,
                            state: event.state,
                            text: event.text.as_deref().map(str::to_string),
                        },
                    );
                    if result.is_ok() {
                        self.request_redraw_for_native_window(native_window_id);
                    }
                    result
                }
                _ => Ok(()),
            },
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(position) => position.y as f32,
                };
                let result = self.apply_event_for_native_window(
                    native_window_id,
                    PlatformEvent::MouseWheel { delta },
                );
                if result.is_ok() {
                    self.request_redraw_for_native_window(native_window_id);
                }
                result
            }
            WindowEvent::CursorMoved { position, .. } => {
                let result = self.apply_event_for_native_window(
                    native_window_id,
                    PlatformEvent::CursorMoved {
                        x: position.x as f32,
                        y: position.y as f32,
                    },
                );
                if result.is_ok() {
                    self.request_redraw_for_native_window(native_window_id);
                }
                result
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let result = self.apply_event_for_native_window(
                    native_window_id,
                    PlatformEvent::MouseInput { state, button },
                );
                if result.is_ok() {
                    self.request_redraw_for_native_window(native_window_id);
                }
                result
            }
            WindowEvent::Touch(touch) => {
                let result = self.apply_event_for_native_window(
                    native_window_id,
                    PlatformEvent::Touch {
                        phase: touch.phase.into(),
                        id: touch.id,
                        x: touch.location.x as f32,
                        y: touch.location.y as f32,
                        pressure: touch.force.map(|force| force.normalized() as f32),
                    },
                );
                if result.is_ok() {
                    self.request_redraw_for_native_window(native_window_id);
                }
                result
            }
            WindowEvent::RedrawRequested => {
                if native_window_id != NativeWindowId::primary() {
                    let frame_result = self
                        .apply_event_for_native_window(
                            native_window_id,
                            PlatformEvent::RedrawRequested,
                        )
                        .and_then(|_| self.apply_window_effects(event_loop));
                    if let Err(err) = frame_result {
                        self.exit_with_error(
                            event_loop,
                            anyhow!("runtime secondary-window redraw failed: {err:#}"),
                        );
                    }
                    return;
                }
                let frame_result = self
                    .apply_event(PlatformEvent::RedrawRequested)
                    .and_then(|_| {
                        self.observe_primary_frame_started();
                        self.run_frame()
                    })
                    .and_then(|_| self.apply_window_effects(event_loop));
                if let Err(err) = frame_result {
                    self.exit_with_error(event_loop, anyhow!("runtime frame failed: {err:#}"));
                    return;
                }
                self.apply_frame_pacing(event_loop);
                return;
            }
            _ => Ok(()),
        };

        if let Err(err) = result.and_then(|_| self.apply_window_effects(event_loop)) {
            self.exit_with_error(event_loop, anyhow!("runtime window event failed: {err:#}"));
            return;
        }
        self.apply_frame_pacing(event_loop);
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        self.dispatch_native_device_event(&event);

        let result = match event {
            DeviceEvent::MouseMotion { delta } => {
                let result = self.apply_event(PlatformEvent::MouseMotion {
                    delta_x: delta.0 as f32,
                    delta_y: delta.1 as f32,
                });
                if result.is_ok() {
                    self.request_redraw_for_native_window(NativeWindowId::primary());
                }
                result
            }
            _ => Ok(()),
        };

        if let Err(err) = result.and_then(|_| self.apply_window_effects(event_loop)) {
            self.exit_with_error(event_loop, anyhow!("runtime device event failed: {err:#}"));
            return;
        }
        self.apply_frame_pacing(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.apply_frame_pacing(event_loop);
    }
}

fn native_window_id_for_winit_event(
    native_windows_by_winit: &BTreeMap<WindowId, NativeWindowId>,
    window_id: WindowId,
) -> Option<NativeWindowId> {
    native_windows_by_winit.get(&window_id).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::runtime::fixed_time::{
        CatchupBudget, FixedTimeConfig, FixedTimeState, SimulationTick,
    };
    use crate::runtime::schedules::{FixedUpdate, PreUpdate};
    use crate::runtime::{Res, ResMut};

    #[derive(Debug, Default, runen_ecs::Component, runen_ecs::Resource)]
    struct FixedTickLog(Vec<u64>);

    fn configure_probe(app: &mut App) {
        app.init_resource::<FixedTickLog>();
        app.insert_resource(FixedTimeConfig {
            step_seconds: 1.0 / 60.0,
        });
        app.insert_resource(CatchupBudget {
            max_steps_per_frame: 4,
        });
        app.add_systems(PreUpdate, set_frame_delta);
        app.add_systems(FixedUpdate, log_tick);
    }

    fn set_frame_delta(mut time: ResMut<crate::plugins::time::domain::Time>) {
        time.delta_seconds = 0.05;
    }

    fn log_tick(tick: Res<SimulationTick>, mut log: ResMut<FixedTickLog>) {
        log.0.push(tick.0);
    }

    #[test]
    fn headless_and_windowed_paths_share_fixed_step_semantics() {
        let mut headless = App::headless();
        configure_probe(&mut headless);
        headless
            .prepare_for_run(true)
            .expect("headless startup should run");
        headless.run_frame().expect("headless frame should run");

        let headless_log = headless
            .world()
            .resource::<FixedTickLog>()
            .expect("headless log resource should exist")
            .0
            .clone();
        let headless_tick = headless
            .world()
            .resource::<SimulationTick>()
            .expect("headless tick should exist")
            .0;
        let headless_fixed = *headless
            .world()
            .resource::<FixedTimeState>()
            .expect("headless fixed state should exist");

        let mut windowed = App::new();
        configure_probe(&mut windowed);
        let mut runner = WinitRunner {
            state: windowed.into_windowed_state(),
            window: None,
            windows: BTreeMap::new(),
            native_windows_by_winit: BTreeMap::new(),
            last_primary_redraw_at: None,
            fatal_error: None,
        };
        runner
            .run_startup_if_needed()
            .expect("windowed startup should run");
        runner.run_frame().expect("windowed frame should run");

        let windowed_log = runner
            .state
            .world
            .resource::<FixedTickLog>()
            .expect("windowed log resource should exist")
            .0
            .clone();
        let windowed_tick = runner
            .state
            .world
            .resource::<SimulationTick>()
            .expect("windowed tick should exist")
            .0;
        let windowed_fixed = *runner
            .state
            .world
            .resource::<FixedTimeState>()
            .expect("windowed fixed state should exist");

        assert_eq!(headless_log, vec![1, 2, 3]);
        assert_eq!(windowed_log, vec![1, 2, 3]);
        assert_eq!(headless_log, windowed_log);
        assert_eq!(headless_tick, windowed_tick);
        assert_eq!(headless_fixed.steps_ran_last_frame, 3);
        assert_eq!(windowed_fixed.steps_ran_last_frame, 3);
        assert_eq!(headless_fixed.saturated_frames, 0);
        assert_eq!(windowed_fixed.saturated_frames, 0);
    }

    #[test]
    fn winit_window_event_route_rejects_unknown_window_id() {
        let native_windows_by_winit = BTreeMap::new();

        assert_eq!(
            native_window_id_for_winit_event(&native_windows_by_winit, WindowId::dummy()),
            None
        );
    }

    #[test]
    fn winit_window_event_route_maps_known_primary_window() {
        let mut native_windows_by_winit = BTreeMap::new();
        native_windows_by_winit.insert(WindowId::dummy(), NativeWindowId::primary());

        assert_eq!(
            native_window_id_for_winit_event(&native_windows_by_winit, WindowId::dummy()),
            Some(NativeWindowId::primary())
        );
    }

    #[test]
    fn winit_window_event_route_maps_known_secondary_window() {
        let secondary_window =
            NativeWindowId::try_from_raw(2).expect("test native window id should be non-zero");
        let mut native_windows_by_winit = BTreeMap::new();
        native_windows_by_winit.insert(WindowId::dummy(), secondary_window);

        assert_eq!(
            native_window_id_for_winit_event(&native_windows_by_winit, WindowId::dummy()),
            Some(secondary_window)
        );
    }

    #[test]
    fn winit_window_event_route_rejects_retired_window_id() {
        let secondary_window =
            NativeWindowId::try_from_raw(2).expect("test native window id should be non-zero");
        let mut native_windows_by_winit = BTreeMap::new();
        native_windows_by_winit.insert(WindowId::dummy(), secondary_window);
        native_windows_by_winit.remove(&WindowId::dummy());

        assert_eq!(
            native_window_id_for_winit_event(&native_windows_by_winit, WindowId::dummy()),
            None
        );
    }

    #[test]
    fn explicit_primary_redraw_request_wakes_on_demand_pacing() {
        let mut app = App::new();
        app.with_frame_pacing(FramePacingPolicyResource::on_demand());
        let mut runner = WinitRunner {
            state: app.into_windowed_state(),
            window: None,
            windows: BTreeMap::new(),
            native_windows_by_winit: BTreeMap::new(),
            last_primary_redraw_at: None,
            fatal_error: None,
        };

        runner.request_redraw_for_native_window(NativeWindowId::primary());

        let registry = runner
            .state
            .world
            .resource::<WindowStateRegistryResource>()
            .expect("window registry should exist");
        assert!(
            registry
                .record(NativeWindowId::primary())
                .expect("primary should exist")
                .redraw_requested
        );
        let pacing = runner
            .state
            .world
            .resource::<FramePacingRuntimeStateResource>()
            .expect("pacing state should exist");
        assert!(pacing.redraw_requested);
    }
}
