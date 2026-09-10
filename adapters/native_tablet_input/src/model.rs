//! Native tablet input DTOs, capabilities, diagnostics, and runtime snapshots.

use std::collections::VecDeque;

use ui_input::{
    Modifiers, PointerBarrelButtons, PointerButton, PointerCalibration, PointerContactState,
    PointerDelta, PointerDeviceId, PointerEventKind, PointerLatencyClass, PointerPosition,
    PointerSample, PointerSampleRole, PointerSourceKind, PointerTilt, PointerToolKind,
    UiInputEvent,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeTabletPlatform {
    Windows,
    Macos,
    Unknown,
}

impl NativeTabletPlatform {
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeTabletVendor {
    Wacom,
    Generic,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeTabletBackendKind {
    WindowsPointer,
    WindowsWintab,
    MacosNsevent,
    MacosWacomDriver,
    WinitFallback,
}

impl NativeTabletBackendKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::WindowsPointer => "Windows Pointer/Ink",
            Self::WindowsWintab => "Wacom Wintab",
            Self::MacosNsevent => "macOS NSEvent",
            Self::MacosWacomDriver => "Wacom macOS Driver",
            Self::WinitFallback => "winit fallback",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum NativeTabletBackendPreference {
    #[default]
    AutoOsFirst,
    WindowsPointer,
    WindowsWintab,
    MacosNsevent,
    MacosWacomDriver,
    WinitFallback,
}

impl NativeTabletBackendPreference {
    pub fn accepts(self, backend: NativeTabletBackendKind) -> bool {
        match self {
            Self::AutoOsFirst => true,
            Self::WindowsPointer => backend == NativeTabletBackendKind::WindowsPointer,
            Self::WindowsWintab => backend == NativeTabletBackendKind::WindowsWintab,
            Self::MacosNsevent => backend == NativeTabletBackendKind::MacosNsevent,
            Self::MacosWacomDriver => backend == NativeTabletBackendKind::MacosWacomDriver,
            Self::WinitFallback => backend == NativeTabletBackendKind::WinitFallback,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeTabletToolKind {
    Mouse,
    Pen,
    Brush,
    Marker,
    Airbrush,
    Eraser,
    Finger,
    Unknown,
}

impl From<NativeTabletToolKind> for PointerToolKind {
    fn from(value: NativeTabletToolKind) -> Self {
        match value {
            NativeTabletToolKind::Mouse => Self::Mouse,
            NativeTabletToolKind::Pen => Self::Pen,
            NativeTabletToolKind::Brush => Self::Brush,
            NativeTabletToolKind::Marker => Self::Marker,
            NativeTabletToolKind::Airbrush => Self::Airbrush,
            NativeTabletToolKind::Eraser => Self::Eraser,
            NativeTabletToolKind::Finger => Self::Finger,
            NativeTabletToolKind::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeTabletCapabilityKind {
    Pressure,
    Tilt,
    Twist,
    TangentialPressure,
    Hover,
    Eraser,
    BarrelButtons,
    CoalescedSamples,
    PredictedSamples,
    Calibration,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeTabletDiagnostic {
    MissingCapability(NativeTabletCapabilityKind),
    BackendUnavailable {
        backend: NativeTabletBackendKind,
        reason: String,
    },
    BackendWarning {
        backend: NativeTabletBackendKind,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct NativeTabletCapabilities {
    pub pressure: bool,
    pub tilt: bool,
    pub twist: bool,
    pub tangential_pressure: bool,
    pub hover: bool,
    pub eraser: bool,
    pub barrel_buttons: bool,
    pub coalesced_samples: bool,
    pub predicted_samples: bool,
    pub calibration: bool,
}

impl NativeTabletCapabilities {
    pub const fn macos_wacom() -> Self {
        Self::full_stylus()
    }

    pub const fn windows_pointer() -> Self {
        Self {
            pressure: true,
            tilt: false,
            twist: false,
            tangential_pressure: false,
            hover: true,
            eraser: false,
            barrel_buttons: false,
            coalesced_samples: true,
            predicted_samples: false,
            calibration: true,
        }
    }

    pub const fn windows_pointer_mouse() -> Self {
        Self {
            pressure: false,
            tilt: false,
            twist: false,
            tangential_pressure: false,
            hover: true,
            eraser: false,
            barrel_buttons: false,
            coalesced_samples: false,
            predicted_samples: false,
            calibration: true,
        }
    }

    pub const fn windows_wintab() -> Self {
        Self::full_stylus()
    }

    pub const fn macos_nsevent() -> Self {
        Self {
            pressure: true,
            tilt: true,
            twist: true,
            tangential_pressure: true,
            hover: true,
            eraser: true,
            barrel_buttons: true,
            coalesced_samples: false,
            predicted_samples: false,
            calibration: true,
        }
    }

    const fn full_stylus() -> Self {
        Self {
            pressure: true,
            tilt: true,
            twist: true,
            tangential_pressure: true,
            hover: true,
            eraser: true,
            barrel_buttons: true,
            coalesced_samples: true,
            predicted_samples: true,
            calibration: true,
        }
    }
}

impl From<NativeTabletCapabilities> for ui_input::PointerDeviceCapabilities {
    fn from(value: NativeTabletCapabilities) -> Self {
        Self {
            pressure: value.pressure,
            tilt: value.tilt,
            twist: value.twist,
            tangential_pressure: value.tangential_pressure,
            hover: value.hover,
            eraser: value.eraser,
            barrel_buttons: value.barrel_buttons,
            coalesced_samples: value.coalesced_samples,
            predicted_samples: value.predicted_samples,
            calibration: value.calibration,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeTabletSample {
    pub position: PointerPosition,
    pub delta: PointerDelta,
    pub timestamp_micros: Option<u64>,
    pub pressure: Option<f32>,
    pub tilt: Option<PointerTilt>,
    pub twist_degrees: Option<f32>,
    pub tangential_pressure: Option<f32>,
    pub contact: PointerContactState,
}

impl NativeTabletSample {
    pub const fn new(position: PointerPosition, delta: PointerDelta) -> Self {
        Self {
            position,
            delta,
            timestamp_micros: None,
            pressure: None,
            tilt: None,
            twist_degrees: None,
            tangential_pressure: None,
            contact: PointerContactState::Contact,
        }
    }

    pub fn with_timestamp_micros(mut self, timestamp_micros: u64) -> Self {
        self.timestamp_micros = Some(timestamp_micros);
        self
    }

    pub fn with_pressure(mut self, pressure: f32) -> Self {
        self.pressure = Some(pressure);
        self
    }

    pub fn with_tilt(mut self, tilt: PointerTilt) -> Self {
        self.tilt = Some(tilt);
        self
    }

    pub fn with_twist_degrees(mut self, twist_degrees: f32) -> Self {
        self.twist_degrees = Some(twist_degrees);
        self
    }

    pub fn with_tangential_pressure(mut self, tangential_pressure: f32) -> Self {
        self.tangential_pressure = Some(tangential_pressure);
        self
    }

    pub fn with_contact(mut self, contact: PointerContactState) -> Self {
        self.contact = contact;
        self
    }

    pub(crate) fn into_pointer_sample(
        self,
        role: PointerSampleRole,
        capabilities: NativeTabletCapabilities,
        calibration: Option<PointerCalibration>,
    ) -> PointerSample {
        let position = calibrated_position(self.position, calibration);
        PointerSample {
            role,
            position,
            delta: self.delta,
            timestamp_micros: self.timestamp_micros,
            pressure: calibrated_pressure(self.pressure, capabilities, calibration),
            tilt: self.tilt.filter(|_| capabilities.tilt),
            twist_degrees: self.twist_degrees.filter(|_| capabilities.twist),
            tangential_pressure: self
                .tangential_pressure
                .filter(|_| capabilities.tangential_pressure),
            contact: if capabilities.hover {
                self.contact
            } else {
                PointerContactState::Contact
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NativeTabletPacket {
    pub platform: NativeTabletPlatform,
    pub vendor: NativeTabletVendor,
    pub backend: NativeTabletBackendKind,
    pub source_kind: PointerSourceKind,
    pub device_id: u64,
    pub kind: PointerEventKind,
    pub position: PointerPosition,
    pub delta: PointerDelta,
    pub event_button: Option<PointerButton>,
    pub modifiers: Modifiers,
    pub click_count: u8,
    pub tool_kind: NativeTabletToolKind,
    pub timestamp_micros: Option<u64>,
    pub contact: PointerContactState,
    pub pressure: Option<f32>,
    pub tilt: Option<PointerTilt>,
    pub twist_degrees: Option<f32>,
    pub tangential_pressure: Option<f32>,
    pub eraser: bool,
    pub barrel_buttons: PointerBarrelButtons,
    pub capabilities: NativeTabletCapabilities,
    pub calibration: Option<PointerCalibration>,
    pub latency_class: PointerLatencyClass,
    pub coalesced_samples: Vec<NativeTabletSample>,
    pub predicted_samples: Vec<NativeTabletSample>,
}

impl NativeTabletPacket {
    pub fn macos_wacom(
        device_id: u64,
        kind: PointerEventKind,
        position: PointerPosition,
        delta: PointerDelta,
    ) -> Self {
        Self::new(
            NativeTabletPlatform::Macos,
            NativeTabletVendor::Wacom,
            NativeTabletBackendKind::MacosWacomDriver,
            PointerSourceKind::Stylus,
            NativeTabletCapabilities::macos_wacom(),
            device_id,
            kind,
            position,
            delta,
        )
    }

    pub fn windows_pointer(
        device_id: u64,
        kind: PointerEventKind,
        position: PointerPosition,
        delta: PointerDelta,
    ) -> Self {
        Self::new(
            NativeTabletPlatform::Windows,
            NativeTabletVendor::Generic,
            NativeTabletBackendKind::WindowsPointer,
            PointerSourceKind::Stylus,
            NativeTabletCapabilities::windows_pointer(),
            device_id,
            kind,
            position,
            delta,
        )
    }

    pub fn windows_pointer_mouse(
        device_id: u64,
        kind: PointerEventKind,
        position: PointerPosition,
        delta: PointerDelta,
    ) -> Self {
        Self::new(
            NativeTabletPlatform::Windows,
            NativeTabletVendor::Generic,
            NativeTabletBackendKind::WindowsPointer,
            PointerSourceKind::Mouse,
            NativeTabletCapabilities::windows_pointer_mouse(),
            device_id,
            kind,
            position,
            delta,
        )
        .with_tool_kind(NativeTabletToolKind::Mouse)
    }

    pub fn windows_wintab(
        device_id: u64,
        kind: PointerEventKind,
        position: PointerPosition,
        delta: PointerDelta,
    ) -> Self {
        Self::new(
            NativeTabletPlatform::Windows,
            NativeTabletVendor::Wacom,
            NativeTabletBackendKind::WindowsWintab,
            PointerSourceKind::Stylus,
            NativeTabletCapabilities::windows_wintab(),
            device_id,
            kind,
            position,
            delta,
        )
    }

    pub fn macos_nsevent(
        device_id: u64,
        kind: PointerEventKind,
        position: PointerPosition,
        delta: PointerDelta,
    ) -> Self {
        Self::new(
            NativeTabletPlatform::Macos,
            NativeTabletVendor::Generic,
            NativeTabletBackendKind::MacosNsevent,
            PointerSourceKind::Stylus,
            NativeTabletCapabilities::macos_nsevent(),
            device_id,
            kind,
            position,
            delta,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        platform: NativeTabletPlatform,
        vendor: NativeTabletVendor,
        backend: NativeTabletBackendKind,
        source_kind: PointerSourceKind,
        capabilities: NativeTabletCapabilities,
        device_id: u64,
        kind: PointerEventKind,
        position: PointerPosition,
        delta: PointerDelta,
    ) -> Self {
        Self {
            platform,
            vendor,
            backend,
            source_kind,
            device_id,
            kind,
            position,
            delta,
            event_button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            tool_kind: NativeTabletToolKind::Pen,
            timestamp_micros: None,
            contact: PointerContactState::Contact,
            pressure: None,
            tilt: None,
            twist_degrees: None,
            tangential_pressure: None,
            eraser: false,
            barrel_buttons: PointerBarrelButtons::none(),
            capabilities,
            calibration: None,
            latency_class: PointerLatencyClass::Normal,
            coalesced_samples: Vec::new(),
            predicted_samples: Vec::new(),
        }
    }

    pub fn with_event_button(mut self, event_button: Option<PointerButton>) -> Self {
        self.event_button = event_button;
        self
    }

    pub fn with_modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    pub fn with_timestamp_micros(mut self, timestamp_micros: u64) -> Self {
        self.timestamp_micros = Some(timestamp_micros);
        self
    }

    pub fn with_tool_kind(mut self, tool_kind: NativeTabletToolKind) -> Self {
        self.tool_kind = tool_kind;
        self
    }

    pub fn with_source_kind(mut self, source_kind: PointerSourceKind) -> Self {
        self.source_kind = source_kind;
        self
    }

    pub fn with_pressure(mut self, pressure: f32) -> Self {
        self.pressure = Some(pressure);
        self
    }

    pub fn with_tilt(mut self, tilt: PointerTilt) -> Self {
        self.tilt = Some(tilt);
        self
    }

    pub fn with_twist_degrees(mut self, twist_degrees: f32) -> Self {
        self.twist_degrees = Some(twist_degrees);
        self
    }

    pub fn with_tangential_pressure(mut self, tangential_pressure: f32) -> Self {
        self.tangential_pressure = Some(tangential_pressure);
        self
    }

    pub fn with_contact(mut self, contact: PointerContactState) -> Self {
        self.contact = contact;
        self
    }

    pub fn with_eraser(mut self, eraser: bool) -> Self {
        self.eraser = eraser;
        if eraser {
            self.tool_kind = NativeTabletToolKind::Eraser;
        }
        self
    }

    pub fn with_barrel_buttons(mut self, barrel_buttons: PointerBarrelButtons) -> Self {
        self.barrel_buttons = barrel_buttons;
        self
    }

    pub fn with_capabilities(mut self, capabilities: NativeTabletCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_calibration(mut self, calibration: PointerCalibration) -> Self {
        self.calibration = Some(calibration);
        self
    }

    pub fn with_latency_class(mut self, latency_class: PointerLatencyClass) -> Self {
        self.latency_class = latency_class;
        self
    }

    pub fn with_coalesced_samples(
        mut self,
        samples: impl IntoIterator<Item = NativeTabletSample>,
    ) -> Self {
        self.coalesced_samples = samples.into_iter().collect();
        self
    }

    pub fn with_predicted_samples(
        mut self,
        samples: impl IntoIterator<Item = NativeTabletSample>,
    ) -> Self {
        self.predicted_samples = samples.into_iter().collect();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeTabletBackendStatus {
    Active,
    Available,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTabletBackendHealth {
    pub backend: NativeTabletBackendKind,
    pub status: NativeTabletBackendStatus,
    pub message: String,
}

impl NativeTabletBackendHealth {
    pub fn active(backend: NativeTabletBackendKind, message: impl Into<String>) -> Self {
        Self {
            backend,
            status: NativeTabletBackendStatus::Active,
            message: message.into(),
        }
    }

    pub fn available(backend: NativeTabletBackendKind, message: impl Into<String>) -> Self {
        Self {
            backend,
            status: NativeTabletBackendStatus::Available,
            message: message.into(),
        }
    }

    pub fn unavailable(backend: NativeTabletBackendKind, message: impl Into<String>) -> Self {
        Self {
            backend,
            status: NativeTabletBackendStatus::Unavailable,
            message: message.into(),
        }
    }

    pub fn error(backend: NativeTabletBackendKind, message: impl Into<String>) -> Self {
        Self {
            backend,
            status: NativeTabletBackendStatus::Error,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTabletDeviceDescriptor {
    pub device_id: PointerDeviceId,
    pub platform: NativeTabletPlatform,
    pub vendor: NativeTabletVendor,
    pub backend: NativeTabletBackendKind,
    pub name: String,
    pub capabilities: NativeTabletCapabilities,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeTabletSampleTelemetry {
    pub packets_this_frame: u32,
    pub samples_this_frame: u32,
    pub coalesced_samples_this_frame: u32,
    pub predicted_samples_this_frame: u32,
    pub dropped_samples_this_frame: u32,
    pub duplicate_samples_this_frame: u32,
    pub max_segment_gap_px: f32,
    pub sample_rate_hz: f32,
    pub pressure_available: bool,
    pub tilt_available: bool,
}

impl Default for NativeTabletSampleTelemetry {
    fn default() -> Self {
        Self {
            packets_this_frame: 0,
            samples_this_frame: 0,
            coalesced_samples_this_frame: 0,
            predicted_samples_this_frame: 0,
            dropped_samples_this_frame: 0,
            duplicate_samples_this_frame: 0,
            max_segment_gap_px: 0.0,
            sample_rate_hz: 0.0,
            pressure_available: false,
            tilt_available: false,
        }
    }
}

impl NativeTabletSampleTelemetry {
    pub fn observe_packet(
        &mut self,
        packet: &NativeTabletPacket,
        previous: Option<PointerPosition>,
    ) {
        self.packets_this_frame = self.packets_this_frame.saturating_add(1);
        let sample_count = 1usize
            .saturating_add(packet.coalesced_samples.len())
            .saturating_add(packet.predicted_samples.len());
        self.samples_this_frame = self.samples_this_frame.saturating_add(sample_count as u32);
        self.coalesced_samples_this_frame = self
            .coalesced_samples_this_frame
            .saturating_add(packet.coalesced_samples.len() as u32);
        self.predicted_samples_this_frame = self
            .predicted_samples_this_frame
            .saturating_add(packet.predicted_samples.len() as u32);
        self.pressure_available |= packet.capabilities.pressure && packet.pressure.is_some();
        self.tilt_available |= packet.capabilities.tilt && packet.tilt.is_some();

        let mut last_position = previous;
        for sample in packet
            .coalesced_samples
            .iter()
            .map(|sample| sample.position)
            .chain(std::iter::once(packet.position))
        {
            if let Some(last) = last_position {
                let dx = sample.x - last.x;
                let dy = sample.y - last.y;
                let gap = (dx * dx + dy * dy).sqrt();
                if gap <= f32::EPSILON {
                    self.duplicate_samples_this_frame =
                        self.duplicate_samples_this_frame.saturating_add(1);
                }
                self.max_segment_gap_px = self.max_segment_gap_px.max(gap);
            }
            last_position = Some(sample);
        }

        if let (Some(first), Some(last)) = (
            packet
                .coalesced_samples
                .first()
                .and_then(|sample| sample.timestamp_micros)
                .or(packet.timestamp_micros),
            packet.timestamp_micros,
        ) {
            let elapsed = last.saturating_sub(first);
            if elapsed > 0 {
                self.sample_rate_hz =
                    (sample_count.saturating_sub(1) as f32) * 1_000_000.0 / elapsed as f32;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, runen_ecs::Component, runen_ecs::Resource)]
pub struct NativeTabletDeviceControlResource {
    pub backend_preference: NativeTabletBackendPreference,
    pub calibration: PointerCalibration,
    pub suppress_winit_fallback_while_native_active: bool,
    pub reset_calibration_requested: bool,
}

impl Default for NativeTabletDeviceControlResource {
    fn default() -> Self {
        Self {
            backend_preference: NativeTabletBackendPreference::AutoOsFirst,
            calibration: PointerCalibration::identity(),
            suppress_winit_fallback_while_native_active: true,
            reset_calibration_requested: false,
        }
    }
}

impl NativeTabletDeviceControlResource {
    pub fn request_reset_calibration(&mut self) {
        self.reset_calibration_requested = true;
    }

    pub fn apply_pending_reset(&mut self) {
        if self.reset_calibration_requested {
            self.calibration = PointerCalibration::identity();
            self.reset_calibration_requested = false;
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, runen_ecs::Component, runen_ecs::Resource)]
pub struct NativeTabletFrameResource {
    pub events: Vec<UiInputEvent>,
    pub devices: Vec<NativeTabletDeviceDescriptor>,
    pub backend_health: Vec<NativeTabletBackendHealth>,
    pub telemetry: NativeTabletSampleTelemetry,
    pub diagnostics: Vec<NativeTabletDiagnostic>,
    pub active_native_contact: bool,
    pub frames_since_native_event: u32,
}

impl NativeTabletFrameResource {
    pub fn drain_events(&mut self) -> Vec<UiInputEvent> {
        std::mem::take(&mut self.events)
    }
}

#[derive(Debug, Clone, Default, PartialEq, runen_ecs::Component, runen_ecs::Resource)]
pub struct NativeTabletRuntimeResource {
    pending_packets: VecDeque<NativeTabletPacket>,
    pub devices: Vec<NativeTabletDeviceDescriptor>,
    pub backend_health: Vec<NativeTabletBackendHealth>,
    pub diagnostics: Vec<NativeTabletDiagnostic>,
    pub active_native_contact: bool,
    frames_since_native_event: u32,
    last_position: Option<PointerPosition>,
}

impl NativeTabletRuntimeResource {
    pub fn push_packet(&mut self, packet: NativeTabletPacket) {
        self.upsert_device(&packet, true);
        self.active_native_contact = matches!(packet.contact, PointerContactState::Contact)
            && !matches!(packet.kind, PointerEventKind::Up | PointerEventKind::Leave);
        if matches!(packet.kind, PointerEventKind::Up | PointerEventKind::Leave) {
            self.active_native_contact = false;
        }
        self.pending_packets.push_back(packet);
    }

    pub fn set_backend_health(&mut self, health: NativeTabletBackendHealth) {
        if let Some(existing) = self
            .backend_health
            .iter_mut()
            .find(|entry| entry.backend == health.backend)
        {
            *existing = health;
        } else {
            self.backend_health.push(health);
        }
    }

    pub fn push_diagnostic(&mut self, diagnostic: NativeTabletDiagnostic) {
        if !self.diagnostics.contains(&diagnostic) {
            self.diagnostics.push(diagnostic);
        }
    }

    pub fn publish_frame(
        &mut self,
        frame: &mut NativeTabletFrameResource,
        control: &mut NativeTabletDeviceControlResource,
    ) {
        control.apply_pending_reset();
        let mut telemetry = NativeTabletSampleTelemetry::default();
        let mut events = Vec::with_capacity(self.pending_packets.len());
        let mut diagnostics = self.diagnostics.clone();
        while let Some(packet) = self.pending_packets.pop_front() {
            telemetry.observe_packet(&packet, self.last_position);
            self.last_position = Some(packet.position);
            let mapping = crate::mapping::map_native_tablet_packet(&packet);
            diagnostics.extend(mapping.diagnostics);
            events.push(mapping.event);
        }
        if events.is_empty() {
            self.frames_since_native_event = self.frames_since_native_event.saturating_add(1);
        } else {
            self.frames_since_native_event = 0;
        }
        frame.events = events;
        frame.devices = self.devices.clone();
        frame.backend_health = self.backend_health.clone();
        frame.telemetry = telemetry;
        frame.diagnostics = diagnostics;
        frame.active_native_contact = self.active_native_contact;
        frame.frames_since_native_event = self.frames_since_native_event;
    }

    fn upsert_device(&mut self, packet: &NativeTabletPacket, active: bool) {
        let descriptor = NativeTabletDeviceDescriptor {
            device_id: PointerDeviceId(packet.device_id),
            platform: packet.platform,
            vendor: packet.vendor,
            backend: packet.backend,
            name: format!("{} device {}", packet.backend.label(), packet.device_id),
            capabilities: packet.capabilities,
            active,
        };
        if let Some(existing) = self
            .devices
            .iter_mut()
            .find(|device| device.device_id == descriptor.device_id)
        {
            *existing = descriptor;
        } else {
            self.devices.push(descriptor);
        }
    }
}

pub(crate) fn calibrated_position(
    position: PointerPosition,
    calibration: Option<PointerCalibration>,
) -> PointerPosition {
    let Some(calibration) = calibration else {
        return position;
    };
    PointerPosition::new(
        position.x + calibration.cursor_offset.x,
        position.y + calibration.cursor_offset.y,
    )
}

pub(crate) fn calibrated_pressure(
    pressure: Option<f32>,
    capabilities: NativeTabletCapabilities,
    calibration: Option<PointerCalibration>,
) -> Option<f32> {
    let pressure = pressure.filter(|_| capabilities.pressure)?;
    let Some(calibration) = calibration else {
        return Some(pressure);
    };
    Some((pressure * calibration.pressure_scale + calibration.pressure_bias).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_frame_tracks_idle_frames_after_active_native_contact() {
        let mut runtime = NativeTabletRuntimeResource::default();
        let mut frame = NativeTabletFrameResource::default();
        let mut control = NativeTabletDeviceControlResource::default();
        runtime.push_packet(NativeTabletPacket::windows_pointer(
            7,
            PointerEventKind::Down,
            PointerPosition::new(10.0, 20.0),
            PointerDelta::ZERO,
        ));

        runtime.publish_frame(&mut frame, &mut control);
        assert!(frame.active_native_contact);
        assert_eq!(frame.frames_since_native_event, 0);
        assert_eq!(frame.events.len(), 1);

        runtime.publish_frame(&mut frame, &mut control);
        assert!(frame.active_native_contact);
        assert_eq!(frame.frames_since_native_event, 1);
        assert!(frame.events.is_empty());
    }

    #[test]
    fn publish_frame_resets_idle_frames_when_native_events_resume() {
        let mut runtime = NativeTabletRuntimeResource::default();
        let mut frame = NativeTabletFrameResource::default();
        let mut control = NativeTabletDeviceControlResource::default();
        runtime.push_packet(NativeTabletPacket::windows_pointer(
            7,
            PointerEventKind::Down,
            PointerPosition::new(10.0, 20.0),
            PointerDelta::ZERO,
        ));
        runtime.publish_frame(&mut frame, &mut control);
        runtime.publish_frame(&mut frame, &mut control);
        assert_eq!(frame.frames_since_native_event, 1);

        runtime.push_packet(NativeTabletPacket::windows_pointer(
            7,
            PointerEventKind::Move,
            PointerPosition::new(12.0, 22.0),
            PointerDelta::new(2.0, 2.0),
        ));
        runtime.publish_frame(&mut frame, &mut control);

        assert_eq!(frame.frames_since_native_event, 0);
        assert_eq!(frame.events.len(), 1);
    }
}
