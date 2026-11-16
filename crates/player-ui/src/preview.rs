//! Offscreen layout preview. No native window, media engine, or video benchmark.
use slint::{
    ComponentHandle, Rgb8Pixel,
    platform::{
        Platform, WindowAdapter,
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
    },
};
use std::{path::Path, rc::Rc};

struct PreviewPlatform(Rc<MinimalSoftwareWindow>);
impl Platform for PreviewPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}

pub fn render(
    path: &Path,
    preferences: &player_ui::Preferences,
) -> Result<(), Box<dyn std::error::Error>> {
    let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
    slint::platform::set_platform(Box::new(PreviewPlatform(window.clone())))?;
    let ui = super::PlayerWindow::new()?;
    super::apply_preferences(
        &ui,
        preferences,
        &std::cell::Cell::new(slint::PhysicalSize::new(1180, 770)),
    );
    let (width, height) = if preferences.compact() {
        (560usize, 280usize)
    } else {
        (1180, 770)
    };
    ui.window()
        .set_size(slint::PhysicalSize::new(width as u32, height as u32));
    ui.show()?;
    let mut pixels = vec![Rgb8Pixel::default(); width * height];
    window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, width);
    });
    let bytes: Vec<u8> = pixels
        .iter()
        .flat_map(|pixel| [pixel.r, pixel.g, pixel.b])
        .collect();
    image::save_buffer(
        path,
        &bytes,
        width as u32,
        height as u32,
        image::ColorType::Rgb8,
    )?;
    Ok(())
}
