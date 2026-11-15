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
        in property <bool> clip-video: true;
        Rectangle {
            clip: root.clip-video;
            border-radius: 12px;
            Image { source: root.frame; image-fit: fill; }
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
    let selector = slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into());
    let selector = if std::env::var("FOXIDE_PROBE_GL_API").as_deref() == Ok("desktop") {
        selector.require_opengl_with_version(3, 3)
    } else {
        selector
    };
    selector.select().unwrap();
    let ui = VideoProbe::new().unwrap();
    ui.set_clip_video(std::env::var_os("FOXIDE_PROBE_UNCLIPPED").is_none());
    let engine = Engine::new().unwrap();
    let samples = Rc::new(RefCell::new(Samples::default()));
    let graphics = Rc::new(RefCell::new(None::<glow::Context>));
    let started = Instant::now();
    let weak_size = ui.as_weak();
    let weak_image = ui.as_weak();
    let weak_wake = ui.as_weak();
    let weak_probe = ui.as_weak();
    let errors = samples.clone();
    let observations = samples.clone();
    attach_observed(
        ui.window(),
        &engine,
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
                        println!("[video-probe] GL_VERSION={} GL_RENDERER={}",
                                 gl.get_parameter_string(glow::VERSION),
                                 gl.get_parameter_string(glow::RENDERER));
                    }
                    *graphics.borrow_mut() = Some(gl);
                }
            } else if matches!(state, slint::RenderingState::AfterRendering) {
                observations.borrow_mut().presentations += 1;
                let Some(w) = weak_probe.upgrade() else {
                    return;
                };
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
                    let inner: &slint::private_unstable_api::re_exports::ImageInner = (&source).into();
                    if let slint::private_unstable_api::re_exports::ImageInner::BorrowedOpenGLTexture(texture) = inner {
                        // SAFETY: test-only diagnostic readback; the borrowed texture
                        // belongs to this current context and the FBO is deleted below.
                        unsafe {
                            let previous = gl.get_parameter_framebuffer(glow::READ_FRAMEBUFFER_BINDING);
                            let fbo = gl.create_framebuffer().unwrap();
                            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(fbo));
                            gl.framebuffer_texture_2d(glow::READ_FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(glow::NativeTexture(texture.texture_id)), 0);
                            let mut raw = [0_u8; 8];
                            gl.read_pixels((size.width/2) as i32, (size.height/8) as i32, 1, 1, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut raw[..4])));
                            gl.read_pixels((size.width/2) as i32, (size.height*7/8) as i32, 1, 1, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut raw[4..])));
                            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, previous);
                            gl.delete_framebuffer(fbo);
                            samples.summaries.push(format!("texture={} raw={raw:?}", texture.texture_id));
                        }
                    }
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
    engine
        .command(vec!["loadfile".into(), fixture, "replace".into()])
        .unwrap();
    let timer = slint::Timer::default();
    let sample_clock = samples.clone();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(20),
        move || {
            if sample_clock.borrow().timing.finished(started.elapsed()) {
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
}
