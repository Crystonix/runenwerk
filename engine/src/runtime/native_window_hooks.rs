//! Native window hook extension points for platform adapters.

use anyhow::Result;
use runen_ecs::World;
use winit::event::{DeviceEvent, WindowEvent};
use winit::event_loop::EventLoopBuilder;
use winit::window::Window;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeWindowHookDiagnostic {
    pub hook_name: &'static str,
    pub stage: NativeWindowHookStage,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeWindowHookStage {
    Attach,
    WindowEvent,
    DeviceEvent,
    Frame,
    Detach,
}

pub trait NativeWindowHook: 'static {
    fn name(&self) -> &'static str;

    fn configure_event_loop(&mut self, _builder: &mut EventLoopBuilder<()>) {}

    fn attach(&mut self, _window: &Window, _world: &mut World) -> Result<()> {
        Ok(())
    }

    fn window_event(
        &mut self,
        _window: &Window,
        _event: &WindowEvent,
        _world: &mut World,
    ) -> Result<()> {
        Ok(())
    }

    fn device_event(&mut self, _event: &DeviceEvent, _world: &mut World) -> Result<()> {
        Ok(())
    }

    fn frame(&mut self, _window: &Window, _world: &mut World) -> Result<()> {
        Ok(())
    }

    fn detach(&mut self, _world: &mut World) -> Result<()> {
        Ok(())
    }
}

#[derive(Default)]
pub struct NativeWindowHookRegistryResource {
    hooks: Vec<Box<dyn NativeWindowHook>>,
    diagnostics: Vec<NativeWindowHookDiagnostic>,
}

impl runen_ecs::Resource for NativeWindowHookRegistryResource {}

impl NativeWindowHookRegistryResource {
    pub fn register_hook(&mut self, hook: impl NativeWindowHook) {
        self.hooks.push(Box::new(hook));
    }

    pub fn hook_count(&self) -> usize {
        self.hooks.len()
    }

    pub fn diagnostics(&self) -> &[NativeWindowHookDiagnostic] {
        &self.diagnostics
    }

    pub fn clear_diagnostics(&mut self) {
        self.diagnostics.clear();
    }

    pub fn configure_event_loop(&mut self, builder: &mut EventLoopBuilder<()>) {
        for hook in &mut self.hooks {
            hook.configure_event_loop(builder);
        }
    }

    pub fn attach_hooks(&mut self, window: &Window, world: &mut World) {
        for hook in &mut self.hooks {
            if let Err(err) = hook.attach(window, world) {
                self.diagnostics.push(NativeWindowHookDiagnostic {
                    hook_name: hook.name(),
                    stage: NativeWindowHookStage::Attach,
                    message: format!("{err:#}"),
                });
            }
        }
    }

    pub fn dispatch_window_event(
        &mut self,
        window: &Window,
        event: &WindowEvent,
        world: &mut World,
    ) {
        for hook in &mut self.hooks {
            if let Err(err) = hook.window_event(window, event, world) {
                self.diagnostics.push(NativeWindowHookDiagnostic {
                    hook_name: hook.name(),
                    stage: NativeWindowHookStage::WindowEvent,
                    message: format!("{err:#}"),
                });
            }
        }
    }

    pub fn dispatch_device_event(&mut self, event: &DeviceEvent, world: &mut World) {
        for hook in &mut self.hooks {
            if let Err(err) = hook.device_event(event, world) {
                self.diagnostics.push(NativeWindowHookDiagnostic {
                    hook_name: hook.name(),
                    stage: NativeWindowHookStage::DeviceEvent,
                    message: format!("{err:#}"),
                });
            }
        }
    }

    pub fn dispatch_frame(&mut self, window: &Window, world: &mut World) {
        for hook in &mut self.hooks {
            if let Err(err) = hook.frame(window, world) {
                self.diagnostics.push(NativeWindowHookDiagnostic {
                    hook_name: hook.name(),
                    stage: NativeWindowHookStage::Frame,
                    message: format!("{err:#}"),
                });
            }
        }
    }

    pub fn detach_hooks(&mut self, world: &mut World) {
        for hook in &mut self.hooks {
            if let Err(err) = hook.detach(world) {
                self.diagnostics.push(NativeWindowHookDiagnostic {
                    hook_name: hook.name(),
                    stage: NativeWindowHookStage::Detach,
                    message: format!("{err:#}"),
                });
            }
        }
    }
}

pub(crate) fn with_native_window_hooks(
    world: &mut World,
    f: impl FnOnce(&mut NativeWindowHookRegistryResource, &mut World),
) {
    let Some(mut registry) = world.remove_resource::<NativeWindowHookRegistryResource>() else {
        return;
    };
    f(&mut registry, world);
    world.insert_resource(registry);
}
