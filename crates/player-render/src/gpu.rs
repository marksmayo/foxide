use crate::native::{Api, Engine, Handle, Param};
use crate::render_contract::{Fbo, TEXTURE_ORIGIN_BOTTOM_LEFT, render_texture};
use crate::thumbnail_cache::{ThumbnailCache, ThumbnailKey};
use glow::HasContext;
use libloading::Library;
use std::{
    collections::HashMap,
    ffi::{CStr, CString, c_char, c_void},
    rc::Rc,
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

const THUMBNAIL_WIDTH: u32 = 256;
const THUMBNAIL_HEIGHT: u32 = 144;
const THUMBNAIL_CACHE_BYTES: usize = 16 * 1024 * 1024;
const THUMBNAIL_BYTES: usize = THUMBNAIL_WIDTH as usize * THUMBNAIL_HEIGHT as usize * 4;

struct GpuTarget {
    texture: glow::Texture,
    framebuffer: glow::Framebuffer,
    size: (u32, u32),
}

struct Renderer {
    api: Arc<Api>,
    gl: glow::Context,
    context: Handle,
    texture: glow::Texture,
    framebuffer: glow::Framebuffer,
    size: (u32, u32),
    thumbnail_targets: Vec<Option<GpuTarget>>,
    thumbnail_cache: ThumbnailCache,
    _resolver: Rc<Resolver>,
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
        let resolver = Rc::new(Resolver { known, libraries });
        Self::with_resolver(api, gl, resolver, callback)
    }

    fn new_shared(
        api: Arc<Api>,
        resolver: Rc<Resolver>,
        callback: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, String> {
        // SAFETY: rendering setup runs while the same Slint OpenGL context is current;
        // Resolver retains the matching platform libraries and known entry points.
        let gl = unsafe {
            glow::Context::from_loader_function(|name| {
                let name_c = CString::new(name).unwrap();
                resolver.lookup(&name_c)
            })
        };
        Self::with_resolver(api, gl, resolver, callback)
    }

    fn with_resolver(
        api: Arc<Api>,
        gl: glow::Context,
        resolver: Rc<Resolver>,
        callback: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, String> {
        let mut init = Init {
            get_proc,
            context: Rc::as_ptr(&resolver).cast_mut().cast(),
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
        let thumbnail_cache = ThumbnailCache::new(THUMBNAIL_CACHE_BYTES, THUMBNAIL_BYTES);
        let extra_thumbnail_slots = thumbnail_cache.capacity().saturating_sub(1);
        let mut renderer = Self {
            api,
            gl,
            context,
            texture,
            framebuffer,
            size: (0, 0),
            thumbnail_targets: (0..extra_thumbnail_slots).map(|_| None).collect(),
            thumbnail_cache,
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

    fn frame(&mut self, width: u32, height: u32) -> Result<Option<slint::Image>, String> {
        self.frame_with_force(width, height, false)
    }

    fn frame_with_force(
        &mut self,
        width: u32,
        height: u32,
        force: bool,
    ) -> Result<Option<slint::Image>, String> {
        let (width, height) = (width.clamp(1, 8192), height.clamp(1, 8192));
        // SAFETY: exclusively called from this window's BeforeRendering callback.
        // The texture name remains valid until RenderingTeardown. Resizing replaces
        // texture storage, not its name, so previously issued GL work stays valid.
        unsafe {
            // In default control mode update drains notifications without GPU
            // allocations. An initial wake is not a decoded video frame.
            let updates = (self.api.render_update)(self.context);
            if self.size == (0, 0) && updates & 1 == 0 && !force {
                return Ok(None);
            }
            let gl = &self.gl;
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
            Ok(Some(
                slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(
                    self.texture.0,
                    [width, height].into(),
                )
                .origin(if TEXTURE_ORIGIN_BOTTOM_LEFT {
                    slint::BorrowedOpenGLTextureOrigin::BottomLeft
                } else {
                    slint::BorrowedOpenGLTextureOrigin::TopLeft
                })
                .build(),
            ))
        }
    }

    fn frame_thumbnail(
        &mut self,
        key: &ThumbnailKey,
        decoder_ready: bool,
    ) -> Result<Option<slint::Image>, String> {
        let Some(reservation) = self.thumbnail_cache.reserve(key.clone()) else {
            return Ok(None);
        };
        let slot = reservation.slot();
        if self.thumbnail_cache.is_ready(key) {
            return self.image_for_thumbnail_slot(slot).map(Some);
        }
        if !decoder_ready {
            return Ok(None);
        }
        if self.frame_thumbnail_slot(slot)? {
            self.thumbnail_cache.mark_ready(key);
            self.image_for_thumbnail_slot(slot).map(Some)
        } else {
            Ok(None)
        }
    }

    fn frame_thumbnail_slot(&mut self, slot: usize) -> Result<bool, String> {
        if slot == 0 {
            return self
                .frame_with_force(THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT, true)
                .map(|frame| frame.is_some());
        }
        let index = slot - 1;
        let target_count = self.thumbnail_targets.len();
        let Some(target) = self.thumbnail_targets.get_mut(index) else {
            return Err(format!(
                "Thumbnail texture slot {slot} exceeds bounded cache capacity {}",
                target_count + 1
            ));
        };
        if target.is_none() {
            let texture = unsafe { self.gl.create_texture() }.map_err(|error| error.to_string())?;
            let framebuffer = match unsafe { self.gl.create_framebuffer() } {
                Ok(framebuffer) => framebuffer,
                Err(error) => {
                    unsafe { self.gl.delete_texture(texture) };
                    return Err(error.to_string());
                }
            };
            *target = Some(GpuTarget {
                texture,
                framebuffer,
                size: (0, 0),
            });
        }
        let mut target = self.thumbnail_targets[index].take().unwrap();
        std::mem::swap(&mut self.texture, &mut target.texture);
        std::mem::swap(&mut self.framebuffer, &mut target.framebuffer);
        std::mem::swap(&mut self.size, &mut target.size);
        let result = self.frame_with_force(THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT, true);
        std::mem::swap(&mut self.texture, &mut target.texture);
        std::mem::swap(&mut self.framebuffer, &mut target.framebuffer);
        std::mem::swap(&mut self.size, &mut target.size);
        self.thumbnail_targets[index] = Some(target);
        result.map(|frame| frame.is_some())
    }

    fn image_for_thumbnail_slot(&self, slot: usize) -> Result<slint::Image, String> {
        let texture = if slot == 0 {
            self.texture
        } else {
            self.thumbnail_targets
                .get(slot - 1)
                .and_then(Option::as_ref)
                .map(|target| target.texture)
                .ok_or_else(|| format!("Thumbnail texture slot {slot} is unavailable"))?
        };
        // SAFETY: this renderer retains the texture until RenderingTeardown, and
        // the Slint image is only used with the same current OpenGL context.
        let builder = unsafe {
            slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(
                texture.0,
                [THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT].into(),
            )
        };
        Ok(builder
            .origin(if TEXTURE_ORIGIN_BOTTOM_LEFT {
                slint::BorrowedOpenGLTextureOrigin::BottomLeft
            } else {
                slint::BorrowedOpenGLTextureOrigin::TopLeft
            })
            .build())
    }
}
impl Drop for Renderer {
    fn drop(&mut self) {
        // SAFETY: RenderingTeardown runs with the original context current.
        // Unregister and free before releasing userdata, textures or the client.
        unsafe {
            (self.api.render_callback)(self.context, None, std::ptr::null_mut());
            (self.api.render_free)(self.context);
            self.gl.delete_framebuffer(self.framebuffer);
            self.gl.delete_texture(self.texture);
            for target in self.thumbnail_targets.iter().flatten() {
                self.gl.delete_framebuffer(target.framebuffer);
                self.gl.delete_texture(target.texture);
            }
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

#[allow(clippy::too_many_arguments)] // The call site supplies renderer and thumbnail callbacks.
pub fn attach_with_thumbnails(
    window: &slint::Window,
    engine: &Engine,
    thumbnails: Arc<crate::thumbnails::ThumbnailController>,
    size: impl Fn() -> (u32, u32) + 'static,
    image: impl Fn(slint::Image) + 'static,
    thumbnail_image: impl Fn(slint::Image, bool) + 'static,
    error: impl Fn(String) + 'static,
    wakeup: Arc<dyn Fn() + Send + Sync>,
) -> Result<(), String> {
    attach_inner(
        window,
        engine,
        Some(thumbnails),
        size,
        image,
        thumbnail_image,
        error,
        wakeup,
        |_, _| {},
    )
}

/// Opt-in rendering observer for compositor diagnostics; called with the render context current.
pub fn attach_observed(
    window: &slint::Window,
    engine: &Engine,
    size: impl Fn() -> (u32, u32) + 'static,
    image: impl Fn(slint::Image) + 'static,
    error: impl Fn(String) + 'static,
    wakeup: Arc<dyn Fn() + Send + Sync>,
    observe: impl FnMut(slint::RenderingState, &slint::GraphicsAPI) + 'static,
) -> Result<(), String> {
    attach_inner(
        window,
        engine,
        None,
        size,
        image,
        |_, _| {},
        error,
        wakeup,
        observe,
    )
}

#[allow(clippy::too_many_arguments)] // Slint owns these independent render callbacks.
fn attach_inner(
    window: &slint::Window,
    engine: &Engine,
    thumbnails: Option<Arc<crate::thumbnails::ThumbnailController>>,
    size: impl Fn() -> (u32, u32) + 'static,
    image: impl Fn(slint::Image) + 'static,
    thumbnail_image: impl Fn(slint::Image, bool) + 'static,
    error: impl Fn(String) + 'static,
    wakeup: Arc<dyn Fn() + Send + Sync>,
    mut observe: impl FnMut(slint::RenderingState, &slint::GraphicsAPI) + 'static,
) -> Result<(), String> {
    let api = engine.api.clone();
    let ready = engine.ready.clone();
    let changed = Arc::new(AtomicBool::new(true));
    let frame_changed = changed.clone();
    let preview_wakeup = wakeup.clone();
    let notify: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        frame_changed.store(true, Ordering::Release);
        wakeup();
    });
    if let Some(thumbnails) = &thumbnails {
        thumbnails.set_wakeup(Arc::new(move || preview_wakeup()));
    }
    let mut renderer: Option<Renderer> = None;
    let mut preview_renderer: Option<Renderer> = None;
    let mut preview_engine: Option<Arc<Engine>> = None;
    let mut resolver: Option<Rc<Resolver>> = None;
    let preview_notify = notify.clone();
    window
        .set_rendering_notifier(move |state, graphics| {
            match state {
                slint::RenderingState::RenderingSetup => {
                    if let slint::GraphicsAPI::NativeOpenGL { get_proc_address } = graphics {
                        match Renderer::new(api.clone(), get_proc_address, notify.clone()) {
                            Ok(value) => {
                                resolver = Some(value._resolver.clone());
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
                        if changed.swap(false, Ordering::AcqRel)
                            || renderer.size != (width.clamp(1, 8192), height.clamp(1, 8192))
                        {
                            match renderer.frame(width, height) {
                                Ok(Some(frame)) => publish_frame(frame, &image),
                                Ok(None) => {}
                                Err(message) => error(message),
                            }
                        }
                    }
                    if let Some(thumbnails) = &thumbnails {
                        let request = thumbnails.current_request();
                        let has_request = request.is_some();
                        if let (Some(request), Some(engine)) =
                            (request, thumbnails.preview_engine())
                        {
                            let needs_preview_renderer = preview_engine
                                .as_ref()
                                .is_none_or(|current| !Arc::ptr_eq(current, &engine));
                            if needs_preview_renderer && let Some(resolver) = resolver.as_ref() {
                                match Renderer::new_shared(
                                    engine.api.clone(),
                                    resolver.clone(),
                                    preview_notify.clone(),
                                ) {
                                    Ok(next) => {
                                        preview_renderer = Some(next);
                                        preview_engine = Some(engine.clone());
                                        engine.ready.store(true, Ordering::Release);
                                    }
                                    Err(message) => error(message),
                                }
                            }
                            if let Some(preview_renderer) = preview_renderer.as_mut() {
                                let decoder_ready = engine.snapshot().is_some_and(|snapshot| {
                                    snapshot.loaded
                                        && snapshot.path == request.media.as_ref()
                                        && (snapshot.position - request.position).abs() <= 0.75
                                });
                                match preview_renderer.frame_thumbnail(&request.key, decoder_ready)
                                {
                                    Ok(Some(frame)) => {
                                        publish_thumbnail_frame(frame, &thumbnail_image)
                                    }
                                    Ok(None) => {}
                                    Err(message) => error(message),
                                }
                            }
                        } else if !has_request {
                            thumbnail_image(slint::Image::default(), false);
                        }
                    }
                }
                slint::RenderingState::RenderingTeardown => {
                    ready.store(false, Ordering::Release);
                    image(slint::Image::default());
                    thumbnail_image(slint::Image::default(), false);
                    renderer = None;
                    preview_renderer = None;
                    preview_engine = None;
                    resolver = None;
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

fn publish_thumbnail_frame(frame: slint::Image, image: &impl Fn(slint::Image, bool)) {
    image(slint::Image::default(), false);
    image(frame, true);
}

#[cfg(all(test, feature = "gpu-integration"))]
#[path = "gpu_integration.rs"]
mod integration;
