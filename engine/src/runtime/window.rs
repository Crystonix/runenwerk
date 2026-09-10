use id_macros::id;
use std::collections::BTreeMap;

#[id]
pub struct NativeWindowId;

impl NativeWindowId {
    pub fn primary() -> Self {
        Self::try_from_raw(1).expect("primary native window id must be non-zero")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowCursorIcon {
    Default,
    ColResize,
    RowResize,
    NwseResize,
    NeswResize,
    Grab,
    Grabbing,
}

#[derive(Debug, Clone, runen_ecs::Component, runen_ecs::Resource)]
pub struct WindowState {
    pub title: String,
    pub size_px: (u32, u32),
    pub scale_factor: f64,
    pub close_requested: bool,
    pub close_intent_pending: bool,
    pub focused: bool,
    pub redraw_requested: bool,
    pub cursor_icon: WindowCursorIcon,
    headless: bool,
}

impl WindowState {
    pub fn windowed(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            size_px: (1280, 720),
            scale_factor: 1.0,
            close_requested: false,
            close_intent_pending: false,
            focused: true,
            redraw_requested: true,
            cursor_icon: WindowCursorIcon::Default,
            headless: false,
        }
    }

    pub fn headless(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            size_px: (1280, 720),
            scale_factor: 1.0,
            close_requested: false,
            close_intent_pending: false,
            focused: true,
            redraw_requested: false,
            cursor_icon: WindowCursorIcon::Default,
            headless: true,
        }
    }

    pub fn request_close(&mut self) {
        self.close_requested = true;
        self.close_intent_pending = false;
    }

    pub fn receive_close_intent(&mut self) {
        self.close_requested = false;
        self.close_intent_pending = true;
    }

    pub fn veto_close(&mut self) {
        self.close_requested = false;
        self.close_intent_pending = false;
    }

    pub fn request_redraw(&mut self) {
        self.redraw_requested = true;
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
        self.request_redraw();
    }

    pub fn set_cursor_icon(&mut self, cursor_icon: WindowCursorIcon) {
        if self.cursor_icon != cursor_icon {
            self.cursor_icon = cursor_icon;
            self.request_redraw();
        }
    }

    pub fn is_headless(&self) -> bool {
        self.headless
    }

    pub(crate) fn set_headless(&mut self, headless: bool) {
        self.headless = headless;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeWindowLifecycleState {
    Requested,
    Created,
    CreationFailed,
    CloseIntentPending,
    CloseApproved,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NativeWindowCreationRequest {
    pub native_window_id: NativeWindowId,
    pub title: String,
    pub size_px: (u32, u32),
}

impl NativeWindowCreationRequest {
    pub fn new(
        native_window_id: NativeWindowId,
        title: impl Into<String>,
        size_px: (u32, u32),
    ) -> Self {
        Self {
            native_window_id,
            title: title.into(),
            size_px: (size_px.0.max(1), size_px.1.max(1)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NativeWindowRecord {
    pub native_window_id: NativeWindowId,
    pub title: String,
    pub size_px: (u32, u32),
    pub scale_factor: f64,
    pub close_requested: bool,
    pub close_intent_pending: bool,
    pub focused: bool,
    pub redraw_requested: bool,
    pub cursor_icon: WindowCursorIcon,
    pub headless: bool,
    pub lifecycle_state: NativeWindowLifecycleState,
    pub failure_reason: Option<String>,
}

impl NativeWindowRecord {
    pub fn from_legacy(native_window_id: NativeWindowId, window: &WindowState) -> Self {
        Self {
            native_window_id,
            title: window.title.clone(),
            size_px: window.size_px,
            scale_factor: window.scale_factor,
            close_requested: window.close_requested,
            close_intent_pending: window.close_intent_pending,
            focused: window.focused,
            redraw_requested: window.redraw_requested,
            cursor_icon: window.cursor_icon,
            headless: window.is_headless(),
            lifecycle_state: if window.close_requested {
                NativeWindowLifecycleState::CloseApproved
            } else if window.close_intent_pending {
                NativeWindowLifecycleState::CloseIntentPending
            } else {
                NativeWindowLifecycleState::Created
            },
            failure_reason: None,
        }
    }

    pub fn request_redraw(&mut self) {
        self.redraw_requested = true;
    }

    pub fn request_close(&mut self) {
        self.approve_close();
    }

    pub fn receive_close_intent(&mut self) {
        self.close_requested = false;
        self.close_intent_pending = true;
        self.lifecycle_state = NativeWindowLifecycleState::CloseIntentPending;
    }

    pub fn approve_close(&mut self) {
        self.close_requested = true;
        self.close_intent_pending = false;
        self.lifecycle_state = NativeWindowLifecycleState::CloseApproved;
    }

    pub fn veto_close(&mut self) {
        self.close_requested = false;
        self.close_intent_pending = false;
        self.lifecycle_state = NativeWindowLifecycleState::Created;
    }

    pub fn mark_creation_failed(&mut self, reason: impl Into<String>) {
        self.close_requested = false;
        self.close_intent_pending = false;
        self.lifecycle_state = NativeWindowLifecycleState::CreationFailed;
        self.failure_reason = Some(reason.into());
    }

    pub fn copy_to_legacy(&self, window: &mut WindowState) {
        window.title = self.title.clone();
        window.size_px = self.size_px;
        window.scale_factor = self.scale_factor;
        window.close_requested = self.close_requested;
        window.close_intent_pending = self.close_intent_pending;
        window.focused = self.focused;
        window.redraw_requested = self.redraw_requested;
        window.cursor_icon = self.cursor_icon;
        window.set_headless(self.headless);
    }
}

#[derive(Debug, Clone, Default, runen_ecs::Component, runen_ecs::Resource)]
pub struct WindowStateRegistryResource {
    primary_window_id: Option<NativeWindowId>,
    next_window_raw: u64,
    records: BTreeMap<NativeWindowId, NativeWindowRecord>,
    pending_creation_requests: Vec<NativeWindowCreationRequest>,
}

impl WindowStateRegistryResource {
    pub fn from_legacy(window: &WindowState) -> Self {
        let primary_window_id = NativeWindowId::primary();
        Self {
            primary_window_id: Some(primary_window_id),
            next_window_raw: primary_window_id.raw().saturating_add(1),
            records: BTreeMap::from([(
                primary_window_id,
                NativeWindowRecord::from_legacy(primary_window_id, window),
            )]),
            pending_creation_requests: Vec::new(),
        }
    }

    pub fn ensure_primary_from_legacy(&mut self, window: &WindowState) -> NativeWindowId {
        let primary_window_id = self
            .primary_window_id
            .unwrap_or_else(NativeWindowId::primary);
        self.primary_window_id = Some(primary_window_id);
        self.next_window_raw = self
            .next_window_raw
            .max(primary_window_id.raw().saturating_add(1));
        self.records.insert(
            primary_window_id,
            NativeWindowRecord::from_legacy(primary_window_id, window),
        );
        primary_window_id
    }

    pub fn primary_window_id(&self) -> Option<NativeWindowId> {
        self.primary_window_id
    }

    pub fn record(&self, native_window_id: NativeWindowId) -> Option<&NativeWindowRecord> {
        self.records.get(&native_window_id)
    }

    pub fn record_mut(
        &mut self,
        native_window_id: NativeWindowId,
    ) -> Option<&mut NativeWindowRecord> {
        self.records.get_mut(&native_window_id)
    }

    pub fn records(&self) -> impl Iterator<Item = &NativeWindowRecord> {
        self.records.values()
    }

    pub fn request_window(
        &mut self,
        title: impl Into<String>,
        size_px: (u32, u32),
    ) -> NativeWindowCreationRequest {
        let native_window_id = self.allocate_window_id();
        let request = NativeWindowCreationRequest::new(native_window_id, title, size_px);
        self.records.insert(
            native_window_id,
            NativeWindowRecord {
                native_window_id,
                title: request.title.clone(),
                size_px: request.size_px,
                scale_factor: 1.0,
                close_requested: false,
                close_intent_pending: false,
                focused: false,
                redraw_requested: true,
                cursor_icon: WindowCursorIcon::Default,
                headless: false,
                lifecycle_state: NativeWindowLifecycleState::Requested,
                failure_reason: None,
            },
        );
        self.pending_creation_requests.push(request.clone());
        request
    }

    pub fn register_created_window(
        &mut self,
        native_window_id: NativeWindowId,
        window: &WindowState,
    ) {
        let mut record = NativeWindowRecord::from_legacy(native_window_id, window);
        record.lifecycle_state = NativeWindowLifecycleState::Created;
        self.records.insert(native_window_id, record);
        self.next_window_raw = self
            .next_window_raw
            .max(native_window_id.raw().saturating_add(1));
        if self.primary_window_id.is_none() {
            self.primary_window_id = Some(native_window_id);
        }
    }

    pub fn take_pending_creation_requests(&mut self) -> Vec<NativeWindowCreationRequest> {
        std::mem::take(&mut self.pending_creation_requests)
    }

    pub fn pending_creation_requests(&self) -> &[NativeWindowCreationRequest] {
        &self.pending_creation_requests
    }

    pub fn remove_window(
        &mut self,
        native_window_id: NativeWindowId,
    ) -> Option<NativeWindowRecord> {
        self.pending_creation_requests
            .retain(|request| request.native_window_id != native_window_id);
        let removed = self.records.remove(&native_window_id);
        if self.primary_window_id == Some(native_window_id) {
            self.primary_window_id = self.records.keys().next().copied();
        }
        removed
    }

    fn allocate_window_id(&mut self) -> NativeWindowId {
        loop {
            let raw = self.next_window_raw.max(1);
            self.next_window_raw = raw.saturating_add(1);
            if let Ok(id) = NativeWindowId::try_from_raw(raw)
                && !self.records.contains_key(&id)
            {
                return id;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_registry_seeds_primary_from_legacy_window() {
        let mut legacy = WindowState::windowed("Runtime");
        legacy.size_px = (1440, 900);

        let registry = WindowStateRegistryResource::from_legacy(&legacy);
        let primary = registry
            .primary_window_id()
            .expect("primary native window should be present");
        let record = registry
            .record(primary)
            .expect("primary native window record should be present");

        assert_eq!(primary, NativeWindowId::primary());
        assert_eq!(record.title, "Runtime");
        assert_eq!(record.size_px, (1440, 900));
        assert_eq!(record.lifecycle_state, NativeWindowLifecycleState::Created);
    }

    #[test]
    fn window_registry_tracks_pending_secondary_window_requests() {
        let legacy = WindowState::windowed("Runtime");
        let mut registry = WindowStateRegistryResource::from_legacy(&legacy);

        let request = registry.request_window("Secondary", (640, 480));

        assert_ne!(request.native_window_id, NativeWindowId::primary());
        assert_eq!(request.title, "Secondary");
        assert_eq!(
            registry.pending_creation_requests(),
            std::slice::from_ref(&request)
        );
        assert_eq!(
            registry
                .record(request.native_window_id)
                .map(|record| record.lifecycle_state),
            Some(NativeWindowLifecycleState::Requested)
        );
    }
}
