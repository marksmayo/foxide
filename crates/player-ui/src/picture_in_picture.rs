#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WindowConfiguration {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) fullscreen: bool,
    pub(super) maximized: bool,
    pub(super) compact: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PictureInPictureMode {
    Main,
    PictureInPicture(WindowConfiguration),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WindowEffect {
    Resize { width: u32, height: u32 },
    Restore(WindowConfiguration),
}

impl PictureInPictureMode {
    pub(super) fn enter(current: WindowConfiguration) -> (Self, WindowEffect) {
        (
            Self::PictureInPicture(current),
            WindowEffect::Resize {
                width: 520,
                height: 350,
            },
        )
    }

    pub(super) fn exit(self) -> Option<(Self, WindowEffect)> {
        match self {
            Self::Main => None,
            Self::PictureInPicture(previous) => Some((Self::Main, WindowEffect::Restore(previous))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PictureInPictureMode, WindowConfiguration, WindowEffect};

    #[test]
    fn entering_picture_in_picture_keeps_the_main_window_configuration_for_restore() {
        let current = WindowConfiguration {
            width: 1180,
            height: 770,
            x: 120,
            y: 80,
            fullscreen: true,
            maximized: false,
            compact: false,
        };

        let (mode, effect) = PictureInPictureMode::enter(current);

        assert_eq!(mode, PictureInPictureMode::PictureInPicture(current));
        assert_eq!(
            effect,
            WindowEffect::Resize {
                width: 520,
                height: 350,
            }
        );
    }

    #[test]
    fn leaving_picture_in_picture_restores_position_size_and_window_modes() {
        let previous = WindowConfiguration {
            width: 1180,
            height: 770,
            x: 120,
            y: 80,
            fullscreen: true,
            maximized: false,
            compact: false,
        };

        assert_eq!(
            PictureInPictureMode::PictureInPicture(previous).exit(),
            Some((PictureInPictureMode::Main, WindowEffect::Restore(previous)))
        );
    }
}
