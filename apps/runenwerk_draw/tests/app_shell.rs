use std::collections::BTreeSet;

use drawing::{
    CanvasCoordinate, CanvasTileId, DrawingTileFormationDiagnostic,
    DrawingTileFormationDiagnosticCode, DrawingTileFormationPolicy, ProductQualityClass, StrokeId,
    StrokeToolKind, form_drawing_ink_tiles, ratify_drawing_document,
};
use engine::SceneRuntimeState;
use engine::plugins::render::inspect::RenderDebugConfigResource;
use engine::plugins::render::{
    FeatureContributionStatus, PreparedRenderFrameRequestResource,
    PreparedRenderProductSelectionResource, PreparedUiFrameResource,
    RenderDynamicTextureTargetRequestRegistryResource, RenderDynamicTextureUploadRegistryResource,
    SurfaceFrameSubmissionRegistryResource,
};
use engine::plugins::{InputState, TouchInputPhase};
use engine::runtime::{
    ProductPublicationRuntimeResource, PublicationBoundary, QuerySnapshotRuntimeResource,
    RuntimeJobExecutorConfig, RuntimeJobExecutorResource, RuntimeProductCacheResource,
};
use native_tablet_input::{
    NativeTabletBackendHealth, NativeTabletBackendKind, NativeTabletFrameResource,
    NativeTabletPacket, NativeTabletSample, map_native_tablet_packet,
};
use product::ProductScaleBand;
use runenwerk_draw::app::{
    DRAWING_UI_SURFACE_ID, DrawingInkGpuValidationMetrics, DrawingInkRuntimeState,
    DrawingInkSurfaceKind, DrawingToolRouteKind, RunenwerkDrawApp, minimal_drawing_document,
};
use runenwerk_draw::runtime::{
    DRAWING_UI_FRAME_PRODUCER_ID, DrawingHostResource, DrawingInkGpuFlowResource,
    build_headless_app, process_drawing_preview_ink_jobs, publish_drawing_ink_products,
    publish_drawing_ink_products_with_executor_and_cache, publish_drawing_ink_query_snapshots,
};
use ui_input::{
    Key, KeyState, KeyboardEvent, Modifiers, PointerButton, PointerContactState, PointerDeviceId,
    PointerEvent, PointerEventKind, PointerLatencyClass, PointerPacket, PointerSample,
    PointerSampleRole, PointerSourceKind, PointerTilt, PointerToolKind, TextInputEvent,
    UiInputEvent,
};
use ui_math::{UiPoint, UiVector};
use ui_render_data::{ProductSurfaceTextureBindingSource, UiPrimitive};
use winit::event::{ElementState, MouseButton as WinitMouseButton};

#[test]
fn minimal_document_opens_as_the_app_shell_document() {
    let document = minimal_drawing_document();
    let report = ratify_drawing_document(&document);
    assert!(report.is_accepted(), "{report:?}");

    let app = RunenwerkDrawApp::new();
    let document = app.document().expect("drawing app should open a document");

    assert_eq!(document.display_name, "Untitled Drawing");
    assert_eq!(document.strokes.len(), 0);
    assert_eq!(document.pending_strokes.len(), 0);
    assert_eq!(app.active_brush_id().raw(), 1);
    assert_eq!(app.active_layer_entry_id().raw(), 1);
    assert!(app.composition_projection().canvas_area_ratio() > 0.45);
    assert!(!app.last_frame().is_empty());
}

#[test]
fn stylus_input_routes_to_preview_stroke_without_committing_document_truth() {
    let mut app = RunenwerkDrawApp::new();
    let position = center_of_canvas(app.composition_projection().canvas_view.screen_bounds);
    let shell_rect_count = rect_primitive_count(app.last_frame());

    let packet = PointerPacket::stylus(PointerDeviceId(42), PointerToolKind::Pen)
        .with_timestamp_micros(10_000)
        .with_pressure(0.72)
        .with_tilt(PointerTilt::new(15.0, -20.0))
        .with_twist_degrees(120.0)
        .with_latency_class(PointerLatencyClass::LowLatencyPreview);

    let handled = app.dispatch_input(&UiInputEvent::Pointer(
        PointerEvent::new(
            PointerEventKind::Down,
            position,
            UiVector::ZERO,
            Some(PointerButton::Primary),
            Modifiers::default(),
            1,
        )
        .with_packet(packet),
    ));

    assert!(handled);
    assert_eq!(app.routed_inputs().len(), 1);
    let routed = &app.routed_inputs()[0];
    assert_eq!(routed.route_kind, DrawingToolRouteKind::BeginPreviewStroke);
    assert_eq!(routed.pressure, Some(0.72));
    assert_eq!(routed.twist_degrees, Some(120.0));
    assert_eq!(routed.device_id, Some(PointerDeviceId(42)));
    assert!(routed.low_latency_preview);

    let preview = app
        .preview_stroke()
        .expect("pointer down should create a preview stroke");
    assert!(preview.active);
    assert_eq!(preview.samples.len(), 1);
    assert_eq!(preview.samples[0].pressure, Some(0.72));
    assert_eq!(preview.samples[0].tool_kind, Some(StrokeToolKind::Pen));
    assert_eq!(preview.samples[0].timestamp_micros, Some(10_000));
    assert_eq!(
        app.document()
            .expect("document should remain open")
            .strokes
            .len(),
        0
    );
    assert!(app.ink_runtime().preview_products().is_empty());
    assert_eq!(
        app.ink_runtime().last_preview_dirty_tile_count(),
        0,
        "dispatch_input must not form preview ink tiles synchronously"
    );
    assert!(
        stroke_primitive_count(app.last_frame()) > 0,
        "active preview should use an immediate stroke primitive"
    );
    assert_eq!(
        rect_primitive_count(app.last_frame()),
        shell_rect_count,
        "ink preview must not use the old per-pixel rect projection path"
    );
    assert_eq!(
        product_surface_primitive_count(app.last_frame()),
        0,
        "preview tile catch-up must not run on the input hot path"
    );
}

#[test]
fn hover_scroll_and_ignored_inputs_do_not_mutate_document_or_preview_state() {
    let mut app = RunenwerkDrawApp::new();
    let position = center_of_canvas(app.composition_projection().canvas_view.screen_bounds);
    let outside = UiPoint::new(
        app.composition_projection().canvas_view.screen_bounds.x - 32.0,
        app.composition_projection().canvas_view.screen_bounds.y - 32.0,
    );
    let initial_revision = app.document().unwrap().revision;
    let initial_stroke_count = app.document().unwrap().strokes.len();

    let hover = PointerEvent::new(
        PointerEventKind::Move,
        position,
        UiVector::ZERO,
        None,
        Modifiers::default(),
        0,
    )
    .with_packet(
        PointerPacket::stylus(PointerDeviceId(42), PointerToolKind::Pen)
            .with_contact(PointerContactState::Hover),
    );
    assert!(app.dispatch_input(&UiInputEvent::Pointer(hover)));

    let scroll = PointerEvent::new(
        PointerEventKind::Scroll,
        position,
        UiVector::new(0.0, -18.0),
        None,
        Modifiers::default(),
        0,
    );
    assert!(app.dispatch_input(&UiInputEvent::Pointer(scroll)));

    let enter = PointerEvent::new(
        PointerEventKind::Enter,
        position,
        UiVector::ZERO,
        None,
        Modifiers::default(),
        0,
    );
    assert!(!app.dispatch_input(&UiInputEvent::Pointer(enter)));

    let leave = PointerEvent::new(
        PointerEventKind::Leave,
        position,
        UiVector::ZERO,
        None,
        Modifiers::default(),
        0,
    );
    assert!(!app.dispatch_input(&UiInputEvent::Pointer(leave)));

    let outside_down = PointerEvent::new(
        PointerEventKind::Down,
        outside,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        1,
    );
    assert!(!app.dispatch_input(&UiInputEvent::Pointer(outside_down)));

    let document = app.document().expect("document should remain open");
    assert_eq!(document.revision, initial_revision);
    assert_eq!(document.strokes.len(), initial_stroke_count);
    assert!(app.preview_stroke().is_none());
    assert_eq!(app.preview_generation(), 0);
    assert_eq!(app.preview_dirty_start_sample_index(), None);
    assert!(app.pending_preview_tile_job().is_none());
    assert!(app.ink_runtime().preview_products().is_empty());
}

#[test]
fn keyboard_control_input_is_observed_without_default_bindings_or_mutation() {
    let mut app = RunenwerkDrawApp::new();
    let initial_revision = app.document().unwrap().revision;

    let handled = app.dispatch_input(&UiInputEvent::Keyboard(KeyboardEvent {
        key: Key::Escape,
        state: KeyState::Pressed,
        modifiers: Modifiers {
            ctrl: true,
            ..Modifiers::default()
        },
    }));

    assert!(
        !handled,
        "keyboard observation is inert and must not stop input propagation in this slice"
    );
    assert_eq!(app.document().unwrap().revision, initial_revision);
    assert_eq!(app.document().unwrap().strokes.len(), 0);
    assert!(app.routed_inputs().is_empty());
    assert!(app.preview_stroke().is_none());
    assert_eq!(app.preview_generation(), 0);
    assert_eq!(app.preview_dirty_start_sample_index(), None);
    assert!(app.pending_preview_tile_job().is_none());
    assert!(app.ink_runtime().preview_products().is_empty());

    let start = screen_point_for_canvas(&app, 512.0, 512.0);
    let end = screen_point_for_canvas(&app, 620.0, 560.0);
    draw_stroke(&mut app, start, end);
    let document = app.document().expect("document should remain open");
    assert_eq!(
        document.strokes[0].stroke_id,
        StrokeId::new(1),
        "control input must not participate in drawing command identity"
    );

    let mut control_app = RunenwerkDrawApp::new();
    let control_start = screen_point_for_canvas(&control_app, 512.0, 512.0);
    let control_end = screen_point_for_canvas(&control_app, 620.0, 560.0);
    draw_stroke(&mut control_app, control_start, control_end);

    let formation = form_drawing_ink_tiles(document, DrawingTileFormationPolicy::preview());
    let control_formation = form_drawing_ink_tiles(
        control_app
            .document()
            .expect("control document should remain open"),
        DrawingTileFormationPolicy::preview(),
    );
    assert_eq!(
        formation.products, control_formation.products,
        "control input must not participate in product or cache identity"
    );
    assert_eq!(formation.determinism_key, control_formation.determinism_key);
}

#[test]
fn text_input_remains_noop_for_draw_tool_control() {
    let mut app = RunenwerkDrawApp::new();
    let initial_revision = app.document().unwrap().revision;

    let handled = app.dispatch_input(&UiInputEvent::Text(TextInputEvent {
        text: "draw".to_string(),
    }));

    assert!(!handled);
    assert_eq!(app.document().unwrap().revision, initial_revision);
    assert_eq!(app.document().unwrap().strokes.len(), 0);
    assert!(app.routed_inputs().is_empty());
    assert!(app.preview_stroke().is_none());
    assert_eq!(app.preview_generation(), 0);
    assert_eq!(app.preview_dirty_start_sample_index(), None);
    assert!(app.pending_preview_tile_job().is_none());
    assert!(app.ink_runtime().preview_products().is_empty());
}

#[test]
fn coalesced_pointer_samples_append_ordered_preview_samples_before_current_sample() {
    let mut app = RunenwerkDrawApp::new();
    let position = center_of_canvas(app.composition_projection().canvas_view.screen_bounds);
    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Down,
        position,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        1,
    )));

    let packet = PointerPacket::stylus(PointerDeviceId(7), PointerToolKind::Pen)
        .with_coalesced_samples([
            PointerSample::new(
                PointerSampleRole::Coalesced,
                UiPoint::new(position.x + 4.0, position.y + 1.0),
                UiVector::new(4.0, 1.0),
            )
            .with_timestamp_micros(10_100)
            .with_pressure(0.4),
            PointerSample::new(
                PointerSampleRole::Coalesced,
                UiPoint::new(position.x + 8.0, position.y + 3.0),
                UiVector::new(4.0, 2.0),
            )
            .with_timestamp_micros(10_200)
            .with_pressure(0.6),
        ])
        .with_timestamp_micros(10_300)
        .with_pressure(0.8);

    app.dispatch_input(&UiInputEvent::Pointer(
        PointerEvent::new(
            PointerEventKind::Move,
            UiPoint::new(position.x + 12.0, position.y + 6.0),
            UiVector::new(4.0, 3.0),
            Some(PointerButton::Primary),
            Modifiers::default(),
            0,
        )
        .with_packet(packet),
    ));

    let preview = app
        .preview_stroke()
        .expect("move should update the active preview stroke");
    assert_eq!(
        preview.samples.len(),
        4,
        "down sample plus two coalesced samples plus the current move sample should be retained"
    );
    assert_eq!(preview.samples[0].sequence, 1);
    assert_eq!(preview.samples[1].sequence, 2);
    assert_eq!(preview.samples[2].sequence, 3);
    assert_eq!(preview.samples[3].sequence, 4);
    assert_eq!(preview.samples[1].timestamp_micros, Some(10_100));
    assert_eq!(preview.samples[2].timestamp_micros, Some(10_200));
    assert_eq!(preview.samples[3].timestamp_micros, Some(10_300));
    assert_eq!(preview.samples[1].pressure, Some(0.4));
    assert_eq!(preview.samples[2].pressure, Some(0.6));
    assert_eq!(preview.samples[3].pressure, Some(0.8));
    assert_eq!(app.routed_inputs()[1].coalesced_sample_count, 2);
    assert_eq!(app.routed_inputs()[1].coalesced_samples.len(), 2);
}

#[test]
fn winit_fallback_press_motion_starts_at_contact_position_without_precontact_samples() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let (pre_contact, contact, after_contact, current) = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        (
            screen_point_for_canvas(&host.app, 480.0, 480.0),
            screen_point_for_canvas(&host.app, 512.0, 512.0),
            screen_point_for_canvas(&host.app, 560.0, 540.0),
            screen_point_for_canvas(&host.app, 620.0, 580.0),
        )
    };
    {
        let input = runtime
            .world_mut()
            .resource_mut::<InputState>()
            .expect("input state should exist");
        input.handle_cursor_moved(pre_contact.x, pre_contact.y);
        input.handle_cursor_moved(contact.x, contact.y);
        input.handle_mouse_input(ElementState::Pressed, WinitMouseButton::Left);
        input.handle_cursor_moved(after_contact.x, after_contact.y);
        input.handle_cursor_moved(current.x, current.y);
    }

    runtime = runtime
        .run_for_frames(1)
        .expect("winit fallback press/motion frame should run");

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    let preview = host
        .app
        .preview_stroke()
        .expect("fallback press should start a preview stroke");
    assert_eq!(
        preview.samples.len(),
        3,
        "pre-contact cursor samples must not be appended after the down sample"
    );
    assert_canvas_position_close(preview.samples[0].position, 512.0, 512.0);
    assert_canvas_position_close(preview.samples[1].position, 560.0, 540.0);
    assert_canvas_position_close(preview.samples[2].position, 620.0, 580.0);
    assert_eq!(preview.samples[0].sequence, 1);
    assert_eq!(preview.samples[1].sequence, 2);
    assert_eq!(preview.samples[2].sequence, 3);
    assert_eq!(
        host.app.routed_inputs()[0].route_kind,
        DrawingToolRouteKind::BeginPreviewStroke
    );
    assert_eq!(
        host.app.routed_inputs()[1].route_kind,
        DrawingToolRouteKind::UpdatePreviewStroke
    );
    assert_eq!(host.app.routed_inputs()[1].coalesced_sample_count, 1);
    assert_eq!(host.app.routed_inputs()[1].coalesced_samples.len(), 1);
}

#[test]
fn winit_fallback_release_motion_ignores_post_release_samples() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let (start, release, post_release) = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        (
            screen_point_for_canvas(&host.app, 512.0, 512.0),
            screen_point_for_canvas(&host.app, 620.0, 560.0),
            screen_point_for_canvas(&host.app, 740.0, 640.0),
        )
    };
    {
        let input = runtime
            .world_mut()
            .resource_mut::<InputState>()
            .expect("input state should exist");
        input.handle_cursor_moved(start.x, start.y);
        input.handle_mouse_input(ElementState::Pressed, WinitMouseButton::Left);
    }
    runtime = runtime
        .run_for_frames(1)
        .expect("winit fallback press frame should run");
    {
        let input = runtime
            .world_mut()
            .resource_mut::<InputState>()
            .expect("input state should exist");
        input.handle_cursor_moved(release.x, release.y);
        input.handle_mouse_input(ElementState::Released, WinitMouseButton::Left);
        input.handle_cursor_moved(post_release.x, post_release.y);
    }

    runtime = runtime
        .run_for_frames(1)
        .expect("winit fallback release/motion frame should run");

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    let preview = host
        .app
        .preview_stroke()
        .expect("fallback release should keep the released preview available");
    assert!(!preview.active);
    assert_eq!(
        preview.samples.len(),
        2,
        "post-release cursor samples must not be appended to the committed stroke"
    );
    assert_canvas_position_close(preview.samples[0].position, 512.0, 512.0);
    assert_canvas_position_close(preview.samples[1].position, 620.0, 560.0);
    assert_eq!(
        host.app
            .document()
            .expect("document should remain open")
            .strokes
            .len(),
        1,
        "release should still commit exactly one stroke"
    );
    assert_eq!(
        host.app
            .routed_inputs()
            .last()
            .expect("release should route an input")
            .route_kind,
        DrawingToolRouteKind::EndPreviewStroke
    );
}

#[test]
fn active_stroke_continues_outside_canvas_and_commits_on_outside_release() {
    let mut app = RunenwerkDrawApp::new();
    let start = screen_point_for_canvas(&app, 64.0, 64.0);
    let outside = app
        .composition_projection()
        .canvas_view
        .canvas_to_screen(CanvasCoordinate::new(-128.0, -96.0))
        .expect("unbounded canvas point should project to screen space");
    let reenter = screen_point_for_canvas(&app, 128.0, 128.0);

    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Down,
        start,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        1,
    )));
    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Move,
        outside,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        0,
    )));
    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Move,
        reenter,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        0,
    )));
    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Up,
        outside,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        0,
    )));

    assert_eq!(
        app.document()
            .expect("document should remain open")
            .strokes
            .len(),
        1,
        "release outside the canvas should commit the captured stroke"
    );
    let preview = app
        .preview_stroke()
        .expect("captured release should keep the released preview available");
    assert!(!preview.active);
    assert_eq!(preview.samples.len(), 4);
    assert!(
        preview.samples[1].position.x < 0.0 && preview.samples[1].position.y < 0.0,
        "the outside-canvas move should remain part of the same stroke"
    );
    assert_eq!(
        app.routed_inputs()[1].route_kind,
        DrawingToolRouteKind::UpdatePreviewStroke
    );
    assert_eq!(
        app.routed_inputs()[3].route_kind,
        DrawingToolRouteKind::EndPreviewStroke
    );
}

#[test]
fn window_touch_history_routes_as_ordered_preview_samples() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let (start, mid, end) = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        (
            screen_point_for_canvas(&host.app, 800.0, 800.0),
            screen_point_for_canvas(&host.app, 900.0, 920.0),
            screen_point_for_canvas(&host.app, 1_000.0, 980.0),
        )
    };
    {
        let input = runtime
            .world_mut()
            .resource_mut::<InputState>()
            .expect("input state should exist");
        input.handle_touch_input(TouchInputPhase::Started, 91, start.x, start.y, Some(0.4));
        input.handle_touch_input(TouchInputPhase::Moved, 91, mid.x, mid.y, Some(0.6));
        input.handle_touch_input(TouchInputPhase::Moved, 91, end.x, end.y, Some(0.8));
    }

    runtime = runtime
        .run_for_frames(1)
        .expect("touch history route frame should run");

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    let preview = host
        .app
        .preview_stroke()
        .expect("touch input should create a preview stroke");
    assert_eq!(
        preview.samples.len(),
        3,
        "touch history should be routed as down plus coalesced move samples"
    );
    assert_eq!(preview.samples[0].pressure, Some(0.4));
    assert_eq!(preview.samples[1].pressure, Some(0.6));
    assert_eq!(preview.samples[2].pressure, Some(0.8));
    assert_eq!(preview.samples[0].sequence, 1);
    assert_eq!(preview.samples[1].sequence, 2);
    assert_eq!(preview.samples[2].sequence, 3);
    assert_eq!(host.app.routed_inputs().len(), 2);
    assert_eq!(host.app.routed_inputs()[1].coalesced_sample_count, 1);
}

#[test]
fn native_tablet_move_burst_routes_as_one_coalesced_preview_update() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let positions = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        [
            screen_point_for_canvas(&host.app, 600.0, 600.0),
            screen_point_for_canvas(&host.app, 650.0, 620.0),
            screen_point_for_canvas(&host.app, 700.0, 650.0),
            screen_point_for_canvas(&host.app, 760.0, 700.0),
        ]
    };

    {
        let native_frame = runtime
            .world_mut()
            .resource_mut::<NativeTabletFrameResource>()
            .expect("native tablet frame resource should exist");
        native_frame.events.push(stylus_pointer_event(
            PointerEventKind::Down,
            positions[0],
            10_000,
            0.3,
        ));
        native_frame.events.push(stylus_pointer_event(
            PointerEventKind::Move,
            positions[1],
            10_100,
            0.5,
        ));
        native_frame.events.push(stylus_pointer_event(
            PointerEventKind::Move,
            positions[2],
            10_200,
            0.7,
        ));
        native_frame.events.push(stylus_pointer_event(
            PointerEventKind::Move,
            positions[3],
            10_300,
            0.9,
        ));
    }

    runtime = runtime
        .run_for_frames(1)
        .expect("native tablet move burst route frame should run");

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    assert_eq!(
        host.app.routed_inputs().len(),
        2,
        "same-frame native move bursts should be routed as one coalesced move"
    );
    assert_eq!(host.app.routed_inputs()[1].coalesced_sample_count, 2);
    let preview = host
        .app
        .preview_stroke()
        .expect("native burst should start a preview stroke");
    assert_eq!(preview.samples.len(), 4);
    assert_eq!(preview.samples[1].timestamp_micros, Some(10_100));
    assert_eq!(preview.samples[2].timestamp_micros, Some(10_200));
    assert_eq!(preview.samples[3].timestamp_micros, Some(10_300));
    assert_eq!(preview.samples[1].pressure, Some(0.5));
    assert_eq!(preview.samples[2].pressure, Some(0.7));
    assert_eq!(preview.samples[3].pressure, Some(0.9));
}

#[test]
fn native_windows_pointer_mouse_history_routes_before_winit_mouse_fallback() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let (start, c1, c2, current) = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        (
            screen_point_for_canvas(&host.app, 512.0, 512.0),
            screen_point_for_canvas(&host.app, 540.0, 530.0),
            screen_point_for_canvas(&host.app, 580.0, 548.0),
            screen_point_for_canvas(&host.app, 620.0, 560.0),
        )
    };
    {
        let input = runtime
            .world_mut()
            .resource_mut::<InputState>()
            .expect("input state should exist");
        input.handle_cursor_moved(start.x, start.y);
        input.handle_mouse_input(ElementState::Pressed, WinitMouseButton::Left);
        input.handle_cursor_moved(current.x, current.y);
    }
    {
        let native_frame = runtime
            .world_mut()
            .resource_mut::<NativeTabletFrameResource>()
            .expect("native tablet frame resource should exist");
        let down = NativeTabletPacket::windows_pointer_mouse(
            601,
            PointerEventKind::Down,
            start,
            UiVector::ZERO,
        )
        .with_event_button(Some(PointerButton::Primary));
        let movement = NativeTabletPacket::windows_pointer_mouse(
            601,
            PointerEventKind::Move,
            current,
            UiVector::new(current.x - c2.x, current.y - c2.y),
        )
        .with_event_button(Some(PointerButton::Primary))
        .with_coalesced_samples([
            NativeTabletSample::new(c1, UiVector::new(c1.x - start.x, c1.y - start.y)),
            NativeTabletSample::new(c2, UiVector::new(c2.x - c1.x, c2.y - c1.y)),
        ]);
        native_frame
            .events
            .push(map_native_tablet_packet(&down).event);
        native_frame
            .events
            .push(map_native_tablet_packet(&movement).event);
    }

    runtime = runtime
        .run_for_frames(1)
        .expect("native mouse route frame should run");

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    let preview = host
        .app
        .preview_stroke()
        .expect("native mouse route should start a preview stroke");
    assert_eq!(
        preview.samples.len(),
        4,
        "native mouse history should preserve coalesced points instead of falling back to one sparse cursor move"
    );
    assert_canvas_position_close(preview.samples[0].position, 512.0, 512.0);
    assert_canvas_position_close(preview.samples[1].position, 540.0, 530.0);
    assert_canvas_position_close(preview.samples[2].position, 580.0, 548.0);
    assert_canvas_position_close(preview.samples[3].position, 620.0, 560.0);
    assert_eq!(
        host.app.routed_inputs().len(),
        2,
        "native mouse input should claim the pointer stream before winit fallback duplicates it"
    );
    assert_eq!(
        host.app.routed_inputs()[1].source_kind,
        PointerSourceKind::Mouse
    );
    assert_eq!(
        host.app.routed_inputs()[1].tool_kind,
        PointerToolKind::Mouse
    );
    assert_eq!(
        host.app.routed_inputs()[1].device_id,
        Some(PointerDeviceId(601))
    );
    assert_eq!(host.app.routed_inputs()[1].coalesced_sample_count, 2);
    assert_eq!(host.app.routed_inputs()[1].coalesced_samples.len(), 2);
}

#[test]
fn native_tablet_events_route_before_winit_touch_fallback() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let (native_start, fallback_start) = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        (
            screen_point_for_canvas(&host.app, 700.0, 700.0),
            screen_point_for_canvas(&host.app, 1_000.0, 1_000.0),
        )
    };
    {
        let native_frame = runtime
            .world_mut()
            .resource_mut::<NativeTabletFrameResource>()
            .expect("native tablet frame resource should exist");
        let packet = NativeTabletPacket::windows_pointer(
            501,
            PointerEventKind::Down,
            native_start,
            UiVector::ZERO,
        )
        .with_pressure(0.7);
        native_frame
            .events
            .push(map_native_tablet_packet(&packet).event);
    }
    {
        let input = runtime
            .world_mut()
            .resource_mut::<InputState>()
            .expect("input state should exist");
        input.handle_touch_input(
            TouchInputPhase::Started,
            91,
            fallback_start.x,
            fallback_start.y,
            Some(0.2),
        );
    }

    runtime = runtime
        .run_for_frames(1)
        .expect("native tablet input route frame should run");

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    assert_eq!(host.app.routed_inputs().len(), 1);
    assert_eq!(
        host.app.routed_inputs()[0].device_id,
        Some(PointerDeviceId(501)),
        "native tablet packet should route and suppress duplicate fallback touch for the frame"
    );
}

#[test]
fn native_tablet_hover_does_not_drop_winit_fallback_contact() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let (native_hover, fallback_start) = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        (
            screen_point_for_canvas(&host.app, 700.0, 700.0),
            screen_point_for_canvas(&host.app, 1_000.0, 1_000.0),
        )
    };
    {
        let native_frame = runtime
            .world_mut()
            .resource_mut::<NativeTabletFrameResource>()
            .expect("native tablet frame resource should exist");
        let packet = NativeTabletPacket::windows_pointer(
            505,
            PointerEventKind::Move,
            native_hover,
            UiVector::ZERO,
        )
        .with_contact(PointerContactState::Hover);
        native_frame
            .events
            .push(map_native_tablet_packet(&packet).event);
    }
    {
        let input = runtime
            .world_mut()
            .resource_mut::<InputState>()
            .expect("input state should exist");
        input.handle_touch_input(
            TouchInputPhase::Started,
            94,
            fallback_start.x,
            fallback_start.y,
            Some(0.3),
        );
    }

    runtime = runtime
        .run_for_frames(1)
        .expect("native hover plus fallback frame should run");

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    assert_eq!(
        host.app.routed_inputs().len(),
        2,
        "hover-only native input should not consume the fallback contact"
    );
    assert_eq!(
        host.app.routed_inputs()[0].route_kind,
        DrawingToolRouteKind::Hover
    );
    assert_eq!(
        host.app.routed_inputs()[1].device_id,
        Some(PointerDeviceId(94))
    );
    assert!(
        host.app.preview_stroke().is_some(),
        "winit fallback contact should still start drawing"
    );
}

#[test]
fn active_native_contact_suppresses_fallback_without_new_native_samples() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let fallback_start = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        screen_point_for_canvas(&host.app, 1_000.0, 1_000.0)
    };
    {
        let native_frame = runtime
            .world_mut()
            .resource_mut::<NativeTabletFrameResource>()
            .expect("native tablet frame resource should exist");
        native_frame.active_native_contact = true;
    }
    {
        let input = runtime
            .world_mut()
            .resource_mut::<InputState>()
            .expect("input state should exist");
        input.handle_touch_input(
            TouchInputPhase::Started,
            92,
            fallback_start.x,
            fallback_start.y,
            Some(0.3),
        );
    }

    runtime = runtime
        .run_for_frames(1)
        .expect("native active-contact suppression frame should run");

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    assert!(
        host.app.routed_inputs().is_empty(),
        "winit fallback input should not duplicate an active native stylus contact"
    );
}

#[test]
fn stale_native_contact_allows_winit_fallback_recovery() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let fallback_start = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        screen_point_for_canvas(&host.app, 1_000.0, 1_000.0)
    };
    {
        let native_frame = runtime
            .world_mut()
            .resource_mut::<NativeTabletFrameResource>()
            .expect("native tablet frame resource should exist");
        native_frame.active_native_contact = true;
        native_frame.frames_since_native_event =
            runenwerk_draw::runtime::systems::NATIVE_CONTACT_FALLBACK_SUPPRESSION_IDLE_FRAME_LIMIT
                + 1;
    }
    {
        let input = runtime
            .world_mut()
            .resource_mut::<InputState>()
            .expect("input state should exist");
        input.handle_touch_input(
            TouchInputPhase::Started,
            93,
            fallback_start.x,
            fallback_start.y,
            Some(0.3),
        );
    }

    runtime = runtime
        .run_for_frames(1)
        .expect("stale native contact fallback recovery frame should run");

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    assert_eq!(
        host.app.routed_inputs().len(),
        1,
        "stale native contact suppression should not block fallback input forever"
    );
    assert_eq!(
        host.app.routed_inputs()[0].device_id,
        Some(PointerDeviceId(93))
    );
    assert!(
        host.app.preview_stroke().is_some(),
        "fallback touch should recover drawing after stale native contact"
    );
}

#[test]
fn native_tablet_coalesced_samples_become_ordered_preview_samples() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let (start, c1, c2, current) = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        (
            screen_point_for_canvas(&host.app, 600.0, 600.0),
            screen_point_for_canvas(&host.app, 650.0, 620.0),
            screen_point_for_canvas(&host.app, 700.0, 650.0),
            screen_point_for_canvas(&host.app, 760.0, 700.0),
        )
    };
    {
        let native_frame = runtime
            .world_mut()
            .resource_mut::<NativeTabletFrameResource>()
            .expect("native tablet frame resource should exist");
        let down =
            NativeTabletPacket::windows_pointer(502, PointerEventKind::Down, start, UiVector::ZERO)
                .with_pressure(0.4);
        let movement = NativeTabletPacket::windows_pointer(
            502,
            PointerEventKind::Move,
            current,
            UiVector::new(current.x - c2.x, current.y - c2.y),
        )
        .with_pressure(0.9)
        .with_timestamp_micros(40)
        .with_coalesced_samples([
            NativeTabletSample::new(c1, UiVector::new(c1.x - start.x, c1.y - start.y))
                .with_timestamp_micros(20)
                .with_pressure(0.5),
            NativeTabletSample::new(c2, UiVector::new(c2.x - c1.x, c2.y - c1.y))
                .with_timestamp_micros(30)
                .with_pressure(0.7),
        ]);
        native_frame
            .events
            .push(map_native_tablet_packet(&down).event);
        native_frame
            .events
            .push(map_native_tablet_packet(&movement).event);
    }

    runtime = runtime
        .run_for_frames(1)
        .expect("native tablet coalesced route frame should run");

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    let preview = host
        .app
        .preview_stroke()
        .expect("native input should start a preview stroke");
    assert_eq!(preview.samples.len(), 4);
    assert_eq!(preview.samples[0].sequence, 1);
    assert_eq!(preview.samples[1].sequence, 2);
    assert_eq!(preview.samples[2].sequence, 3);
    assert_eq!(preview.samples[3].sequence, 4);
    assert_eq!(preview.samples[1].pressure, Some(0.5));
    assert_eq!(preview.samples[2].pressure, Some(0.7));
    assert_eq!(preview.samples[3].pressure, Some(0.9));
}

#[test]
fn native_tablet_hover_release_ends_and_commits_active_stroke() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let (start, end) = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        (
            screen_point_for_canvas(&host.app, 600.0, 600.0),
            screen_point_for_canvas(&host.app, 760.0, 700.0),
        )
    };
    {
        let native_frame = runtime
            .world_mut()
            .resource_mut::<NativeTabletFrameResource>()
            .expect("native tablet frame resource should exist");
        let down =
            NativeTabletPacket::windows_pointer(504, PointerEventKind::Down, start, UiVector::ZERO)
                .with_pressure(0.4);
        native_frame
            .events
            .push(map_native_tablet_packet(&down).event);
    }
    runtime = runtime
        .run_for_frames(1)
        .expect("native tablet down frame should run");

    {
        let native_frame = runtime
            .world_mut()
            .resource_mut::<NativeTabletFrameResource>()
            .expect("native tablet frame resource should exist");
        let up = NativeTabletPacket::windows_pointer(
            504,
            PointerEventKind::Up,
            end,
            UiVector::new(end.x - start.x, end.y - start.y),
        )
        .with_contact(PointerContactState::Hover);
        native_frame
            .events
            .push(map_native_tablet_packet(&up).event);
    }
    runtime = runtime
        .run_for_frames(1)
        .expect("native tablet hover-release frame should run");

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    let preview = host
        .app
        .preview_stroke()
        .expect("native hover-release should preserve released preview");
    assert!(!preview.active);
    assert_eq!(
        host.app.document().expect("document is open").strokes.len(),
        1,
        "native hover release should commit the active stroke"
    );
    assert_eq!(
        host.app
            .routed_inputs()
            .last()
            .map(|input| input.route_kind),
        Some(DrawingToolRouteKind::EndPreviewStroke)
    );
}

#[test]
fn native_tablet_hover_down_still_begins_stroke_from_event_kind() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let start = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        screen_point_for_canvas(&host.app, 600.0, 600.0)
    };
    {
        let native_frame = runtime
            .world_mut()
            .resource_mut::<NativeTabletFrameResource>()
            .expect("native tablet frame resource should exist");
        let down =
            NativeTabletPacket::windows_pointer(506, PointerEventKind::Down, start, UiVector::ZERO)
                .with_contact(PointerContactState::Hover);
        native_frame
            .events
            .push(map_native_tablet_packet(&down).event);
    }

    runtime = runtime
        .run_for_frames(1)
        .expect("native hover-down frame should run");

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    assert_eq!(
        host.app
            .routed_inputs()
            .last()
            .map(|input| input.route_kind),
        Some(DrawingToolRouteKind::BeginPreviewStroke)
    );
    assert!(
        host.app
            .preview_stroke()
            .is_some_and(|preview| preview.active),
        "native down event kind should start drawing even when the contact flag is stale"
    );
}

#[test]
fn native_tablet_diagnostics_project_into_composition_support_panel() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    {
        let native_frame = runtime
            .world_mut()
            .resource_mut::<NativeTabletFrameResource>()
            .expect("native tablet frame resource should exist");
        native_frame
            .backend_health
            .push(NativeTabletBackendHealth::active(
                NativeTabletBackendKind::WindowsPointer,
                "test active backend",
            ));
        native_frame.telemetry.sample_rate_hz = 180.0;
        native_frame.telemetry.max_segment_gap_px = 18.0;
        native_frame.telemetry.pressure_available = true;
        native_frame.telemetry.tilt_available = true;
    }

    runtime = runtime
        .run_for_frames(1)
        .expect("native tablet diagnostics frame should run");

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    let panel = &host.app.composition_projection().tablet_panel;
    assert_eq!(panel.active_backend, "Windows Pointer/Ink");
    assert_eq!(panel.sample_rate_hz, 180.0);
    assert_eq!(panel.max_segment_gap_px, 18.0);
    assert!(panel.pressure_available);
    assert!(panel.tilt_available);
}

#[test]
fn pointer_up_commits_preview_stroke_into_authoritative_document() {
    let mut app = RunenwerkDrawApp::new();
    let position = center_of_canvas(app.composition_projection().canvas_view.screen_bounds);

    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Down,
        position,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        1,
    )));
    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Up,
        UiPoint::new(position.x + 5.0, position.y + 3.0),
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        0,
    )));

    let preview = app
        .preview_stroke()
        .expect("pointer up should finish the active preview stroke");
    assert!(!preview.active);
    assert_eq!(preview.samples.len(), 2);
    assert_eq!(
        app.document()
            .expect("document should remain open")
            .strokes
            .len(),
        1
    );
    assert!(app.ink_runtime().formed_products().is_empty());
    assert!(app.ink_runtime().preview_products().is_empty());
    assert!(
        stroke_primitive_count(app.last_frame()) > 0,
        "released preview should remain visible through the immediate stroke primitive"
    );
}

#[test]
fn released_preview_products_stay_visible_until_committed_replacement() {
    let mut app = RunenwerkDrawApp::new();
    let mut executor = RuntimeJobExecutorResource::with_config(RuntimeJobExecutorConfig::serial());
    let start = screen_point_for_canvas(&app, 512.0, 512.0);
    let end = screen_point_for_canvas(&app, 760.0, 640.0);

    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Down,
        start,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        1,
    )));
    let preview_report = process_drawing_preview_ink_jobs(&mut app, &mut executor);
    assert!(
        preview_report.applied_count > 0,
        "active preview catch-up should form preview products"
    );
    assert!(!app.ink_runtime().preview_products().is_empty());
    assert!(
        product_surface_primitive_count(app.last_frame()) > 0,
        "active preview products should be visible as the primary stroke representation"
    );
    assert_eq!(
        stroke_primitive_count(app.last_frame()),
        0,
        "full immediate stroke should be suppressed once preview products exist"
    );

    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Up,
        end,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        0,
    )));

    let preview = app
        .preview_stroke()
        .expect("released preview should remain available until accepted committed ink");
    assert!(!preview.active);
    assert!(!app.ink_runtime().preview_products().is_empty());
    assert_eq!(app.preview_dirty_start_sample_index(), None);
    assert!(app.pending_preview_tile_job().is_none());
    assert!(
        product_surface_primitive_count(app.last_frame()) > 0,
        "released preview should remain visible through provisional preview products"
    );
    assert_eq!(
        stroke_primitive_count(app.last_frame()),
        1,
        "released preview may keep a bounded immediate tail until committed products replace it"
    );

    let post_commit_report = process_drawing_preview_ink_jobs(&mut app, &mut executor);
    assert_eq!(post_commit_report.submitted_count, 0);
    assert_eq!(post_commit_report.applied_count, 0);
    assert!(
        !app.ink_runtime().preview_products().is_empty(),
        "released provisional products should remain until committed products are accepted"
    );

    let mut publications = ProductPublicationRuntimeResource::default();
    let mut snapshots = QuerySnapshotRuntimeResource::default();
    publish_visible_ink_until_clean(&mut app, &mut publications, &mut snapshots);

    assert!(app.ink_runtime().preview_products().is_empty());
    assert!(app.ink_runtime().visible_product_count() > 0);
    assert!(stroke_primitive_count(app.last_frame()) == 0);
    assert!(product_surface_primitive_count(app.last_frame()) > 0);
}

#[test]
fn committed_ink_products_publish_snapshot_and_become_visible_only_after_publication_boundaries() {
    let mut app = RunenwerkDrawApp::new();
    let shell_only_count = rect_primitive_count(app.last_frame());
    let position = center_of_canvas(app.composition_projection().canvas_view.screen_bounds);
    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Down,
        position,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        1,
    )));
    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Move,
        UiPoint::new(position.x + 12.0, position.y + 4.0),
        UiVector::new(12.0, 4.0),
        Some(PointerButton::Primary),
        Modifiers::default(),
        0,
    )));
    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Up,
        UiPoint::new(position.x + 18.0, position.y + 8.0),
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        0,
    )));

    assert_eq!(app.document().unwrap().strokes.len(), 1);
    assert!(app.ink_runtime().formed_products().is_empty());
    assert!(app.ink_runtime().preview_products().is_empty());
    assert!(stroke_primitive_count(app.last_frame()) > 0);

    let mut publications = ProductPublicationRuntimeResource::default();
    let mut snapshots = QuerySnapshotRuntimeResource::default();
    let publication =
        publish_drawing_ink_products(&mut app, &mut publications, &publication_boundary());
    assert_eq!(publication.published_count, 1);
    assert!(!app.ink_runtime().formed_products().is_empty());
    assert!(app.ink_runtime().accepted_snapshot_ids().is_empty());
    assert!(app.ink_runtime().visible_products().next().is_none());
    let repeated_publication =
        publish_drawing_ink_products(&mut app, &mut publications, &publication_boundary());
    assert_eq!(repeated_publication.published_count, 0);

    let query =
        publish_drawing_ink_query_snapshots(&mut app, &mut snapshots, &publication_boundary());
    assert!(query.published_count > 0);
    assert!(!app.ink_runtime().accepted_snapshot_ids().is_empty());
    assert!(app.ink_runtime().preview_products().is_empty());
    assert_eq!(stroke_primitive_count(app.last_frame()), 0);
    assert!(app.ink_runtime().visible_products().next().is_some());

    let size = app.composition_projection().window_size;
    app.rebuild_frame(size);
    assert_eq!(
        rect_primitive_count(app.last_frame()),
        shell_only_count,
        "accepted ink should no longer expand into UI rect primitives"
    );
    assert_eq!(
        product_surface_primitive_count(app.last_frame()),
        app.ink_runtime().visible_product_count(),
        "accepted current ink tile snapshots should project one product surface per visible tile"
    );
    assert!(
        product_surface_primitive_count(app.last_frame()) > 0,
        "accepted current ink tile snapshots should project visible committed ink"
    );
}

#[test]
fn pointer_release_keeps_last_accepted_ink_visible_before_new_publication_boundaries() {
    let mut app = RunenwerkDrawApp::new();
    let position = center_of_canvas(app.composition_projection().canvas_view.screen_bounds);
    draw_stroke(
        &mut app,
        position,
        UiPoint::new(position.x + 18.0, position.y + 8.0),
    );

    let mut publications = ProductPublicationRuntimeResource::default();
    let mut snapshots = QuerySnapshotRuntimeResource::default();
    publish_drawing_ink_products(&mut app, &mut publications, &publication_boundary());
    publish_drawing_ink_query_snapshots(&mut app, &mut snapshots, &publication_boundary());
    assert!(app.ink_runtime().visible_products().next().is_some());
    let accepted_count = app.ink_runtime().visible_products().count();

    draw_stroke(
        &mut app,
        UiPoint::new(position.x + 24.0, position.y + 16.0),
        UiPoint::new(position.x + 42.0, position.y + 24.0),
    );

    assert_eq!(app.ink_runtime().visible_products().count(), accepted_count);
    assert!(app.ink_runtime().preview_products().is_empty());
    assert!(
        stroke_primitive_count(app.last_frame()) > 0,
        "new released preview should overlay the last accepted committed ink immediately"
    );
}

#[test]
fn two_committed_strokes_publish_and_remain_drawable() {
    let mut app = RunenwerkDrawApp::new();
    let first_start = screen_point_for_canvas(&app, 512.0, 512.0);
    let first_end = screen_point_for_canvas(&app, 620.0, 560.0);
    let second_start = screen_point_for_canvas(&app, 3_100.0, 3_100.0);
    let second_end = screen_point_for_canvas(&app, 3_240.0, 3_180.0);
    let mut publications = ProductPublicationRuntimeResource::default();
    let mut snapshots = QuerySnapshotRuntimeResource::default();

    draw_stroke(&mut app, first_start, first_end);
    publish_drawing_ink_products(&mut app, &mut publications, &publication_boundary());
    publish_drawing_ink_query_snapshots(&mut app, &mut snapshots, &publication_boundary());
    let first_tiles = visible_tile_ids(&app);
    assert!(!first_tiles.is_empty());

    draw_stroke(&mut app, second_start, second_end);
    assert!(
        first_tiles.is_subset(&visible_tile_ids(&app)),
        "releasing the second stroke must preserve first-stroke visible tiles until the next publication boundary accepts them"
    );
    publish_drawing_ink_products(&mut app, &mut publications, &publication_boundary());
    publish_drawing_ink_query_snapshots(&mut app, &mut snapshots, &publication_boundary());

    assert_eq!(app.document().expect("document is open").strokes.len(), 2);
    assert!(app.ink_runtime().visible_products().next().is_some());
    assert!(
        first_tiles.is_subset(&visible_tile_ids(&app)),
        "accepting the second stroke should not clear unrelated first-stroke tiles"
    );
    assert!(app.ink_runtime().preview_products().is_empty());
}

#[test]
fn four_committed_strokes_publish_and_remain_drawable() {
    let mut app = RunenwerkDrawApp::new();
    let mut publications = ProductPublicationRuntimeResource::default();
    let mut snapshots = QuerySnapshotRuntimeResource::default();
    let strokes = [
        ((512.0, 512.0), (620.0, 560.0)),
        ((3_100.0, 3_100.0), (3_240.0, 3_180.0)),
        ((1_260.0, 2_720.0), (1_460.0, 2_820.0)),
        ((2_540.0, 920.0), (2_760.0, 1_020.0)),
    ];

    for (index, (start, end)) in strokes.into_iter().enumerate() {
        let start = screen_point_for_canvas(&app, start.0, start.1);
        let end = screen_point_for_canvas(&app, end.0, end.1);
        draw_and_accept_stroke(&mut app, &mut publications, &mut snapshots, start, end);
        assert_eq!(
            app.document().expect("document is open").strokes.len(),
            index + 1
        );
        assert!(
            app.ink_runtime().visible_product_count() > 0,
            "accepted stroke {} should leave visible committed ink",
            index + 1
        );
        assert!(
            app.ink_runtime().dirty_tiles().is_empty(),
            "accepted stroke {} should not leave a blocked dirty backlog",
            index + 1
        );
    }

    app.rebuild_frame(app.composition_projection().window_size);
    assert_eq!(app.document().expect("document is open").strokes.len(), 4);
    assert!(
        product_surface_primitive_count(app.last_frame()) > 0,
        "four committed strokes should still project drawable product surfaces"
    );
    assert!(app.ink_runtime().preview_products().is_empty());
}

#[test]
fn runtime_winit_fallback_publishes_four_strokes_through_publication_boundaries() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    runtime.insert_resource(RuntimeJobExecutorResource::with_config(
        RuntimeJobExecutorConfig::serial(),
    ));
    let strokes = [
        ((512.0, 512.0), (620.0, 560.0)),
        ((3_100.0, 3_100.0), (3_240.0, 3_180.0)),
        ((1_260.0, 2_720.0), (1_460.0, 2_820.0)),
        ((2_540.0, 920.0), (2_760.0, 1_020.0)),
    ];

    let mut previous_visible_count = 0;
    for (index, (start, end)) in strokes.into_iter().enumerate() {
        let (start, end) = {
            let host = runtime
                .world()
                .resource::<DrawingHostResource>()
                .expect("drawing host resource should exist");
            (
                screen_point_for_canvas(&host.app, start.0, start.1),
                screen_point_for_canvas(&host.app, end.0, end.1),
            )
        };
        runtime = run_winit_fallback_stroke(runtime, start, &[], end);
        runtime = run_committed_ink_until_clean(runtime, index + 1);

        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        assert_eq!(
            host.app.document().expect("document is open").strokes.len(),
            index + 1
        );
        let visible_count = host.app.ink_runtime().visible_product_count();
        assert!(
            visible_count > previous_visible_count,
            "runtime stroke {} should add committed visible ink products",
            index + 1
        );
        assert!(
            host.app.ink_runtime().dirty_tiles().is_empty(),
            "runtime stroke {} should drain committed dirty tiles",
            index + 1
        );
        previous_visible_count = visible_count;
    }

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    assert_eq!(
        host.app.document().expect("document is open").strokes.len(),
        4
    );
    assert!(
        product_surface_primitive_count(host.app.last_frame()) > 0,
        "four runtime-routed strokes should remain projected as product surfaces"
    );
    assert!(host.app.ink_runtime().preview_products().is_empty());
}

#[test]
fn runtime_winit_fallback_recovers_after_rapid_worker_pool_strokes() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let strokes = [
        ((512.0, 512.0), (620.0, 560.0)),
        ((3_100.0, 3_100.0), (3_240.0, 3_180.0)),
        ((1_260.0, 2_720.0), (1_460.0, 2_820.0)),
        ((2_540.0, 920.0), (2_760.0, 1_020.0)),
    ];

    for (start, end) in strokes {
        let (start, end) = {
            let host = runtime
                .world()
                .resource::<DrawingHostResource>()
                .expect("drawing host resource should exist");
            (
                screen_point_for_canvas(&host.app, start.0, start.1),
                screen_point_for_canvas(&host.app, end.0, end.1),
            )
        };
        runtime = run_winit_fallback_stroke(runtime, start, &[], end);
    }

    {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        assert_eq!(
            host.app.document().expect("document is open").strokes.len(),
            4
        );
        assert!(
            stroke_primitive_count(host.app.last_frame()) >= 4,
            "rapid strokes should stay visible as released-stroke overlays while final tiles catch up"
        );
    }

    runtime = run_committed_ink_until_clean(runtime, 4);

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    assert_eq!(
        host.app.document().expect("document is open").strokes.len(),
        4
    );
    assert!(
        host.app.ink_runtime().visible_product_count() >= 4,
        "rapid worker-pool strokes should eventually publish all committed ink"
    );
    assert!(
        product_surface_primitive_count(host.app.last_frame()) > 0,
        "rapid worker-pool strokes should project committed product surfaces after catch-up"
    );
    assert_eq!(
        stroke_primitive_count(host.app.last_frame()),
        0,
        "released-stroke overlays should clear once committed products are current"
    );
}

#[test]
fn runtime_winit_fallback_keeps_long_stroke_publishing_through_publication_boundaries() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    runtime.insert_resource(RuntimeJobExecutorResource::with_config(
        RuntimeJobExecutorConfig::serial(),
    ));
    let (start, moves, end) = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        let canvas = host.app.composition_projection().canvas_view.screen_bounds;
        let start = UiPoint::new(canvas.x + 1.0, canvas.y + 1.0);
        let end = UiPoint::new(
            canvas.x + canvas.width - 1.0,
            canvas.y + canvas.height - 1.0,
        );
        let mut moves = Vec::new();
        for step in 1..=20 {
            let t = step as f32 / 21.0;
            moves.push(UiPoint::new(
                start.x + (end.x - start.x) * t,
                start.y + (end.y - start.y) * t,
            ));
        }
        (start, moves, end)
    };

    runtime = run_winit_fallback_stroke(runtime, start, &moves, end);
    runtime = run_committed_ink_until_clean(runtime, 1);

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    let app = &host.app;
    assert_eq!(app.document().expect("document is open").strokes.len(), 1);
    assert_eq!(
        app.document().expect("document is open").strokes[0]
            .samples
            .len(),
        22,
        "runtime long stroke should keep every routed drag sample"
    );
    assert!(
        app.ink_runtime().visible_product_count() > 1,
        "runtime long stroke should publish multiple committed ink products"
    );
    assert!(
        product_surface_primitive_count(app.last_frame()) > 1,
        "runtime long stroke should project as multiple product surfaces"
    );
    assert!(app.ink_runtime().diagnostics().iter().all(|diagnostic| {
        diagnostic.code != DrawingTileFormationDiagnosticCode::TooManyAffectedTiles
    }));
}

#[test]
fn active_preview_survives_previous_committed_query_acceptance() {
    let mut app = RunenwerkDrawApp::new();
    let mut publications = ProductPublicationRuntimeResource::default();
    let mut snapshots = QuerySnapshotRuntimeResource::default();
    let mut executor = RuntimeJobExecutorResource::with_config(RuntimeJobExecutorConfig::serial());

    let first_start = screen_point_for_canvas(&app, 384.0, 384.0);
    let first_end = screen_point_for_canvas(&app, 520.0, 440.0);
    draw_stroke(&mut app, first_start, first_end);
    let publication =
        publish_drawing_ink_products(&mut app, &mut publications, &publication_boundary());
    assert!(publication.published_count > 0);
    assert!(
        !app.ink_runtime().published_descriptors().is_empty(),
        "first stroke should be waiting for query snapshot acceptance"
    );

    let second_start = screen_point_for_canvas(&app, 1_240.0, 1_240.0);
    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Down,
        second_start,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        1,
    )));
    process_drawing_preview_ink_jobs(&mut app, &mut executor);
    assert!(
        app.preview_stroke().is_some_and(|preview| preview.active),
        "second stroke should be actively previewing"
    );
    assert!(
        !app.ink_runtime().preview_products().is_empty(),
        "active preview should have catch-up products before the old query snapshot accepts"
    );

    let query =
        publish_drawing_ink_query_snapshots(&mut app, &mut snapshots, &publication_boundary());
    assert!(query.published_count > 0);

    assert!(
        app.preview_stroke().is_some_and(|preview| preview.active),
        "accepting an older committed snapshot must not cancel the current stroke session"
    );
    assert!(
        !app.ink_runtime().preview_products().is_empty(),
        "accepting committed ink must preserve active preview products"
    );
}

#[test]
fn formation_failure_preserves_last_good_visible_ink_and_records_diagnostics() {
    let mut app = RunenwerkDrawApp::new();
    let position = center_of_canvas(app.composition_projection().canvas_view.screen_bounds);
    draw_stroke(
        &mut app,
        position,
        UiPoint::new(position.x + 18.0, position.y + 8.0),
    );

    let mut publications = ProductPublicationRuntimeResource::default();
    let mut snapshots = QuerySnapshotRuntimeResource::default();
    publish_drawing_ink_products(&mut app, &mut publications, &publication_boundary());
    publish_drawing_ink_query_snapshots(&mut app, &mut snapshots, &publication_boundary());
    assert!(app.ink_runtime().visible_products().next().is_some());
    let accepted_count = app.ink_runtime().visible_product_count();
    app.ink_runtime_mut().record_failed_generation(
        "test.failed.formation".to_string(),
        vec![DrawingTileFormationDiagnostic::blocking(
            DrawingTileFormationDiagnosticCode::InvalidPolicy,
            "test formation failure",
        )],
        true,
    );

    assert_eq!(app.ink_runtime().visible_product_count(), accepted_count);
    assert!(app.ink_runtime().visible_products().next().is_some());
    assert!(app.ink_runtime().preview_products().is_empty());
    assert!(app.ink_runtime().diagnostics().iter().any(|diagnostic| {
        diagnostic.code == DrawingTileFormationDiagnosticCode::InvalidPolicy
    }));
}

#[test]
fn long_stroke_batches_dirty_tiles_instead_of_clearing_canvas() {
    let mut app = RunenwerkDrawApp::new();
    let mut publications = ProductPublicationRuntimeResource::default();
    let mut snapshots = QuerySnapshotRuntimeResource::default();
    let canvas = app.composition_projection().canvas_view.screen_bounds;
    draw_stroke(
        &mut app,
        UiPoint::new(canvas.x + 1.0, canvas.y + 1.0),
        UiPoint::new(
            canvas.x + canvas.width - 1.0,
            canvas.y + canvas.height - 1.0,
        ),
    );

    assert_eq!(app.document().expect("document is open").strokes.len(), 1);
    assert!(
        app.ink_runtime().dirty_tiles().len() > drawing::DEFAULT_MAX_AFFECTED_INK_TILES,
        "the diagonal canvas stroke should exceed one interactive formation batch"
    );

    let mut batches = 0;
    while !app.ink_runtime().dirty_tiles().is_empty() {
        batches += 1;
        assert!(batches <= 8, "dirty tile batches should drain promptly");
        let report =
            publish_drawing_ink_products(&mut app, &mut publications, &publication_boundary());
        assert!(
            report.published_count > 0 || report.rejected_count == 0,
            "long stroke batches must not reject solely because the whole stroke spans many tiles"
        );
        publish_drawing_ink_query_snapshots(&mut app, &mut snapshots, &publication_boundary());
    }

    assert!(
        batches > 1,
        "long stroke should be formed as more than one bounded batch"
    );
    assert!(app.ink_runtime().visible_product_count() > 0);
    assert!(app.ink_runtime().diagnostics().iter().all(|diagnostic| {
        diagnostic.code != DrawingTileFormationDiagnosticCode::TooManyAffectedTiles
    }));
    app.rebuild_frame(app.composition_projection().window_size);
    let product_surface_count = product_surface_primitive_count(app.last_frame());
    assert!(
        product_surface_count > 1,
        "long stroke should render as tile-count-scaled product surfaces"
    );
    assert!(
        product_surface_count <= app.ink_runtime().visible_product_count(),
        "offscreen or clipped tiles may be stored without producing screen primitives"
    );
}

#[test]
fn long_active_preview_updates_only_dirty_tail_tiles() {
    let mut app = RunenwerkDrawApp::new();
    let mut executor = RuntimeJobExecutorResource::with_config(RuntimeJobExecutorConfig::serial());
    let start = screen_point_for_canvas(&app, 64.0, 64.0);
    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Down,
        start,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        1,
    )));
    process_drawing_preview_ink_jobs(&mut app, &mut executor);

    let mut max_dirty_tiles = app.ink_runtime().last_preview_dirty_tile_count();
    for step in 1..=20 {
        let canvas_position = 64.0 + step as f64 * 192.0;
        let screen_position = screen_point_for_canvas(&app, canvas_position, canvas_position);
        app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
            PointerEventKind::Move,
            screen_position,
            UiVector::ZERO,
            Some(PointerButton::Primary),
            Modifiers::default(),
            0,
        )));
        let report = process_drawing_preview_ink_jobs(&mut app, &mut executor);
        assert!(
            report.submitted_count <= 1,
            "preview scheduling should keep at most one catch-up job per update"
        );
        max_dirty_tiles = max_dirty_tiles.max(app.ink_runtime().last_preview_dirty_tile_count());
        assert!(
            app.ink_runtime().last_preview_dirty_tile_count() <= 8,
            "each small preview update should only reform the current stroke tail tiles"
        );
    }

    let preview = app
        .preview_stroke()
        .expect("long active stroke should still be previewing");
    assert_eq!(preview.samples.len(), 21);
    assert_eq!(
        stroke_primitive_count(app.last_frame()),
        0,
        "the app must not keep projecting an unbounded immediate polyline once preview products exist"
    );
    assert!(
        product_surface_primitive_count(app.last_frame()) > 0,
        "accumulated preview products should be the active long-stroke representation"
    );
    assert!(
        app.ink_runtime().preview_products().len() > max_dirty_tiles,
        "the accumulated preview may span more tiles than any single dirty-tail update"
    );
    assert!(app.ink_runtime().diagnostics().iter().all(|diagnostic| {
        diagnostic.code != DrawingTileFormationDiagnosticCode::TooManyAffectedTiles
    }));
}

#[test]
fn long_active_immediate_preview_preserves_full_extent_with_bounded_points() {
    let mut app = RunenwerkDrawApp::new();
    let start = screen_point_for_canvas(&app, 64.0, 64.0);
    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Down,
        start,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        1,
    )));

    let mut end = start;
    for step in 1..=1_500 {
        let canvas_position = 64.0 + step as f64 * 2.4;
        end = screen_point_for_canvas(&app, canvas_position, canvas_position);
        app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
            PointerEventKind::Move,
            end,
            UiVector::ZERO,
            Some(PointerButton::Primary),
            Modifiers::default(),
            0,
        )));
    }

    let preview = app
        .preview_stroke()
        .expect("long active stroke should still be previewing");
    assert_eq!(preview.samples.len(), 1_501);
    let strokes = stroke_primitives(app.last_frame());
    assert_eq!(strokes.len(), 1);
    assert!(
        strokes[0].points.len() <= 1_024,
        "immediate preview should preserve a bounded render primitive"
    );
    assert_point_close(
        *strokes[0]
            .points
            .first()
            .expect("stroke primitive should keep the first point"),
        start,
    );
    assert_point_close(
        *strokes[0]
            .points
            .last()
            .expect("stroke primitive should keep the last point"),
        end,
    );
}

#[test]
fn runtime_uploads_preview_products_only_when_generation_changes() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    runtime.insert_resource(RuntimeJobExecutorResource::with_config(
        RuntimeJobExecutorConfig::serial(),
    ));

    {
        let host = runtime
            .world_mut()
            .resource_mut::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        let app = &mut host.app;
        let start = screen_point_for_canvas(app, 64.0, 64.0);
        app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
            PointerEventKind::Down,
            start,
            UiVector::ZERO,
            Some(PointerButton::Primary),
            Modifiers::default(),
            1,
        )));
    }

    runtime = runtime
        .run_for_frames(1)
        .expect("initial preview job frame should run");

    let mut first_dirty_tail_count = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        assert!(!host.app.ink_runtime().preview_products().is_empty());
        host.app.ink_runtime().last_preview_dirty_tile_count()
    };

    for step in 1..=8 {
        {
            let host = runtime
                .world_mut()
                .resource_mut::<DrawingHostResource>()
                .expect("drawing host resource should exist");
            let app = &mut host.app;
            let canvas_position = 64.0 + step as f64 * 96.0;
            let screen_position = screen_point_for_canvas(app, canvas_position, canvas_position);
            app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
                PointerEventKind::Move,
                screen_position,
                UiVector::ZERO,
                Some(PointerButton::Primary),
                Modifiers::default(),
                0,
            )));
        }

        runtime = runtime
            .run_for_frames(1)
            .expect("preview catch-up frame should run");

        let dirty_tail_count = {
            let host = runtime
                .world()
                .resource::<DrawingHostResource>()
                .expect("drawing host resource should exist");
            host.app.ink_runtime().last_preview_dirty_tile_count()
        };
        first_dirty_tail_count = first_dirty_tail_count.max(dirty_tail_count);
        assert!(dirty_tail_count <= 8);
    }

    let preview_count = {
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        let preview_count = host.app.ink_runtime().preview_products().len();
        assert!(
            preview_count > first_dirty_tail_count,
            "the active preview should accumulate more tiles than the latest dirty tail"
        );
        preview_count
    };

    runtime = runtime
        .run_for_frames(1)
        .expect("stable preview frame should run");
    assert_eq!(
        ink_upload_count(&runtime),
        0,
        "unchanged preview products must not be re-uploaded every frame"
    );

    let next_dirty_tail_count = {
        {
            let host = runtime
                .world_mut()
                .resource_mut::<DrawingHostResource>()
                .expect("drawing host resource should exist");
            let app = &mut host.app;
            let screen_position = screen_point_for_canvas(app, 1_000.0, 1_000.0);
            app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
                PointerEventKind::Move,
                screen_position,
                UiVector::ZERO,
                Some(PointerButton::Primary),
                Modifiers::default(),
                0,
            )));
        }

        runtime = runtime
            .run_for_frames(1)
            .expect("dirty preview tail upload frame should run");
        let host = runtime
            .world()
            .resource::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        host.app.ink_runtime().last_preview_dirty_tile_count()
    };

    let upload_count = ink_upload_count(&runtime);
    assert!(
        upload_count > 0,
        "moving the active stroke should upload the changed preview tail"
    );
    assert!(
        upload_count <= next_dirty_tail_count,
        "preview uploads should be bounded by the latest dirty tail"
    );
    assert!(
        upload_count < preview_count,
        "preview uploads should not scale with the whole long active stroke"
    );
    assert!(
        next_dirty_tail_count <= first_dirty_tail_count.max(8),
        "the follow-up move should stay within the bounded dirty-tail policy"
    );
}

#[test]
fn runtime_bridges_preview_and_final_tiles_to_product_surfaces() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    runtime.insert_resource(RuntimeJobExecutorResource::with_config(
        RuntimeJobExecutorConfig::serial(),
    ));

    let end = {
        let host = runtime
            .world_mut()
            .resource_mut::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        let start = screen_point_for_canvas(&host.app, 512.0, 512.0);
        let end = screen_point_for_canvas(&host.app, 704.0, 640.0);
        host.app
            .dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
                PointerEventKind::Down,
                start,
                UiVector::ZERO,
                Some(PointerButton::Primary),
                Modifiers::default(),
                1,
            )));
        end
    };

    runtime = runtime
        .run_for_frames(1)
        .expect("preview product-surface frame should run");
    assert!(
        dynamic_target_ids(&runtime)
            .iter()
            .any(|target_id| target_id.starts_with("preview.preview.")),
        "active preview tiles should request preview dynamic texture targets"
    );

    let final_visible_count = {
        let host = runtime
            .world_mut()
            .resource_mut::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        let mut publications = ProductPublicationRuntimeResource::default();
        let mut snapshots = QuerySnapshotRuntimeResource::default();
        host.app
            .dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
                PointerEventKind::Up,
                end,
                UiVector::ZERO,
                Some(PointerButton::Primary),
                Modifiers::default(),
                0,
            )));
        publish_visible_ink_until_clean(&mut host.app, &mut publications, &mut snapshots);
        assert!(
            host.app
                .ink_runtime()
                .visible_products()
                .all(|product| { product.metadata.quality_class == ProductQualityClass::Final })
        );
        let final_visible_count = host.app.ink_runtime().visible_product_count();
        assert!(final_visible_count > 0);
        final_visible_count
    };

    runtime = runtime
        .run_for_frames(1)
        .expect("final product-surface frame should run");
    assert!(
        dynamic_target_ids(&runtime)
            .iter()
            .any(|target_id| target_id.starts_with("committed.final.")),
        "accepted committed tiles should request final-quality dynamic texture targets"
    );

    let product_selections = runtime
        .world()
        .resource::<PreparedRenderProductSelectionResource>()
        .expect("prepared product selection resource should exist")
        .snapshot();
    let canvas_selection = product_selections
        .iter()
        .find(|selection| selection.view_id == "runenwerk.draw.canvas")
        .expect("drawing canvas should publish one product selection");
    assert_eq!(
        canvas_selection.selected_products.len(),
        final_visible_count
    );
    assert!(
        canvas_selection
            .selected_products
            .iter()
            .all(|product| { product.scale_band == ProductScaleBand::Final })
    );

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    assert_eq!(
        product_surface_primitive_count(host.app.last_frame()),
        final_visible_count,
        "final accepted tiles should remain projected as product-surface UI primitives"
    );
}

#[test]
fn runtime_requests_gpu_ink_validation_through_public_render_flow() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let gpu_flow = *runtime
        .world()
        .resource::<DrawingInkGpuFlowResource>()
        .expect("drawing GPU ink flow resource should exist");
    let mut publications = ProductPublicationRuntimeResource::default();
    let mut snapshots = QuerySnapshotRuntimeResource::default();

    {
        let host = runtime
            .world_mut()
            .resource_mut::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        let start = screen_point_for_canvas(&host.app, 512.0, 512.0);
        let end = screen_point_for_canvas(&host.app, 704.0, 640.0);
        draw_and_accept_stroke(&mut host.app, &mut publications, &mut snapshots, start, end);
    }

    runtime = runtime
        .run_for_frames(1)
        .expect("GPU ink validation request frame should run");
    assert!(
        dynamic_target_ids(&runtime)
            .iter()
            .any(|target_id| target_id.starts_with("gpu.committed.final.")),
        "GPU validation should request a committed final GPU dynamic target"
    );

    let frame_requests = runtime
        .world()
        .resource::<PreparedRenderFrameRequestResource>()
        .expect("prepared frame requests should exist");
    let invocations = frame_requests.requested_flow_invocations();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].flow_id, gpu_flow.flow_id);
    assert_eq!(invocations[0].target_alias_bindings.len(), 2);

    let debug_config = runtime
        .world()
        .resource::<RenderDebugConfigResource>()
        .expect("render debug config should exist");
    assert_eq!(debug_config.texture_diffs.len(), 1);
    let diff = &debug_config.texture_diffs[0];
    assert!(diff.id.starts_with("runenwerk.draw.ink.gpu.diff."));
    assert_eq!(diff.max_channel_delta, Some(2));
    assert_eq!(diff.max_changed_pixels_per_million, Some(10_000));
}

#[test]
fn gpu_validation_promotion_and_failure_keep_cpu_fallback_available() {
    let mut app = RunenwerkDrawApp::new();
    let mut publications = ProductPublicationRuntimeResource::default();
    let mut snapshots = QuerySnapshotRuntimeResource::default();
    let start = screen_point_for_canvas(&app, 512.0, 512.0);
    let end = screen_point_for_canvas(&app, 704.0, 640.0);
    draw_and_accept_stroke(&mut app, &mut publications, &mut snapshots, start, end);
    let product = app
        .ink_runtime()
        .visible_products()
        .next()
        .expect("accepted stroke should form a visible product")
        .clone();

    assert!(
        product_surface_target_ids(app.last_frame())
            .iter()
            .any(|target_id| target_id.starts_with("committed.final."))
    );

    app.ink_runtime_mut().record_gpu_validation_pass(
        DrawingInkSurfaceKind::Committed,
        &product,
        DrawingInkGpuValidationMetrics {
            max_channel_delta: 2,
            changed_pixel_count: 0,
            total_pixel_count: product.payload.width as u64 * product.payload.height as u64,
            changed_pixel_ratio: 0.0,
        },
    );
    app.set_window_size(app.composition_projection().window_size)
        .unwrap();
    assert!(
        product_surface_target_ids(app.last_frame())
            .iter()
            .any(|target_id| target_id.starts_with("gpu.committed.final.")),
        "passed GPU validation should promote the visible product surface to the GPU target"
    );

    app.ink_runtime_mut().record_gpu_validation_failure(
        DrawingInkSurfaceKind::Committed,
        &product,
        "forced validation failure",
    );
    app.set_window_size(app.composition_projection().window_size)
        .unwrap();
    assert!(
        product_surface_target_ids(app.last_frame())
            .iter()
            .any(|target_id| target_id.starts_with("committed.final.")),
        "GPU validation failure should keep the CPU committed surface visible"
    );
}

#[test]
fn stale_gpu_validation_does_not_promote_new_tile_generation() {
    let mut app = RunenwerkDrawApp::new();
    let mut publications = ProductPublicationRuntimeResource::default();
    let mut snapshots = QuerySnapshotRuntimeResource::default();
    let start = screen_point_for_canvas(&app, 512.0, 512.0);
    let end = screen_point_for_canvas(&app, 704.0, 640.0);
    draw_and_accept_stroke(&mut app, &mut publications, &mut snapshots, start, end);
    let old_product = app
        .ink_runtime()
        .visible_products()
        .next()
        .expect("accepted stroke should form a visible product")
        .clone();
    app.ink_runtime_mut().record_gpu_validation_pass(
        DrawingInkSurfaceKind::Committed,
        &old_product,
        DrawingInkGpuValidationMetrics {
            max_channel_delta: 0,
            changed_pixel_count: 0,
            total_pixel_count: old_product.payload.width as u64 * old_product.payload.height as u64,
            changed_pixel_ratio: 0.0,
        },
    );

    draw_and_accept_stroke(&mut app, &mut publications, &mut snapshots, start, end);
    let new_product = app
        .ink_runtime()
        .visible_products()
        .find(|product| product.metadata.tile_id == old_product.metadata.tile_id)
        .expect("second stroke should refresh the same visible tile")
        .clone();
    assert_ne!(
        old_product.descriptor_generation,
        new_product.descriptor_generation
    );
    app.set_window_size(app.composition_projection().window_size)
        .unwrap();

    let target_ids = product_surface_target_ids(app.last_frame());
    assert!(
        target_ids
            .iter()
            .any(|target_id| target_id.starts_with("committed.final."))
    );
    assert!(
        target_ids
            .iter()
            .all(|target_id| !target_id.starts_with("gpu.committed.final.")),
        "stale GPU validation must not promote a newer tile generation"
    );
}

#[test]
fn runtime_uploads_only_new_committed_products_after_more_strokes() {
    let mut runtime = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run its startup frame");
    let mut publications = ProductPublicationRuntimeResource::default();
    let mut snapshots = QuerySnapshotRuntimeResource::default();

    let first_visible_count = {
        let host = runtime
            .world_mut()
            .resource_mut::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        let first_start = screen_point_for_canvas(&host.app, 512.0, 512.0);
        let first_end = screen_point_for_canvas(&host.app, 620.0, 560.0);
        draw_and_accept_stroke(
            &mut host.app,
            &mut publications,
            &mut snapshots,
            first_start,
            first_end,
        );
        host.app.ink_runtime().visible_product_count()
    };
    assert!(first_visible_count > 0);

    runtime = runtime
        .run_for_frames(1)
        .expect("first committed upload frame should run");
    assert_eq!(
        ink_upload_count(&runtime),
        first_visible_count,
        "the first visible generation should upload once"
    );
    runtime = runtime
        .run_for_frames(1)
        .expect("stable committed frame should run");
    assert_eq!(
        ink_upload_count(&runtime),
        0,
        "unchanged committed ink must not be re-uploaded every frame"
    );

    let total_visible_count = {
        let host = runtime
            .world_mut()
            .resource_mut::<DrawingHostResource>()
            .expect("drawing host resource should exist");
        let second_start = screen_point_for_canvas(&host.app, 3_100.0, 3_100.0);
        let second_end = screen_point_for_canvas(&host.app, 3_240.0, 3_180.0);
        draw_and_accept_stroke(
            &mut host.app,
            &mut publications,
            &mut snapshots,
            second_start,
            second_end,
        );
        host.app.ink_runtime().visible_product_count()
    };
    assert!(total_visible_count > first_visible_count);

    runtime = runtime
        .run_for_frames(1)
        .expect("second committed upload frame should run");
    let upload_count = ink_upload_count(&runtime);
    assert!(upload_count > 0);
    assert!(
        upload_count < total_visible_count,
        "adding another stroke should upload only new or changed products, not all existing ink"
    );
    runtime = runtime
        .run_for_frames(1)
        .expect("second stable committed frame should run");
    assert_eq!(ink_upload_count(&runtime), 0);
}

#[test]
fn committed_ink_cache_hit_stages_products_without_job_submission() {
    let mut app = RunenwerkDrawApp::new();
    let mut publications = ProductPublicationRuntimeResource::default();
    let mut snapshots = QuerySnapshotRuntimeResource::default();
    let mut executor = RuntimeJobExecutorResource::with_config(RuntimeJobExecutorConfig::serial());
    let mut cache = RuntimeProductCacheResource::default();
    app.ink_runtime_mut().set_tile_cache_budget_bytes(1);
    let start = screen_point_for_canvas(&app, 512.0, 512.0);
    let end = screen_point_for_canvas(&app, 620.0, 560.0);

    draw_stroke(&mut app, start, end);
    let first_report = publish_drawing_ink_products_with_executor_and_cache(
        &mut app,
        &mut publications,
        &mut executor,
        &mut cache,
        &publication_boundary(),
    );
    assert!(first_report.published_count > 0);
    publish_drawing_ink_query_snapshots(&mut app, &mut snapshots, &publication_boundary());
    assert!(app.ink_runtime().visible_product_count() > 0);
    let submitted_after_first = executor.diagnostics().submitted_count;
    let cached_entries = cache.snapshot().entry_count;
    assert_eq!(cached_entries, app.ink_runtime().visible_product_count());
    assert_eq!(
        app.ink_runtime().tile_cache_entry_count(),
        app.ink_runtime().visible_product_count(),
        "current visible committed tiles stay protected even when the cache is over budget"
    );
    assert!(
        app.ink_runtime().tile_cache_payload_bytes() > app.ink_runtime().tile_cache_budget_bytes()
    );

    let cached_tiles = visible_tile_ids(&app);
    let last_access_before = app
        .ink_runtime()
        .tile_cache_metadata()
        .iter()
        .map(|metadata| metadata.last_access_frame)
        .max()
        .unwrap_or(0);
    app.ink_runtime_mut().mark_dirty_tiles(cached_tiles.clone());
    let second_report = publish_drawing_ink_products_with_executor_and_cache(
        &mut app,
        &mut publications,
        &mut executor,
        &mut cache,
        &publication_boundary(),
    );

    assert!(second_report.published_count > 0);
    assert_eq!(
        executor.diagnostics().submitted_count,
        submitted_after_first,
        "cache hit should stage cached products without submitting another runtime job"
    );
    publish_drawing_ink_query_snapshots(&mut app, &mut snapshots, &publication_boundary());
    assert!(cached_tiles.is_disjoint(app.ink_runtime().dirty_tiles()));
    assert!(
        cache
            .snapshot()
            .decisions
            .iter()
            .any(|decision| decision.kind == product::ProductCacheDecisionKind::Hit)
    );
    assert!(
        app.ink_runtime()
            .journal()
            .iter()
            .any(|entry| entry.summary.contains("committed ink cache hit"))
    );
    assert!(
        app.ink_runtime()
            .tile_cache_metadata()
            .iter()
            .all(|metadata| metadata.last_access_frame > last_access_before),
        "cache hits should refresh tile LRU metadata"
    );
}

#[test]
fn app_derived_tile_cache_records_preview_and_final_metadata() {
    let mut app = RunenwerkDrawApp::new();
    let start = screen_point_for_canvas(&app, 512.0, 512.0);
    let end = screen_point_for_canvas(&app, 1_420.0, 1_160.0);
    draw_stroke(&mut app, start, end);
    let document = app
        .document()
        .expect("stroke commit should keep the document open")
        .clone();
    let preview_policy = DrawingTileFormationPolicy::preview();
    let final_policy = DrawingTileFormationPolicy::final_quality();
    let preview = form_drawing_ink_tiles(&document, preview_policy);
    let final_quality = form_drawing_ink_tiles(&document, final_policy);
    assert!(preview.is_accepted(), "{:?}", preview.diagnostics);
    assert!(
        final_quality.is_accepted(),
        "{:?}",
        final_quality.diagnostics
    );
    assert!(!preview.products.is_empty());
    assert!(!final_quality.products.is_empty());

    let mut ink = DrawingInkRuntimeState::default();
    ink.record_cached_products(preview.products.iter());
    ink.record_cached_products(final_quality.products.iter());

    let metadata = ink.tile_cache_metadata();
    let preview_metadata = metadata
        .iter()
        .find(|metadata| metadata.quality_class == ProductQualityClass::Preview)
        .expect("preview cache metadata should be recorded");
    let preview_product = preview
        .products
        .iter()
        .find(|product| product.metadata.tile_id == preview_metadata.tile_id)
        .expect("preview metadata should point at a preview product");
    assert_eq!(
        preview_metadata.descriptor_generation,
        preview_product.descriptor_generation
    );
    assert_eq!(
        preview_metadata.source_revision,
        preview_product.metadata.source_document_revision
    );
    assert_eq!(
        preview_metadata.formation_version,
        preview_policy.formation_version
    );
    assert_eq!(
        preview_metadata.payload_size_bytes,
        preview_product.payload.byte_len()
    );

    let final_metadata = metadata
        .iter()
        .find(|metadata| metadata.quality_class == ProductQualityClass::Final)
        .expect("final cache metadata should be recorded");
    let final_product = final_quality
        .products
        .iter()
        .find(|product| product.metadata.tile_id == final_metadata.tile_id)
        .expect("final metadata should point at a final product");
    assert_eq!(
        final_metadata.descriptor_generation,
        final_product.descriptor_generation
    );
    assert_eq!(
        final_metadata.source_revision,
        final_product.metadata.source_document_revision
    );
    assert_eq!(
        final_metadata.formation_version,
        final_policy.formation_version
    );
    assert_eq!(
        final_metadata.payload_size_bytes,
        final_product.payload.byte_len()
    );
}

#[test]
fn app_derived_tile_cache_evicts_least_recent_unprotected_tile() {
    let products = formed_committed_products_for_cache_tests();
    assert!(
        products.len() >= 3,
        "cache eviction coverage needs at least three formed tiles"
    );
    let payload_size = products[0].payload.byte_len();
    let mut ink = DrawingInkRuntimeState::default();
    ink.set_tile_cache_budget_bytes(payload_size * 2);
    ink.record_cached_products(products[..2].iter());
    assert_eq!(ink.tile_cache_entry_count(), 2);

    let first_source_key = products[0].cache_key.clone();
    let first_tile = products[0].metadata.tile_id;
    let second_tile = products[1].metadata.tile_id;
    let third_tile = products[2].metadata.tile_id;
    assert!(
        ink.cached_product_for_source_key(&first_source_key)
            .is_some()
    );

    ink.record_cached_products([&products[2]]);
    let cached_tiles = ink
        .tile_cache_metadata()
        .iter()
        .map(|metadata| metadata.tile_id)
        .collect::<BTreeSet<_>>();

    assert!(cached_tiles.contains(&first_tile));
    assert!(cached_tiles.contains(&third_tile));
    assert!(
        !cached_tiles.contains(&second_tile),
        "the older unprotected tile should be evicted before the refreshed tile"
    );
    assert!(ink.tile_cache_payload_bytes() <= ink.tile_cache_budget_bytes());
}

#[test]
fn app_derived_tile_cache_keeps_pending_candidates_over_budget() {
    let products = formed_committed_products_for_cache_tests();
    assert!(
        products.len() >= 2,
        "candidate protection coverage needs multiple formed tiles"
    );
    let payload_size = products[0].payload.byte_len();
    let mut ink = DrawingInkRuntimeState::default();
    ink.set_tile_cache_budget_bytes(payload_size);
    ink.record_candidate_products(
        "candidate-cache-protection".to_string(),
        products.iter().map(|product| product.metadata.tile_id),
        products.clone(),
        Vec::new(),
        Vec::new(),
    );
    ink.record_cached_products(products.iter());

    assert_eq!(
        ink.tile_cache_entry_count(),
        products.len(),
        "pending candidate tiles should not be evicted to satisfy the memory budget"
    );
    assert!(ink.tile_cache_payload_bytes() > ink.tile_cache_budget_bytes());
}

#[test]
fn query_snapshots_wait_for_product_publication() {
    let mut app = RunenwerkDrawApp::new();
    let mut snapshots = QuerySnapshotRuntimeResource::default();

    let report =
        publish_drawing_ink_query_snapshots(&mut app, &mut snapshots, &publication_boundary());

    assert_eq!(report.published_count, 0);
    assert!(snapshots.current_snapshots().is_empty());
}

#[test]
fn headless_runtime_starts_and_submits_canvas_first_frame() {
    let app = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run a headless frame");
    let host = app
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    assert!(host.app.document().is_some());
    assert!(host.app.composition_projection().canvas_area_ratio() > 0.45);

    let submissions = app
        .world()
        .resource::<SurfaceFrameSubmissionRegistryResource>()
        .expect("ui frame submission registry should exist");
    let submission = submissions
        .get(&DRAWING_UI_FRAME_PRODUCER_ID)
        .expect("drawing app should submit a UI frame");
    assert_eq!(submission.frame.surfaces[0].id, DRAWING_UI_SURFACE_ID);
    assert!(submission.primitive_count_hint() >= 4);
}

#[test]
fn drawing_runtime_uses_worker_backed_jobs_by_default() {
    let app = build_headless_app().expect("headless app construction should succeed");
    let executor = app
        .world()
        .resource::<RuntimeJobExecutorResource>()
        .expect("runtime job executor should exist");
    assert_eq!(
        executor.config(),
        &RuntimeJobExecutorConfig::worker_pool(2, 64),
        "Draw should use worker-backed jobs unless the app replaces the executor"
    );
}

#[test]
fn headless_runtime_installs_visible_shell_render_prerequisites() {
    let app = build_headless_app()
        .expect("headless app construction should succeed")
        .run_for_frames(1)
        .expect("drawing app should run a headless frame");

    app.world()
        .resource::<SceneRuntimeState>()
        .expect("drawing app should install scene runtime state");

    let prepared_ui = app
        .world()
        .resource::<PreparedUiFrameResource>()
        .expect("prepared ui frame resource should exist");
    assert_eq!(prepared_ui.status, FeatureContributionStatus::Ready);

    let prepared_submission = prepared_ui
        .payload
        .submissions
        .iter()
        .find(|submission| submission.producer_id == DRAWING_UI_FRAME_PRODUCER_ID)
        .expect("drawing app should prepare its UI frame for render composition");
    assert_eq!(
        prepared_submission.frame.surfaces[0].id,
        DRAWING_UI_SURFACE_ID
    );
    assert!(prepared_submission.primitive_count_hint() >= 4);
}

fn center_of_canvas(rect: ui_math::UiRect) -> UiPoint {
    UiPoint::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
}

fn screen_point_for_canvas(app: &RunenwerkDrawApp, x: f64, y: f64) -> UiPoint {
    app.composition_projection()
        .canvas_view
        .canvas_to_screen(CanvasCoordinate::new(x, y))
        .expect("test canvas point should project into the composition")
}

fn assert_canvas_position_close(actual: CanvasCoordinate, expected_x: f64, expected_y: f64) {
    let epsilon = 0.001;
    assert!(
        (actual.x - expected_x).abs() <= epsilon && (actual.y - expected_y).abs() <= epsilon,
        "expected canvas position ({expected_x}, {expected_y}), got ({}, {})",
        actual.x,
        actual.y
    );
}

fn assert_point_close(actual: UiPoint, expected: UiPoint) {
    let epsilon = 0.001;
    assert!(
        (actual.x - expected.x).abs() <= epsilon && (actual.y - expected.y).abs() <= epsilon,
        "expected screen point ({}, {}), got ({}, {})",
        expected.x,
        expected.y,
        actual.x,
        actual.y
    );
}

fn stylus_pointer_event(
    kind: PointerEventKind,
    position: UiPoint,
    timestamp_micros: u64,
    pressure: f32,
) -> UiInputEvent {
    UiInputEvent::Pointer(
        PointerEvent::new(
            kind,
            position,
            UiVector::ZERO,
            Some(PointerButton::Primary),
            Modifiers::default(),
            if kind == PointerEventKind::Down { 1 } else { 0 },
        )
        .with_packet(
            PointerPacket::stylus(PointerDeviceId(901), PointerToolKind::Pen)
                .with_contact(PointerContactState::Contact)
                .with_timestamp_micros(timestamp_micros)
                .with_pressure(pressure)
                .with_latency_class(PointerLatencyClass::LowLatencyPreview),
        ),
    )
}

fn draw_stroke(app: &mut RunenwerkDrawApp, start: UiPoint, end: UiPoint) {
    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Down,
        start,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        1,
    )));
    app.dispatch_input(&UiInputEvent::Pointer(PointerEvent::new(
        PointerEventKind::Up,
        end,
        UiVector::ZERO,
        Some(PointerButton::Primary),
        Modifiers::default(),
        0,
    )));
}

fn draw_and_accept_stroke(
    app: &mut RunenwerkDrawApp,
    publications: &mut ProductPublicationRuntimeResource,
    snapshots: &mut QuerySnapshotRuntimeResource,
    start: UiPoint,
    end: UiPoint,
) {
    draw_stroke(app, start, end);
    publish_visible_ink_until_clean(app, publications, snapshots);
}

fn publish_visible_ink_until_clean(
    app: &mut RunenwerkDrawApp,
    publications: &mut ProductPublicationRuntimeResource,
    snapshots: &mut QuerySnapshotRuntimeResource,
) {
    let mut batches = 0;
    while !app.ink_runtime().dirty_tiles().is_empty() {
        batches += 1;
        assert!(batches <= 8, "dirty tile batches should drain promptly");
        publish_drawing_ink_products(app, publications, &publication_boundary());
        publish_drawing_ink_query_snapshots(app, snapshots, &publication_boundary());
    }
}

fn run_winit_fallback_stroke(
    mut runtime: engine::App,
    start: UiPoint,
    moves: &[UiPoint],
    end: UiPoint,
) -> engine::App {
    {
        let input = runtime
            .world_mut()
            .resource_mut::<InputState>()
            .expect("input state should exist");
        input.handle_cursor_moved(start.x, start.y);
        input.handle_mouse_input(ElementState::Pressed, WinitMouseButton::Left);
    }
    runtime = runtime
        .run_for_frames(1)
        .expect("runtime press frame should run");

    for point in moves {
        {
            let input = runtime
                .world_mut()
                .resource_mut::<InputState>()
                .expect("input state should exist");
            input.handle_cursor_moved(point.x, point.y);
        }
        runtime = runtime
            .run_for_frames(1)
            .expect("runtime move frame should run");
    }

    {
        let input = runtime
            .world_mut()
            .resource_mut::<InputState>()
            .expect("input state should exist");
        input.handle_cursor_moved(end.x, end.y);
        input.handle_mouse_input(ElementState::Released, WinitMouseButton::Left);
    }
    runtime
        .run_for_frames(1)
        .expect("runtime release frame should run")
}

fn run_committed_ink_until_clean(mut runtime: engine::App, expected_strokes: usize) -> engine::App {
    for _ in 0..500 {
        let clean = {
            let host = runtime
                .world()
                .resource::<DrawingHostResource>()
                .expect("drawing host resource should exist");
            let document = host.app.document().expect("document should remain open");
            document.strokes.len() == expected_strokes
                && host.app.ink_runtime().dirty_tiles().is_empty()
                && host.app.ink_runtime().pending_formation_key().is_none()
                && host.app.ink_runtime().visible_product_count() > 0
        };
        if clean {
            return runtime;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
        runtime = runtime
            .run_for_frames(1)
            .expect("runtime committed ink drain frame should run");
    }

    let host = runtime
        .world()
        .resource::<DrawingHostResource>()
        .expect("drawing host resource should exist");
    panic!(
        "runtime committed ink did not drain: strokes={} expected={} dirty_tiles={} pending={:?} visible={}",
        host.app
            .document()
            .expect("document should remain open")
            .strokes
            .len(),
        expected_strokes,
        host.app.ink_runtime().dirty_tiles().len(),
        host.app.ink_runtime().pending_formation_key(),
        host.app.ink_runtime().visible_product_count()
    );
}

fn formed_committed_products_for_cache_tests() -> Vec<drawing::DrawingInkTileProduct> {
    let mut app = RunenwerkDrawApp::new();
    let start = screen_point_for_canvas(&app, 64.0, 64.0);
    let end = screen_point_for_canvas(&app, 1_920.0, 1_760.0);
    draw_stroke(&mut app, start, end);
    let formation = form_drawing_ink_tiles(
        app.document()
            .expect("stroke commit should keep the document open"),
        DrawingTileFormationPolicy::preview(),
    );
    assert!(formation.is_accepted(), "{:?}", formation.diagnostics);
    formation.products
}

fn ink_upload_count(app: &engine::App) -> usize {
    app.world()
        .resource::<RenderDynamicTextureUploadRegistryResource>()
        .expect("dynamic texture upload registry should exist")
        .snapshot()
        .len()
}

fn dynamic_target_ids(app: &engine::App) -> Vec<String> {
    app.world()
        .resource::<RenderDynamicTextureTargetRequestRegistryResource>()
        .expect("dynamic target registry should exist")
        .snapshot()
        .into_iter()
        .map(|descriptor| descriptor.key.target_id)
        .collect()
}

fn publication_boundary() -> PublicationBoundary {
    PublicationBoundary::new(5, "Update", 0)
}

fn rect_primitive_count(frame: &ui_render_data::UiFrame) -> usize {
    frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .filter(|primitive| matches!(primitive, UiPrimitive::Rect(_)))
        .count()
}

fn product_surface_primitive_count(frame: &ui_render_data::UiFrame) -> usize {
    frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .filter(|primitive| matches!(primitive, UiPrimitive::ProductSurface(_)))
        .count()
}

fn product_surface_target_ids(frame: &ui_render_data::UiFrame) -> Vec<String> {
    frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .filter_map(|primitive| match primitive {
            UiPrimitive::ProductSurface(surface) => match &surface.source {
                ProductSurfaceTextureBindingSource::DynamicTexture { target_id, .. } => {
                    Some(target_id.clone())
                }
            },
            _ => None,
        })
        .collect()
}

fn stroke_primitive_count(frame: &ui_render_data::UiFrame) -> usize {
    stroke_primitives(frame).len()
}

fn stroke_primitives(frame: &ui_render_data::UiFrame) -> Vec<&ui_render_data::StrokePrimitive> {
    frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .filter_map(|primitive| match primitive {
            UiPrimitive::Stroke(stroke) => Some(stroke),
            _ => None,
        })
        .collect()
}

fn visible_tile_ids(app: &RunenwerkDrawApp) -> BTreeSet<CanvasTileId> {
    app.ink_runtime()
        .visible_products()
        .map(|product| product.metadata.tile_id)
        .collect()
}
