//! Opt-in compositor regression: real libmpv -> GL texture -> Slint -> pixels.
//! Readback exists only in this test, never in the production playback path.
#[path = "cadence_window.rs"]
mod cadence_window;
use super::*;
use cadence_window::CadenceWindow;
use slint::ComponentHandle;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    rc::Rc,
    time::{Duration, Instant},
};

slint::slint! {
    export component VideoProbe inherits Window {
        width: 320px; height: 180px; background: lime;
        in property <image> frame;
        in property <image> thumbnail-frame;
        in property <bool> thumbnail-available: false;
        in property <bool> clip-video: true;
        Rectangle {
            clip: root.clip-video;
            border-radius: 12px;
            // Leave the right side uncovered so preview pixels can be
            // distinguished from the main video beneath them.
            Image { source: root.frame; width: 180px; height: 180px; image-fit: fill; }
        }
        if (root.thumbnail-available): Image {
            source: root.thumbnail-frame;
            x: 192px; y: 8px; width: 120px; height: 68px;
            image-fit: contain;
        }
    }
}

#[test]
fn publishing_a_new_frame_invalidates_layers_even_when_texture_identity_is_reused() {
    struct TestPlatform;
    impl slint::platform::Platform for TestPlatform {
        fn create_window_adapter(
            &self,
        ) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
            Ok(
                slint::platform::software_renderer::MinimalSoftwareWindow::new(
                    slint::platform::software_renderer::RepaintBufferType::ReusedBuffer,
                ),
            )
        }
    }
    slint::platform::set_platform(Box::new(TestPlatform)).unwrap();
    let ui = VideoProbe::new().unwrap();
    let frame = slint::Image::from_rgba8(slint::SharedPixelBuffer::new(8, 8));
    publish_frame(frame.clone(), None, &|image, _| ui.set_frame(image));
    // Use the same dependency tracker as Slint's rounded-layer cache. This
    // checks invalidation itself, without needing a native window or GPU.
    let source = Box::pin(slint::private_unstable_api::re_exports::Property::<
        slint::Image,
    >::default());
    let weak = ui.as_weak();
    source
        .as_ref()
        .set_binding(move || weak.upgrade().unwrap().get_frame());
    let tracker = Box::pin(slint::private_unstable_api::re_exports::PropertyTracker::<
        false,
    >::default());
    tracker.as_ref().evaluate(|| source.as_ref().get());
    assert!(
        !tracker.is_dirty(),
        "The initial layer must be clean before the next frame"
    );
    // GPU pixels can change while texture id, dimensions and origin stay equal.
    publish_frame(frame.clone(), None, &|image, _| ui.set_frame(image));
    assert!(
        tracker.is_dirty(),
        "A new frame with reused image identity did not invalidate the cached video layer"
    );
    assert_eq!(
        ui.get_frame(),
        frame,
        "The published frame must remain selected after invalidation"
    );
}

#[derive(Default)]
struct Samples {
    timing: CadenceWindow,
    presentations: usize,
    summaries: Vec<String>,
    colors: Vec<[u8; 6]>,
    read_errors: Vec<u32>,
    thumbnail_frames: HashMap<u64, u64>,
    thumbnail_pixels: Vec<[u8; 6]>,
    upright: usize,
    inverted: usize,
    frames: HashSet<u64>,
    errors: Vec<String>,
}

#[test]
#[ignore = "Requires an interactive or Xvfb OpenGL display, libmpv, and FOXIDE_RENDER_FIXTURE"]
fn composited_video_is_upright_and_advances_at_video_cadence() {
    let fixture = std::env::var("FOXIDE_RENDER_FIXTURE")
        .expect("Set FOXIDE_RENDER_FIXTURE to the asymmetric test video");
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()
        .unwrap();
    let ui = VideoProbe::new().unwrap();
    let engine = Rc::new(Engine::new().unwrap());
    let thumbnails = Arc::new(crate::ThumbnailController::new());
    let samples = Rc::new(RefCell::new(Samples::default()));
    let ready_buckets = Rc::new(RefCell::new(HashSet::<u64>::new()));
    let displayed_bucket = Rc::new(std::cell::Cell::new(None::<u64>));
    let second_requested = Rc::new(std::cell::Cell::new(false));
    let graphics = Rc::new(RefCell::new(None::<glow::Context>));
    let started = Instant::now();
    let weak_size = ui.as_weak();
    let weak_image = ui.as_weak();
    let weak_wake = ui.as_weak();
    let weak_probe = ui.as_weak();
    let errors = samples.clone();
    let observations = samples.clone();
    let load_engine = engine.clone();
    let observer_thumbnails = thumbnails.clone();
    let observer_buckets = ready_buckets.clone();
    let observer_displayed_bucket = displayed_bucket.clone();
    let observer_second_requested = second_requested.clone();
    let observer_fixture = fixture.clone();
    attach_inner(
        ui.window(),
        &engine,
        Some(thumbnails.clone()),
        move || {
            weak_size
                .upgrade()
                .map(|w| {
                    let s = w.window().size();
                    (s.width, s.height)
                })
                .unwrap_or((1, 1))
        },
        || false,
        move |frame, _pip_frame| {
            if let Some(w) = weak_image.upgrade() {
                w.set_frame(frame);
            }
        },
        {
            let weak = ui.as_weak();
            let controller = thumbnails.clone();
            let buckets = ready_buckets.clone();
            let displayed = displayed_bucket.clone();
            move |frame, available| {
                if let Some(w) = weak.upgrade() {
                    w.set_thumbnail_frame(frame.clone());
                    w.set_thumbnail_available(available);
                }
                if available {
                    if let Some(request) = controller.current_request() {
                        let bucket = (request.position / 2.0).floor() as u64;
                        displayed.set(Some(bucket));
                        buckets.borrow_mut().insert(bucket);
                    } else {
                        displayed.set(None);
                    }
                    assert_eq!(frame.size().width, 256);
                    assert_eq!(frame.size().height, 144);
                } else {
                    displayed.set(None);
                }
            }
        },
        move |error| errors.borrow_mut().errors.push(error),
        Arc::new(move || {
            let weak = weak_wake.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = weak.upgrade() {
                    w.window().request_redraw();
                }
            });
        }),
        move |state, api| {
            if matches!(state, slint::RenderingState::RenderingSetup) {
                if let slint::GraphicsAPI::NativeOpenGL { get_proc_address } = api {
                    // SAFETY: the test observer runs with Slint's context current.
                    let gl = unsafe {
                        glow::Context::from_loader_function(|name| {
                            get_proc_address(&CString::new(name).unwrap())
                        })
                    };
                    // SAFETY: setup observer runs with the current probe context.
                    unsafe {
                        println!(
                            "[video-probe] GL_VERSION={} GL_RENDERER={}",
                            gl.get_parameter_string(glow::VERSION),
                            gl.get_parameter_string(glow::RENDERER)
                        );
                    }
                    *graphics.borrow_mut() = Some(gl);
                }
            } else if matches!(state, slint::RenderingState::AfterRendering) {
                let presentation = {
                    let mut samples = observations.borrow_mut();
                    samples.presentations += 1;
                    samples.presentations
                };
                let Some(w) = weak_probe.upgrade() else {
                    return;
                };
                if presentation == 1 {
                    if w.get_frame().size().width != 0 || w.get_frame().size().height != 0 {
                        observations
                            .borrow_mut()
                            .errors
                            .push("Published a video surface before any media was opened".into());
                    }
                    // This only queues a control command; no normal libmpv API
                    // is called on the rendering thread.
                    load_engine
                        .command(vec![
                            "loadfile".into(),
                            observer_fixture.clone(),
                            "replace".into(),
                        ])
                        .unwrap();
                    assert!(observer_thumbnails.request(observer_fixture.clone(), 1, 2.0, 10.0));
                } else if observer_buckets.borrow().contains(&1)
                    && !observer_second_requested.replace(true)
                {
                    assert!(observer_thumbnails.request(observer_fixture.clone(), 1, 4.0, 10.0));
                }
                let size = w.window().size();
                if size.width == 0 || size.height < 8 {
                    return;
                }
                let gl = graphics.borrow();
                let Some(gl) = gl.as_ref() else { return };
                let mut pixels = vec![0_u8; size.width as usize * size.height as usize * 4];
                // SAFETY: test-only readback from the completed window backbuffer.
                // The allocated RGBA byte slice exactly covers the requested size.
                unsafe {
                    let previous = gl.get_parameter_framebuffer(glow::READ_FRAMEBUFFER_BINDING);
                    gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
                    gl.read_pixels(
                        0,
                        0,
                        size.width as i32,
                        size.height as i32,
                        glow::RGBA,
                        glow::UNSIGNED_BYTE,
                        glow::PixelPackData::Slice(Some(&mut pixels)),
                    );
                    gl.bind_framebuffer(glow::READ_FRAMEBUFFER, previous);
                }
                let pixel = |x: u32, y: u32| {
                    let i = ((y * size.width + x) * 4) as usize;
                    &pixels[i..i + 3]
                };
                // OpenGL readback row zero is the bottom of the final window.
                let top = pixel(size.width / 4, size.height * 7 / 8);
                let bottom = pixel(size.width / 4, size.height / 8);
                let red = |p: &[u8]| p[0] > 150 && p[1] < 80 && p[2] < 80;
                let blue = |p: &[u8]| p[2] > 150 && p[0] < 80 && p[1] < 80;
                let mut samples = observations.borrow_mut();
                if samples.summaries.len() < 2 {
                    let max = pixels.iter().copied().max().unwrap_or(0);
                    let nonblack = pixels
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0)
                        .count();
                    let source = w.get_frame();
                    let image = source.size();
                    samples.summaries.push(format!(
                        "surface={}x{} image={}x{} first={:?} max={} nonblack={}",
                        size.width,
                        size.height,
                        image.width,
                        image.height,
                        &pixels[..4],
                        max,
                        nonblack
                    ));
                }
                if samples.colors.len() < 4 {
                    samples
                        .colors
                        .push([top[0], top[1], top[2], bottom[0], bottom[1], bottom[2]]);
                }
                if w.get_thumbnail_available() {
                    // The preview sits on the lime background, outside the
                    // main video rectangle. Red-over-blue pixels prove that
                    // the decoded frame was uploaded and composited.
                    let preview_top = pixel(200, size.height - 20);
                    let preview_bottom = pixel(200, size.height - 67);
                    if let Some(bucket) = observer_displayed_bucket.get() {
                        let mut frame_hash = std::hash::DefaultHasher::new();
                        for ui_y in 8..76 {
                            let read_y = size.height - 1 - ui_y;
                            for x in 192..312 {
                                let i = ((read_y * size.width + x) * 4) as usize;
                                pixels[i..i + 3].hash(&mut frame_hash);
                            }
                        }
                        samples
                            .thumbnail_frames
                            .entry(bucket)
                            .or_insert_with(|| frame_hash.finish());
                    }
                    if samples.thumbnail_pixels.len() < 8 {
                        samples.thumbnail_pixels.push([
                            preview_top[0],
                            preview_top[1],
                            preview_top[2],
                            preview_bottom[0],
                            preview_bottom[1],
                            preview_bottom[2],
                        ]);
                    }
                }
                // SAFETY: the observer still runs with the same GL context current.
                let read_error = unsafe { gl.get_error() };
                if read_error != 0 {
                    samples.read_errors.push(read_error);
                }
                samples.timing.observe(
                    started.elapsed(),
                    (red(top) && blue(bottom)) || (blue(top) && red(bottom)),
                );
                if red(top) && blue(bottom) {
                    samples.upright += 1;
                }
                if blue(top) && red(bottom) {
                    samples.inverted += 1;
                }
                if (red(top) && blue(bottom)) || (blue(top) && red(bottom)) {
                    let mut hash = std::hash::DefaultHasher::new();
                    pixels.hash(&mut hash);
                    samples.frames.insert(hash.finish());
                }
            }
        },
    )
    .unwrap();
    let timer = slint::Timer::default();
    let sample_clock = samples.clone();
    let timer_buckets = ready_buckets.clone();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(20),
        move || {
            let elapsed = started.elapsed();
            if (sample_clock.borrow().timing.finished(elapsed)
                && timer_buckets.borrow().contains(&1)
                && timer_buckets.borrow().contains(&2))
                || elapsed >= Duration::from_secs(20)
            {
                slint::quit_event_loop().unwrap();
            }
        },
    );
    ui.run().unwrap();
    if let Some(state) = engine.snapshot() {
        println!(
            "[video-probe] loaded={} paused={} position={} duration={} decoder={}",
            state.loaded, state.paused, state.position, state.duration, state.hwdec
        );
    }
    let samples = samples.borrow();
    if samples.frames.len() < 75
        && let Ok(logs) = engine.diagnostics.lock()
    {
        println!("[engine-diagnostics] {}", logs.join(""));
    }
    println!(
        "[video-probe] presentations={} colors={:?} read_errors={:?}",
        samples.presentations, samples.colors, samples.read_errors
    );
    println!("[video-probe] summaries={:?}", samples.summaries);
    println!(
        "[video-probe] thumbnail_frames={:?}",
        samples.thumbnail_frames
    );
    println!(
        "[video-probe] thumbnail_pixels={:?}",
        samples.thumbnail_pixels
    );
    println!(
        "upright={}, inverted={}, unique_composited_frames={}, errors={:?}",
        samples.upright,
        samples.inverted,
        samples.frames.len(),
        samples.errors
    );
    assert!(
        samples.timing.complete(started.elapsed()),
        "Video did not start within 15 seconds or complete the five-second cadence window"
    );
    assert!(
        samples.read_errors.is_empty(),
        "OpenGL readback errors: {:?}",
        samples.read_errors
    );
    assert!(
        samples.errors.is_empty(),
        "Native rendering errors: {:?}",
        samples.errors
    );
    assert!(
        samples.upright > 0 && samples.inverted == 0,
        "Video orientation is wrong: upright={}, inverted={}",
        samples.upright,
        samples.inverted
    );
    assert!(
        samples.frames.len() >= 75,
        "Video stalled: only {} distinct composited frames over five seconds after first presented video",
        samples.frames.len()
    );
    let buckets = ready_buckets.borrow();
    let preview_engine = thumbnails.preview_engine();
    let preview_snapshot = preview_engine.as_ref().and_then(|engine| engine.snapshot());
    assert!(
        buckets.contains(&1),
        "first hover preview was not rendered; preview_engine={} loaded={} request_matches={} path_matches={} position={} duration={} error={:?} renderer_errors={:?}",
        preview_engine.is_some(),
        preview_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.loaded),
        preview_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.requested_path.as_ref() == fixture.as_str()),
        preview_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.path == fixture),
        preview_snapshot
            .as_ref()
            .map_or(-1.0, |snapshot| snapshot.position),
        preview_snapshot
            .as_ref()
            .map_or(-1.0, |snapshot| snapshot.duration),
        preview_snapshot
            .as_ref()
            .map(|snapshot| snapshot.error.as_str()),
        samples.errors
    );
    assert!(
        buckets.contains(&2),
        "second hover preview did not use another cache slot"
    );
    assert!(
        samples.thumbnail_pixels.iter().any(|pixels| {
            pixels[0] > 150
                && pixels[1] < 80
                && pixels[2] < 80
                && pixels[5] > 150
                && pixels[3] < 80
                && pixels[4] < 80
        }),
        "the real-media thumbnail did not composite its red-over-blue fixture pixels"
    );
    assert!(
        samples.thumbnail_frames.contains_key(&1) && samples.thumbnail_frames.contains_key(&2),
        "real-media pixels were not captured for both requested time buckets"
    );
    assert_ne!(
        samples.thumbnail_frames.get(&1),
        samples.thumbnail_frames.get(&2),
        "hovering at different time buckets reused the same composited thumbnail frame"
    );
}
