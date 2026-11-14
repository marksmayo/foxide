//! Opt-in compositor regression: real libmpv -> GL texture -> Slint -> pixels.
//! Readback exists only in this test, never in the production playback path.
use super::*;
use slint::ComponentHandle;
use std::{cell::RefCell, collections::HashSet, hash::{Hash, Hasher}, rc::Rc, time::{Duration, Instant}};

slint::slint! {
    export component VideoProbe inherits Window {
        width: 320px; height: 180px; background: black;
        in property <image> frame;
        Image { source: root.frame; image-fit: fill; }
    }
}

#[derive(Default)]
struct Samples {
    upright: usize,
    inverted: usize,
    frames: HashSet<u64>,
    errors: Vec<String>,
}

#[test]
#[ignore = "Requires an interactive or Xvfb OpenGL display, libmpv, and FOXIDE_RENDER_FIXTURE"]
fn composited_video_is_upright_and_advances_at_video_cadence() {
    let fixture = std::env::var("FOXIDE_RENDER_FIXTURE").expect("Set FOXIDE_RENDER_FIXTURE to the asymmetric test video");
    slint::BackendSelector::new().backend_name("winit".into()).renderer_name("femtovg".into()).select().unwrap();
    let ui = VideoProbe::new().unwrap();
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
    attach_observed(ui.window(), &engine,
        move || weak_size.upgrade().map(|w| { let s = w.window().size(); (s.width, s.height) }).unwrap_or((1, 1)),
        move |frame| if let Some(w) = weak_image.upgrade() { w.set_frame(frame); },
        move |error| errors.borrow_mut().errors.push(error),
        Arc::new(move || { let weak = weak_wake.clone(); let _ = slint::invoke_from_event_loop(move || if let Some(w) = weak.upgrade() { w.window().request_redraw(); }); }),
        move |state, api| {
            if matches!(state, slint::RenderingState::RenderingSetup) {
                if let slint::GraphicsAPI::NativeOpenGL { get_proc_address } = api {
                    // SAFETY: the test observer runs with Slint's context current.
                    *graphics.borrow_mut() = Some(unsafe { glow::Context::from_loader_function(|name| get_proc_address(&CString::new(name).unwrap())) });
                }
            } else if matches!(state, slint::RenderingState::AfterRendering) && started.elapsed().as_secs_f64() >= 1.0 {
                let Some(w) = weak_probe.upgrade() else { return };
                let size = w.window().size();
                if size.width == 0 || size.height < 8 { return; }
                let gl = graphics.borrow();
                let Some(gl) = gl.as_ref() else { return };
                let mut pixels = vec![0_u8; size.width as usize * size.height as usize * 4];
                // SAFETY: test-only readback from the completed window backbuffer.
                // The allocated RGBA byte slice exactly covers the requested size.
                unsafe {
                    let previous = gl.get_parameter_framebuffer(glow::READ_FRAMEBUFFER_BINDING);
                    gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
                    gl.read_pixels(0, 0, size.width as i32, size.height as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut pixels)));
                    gl.bind_framebuffer(glow::READ_FRAMEBUFFER, previous);
                }
                let pixel = |y: u32| { let i = ((y * size.width + size.width/2) * 4) as usize; &pixels[i..i+3] };
                // OpenGL readback row zero is the bottom of the final window.
                let top = pixel(size.height * 7 / 8);
                let bottom = pixel(size.height / 8);
                let red = |p: &[u8]| p[0] > 150 && p[1] < 80 && p[2] < 80;
                let blue = |p: &[u8]| p[2] > 150 && p[0] < 80 && p[1] < 80;
                let mut samples = observations.borrow_mut();
                if red(top) && blue(bottom) { samples.upright += 1; }
                if blue(top) && red(bottom) { samples.inverted += 1; }
                if (red(top) && blue(bottom)) || (blue(top) && red(bottom)) {
                    let mut hash = std::hash::DefaultHasher::new();
                    pixels.hash(&mut hash);
                    samples.frames.insert(hash.finish());
                }
            }
        }).unwrap();
    engine.command(vec!["loadfile".into(), fixture, "replace".into()]).unwrap();
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::SingleShot, Duration::from_secs(6), || { slint::quit_event_loop().unwrap(); });
    ui.run().unwrap();
    let samples = samples.borrow();
    println!("upright={}, inverted={}, unique_composited_frames={}, errors={:?}", samples.upright, samples.inverted, samples.frames.len(), samples.errors);
    assert!(samples.errors.is_empty(), "Native rendering errors: {:?}", samples.errors);
    assert!(samples.upright > 0 && samples.inverted == 0, "Video orientation is wrong: upright={}, inverted={}", samples.upright, samples.inverted);
    assert!(samples.frames.len() >= 75, "Video stalled: only {} distinct composited frames over five seconds of 30fps media", samples.frames.len());
}
