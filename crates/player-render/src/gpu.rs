use crate::native::{Api, Engine, Handle, Param};
use crate::render_contract::{Fbo, TEXTURE_ORIGIN_BOTTOM_LEFT, render_texture};
use glow::HasContext;
use libloading::Library;
use std::{
    collections::HashMap,
    ffi::{CStr, CString, c_char, c_void},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

type GetProc = unsafe extern "system" fn(*const c_char) -> *const c_void;
struct Resolver {
    known: HashMap<String, *const c_void>,
    libraries: Vec<Library>,
}
impl Resolver {
    #[cfg(test)]
    fn current_context(&self) -> usize {
        for name in [
            c"eglGetCurrentContext",
            c"glXGetCurrentContext",
            c"wglGetCurrentContext",
            c"CGLGetCurrentContext",
        ] {
            for library in &self.libraries {
                // SAFETY: retained graphics libraries expose these no-argument context getters.
                if let Ok(get) = unsafe {
                    library.get::<unsafe extern "system" fn() -> *const c_void>(
                        name.to_bytes_with_nul(),
                    )
                } {
                    let context = unsafe { get() } as usize;
                    if context != 0 {
                        return context;
                    }
                }
            }
        }
        0
    }
    fn lookup(&self, name: &CStr) -> *const c_void {
        if let Some(value) = self.known.get(name.to_string_lossy().as_ref()) {
            return *value;
        }
        for library in &self.libraries {
            // SAFETY: libraries remain loaded with the renderer; lookups occur
            // only while Slint's original GL context is current on this thread.
            unsafe {
                for resolver in [
                    c"wglGetProcAddress",
                    c"glXGetProcAddressARB",
                    c"eglGetProcAddress",
                ] {
                    if let Ok(function) = library.get::<GetProc>(resolver.to_bytes_with_nul()) {
                        let pointer = function(name.as_ptr());
                        if !pointer.is_null()
                            && ![1, 2, 3, usize::MAX].contains(&(pointer as usize))
                        {
                            return pointer;
                        }
                    }
                }
                if let Ok(symbol) = library.get::<*const c_void>(name.to_bytes_with_nul()) {
                    return *symbol;
                }
            }
        }
        std::ptr::null()
    }
}
extern "C" fn get_proc(context: *mut c_void, name: *const c_char) -> *mut c_void {
    if context.is_null() || name.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: context points to Renderer's stable boxed Resolver, retained until
    // render_free returns. libmpv supplies a valid NUL-terminated function name.
    unsafe {
        (&*context.cast::<Resolver>())
            .lookup(CStr::from_ptr(name))
            .cast_mut()
    }
}
extern "C" fn wake(context: *mut c_void) {
    if context.is_null() {
        return;
    }
    // SAFETY: the boxed callback is retained through callback unregister + free.
    // Never unwind across C or invoke rendering/client APIs from this callback.
    let callback = unsafe { &*context.cast::<Arc<dyn Fn() + Send + Sync>>() };
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback()));
}
#[repr(C)]
struct Init {
    get_proc: extern "C" fn(*mut c_void, *const c_char) -> *mut c_void,
    context: *mut c_void,
}

struct Renderer {
    api: Arc<Api>,
    gl: glow::Context,
    context: Handle,
    texture: glow::Texture,
    framebuffer: glow::Framebuffer,
    size: (u32, u32),
    #[cfg(test)]
    entry_states: HashMap<[i32; 10], usize>,
    #[cfg(test)]
    output_colors: HashMap<[u8; 8], usize>,
    #[cfg(test)]
    created_context: usize,
    #[cfg(test)]
    probe_sync: String,
    #[cfg(test)]
    probe_readback: bool,
    _resolver: Box<Resolver>,
    callback: Box<Arc<dyn Fn() + Send + Sync>>,
}
impl Renderer {
    fn new(
        api: Arc<Api>,
        get: &dyn Fn(&CStr) -> *const c_void,
        callback: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, String> {
        let mut known = HashMap::new();
        // SAFETY: Slint guarantees NativeOpenGL's context is current throughout
        // the rendering notifier. Function pointers are copied, never the
        // borrowed get_proc closure. Additional lookups use retained OS libraries.
        let gl = unsafe {
            glow::Context::from_loader_function(|name| {
                let name_c = CString::new(name).unwrap();
                let pointer = get(&name_c);
                known.insert(name.to_owned(), pointer);
                pointer
            })
        };
        let names = if cfg!(windows) {
            vec!["opengl32.dll"]
        } else if cfg!(target_os = "macos") {
            vec!["/System/Library/Frameworks/OpenGL.framework/OpenGL"]
        } else {
            vec!["libGL.so.1", "libEGL.so.1"]
        };
        // SAFETY: system graphics libraries are retained for the entire resolver lifetime.
        let libraries = names
            .into_iter()
            .filter_map(|name| unsafe { Library::new(name).ok() })
            .collect();
        let mut resolver = Box::new(Resolver { known, libraries });
        let mut init = Init {
            get_proc,
            context: (&mut *resolver as *mut Resolver).cast(),
        };
        let mut params = [
            Param {
                kind: 1,
                data: c"opengl".as_ptr().cast_mut().cast(),
            },
            Param {
                kind: 2,
                data: (&mut init as *mut Init).cast(),
            },
            Param {
                kind: 0,
                data: std::ptr::null_mut(),
            },
        ];
        let mut context = std::ptr::null_mut();
        // SAFETY: ABI parameter layouts match render.h/render_gl.h; all parameter
        // data lives through create, and Resolver outlives the render context.
        let result = unsafe { (api.render_create)(&mut context, api.handle, params.as_mut_ptr()) };
        if result < 0 {
            return Err(format!("GPU video initialization failed ({result})"));
        }
        // SAFETY: context is current; resource names are generated by this GL implementation.
        let (texture, framebuffer) = unsafe {
            let texture = match gl.create_texture() {
                Ok(t) => t,
                Err(e) => {
                    (api.render_free)(context);
                    return Err(e);
                }
            };
            let framebuffer = match gl.create_framebuffer() {
                Ok(f) => f,
                Err(e) => {
                    gl.delete_texture(texture);
                    (api.render_free)(context);
                    return Err(e);
                }
            };
            (texture, framebuffer)
        };
        let mut renderer = Self {
            api,
            gl,
            context,
            texture,
            framebuffer,
            size: (0, 0),
            #[cfg(test)]
            entry_states: HashMap::new(),
            #[cfg(test)]
            output_colors: HashMap::new(),
            #[cfg(test)]
            created_context: resolver.current_context(),
            #[cfg(test)]
            probe_sync: std::env::var("FOXIDE_PROBE_SYNC").unwrap_or_default(),
            #[cfg(test)]
            probe_readback: std::env::var_os("FOXIDE_PROBE_READBACK").is_some(),
            _resolver: resolver,
            callback: Box::new(callback),
        };
        // SAFETY: callback userdata is stable, retained until context free completes.
        unsafe {
            (renderer.api.render_callback)(
                renderer.context,
                Some(wake),
                (&mut *renderer.callback as *mut Arc<dyn Fn() + Send + Sync>).cast(),
            );
        }
        Ok(renderer)
    }

    fn frame(&mut self, width: u32, height: u32) -> Result<slint::Image, String> {
        let (width, height) = (width.clamp(1, 8192), height.clamp(1, 8192));
        // SAFETY: exclusively called from this window's BeforeRendering callback.
        // The texture name remains valid until RenderingTeardown. Resizing replaces
        // texture storage, not its name, so previously issued GL work stays valid.
        unsafe {
            let gl = &self.gl;
            #[cfg(test)]
            if self.created_context != 0 {
                assert_eq!(
                    self.created_context,
                    self._resolver.current_context(),
                    "libmpv render context changed between setup and frame"
                );
            }
            let framebuffer = gl.get_parameter_framebuffer(glow::DRAW_FRAMEBUFFER_BINDING);
            let read_framebuffer = gl.get_parameter_framebuffer(glow::READ_FRAMEBUFFER_BINDING);
            let texture = gl.get_parameter_texture(glow::TEXTURE_BINDING_2D);
            let active_texture = gl.get_parameter_i32(glow::ACTIVE_TEXTURE);
            // Slint finishes on texture unit 1 and can leave the borrowed video
            // bound on unit 0. libmpv's render_gl.h requires standard GL defaults
            // on entry. In particular, the destination must not remain bound as
            // an input texture while libmpv initializes its passes.
            let texture_zero = if active_texture as u32 != glow::TEXTURE0 {
                gl.active_texture(glow::TEXTURE0);
                let bound = gl.get_parameter_texture(glow::TEXTURE_BINDING_2D);
                gl.active_texture(active_texture as u32);
                gl.bind_texture(glow::TEXTURE_2D, None);
                bound
            } else {
                texture
            };
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, None);
            #[cfg(test)]
            {
                let mut mask = [0; 4];
                gl.get_parameter_i32_slice(glow::COLOR_WRITEMASK, &mut mask);
                let state = [
                    active_texture,
                    mask[0],
                    mask[1],
                    mask[2],
                    mask[3],
                    gl.get_parameter_i32(glow::PIXEL_UNPACK_BUFFER_BINDING),
                    gl.get_parameter_i32(glow::DRAW_BUFFER0),
                    gl.get_parameter_i32(glow::UNPACK_ROW_LENGTH),
                    gl.get_parameter_i32(glow::UNPACK_SKIP_ROWS),
                    gl.get_parameter_i32(glow::UNPACK_SKIP_PIXELS),
                ];
                *self.entry_states.entry(state).or_default() += 1;
            }
            let program = gl.get_parameter_program(glow::CURRENT_PROGRAM);
            let vao = gl.get_parameter_vertex_array(glow::VERTEX_ARRAY_BINDING);
            let buffer = gl.get_parameter_buffer(glow::ARRAY_BUFFER_BINDING);
            let mut viewport = [0; 4];
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);
            let mut scissor = [0; 4];
            gl.get_parameter_i32_slice(glow::SCISSOR_BOX, &mut scissor);
            let mut clear = [0.0; 4];
            gl.get_parameter_f32_slice(glow::COLOR_CLEAR_VALUE, &mut clear);
            let blend = [
                gl.get_parameter_i32(glow::BLEND_SRC_RGB),
                gl.get_parameter_i32(glow::BLEND_DST_RGB),
                gl.get_parameter_i32(glow::BLEND_SRC_ALPHA),
                gl.get_parameter_i32(glow::BLEND_DST_ALPHA),
            ];
            let flags = [
                glow::BLEND,
                glow::SCISSOR_TEST,
                glow::DEPTH_TEST,
                glow::STENCIL_TEST,
                glow::CULL_FACE,
            ]
            .map(|flag| (flag, gl.is_enabled(flag)));
            if self.size != (width, height) {
                gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
                for (key, value) in [
                    (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
                    (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
                    (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                    (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
                ] {
                    gl.tex_parameter_i32(glow::TEXTURE_2D, key, value as i32);
                }
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    width as i32,
                    height as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(None),
                );
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
                gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    Some(self.texture),
                    0,
                );
                self.size = (width, height);
            }
            let mut fbo = Fbo {
                id: self.framebuffer.0.get() as i32,
                width: width as i32,
                height: height as i32,
                format: glow::RGBA8 as i32,
            };
            // libmpv requires standard GL capability defaults on entry. Slint
            // can leave clipping/blending enabled after flushing cached layers;
            // inheriting those states can discard the entire offscreen frame.
            // The caller's capability states are restored below.
            for (flag, _) in flags {
                gl.disable(flag);
            }
            // Resizing above temporarily binds the output texture for allocation.
            gl.bind_texture(glow::TEXTURE_2D, None);
            let result = render_texture(self.api.render, self.context, &mut fbo);
            #[cfg(not(test))]
            gl.flush();
            #[cfg(test)]
            match self.probe_sync.as_str() {
                "none" => {}
                "finish" => gl.finish(),
                _ => gl.flush(),
            }
            #[cfg(test)]
            if self.probe_readback {
                gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.framebuffer));
                let mut raw = [0_u8; 8];
                gl.read_pixels(
                    (width / 2) as i32,
                    (height / 8) as i32,
                    1,
                    1,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelPackData::Slice(Some(&mut raw[..4])),
                );
                gl.read_pixels(
                    (width / 2) as i32,
                    (height * 7 / 8) as i32,
                    1,
                    1,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelPackData::Slice(Some(&mut raw[4..])),
                );
                *self.output_colors.entry(raw).or_default() += 1;
            }
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, framebuffer);
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, read_framebuffer);
            if active_texture as u32 != glow::TEXTURE0 {
                gl.active_texture(glow::TEXTURE0);
                gl.bind_texture(glow::TEXTURE_2D, texture_zero);
            }
            gl.active_texture(active_texture as u32);
            gl.bind_texture(glow::TEXTURE_2D, texture);
            gl.use_program(program);
            gl.bind_vertex_array(vao);
            gl.bind_buffer(glow::ARRAY_BUFFER, buffer);
            gl.viewport(viewport[0], viewport[1], viewport[2], viewport[3]);
            gl.scissor(scissor[0], scissor[1], scissor[2], scissor[3]);
            gl.clear_color(clear[0], clear[1], clear[2], clear[3]);
            gl.blend_func_separate(
                blend[0] as u32,
                blend[1] as u32,
                blend[2] as u32,
                blend[3] as u32,
            );
            for (flag, enabled) in flags {
                if enabled {
                    gl.enable(flag);
                } else {
                    gl.disable(flag);
                }
            }
            if result < 0 {
                return Err(format!("Video rendering failed ({result})"));
            }
            Ok(slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(
                self.texture.0,
                [width, height].into(),
            )
            .origin(if TEXTURE_ORIGIN_BOTTOM_LEFT {
                slint::BorrowedOpenGLTextureOrigin::BottomLeft
            } else {
                slint::BorrowedOpenGLTextureOrigin::TopLeft
            })
            .build())
        }
    }
}
impl Drop for Renderer {
    fn drop(&mut self) {
        #[cfg(test)]
        println!("[video-entry-states] {:?}", self.entry_states);
        #[cfg(test)]
        println!("[video-before-compositor] {:?}", self.output_colors);
        // SAFETY: RenderingTeardown runs with the original context current.
        // Unregister and free before releasing userdata, textures or the client.
        unsafe {
            (self.api.render_callback)(self.context, None, std::ptr::null_mut());
            (self.api.render_free)(self.context);
            self.gl.delete_framebuffer(self.framebuffer);
            self.gl.delete_texture(self.texture);
        }
    }
}

/// Attach one same-context GPU video surface. Does not certify hardware decoding
/// or native zero-copy interop: those remain observable engine/platform properties.
pub fn attach(
    window: &slint::Window,
    engine: &Engine,
    size: impl Fn() -> (u32, u32) + 'static,
    image: impl Fn(slint::Image) + 'static,
    error: impl Fn(String) + 'static,
    wakeup: Arc<dyn Fn() + Send + Sync>,
) -> Result<(), String> {
    attach_observed(window, engine, size, image, error, wakeup, |_, _| {})
}

fn attach_observed(
    window: &slint::Window,
    engine: &Engine,
    size: impl Fn() -> (u32, u32) + 'static,
    image: impl Fn(slint::Image) + 'static,
    error: impl Fn(String) + 'static,
    wakeup: Arc<dyn Fn() + Send + Sync>,
    mut observe: impl FnMut(slint::RenderingState, &slint::GraphicsAPI) + 'static,
) -> Result<(), String> {
    let api = engine.api.clone();
    let ready = engine.ready.clone();
    let changed = Arc::new(AtomicBool::new(true));
    let frame_changed = changed.clone();
    let notify: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        frame_changed.store(true, Ordering::Release);
        wakeup();
    });
    let mut renderer: Option<Renderer> = None;
    window
        .set_rendering_notifier(move |state, graphics| {
            match state {
                slint::RenderingState::RenderingSetup => {
                    if let slint::GraphicsAPI::NativeOpenGL { get_proc_address } = graphics {
                        match Renderer::new(api.clone(), get_proc_address, notify.clone()) {
                            Ok(value) => {
                                renderer = Some(value);
                                ready.store(true, Ordering::Release);
                            }
                            Err(message) => error(message),
                        }
                    } else {
                        error("This playback build needs the OpenGL renderer".into());
                    }
                }
                slint::RenderingState::BeforeRendering => {
                    if let Some(renderer) = renderer.as_mut() {
                        let (width, height) = size();
                        if !changed.swap(false, Ordering::AcqRel)
                            && renderer.size == (width.clamp(1, 8192), height.clamp(1, 8192))
                        {
                            return;
                        }
                        match renderer.frame(width, height) {
                            Ok(frame) => publish_frame(frame, &image),
                            Err(message) => error(message),
                        }
                    }
                }
                slint::RenderingState::RenderingTeardown => {
                    ready.store(false, Ordering::Release);
                    image(slint::Image::default());
                    renderer = None;
                }
                #[cfg(test)]
                slint::RenderingState::AfterRendering => {
                    if let Some(renderer) = renderer.as_ref()
                        && renderer.created_context != 0
                    {
                        assert_eq!(
                            renderer.created_context,
                            renderer._resolver.current_context(),
                            "Slint composited using a different GL context"
                        );
                    }
                }
                _ => {}
            }
            observe(state, graphics);
        })
        .map_err(|e| e.to_string())
}

fn publish_frame(frame: slint::Image, image: &impl Fn(slint::Image)) {
    // Borrowed image equality includes texture id/size/origin, not its changing
    // GPU pixels. Rebinding the same image alone leaves rounded clipping layers
    // clean and displaying stale video. Invalidate and restore synchronously,
    // before Slint draws, so no intermediate blank frame is presented.
    image(slint::Image::default());
    image(frame);
}

#[cfg(all(test, feature = "gpu-integration"))]
#[path = "gpu_integration.rs"]
mod integration;
