//! Runtime plugin that bridges native tablet backends into UI input packets.

use anyhow::Result;
use engine::prelude::{App, Plugin};
use engine::runtime::{NativeWindowHook, NativeWindowHookRegistryResource};
use runen_ecs::World;
use winit::event::{DeviceEvent, WindowEvent};
use winit::event_loop::EventLoopBuilder;
use winit::window::Window;

use crate::backend::{
    MacosNseventBackend, NativeTabletBackendAdapter, WindowsPointerBackend, WindowsWintabBackend,
};
use crate::model::{
    NativeTabletDeviceControlResource, NativeTabletFrameResource, NativeTabletRuntimeResource,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct NativeTabletRuntimePlugin;

impl Plugin for NativeTabletRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NativeTabletRuntimeResource>();
        app.init_resource::<NativeTabletFrameResource>();
        app.init_resource::<NativeTabletDeviceControlResource>();
        app.init_resource::<NativeWindowHookRegistryResource>();
        if let Ok(registry) = app
            .world_mut()
            .resource_mut::<NativeWindowHookRegistryResource>()
        {
            registry.register_hook(NativeTabletRuntimeHook::default());
        }
    }
}

#[derive(Debug, Default)]
struct NativeTabletRuntimeHook {
    windows_pointer: WindowsPointerBackend,
    windows_wintab: WindowsWintabBackend,
    macos_nsevent: MacosNseventBackend,
}

impl NativeWindowHook for NativeTabletRuntimeHook {
    fn name(&self) -> &'static str {
        "native_tablet_input"
    }

    fn configure_event_loop(&mut self, builder: &mut EventLoopBuilder<()>) {
        self.windows_pointer.configure_event_loop(builder);
        self.windows_wintab.configure_event_loop(builder);
        self.macos_nsevent.configure_event_loop(builder);
    }

    fn attach(&mut self, window: &Window, world: &mut World) -> Result<()> {
        with_runtime_and_control(world, |runtime, control| {
            self.windows_pointer.attach(window, runtime, control);
            self.windows_wintab.attach(window, runtime, control);
            self.macos_nsevent.attach(window, runtime, control);
        });
        Ok(())
    }

    fn window_event(
        &mut self,
        window: &Window,
        event: &WindowEvent,
        world: &mut World,
    ) -> Result<()> {
        with_runtime_and_control(world, |runtime, control| {
            self.windows_pointer
                .window_event(window, event, runtime, control);
            self.windows_wintab
                .window_event(window, event, runtime, control);
            self.macos_nsevent
                .window_event(window, event, runtime, control);
        });
        Ok(())
    }

    fn device_event(&mut self, event: &DeviceEvent, world: &mut World) -> Result<()> {
        with_runtime_and_control(world, |runtime, control| {
            self.windows_pointer.device_event(event, runtime, control);
            self.windows_wintab.device_event(event, runtime, control);
            self.macos_nsevent.device_event(event, runtime, control);
        });
        Ok(())
    }

    fn frame(&mut self, _window: &Window, world: &mut World) -> Result<()> {
        ensure_tablet_resources(world);
        let mut runtime = world
            .remove_resource::<NativeTabletRuntimeResource>()
            .unwrap_or_default();
        let mut frame = world
            .remove_resource::<NativeTabletFrameResource>()
            .unwrap_or_default();
        let mut control = world
            .remove_resource::<NativeTabletDeviceControlResource>()
            .unwrap_or_default();

        self.windows_pointer.frame(&mut runtime, &control);
        self.windows_wintab.frame(&mut runtime, &control);
        self.macos_nsevent.frame(&mut runtime, &control);
        runtime.publish_frame(&mut frame, &mut control);

        world.insert_resource(runtime);
        world.insert_resource(frame);
        world.insert_resource(control);
        Ok(())
    }

    fn detach(&mut self, world: &mut World) -> Result<()> {
        with_runtime_and_control(world, |runtime, _control| {
            self.windows_pointer.detach(runtime);
            self.windows_wintab.detach(runtime);
            self.macos_nsevent.detach(runtime);
        });
        Ok(())
    }
}

fn with_runtime_and_control(
    world: &mut World,
    f: impl FnOnce(&mut NativeTabletRuntimeResource, &NativeTabletDeviceControlResource),
) {
    ensure_tablet_resources(world);
    let control = world
        .resource::<NativeTabletDeviceControlResource>()
        .cloned()
        .unwrap_or_default();
    if let Ok(runtime) = world.resource_mut::<NativeTabletRuntimeResource>() {
        f(runtime, &control);
    }
}

fn ensure_tablet_resources(world: &mut World) {
    if !world.has_resource::<NativeTabletRuntimeResource>() {
        world.insert_resource(NativeTabletRuntimeResource::default());
    }
    if !world.has_resource::<NativeTabletFrameResource>() {
        world.insert_resource(NativeTabletFrameResource::default());
    }
    if !world.has_resource::<NativeTabletDeviceControlResource>() {
        world.insert_resource(NativeTabletDeviceControlResource::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::runtime::NativeWindowHookRegistryResource;

    #[test]
    fn plugin_registers_native_window_hook_and_resources() {
        let mut app = App::headless();
        NativeTabletRuntimePlugin.build(&mut app);

        assert!(app.world().has_resource::<NativeTabletRuntimeResource>());
        assert!(app.world().has_resource::<NativeTabletFrameResource>());
        assert!(
            app.world()
                .has_resource::<NativeTabletDeviceControlResource>()
        );
        let registry = app
            .world()
            .resource::<NativeWindowHookRegistryResource>()
            .expect("hook registry should exist");
        assert!(registry.hook_count() >= 1);
    }
}
