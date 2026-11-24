//! Opt-in compositor regression: real libmpv -> GL texture -> Slint -> pixels.
//! Readback exists only in this test, never in the production playback path.
#[path = "cadence_window.rs"]
mod cadence_window;
use super::*;
use cadence_window::CadenceWindow;
use slint::ComponentHandle;
use std::{
    cell::RefCell,
    collections::HashSet,
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
            Image { source: root.frame; image-fit: fill; }
        }
        if (root.thumbnail-available): Image {
            source: root.thumbnail-frame;
            x: 8px; y: 8px; width: 128px; height: 72px;
            image-fit: contain;
        }
    }
    export component AudioProbe inherits Window {
        width: 960px; height: 270px; background: black;
        in property <image> frame;
        Image { source: root.frame; width: 100%; height: 100%; image-fit: fill; }
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
    publish_frame(frame.clone(), &|image| ui.set_frame(image));
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
    publish_frame(frame.clone(), &|image| ui.set_frame(image));
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
        move |frame| {
            if let Some(w) = weak_image.upgrade() {
                w.set_frame(frame);
            }
        },
        {
            let weak = ui.as_weak();
            let controller = thumbnails.clone();
            let buckets = ready_buckets.clone();
            move |frame, available| {
                if let Some(w) = weak.upgrade() {
                    w.set_thumbnail_frame(frame.clone());
                    w.set_thumbnail_available(available);
                }
                if available {
                    if let Some(request) = controller.current_request() {
                        buckets
                            .borrow_mut()
                            .insert((request.position / 2.0).floor() as u64);
                    }
                    assert_eq!(frame.size().width, 256);
                    assert_eq!(frame.size().height, 144);
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
                let pixel = |y: u32| {
                    let i = ((y * size.width + size.width / 2) * 4) as usize;
                    &pixels[i..i + 3]
                };
                // OpenGL readback row zero is the bottom of the final window.
                let top = pixel(size.height * 7 / 8);
                let bottom = pixel(size.height / 8);
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
}

#[test]
#[ignore = "Requires Xvfb/OpenGL, libmpv and FOXIDE_AUDIO_VISUALIZATION_FIXTURE"]
fn audio_visualization_renders_changing_waveform_pixels() {
    use std::cell::Cell;

    let fixture = std::env::var("FOXIDE_AUDIO_VISUALIZATION_FIXTURE")
        .expect("Set FOXIDE_AUDIO_VISUALIZATION_FIXTURE to a frequency-sweep WAV");
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()
        .unwrap();
    let ui = AudioProbe::new().unwrap();
    let engine = Rc::new(Engine::new().unwrap());
    let started = Instant::now();
    let graphics = Rc::new(RefCell::new(None::<glow::Context>));
    let graph_sent = Rc::new(Cell::new(false));
    let start_position = Rc::new(Cell::new(0.0));
    let graph_started = Rc::new(Cell::new(None::<Instant>));
    let frame_times = Rc::new(RefCell::new(Vec::<Duration>::new()));
    let rss_baseline_kib = Rc::new(Cell::new(0_u64));
    let rss_peak_kib = Rc::new(Cell::new(0_u64));
    let samples = Rc::new(RefCell::new((
        0_usize,
        HashSet::<u64>::new(),
        Vec::new(),
        0_usize,
        (0_u32, 0_u32),
        false,
    )));

    let weak_size = ui.as_weak();
    let weak_image = ui.as_weak();
    let weak_wakeup = ui.as_weak();
    let weak_readback = ui.as_weak();
    let image_samples = samples.clone();
    let rendered_graph_started = graph_started.clone();
    let rendered_frame_times = frame_times.clone();
    let wakeup = Arc::new(move || {
        let weak = weak_wakeup.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(window) = weak.upgrade() {
                window.window().request_redraw();
            }
        });
    });

    let errors = samples.clone();
    let observed_samples = samples.clone();
    let observed_graph_sent = graph_sent.clone();
    let observed_graphics = graphics.clone();
    attach_inner(
        ui.window(),
        &engine,
        None,
        move || {
            weak_size
                .upgrade()
                .map(|window| {
                    let size = window.window().size();
                    (size.width, size.height)
                })
                .unwrap_or((1, 1))
        },
        move |frame| {
            if let Some(window) = weak_image.upgrade() {
                let size = frame.size();
                image_samples.borrow_mut().4 = (size.width, size.height);
                window.set_frame(frame);
            }
        },
        |_, _| {},
        move |error| errors.borrow_mut().2.push(error),
        wakeup,
        move |state, api| {
            if matches!(state, slint::RenderingState::RenderingSetup) {
                if let slint::GraphicsAPI::NativeOpenGL { get_proc_address } = api {
                    observed_samples.borrow_mut().5 = true;
                    // SAFETY: setup observer runs with the compositor context current.
                    let gl = unsafe {
                        glow::Context::from_loader_function(|name| {
                            get_proc_address(&CString::new(name).unwrap())
                        })
                    };
                    *observed_graphics.borrow_mut() = Some(gl);
                } else {
                    observed_samples
                        .borrow_mut()
                        .2
                        .push("Slint did not provide a native OpenGL context".into());
                }
            } else if matches!(state, slint::RenderingState::AfterRendering)
                && observed_graph_sent.get()
            {
                let Some(window) = weak_readback.upgrade() else {
                    return;
                };
                let size = window.window().size();
                let graphics = observed_graphics.borrow();
                let Some(gl) = graphics.as_ref() else {
                    return;
                };
                if size.width == 0 || size.height == 0 {
                    return;
                }
                let mut pixels = vec![0_u8; size.width as usize * size.height as usize * 4];
                // SAFETY: readback covers the full completed window buffer.
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
                let green_pixels = pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|pixel| {
                        u16::from(pixel[1]) > u16::from(pixel[0]) + 8
                            && u16::from(pixel[1]) > u16::from(pixel[2]) + 12
                    })
                    .count();
                let nonblack_pixels = pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|pixel| pixel[0] > 10 || pixel[1] > 10 || pixel[2] > 10)
                    .count();
                let mut samples = observed_samples.borrow_mut();
                samples.3 = samples.3.max(nonblack_pixels);
                if green_pixels > 0 {
                    let mut hash = std::hash::DefaultHasher::new();
                    pixels.hash(&mut hash);
                    samples.0 = samples.0.max(green_pixels);
                    if samples.1.insert(hash.finish())
                        && let Some(graph_started) = rendered_graph_started.get()
                    {
                        rendered_frame_times
                            .borrow_mut()
                            .push(graph_started.elapsed());
                    }
                }
            }
        },
    )
    .unwrap();

    engine
        .command(vec!["set".into(), "ao".into(), "null".into()])
        .unwrap();
    engine
        .command(vec!["loadfile".into(), fixture, "replace".into()])
        .unwrap();
    let timer_engine = engine.clone();
    let timer_graph_sent = graph_sent.clone();
    let timer_start_position = start_position.clone();
    let timer_graph_started = graph_started.clone();
    let timer_rss_baseline = rss_baseline_kib.clone();
    let timer_rss_peak = rss_peak_kib.clone();
    let timer_samples = samples.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(20),
        move || {
            let snapshot = timer_engine.snapshot().unwrap_or_default();
            if !snapshot.error.is_empty() {
                timer_samples.borrow_mut().2.push(snapshot.error);
                slint::quit_event_loop().unwrap();
                return;
            }
            if timer_graph_sent.get() && !snapshot.command_error.is_empty() {
                timer_samples.borrow_mut().2.push(snapshot.command_error);
                slint::quit_event_loop().unwrap();
                return;
            }
            if !timer_graph_sent.get()
                && snapshot.loaded
                && let Some(track) = snapshot
                    .tracks
                    .iter()
                    .find(|track| track.kind == "audio" && track.selected)
            {
                let command = crate::audio_visualization_filter(track.id);
                if let Err(error) =
                    timer_engine.command(vec!["set".into(), "lavfi-complex".into(), command])
                {
                    timer_samples.borrow_mut().2.push(error);
                    slint::quit_event_loop().unwrap();
                    return;
                }
                timer_start_position.set(snapshot.position);
                timer_graph_sent.set(true);
                let now = Instant::now();
                timer_graph_started.set(Some(now));
                if let Some(rss) = process_rss_kib() {
                    timer_rss_baseline.set(rss);
                    timer_rss_peak.set(rss);
                }
            }
            if let Some(graph_started) = timer_graph_started.get() {
                if let Some(rss) = process_rss_kib() {
                    timer_rss_peak.set(timer_rss_peak.get().max(rss));
                }
                if graph_started.elapsed() >= Duration::from_secs(3) {
                    slint::quit_event_loop().unwrap();
                }
            } else if started.elapsed() >= Duration::from_secs(20) {
                timer_samples
                    .borrow_mut()
                    .2
                    .push("Timed out waiting for the audio waveform graph".into());
                slint::quit_event_loop().unwrap();
            }
        },
    );
    ui.run().unwrap();
    let snapshot = engine.snapshot().unwrap_or_default();
    let captured = samples.borrow();
    let (max_green_pixels, frames, errors, max_nonblack_pixels, frame_size, rendering_setup) =
        &*captured;
    let diagnostics = engine
        .diagnostics
        .lock()
        .map(|logs| {
            logs.iter()
                .filter(|line| {
                    line.contains("lavfi")
                        || line.contains("AVFilter")
                        || line.contains("showwaves")
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let observed_frame_times = frame_times.borrow();
    let frame_span = observed_frame_times
        .first()
        .zip(observed_frame_times.last())
        .map(|(first, last)| last.saturating_sub(*first))
        .unwrap_or_default();
    let unique_frame_rate = if frame_span.is_zero() {
        0.0
    } else {
        observed_frame_times.len().saturating_sub(1) as f64 / frame_span.as_secs_f64()
    };
    let max_frame_interval = observed_frame_times
        .windows(2)
        .map(|pair| pair[1].saturating_sub(pair[0]))
        .max()
        .unwrap_or_default();
    let rss_delta_kib = rss_peak_kib.get().saturating_sub(rss_baseline_kib.get());
    println!(
        "waveform max_green_pixels={max_green_pixels} max_nonblack_pixels={max_nonblack_pixels} unique_frames={} unique_frame_rate_hz={unique_frame_rate:.2} max_frame_interval_ms={} frame_size={frame_size:?} rss_baseline_kib={} rss_sampled_peak_kib={} rss_sampled_delta_kib={} rendering_setup={rendering_setup} position={} loaded={} command_error={:?} errors={errors:?} diagnostics={diagnostics:?}",
        frames.len(),
        max_frame_interval.as_millis(),
        rss_baseline_kib.get(),
        rss_peak_kib.get(),
        rss_delta_kib,
        snapshot.position,
        snapshot.loaded,
        snapshot.command_error
    );
    assert!(errors.is_empty(), "Native renderer errors: {errors:?}");
    assert!(
        graph_sent.get(),
        "Audio visualization graph was not submitted"
    );
    assert!(
        snapshot.position > start_position.get() + 0.1,
        "Audio did not continue with the visualization graph active"
    );
    assert!(
        *max_green_pixels >= 10,
        "No visible green waveform pixels reached the desktop compositor"
    );
    assert!(
        frames.len() >= 5,
        "Waveform was static or never rendered: only {} distinct frames",
        frames.len()
    );
    assert!(
        unique_frame_rate >= 20.0,
        "Waveform frame rate {unique_frame_rate:.2} Hz fell below the 20 Hz CI budget"
    );
    assert!(
        max_frame_interval <= Duration::from_millis(250),
        "Waveform frame gap {} ms exceeded the 250 ms CI budget",
        max_frame_interval.as_millis()
    );
    if rss_baseline_kib.get() > 0 {
        assert!(
            rss_delta_kib <= 64 * 1024,
            "Waveform RSS growth {rss_delta_kib} KiB exceeded the 65536 KiB CI budget"
        );
    }
}

fn process_rss_kib() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let resident = status.lines().find(|line| line.starts_with("VmRSS:"))?;
        return resident.split_whitespace().nth(1)?.parse().ok();
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}
