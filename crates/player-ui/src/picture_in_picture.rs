#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PictureInPictureMode {
    Main,
    PictureInPicture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WindowEffect {
    ShowSeparateWindow,
    HideSeparateWindow,
}

impl PictureInPictureMode {
    pub(super) fn enter(self) -> Option<(Self, WindowEffect)> {
        match self {
            Self::Main => Some((Self::PictureInPicture, WindowEffect::ShowSeparateWindow)),
            Self::PictureInPicture => None,
        }
    }

    pub(super) fn exit(self) -> Option<(Self, WindowEffect)> {
        match self {
            Self::Main => None,
            Self::PictureInPicture => Some((Self::Main, WindowEffect::HideSeparateWindow)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PictureInPictureMode, WindowEffect};

    #[test]
    fn entering_picture_in_picture_opens_a_separate_window() {
        assert_eq!(
            PictureInPictureMode::Main.enter(),
            Some((
                PictureInPictureMode::PictureInPicture,
                WindowEffect::ShowSeparateWindow
            ))
        );
    }

    #[test]
    fn returning_to_main_hides_the_overlay_and_keeps_main_window_open() {
        assert_eq!(
            PictureInPictureMode::PictureInPicture.exit(),
            Some((PictureInPictureMode::Main, WindowEffect::HideSeparateWindow))
        );
    }
}
