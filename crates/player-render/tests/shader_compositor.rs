//! Test-only compositor acceptance for trusted shader apply, failure and reset.
use glow::HasContext;
use player_render::{Engine, attach_observed};
use slint::ComponentHandle;
use std::{
    cell::RefCell,
    ffi::CString,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

slint::slint! {
    export component ArtworkProbe inherits Window {
        width: 128px; height: 128px; background: lime;
        in property <image> frame;
        Image { source: root.frame; width: parent.width; height: parent.height; image-fit: contain; }
    }
}
#[derive(Default)]
struct Proof {
    phase: u8,
    logged_phase: Option<u8>,
    loaded: bool,
    upright: bool,
    pixels: Vec<u8>,
    size: (u32, u32),
    error: String,
}

#[test]
#[ignore = "Requires libmpv and a native or Xvfb OpenGL display"]
fn selected_shader_changes_pixels_reports_errors_and_can_be_cleared() {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()
        .unwrap();
    let ui = ArtworkProbe::new().unwrap();
    let engine = Rc::new(Engine::new().unwrap());
    let proof = Rc::new(RefCell::new(Proof::default()));
    let graphics = Rc::new(RefCell::new(None::<glow::Context>));
    let (weak_frame, weak_wake, weak_probe) = (ui.as_weak(), ui.as_weak(), ui.as_weak());
    let (errors, observed, playback) = (proof.clone(), proof.clone(), engine.clone());
    let opened = Rc::new(std::cell::Cell::new(false));
    let open_once = opened.clone();
    attach_observed(ui.window(), &engine, || (56, 56),
        move |frame| { if let Some(ui) = weak_frame.upgrade() { ui.set_frame(frame); } },
        move |error| errors.borrow_mut().error = error,
        Arc::new(move || { let weak = weak_wake.clone(); let _ = slint::invoke_from_event_loop(move || { if let Some(ui) = weak.upgrade() { ui.window().request_redraw(); } }); }),
        move |state, api| {
            if matches!(state, slint::RenderingState::RenderingSetup) {
                if let slint::GraphicsAPI::NativeOpenGL { get_proc_address } = api {
                    // SAFETY: this observer executes with Slint's current GL context.
                    *graphics.borrow_mut() = Some(unsafe { glow::Context::from_loader_function(|name| get_proc_address(&CString::new(name).unwrap())) });
                }
                return;
            }
            if !matches!(state, slint::RenderingState::AfterRendering) { return; }
            if !open_once.replace(true) {
                let media = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/artwork.flac");
                playback.command(vec!["loadfile".into(), media.to_str().unwrap().into(), "replace".into()]).unwrap();
            }
            let Some(ui) = weak_probe.upgrade() else { return; };
            let Some(snapshot) = playback.snapshot() else { return; };
            if !snapshot.error.is_empty() { observed.borrow_mut().error = snapshot.error; return; }
            if !snapshot.loaded || !snapshot.has_artwork { return; }
            assert!(!snapshot.tracks.iter().any(|track| track.kind == "video" && track.selected));
            let image = ui.get_frame().size();
            if image.width != 56 || image.height != 56 { return; }
            let size = ui.window().size();
            if size.width == 0 || size.height < 8 { return; }
            let gl = graphics.borrow();
            let Some(gl) = gl.as_ref() else { return; };
            let mut pixels = vec![0u8; (size.width * size.height * 4) as usize];
            // SAFETY: test-only readback of the completed window, into an exact-size RGBA buffer.
            unsafe {
                let previous = gl.get_parameter_framebuffer(glow::READ_FRAMEBUFFER_BINDING);
                gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
                gl.read_pixels(0, 0, size.width as i32, size.height as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut pixels)));
                gl.bind_framebuffer(glow::READ_FRAMEBUFFER, previous);
                assert_eq!(gl.get_error(), 0, "Artwork readback must not produce a GL error");
            }
            let at = |y: u32| { let i = ((y * size.width + size.width / 2) * 4) as usize; &pixels[i..i+3] };
            let (top, bottom) = (at(size.height * 7 / 8), at(size.height / 8));
            let upright = top[0] > 150 && top[1] < 80 && top[2] < 80 && bottom[2] > 150 && bottom[0] < 80 && bottom[1] < 80;
            { let mut progress = observed.borrow_mut(); if progress.logged_phase != Some(progress.phase) {
                println!("[shader-proof] phase={} surface={}x{} top={top:?} bottom={bottom:?} warning={}", progress.phase, size.width, size.height, snapshot.command_error);
                progress.logged_phase = Some(progress.phase);
            } }
            let inverted = top[0] < 80 && top[1] > 150 && top[2] > 150 && bottom[0] > 150 && bottom[1] > 150 && bottom[2] < 80;
            let fixture = |name: &str| player_services::ShaderFile::new(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)).unwrap();
            let mut proof = observed.borrow_mut();
            if proof.phase == 0 && upright {
                proof.loaded = true;
                playback.set_shaders(vec![fixture("invert,colors;fixture.glsl")]).unwrap();
                proof.phase = 1;
            } else if proof.phase == 1 && inverted {
                proof.pixels = pixels;
                proof.size = (size.width, size.height);
                playback.set_shaders(vec![fixture("invalid-shader.glsl")]).unwrap();
                proof.phase = 2;
            } else if proof.phase == 2 && snapshot.command_error.contains("Renderer warning") {
                assert!(snapshot.loaded, "Invalid shader must not stop playback");
                playback.set_shaders(vec![]).unwrap();
                proof.phase = 3;
            } else if proof.phase == 3 && upright && snapshot.command_error.is_empty() {
                proof.upright = true;
                proof.phase = 4;
            }
        }).unwrap();
    let started = Instant::now();
    let timer = slint::Timer::default();
    let (check, weak) = (proof.clone(), ui.as_weak());
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(20),
        move || {
            if check.borrow().phase == 4
                || !check.borrow().error.is_empty()
                || started.elapsed() > Duration::from_secs(15)
            {
                slint::quit_event_loop().unwrap();
            } else if let Some(ui) = weak.upgrade() {
                ui.window().request_redraw();
            }
        },
    );
    ui.run().unwrap();
    let proof = proof.borrow();
    assert!(proof.error.is_empty(), "{}", proof.error);
    assert!(
        proof.loaded && proof.upright && proof.phase == 4,
        "Shader must invert real pixels, report its compile failure without stopping playback, then restore original pixels (phase {})",
        proof.phase
    );
    if let Some(path) = std::env::var_os("FOXIDE_SHADER_PROOF") {
        let mut ppm = format!("P6\n{} {}\n255\n", proof.size.0, proof.size.1).into_bytes();
        for row in (0..proof.size.1).rev() {
            let start = (row * proof.size.0 * 4) as usize;
            for pixel in proof.pixels[start..start + (proof.size.0 * 4) as usize]
                .as_chunks::<4>()
                .0
            {
                ppm.extend_from_slice(&pixel[..3]);
            }
        }
        std::fs::write(path, ppm).unwrap();
    }
}
