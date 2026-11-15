//! The libmpv-to-Slint texture contract, independent of a window or GL driver.
use std::ffi::c_int;

pub(crate) type Handle = *mut std::ffi::c_void;
#[repr(C)]
pub(crate) struct Param {
    pub kind: c_int,
    pub data: *mut std::ffi::c_void,
}
pub(crate) const TEXTURE_ORIGIN_BOTTOM_LEFT: bool = false;

#[repr(C)]
pub(crate) struct Fbo {
    pub id: i32,
    pub width: i32,
    pub height: i32,
    pub format: i32,
}

pub(crate) fn render_texture(
    render: unsafe extern "C" fn(Handle, *mut Param) -> c_int,
    context: Handle,
    fbo: &mut Fbo,
) -> c_int {
    // A texture consumed with Slint's TopLeft origin is not an OpenGL default
    // framebuffer. An extra libmpv FLIP_Y here turns the displayed video upside down.
    let mut flip = 0_i32;
    let mut block = 0_i32;
    let mut params = [
        Param {
            kind: 3,
            data: (fbo as *mut Fbo).cast(),
        },
        Param {
            kind: 4,
            data: (&mut flip as *mut i32).cast(),
        },
        Param {
            kind: 12,
            data: (&mut block as *mut i32).cast(),
        },
        Param {
            kind: 0,
            data: std::ptr::null_mut(),
        },
    ];
    // SAFETY: caller's renderer owns the live context. Parameter values and FBO
    // are ABI-compatible and stay alive until the synchronous render returns.
    unsafe { render(context, params.as_mut_ptr()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Distinct top/bottom rows make an accidental extra flip observable.
    const SOURCE: [u32; 4] = [0xff0000ff, 0xffff00ff, 0x00ff00ff, 0x0000ffff];
    struct MockTexture {
        rows: [u32; 4],
        blocked: bool,
    }

    unsafe extern "C" fn mpv_render(context: Handle, mut params: *mut Param) -> c_int {
        // SAFETY: this test supplies correctly typed, live mock context/params.
        unsafe {
            let texture = &mut *context.cast::<MockTexture>();
            texture.rows = SOURCE;
            while (*params).kind != 0 {
                match (*params).kind {
                    // render.h: FLIP_Y=0 renders normally; nonzero flips output.
                    4 if *(*params).data.cast::<i32>() != 0 => texture.rows.reverse(),
                    12 => texture.blocked = *(*params).data.cast::<i32>() != 0,
                    _ => {}
                }
                params = params.add(1);
            }
        }
        0
    }

    #[test]
    fn asymmetric_video_rows_keep_their_orientation_through_both_apis() {
        let mut texture = MockTexture {
            rows: [0; 4],
            blocked: true,
        };
        let mut fbo = Fbo {
            id: 1,
            width: 1,
            height: 4,
            format: 0x8058,
        };
        assert_eq!(
            render_texture(
                mpv_render,
                (&mut texture as *mut MockTexture).cast(),
                &mut fbo
            ),
            0
        );
        // Slint image.rs: TopLeft keeps texture rows; BottomLeft reverses them.
        let mut displayed = texture.rows;
        if TEXTURE_ORIGIN_BOTTOM_LEFT {
            displayed.reverse();
        }
        assert_eq!(displayed, SOURCE, "libmpv/Slint handoff inverted the video");
        assert!(
            !texture.blocked,
            "Video rendering must not wait for a target time on the UI thread"
        );
    }
}
