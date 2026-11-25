//! Test-only GL readback of the 360-degree view after Slint composition.
#![cfg(feature = "gpu-integration")]
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
    export component PanoramaProbe inherits Window {
        width: 256px; height: 144px; background: lime;
        in property <image> frame;
        Image { source: root.frame; width: parent.width; height: parent.height; image-fit: contain; }
    }
}

#[derive(Default)]
struct Proof {
    phase: u8,
    baseline: Option<[u8; 3]>,
    turned: Option<[u8; 3]>,
    restored: Option<[u8; 3]>,
    error: String,
}

fn center_pixel(gl: &glow::Context, width: u32, height: u32) -> Result<[u8; 3], String> {
    if width == 0 || height == 0 {
        return Err("The compositor surface has no pixels".into());
    }
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    // SAFETY: this test reads the completed Slint framebuffer into an exact-sized buffer while
    // attach_observed has made the same OpenGL context current.
    unsafe {
        let previous = gl.get_parameter_framebuffer(glow::READ_FRAMEBUFFER_BINDING);
        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
        gl.read_pixels(
            0,
            0,
            width as i32,
            height as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(Some(&mut rgba)),
        );
        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, previous);
        if gl.get_error() != 0 {
            return Err("Compositor pixel readback produced an OpenGL error".into());
        }
    }
    let offset = (((height / 2) * width + width / 2) * 4) as usize;
    Ok([rgba[offset], rgba[offset + 1], rgba[offset + 2]])
}

fn is_blue(pixel: [u8; 3]) -> bool {
    pixel[2] > 150 && pixel[0] < 90 && pixel[1] < 110
}

fn is_red(pixel: [u8; 3]) -> bool {
    pixel[0] > 150 && pixel[1] < 90 && pixel[2] < 110
}

fn close_pixels(left: [u8; 3], right: [u8; 3]) -> bool {
    left.into_iter()
        .zip(right)
        .all(|(left, right)| left.abs_diff(right) <= 32)
}

#[test]
#[ignore = "Requires libmpv, the generated panorama fixture, and a native or Xvfb OpenGL display"]
fn panorama_view_preset_changes_real_pixels_and_off_restores_the_source_view() {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()
        .unwrap();

    let fixture = PathBuf::from(
        std::env::var_os("FOXIDE_PANORAMA_FIXTURE")
            .expect("the video compositor workflow generates the panorama fixture"),
    );
    let ui = PanoramaProbe::new().unwrap();
    let engine = Rc::new(Engine::new().unwrap());
    engine
        .command(vec!["set".into(), "ao".into(), "null".into()])
        .unwrap();
    engine
        .command(vec!["set".into(), "loop-file".into(), "inf".into()])
        .unwrap();

    let proof = Rc::new(RefCell::new(Proof::default()));
    let graphics = Rc::new(RefCell::new(None::<glow::Context>));
    let opened = Rc::new(std::cell::Cell::new(false));
    let (weak_frame, weak_wake, weak_probe) = (ui.as_weak(), ui.as_weak(), ui.as_weak());
    let (errors, observed, playback) = (proof.clone(), proof.clone(), engine.clone());
    let open_once = opened.clone();

    attach_observed(
        ui.window(),
        &engine,
        || (128, 72),
        move |frame| {
            if let Some(ui) = weak_frame.upgrade() {
                ui.set_frame(frame);
            }
        },
        move |error| errors.borrow_mut().error = error,
        Arc::new(move || {
            let weak = weak_wake.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    ui.window().request_redraw();
                }
            });
        }),
        move |render_state, api| {
            if matches!(render_state, slint::RenderingState::RenderingSetup) {
                if let slint::GraphicsAPI::NativeOpenGL { get_proc_address } = api {
                    // SAFETY: this observer runs while Slint's OpenGL context is current.
                    *graphics.borrow_mut() = Some(unsafe {
                        glow::Context::from_loader_function(|name| {
                            get_proc_address(&CString::new(name).unwrap())
                        })
                    });
                }
                return;
            }
            if !matches!(render_state, slint::RenderingState::AfterRendering) {
                return;
            }
            if !open_once.replace(true)
                && let Err(error) = playback.command(vec![
                    "loadfile".into(),
                    fixture.to_string_lossy().into_owned(),
                    "replace".into(),
                ])
            {
                observed.borrow_mut().error = error;
                return;
            }

            let Some(ui) = weak_probe.upgrade() else {
                return;
            };
            let Some(snapshot) = playback.snapshot() else {
                return;
            };
            if !snapshot.error.is_empty() {
                observed.borrow_mut().error = snapshot.error;
                return;
            }
            if !snapshot.loaded || ui.get_frame().size().width == 0 {
                return;
            }
            let size = ui.window().size();
            let gl = graphics.borrow();
            let Some(gl) = gl.as_ref() else { return };
            let pixel = match center_pixel(gl, size.width, size.height) {
                Ok(pixel) => pixel,
                Err(error) => {
                    observed.borrow_mut().error = error;
                    return;
                }
            };

            let mut proof = observed.borrow_mut();
            match proof.phase {
                0 if is_blue(pixel) => {
                    proof.baseline = Some(pixel);
                    proof.phase = 1;
                    drop(proof);
                    let command = vec![
                        "vf".into(),
                        "add".into(),
                        "@foxide-panorama360:lavfi=[v360=input=e:output=flat:w=1280:h=720:h_fov=90:v_fov=60:yaw=90:pitch=0]".into(),
                    ];
                    if let Err(error) = playback.command(command) {
                        observed.borrow_mut().error = error;
                    }
                }
                1 if is_red(pixel) => {
                    proof.turned = Some(pixel);
                    proof.phase = 2;
                    drop(proof);
                    let command = vec!["vf".into(), "remove".into(), "@foxide-panorama360".into()];
                    if let Err(error) = playback.command(command) {
                        observed.borrow_mut().error = error;
                    }
                }
                2 if is_blue(pixel)
                    && proof
                        .baseline
                        .is_some_and(|baseline| close_pixels(baseline, pixel)) =>
                {
                    proof.restored = Some(pixel);
                    proof.phase = 3;
                    println!(
                        "[panorama-proof] baseline={:?} yaw90={:?} restored={:?}",
                        proof.baseline, proof.turned, proof.restored
                    );
                }
                _ => {}
            }
        },
    )
    .unwrap();

    let started = Instant::now();
    let timer = slint::Timer::default();
    let (check, weak) = (proof.clone(), ui.as_weak());
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(20),
        move || {
            if check.borrow().phase == 3
                || !check.borrow().error.is_empty()
                || started.elapsed() > Duration::from_secs(20)
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
        proof.phase == 3
            && proof.baseline.is_some_and(is_blue)
            && proof.turned.is_some_and(is_red)
            && proof.restored.is_some_and(is_blue),
        "The 360-degree right preset must change composited blue pixels to red and Off must restore them; phases reached: {}, samples: {:?}",
        proof.phase,
        (proof.baseline, proof.turned, proof.restored)
    );
}
