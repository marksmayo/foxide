"""Diagnostic control: libmpv -> SDL OpenGL FBO, with no Foxide or Slint code."""
import ctypes as c
import os
import threading
import time


def bind(lib, name, result, *args):
    fn = getattr(lib, name)
    fn.restype, fn.argtypes = result, args
    return fn


class Param(c.Structure):
    _fields_ = [("kind", c.c_int), ("data", c.c_void_p)]


class Fbo(c.Structure):
    _fields_ = [("id", c.c_int), ("w", c.c_int), ("h", c.c_int), ("format", c.c_int)]


def main():
    sdl = c.CDLL("libSDL2-2.0.so.0")
    mpv = c.CDLL("libmpv.so.2")
    assert bind(sdl, "SDL_Init", c.c_int, c.c_uint)(0x20) == 0
    attr = bind(sdl, "SDL_GL_SetAttribute", c.c_int, c.c_int, c.c_int)
    attr(17, 3)  # major
    attr(18, 3)  # minor
    attr(21, 1)  # core profile
    window = bind(sdl, "SDL_CreateWindow", c.c_void_p, c.c_char_p, c.c_int, c.c_int, c.c_int, c.c_int, c.c_uint)(b"libmpv control", 0, 0, 320, 180, 2 | 4)
    assert window
    context = bind(sdl, "SDL_GL_CreateContext", c.c_void_p, c.c_void_p)(window)
    assert context
    get = bind(sdl, "SDL_GL_GetProcAddress", c.c_void_p, c.c_char_p)

    def gl(name, result, *args):
        pointer = get(name.encode())
        assert pointer, name
        return c.CFUNCTYPE(result, *args)(pointer)

    print("Control GL", gl("glGetString", c.c_char_p, c.c_uint)(0x1F02), flush=True)
    handle = bind(mpv, "mpv_create", c.c_void_p)()
    option = bind(mpv, "mpv_set_option_string", c.c_int, c.c_void_p, c.c_char_p, c.c_char_p)
    for key, value in [(b"config", b"no"), (b"load-scripts", b"no"), (b"vo", b"libmpv"), (b"hwdec", b"auto-safe"), (b"terminal", b"no"), (b"keep-open", b"yes")]:
        assert option(handle, key, value) >= 0
    assert bind(mpv, "mpv_initialize", c.c_int, c.c_void_p)(handle) >= 0
    lookup_type = c.CFUNCTYPE(c.c_void_p, c.c_void_p, c.c_char_p)
    lookup = lookup_type(lambda _, name: get(name))

    class Init(c.Structure):
        _fields_ = [("lookup", lookup_type), ("context", c.c_void_p)]

    init = Init(lookup, None)
    api = c.create_string_buffer(b"opengl")
    params = (Param * 3)(Param(1, c.addressof(api)), Param(2, c.addressof(init)), Param())
    renderer = c.c_void_p()
    assert bind(mpv, "mpv_render_context_create", c.c_int, c.POINTER(c.c_void_p), c.c_void_p, c.POINTER(Param))(c.byref(renderer), handle, params) >= 0
    wake_type = c.CFUNCTYPE(None, c.c_void_p)
    wake = wake_type(lambda _: None)
    bind(mpv, "mpv_render_context_set_update_callback", None, c.c_void_p, wake_type, c.c_void_p)(renderer, wake, None)
    update = bind(mpv, "mpv_render_context_update", c.c_uint64, c.c_void_p)
    render = bind(mpv, "mpv_render_context_render", c.c_int, c.c_void_p, c.POINTER(Param))
    texture, framebuffer = c.c_uint(), c.c_uint()
    gl("glGenTextures", None, c.c_int, c.POINTER(c.c_uint))(1, c.byref(texture))
    gl("glBindTexture", None, c.c_uint, c.c_uint)(0x0DE1, texture)
    for name, value in [(0x2801, 0x2601), (0x2800, 0x2601)]:
        gl("glTexParameteri", None, c.c_uint, c.c_uint, c.c_int)(0x0DE1, name, value)
    gl("glTexImage2D", None, c.c_uint, c.c_int, c.c_int, c.c_int, c.c_int, c.c_int, c.c_uint, c.c_uint, c.c_void_p)(0x0DE1, 0, 0x8058, 320, 180, 0, 0x1908, 0x1401, None)
    gl("glBindTexture", None, c.c_uint, c.c_uint)(0x0DE1, 0)
    gl("glGenFramebuffers", None, c.c_int, c.POINTER(c.c_uint))(1, c.byref(framebuffer))
    bind_fbo = gl("glBindFramebuffer", None, c.c_uint, c.c_uint)
    bind_fbo(0x8D40, framebuffer)
    gl("glFramebufferTexture2D", None, c.c_uint, c.c_uint, c.c_uint, c.c_uint, c.c_int)(0x8D40, 0x8CE0, 0x0DE1, texture, 0)
    assert gl("glCheckFramebufferStatus", c.c_uint, c.c_uint)(0x8D40) == 0x8CD5
    bind_fbo(0x8D40, 0)
    fbo = Fbo(framebuffer.value, 320, 180, 0x8058)
    flip, block = c.c_int(0), c.c_int(0)
    params = (Param * 4)(Param(3, c.addressof(fbo)), Param(4, c.addressof(flip)), Param(12, c.addressof(block)), Param())
    command = bind(mpv, "mpv_command", c.c_int, c.c_void_p, c.POINTER(c.c_char_p))
    ready = threading.Event()

    def load():
        args = (c.c_char_p * 4)(b"loadfile", os.fsencode(os.environ["FOXIDE_RENDER_FIXTURE"]), b"replace", None)
        assert command(handle, args) >= 0
        ready.set()

    worker = threading.Thread(target=load)
    worker.start()
    pixels = (c.c_ubyte * (320 * 180 * 4))()
    read = gl("glReadPixels", None, c.c_int, c.c_int, c.c_int, c.c_int, c.c_uint, c.c_uint, c.c_void_p)
    pump = bind(sdl, "SDL_PumpEvents", None)
    first, started, frames, inverted = None, time.monotonic(), set(), 0
    while time.monotonic() - started < 15:
        pump()
        if update(renderer) & 1:
            assert render(renderer, params) >= 0
            bind_fbo(0x8CA8, framebuffer)
            read(0, 0, 320, 180, 0x1908, 0x1401, pixels)
            bind_fbo(0x8CA8, 0)
            data = bytes(pixels)
            top = data[((180 // 8) * 320 + 160) * 4:][:3]
            bottom = data[((180 * 7 // 8) * 320 + 160) * 4:][:3]
            if top[0] > 150 and bottom[2] > 150:
                first = first or time.monotonic()
                frames.add(data)
            if top[2] > 150 and bottom[0] > 150:
                inverted += 1
        if first and time.monotonic() - first >= 5:
            break
        time.sleep(0.001)
    print(f"SDL control unique={len(frames)} inverted={inverted}", flush=True)
    worker.join(timeout=2)
    bind(mpv, "mpv_render_context_free", None, c.c_void_p)(renderer)
    bind(mpv, "mpv_terminate_destroy", None, c.c_void_p)(handle)
    bind(sdl, "SDL_Quit", None)()
    assert ready.is_set() and first and len(frames) >= 75 and inverted == 0


if __name__ == "__main__":
    main()
