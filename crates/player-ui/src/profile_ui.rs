use crate::PlayerWindow;
use player_render::Engine;
use player_services::DisplayProfile;
use slint::ComponentHandle;
use std::{
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
};
struct Selection {
    generation: u64,
    result: Result<Option<DisplayProfile>, String>,
}
#[derive(Clone)]
struct Worker {
    sender: mpsc::Sender<Selection>,
    generation: Arc<AtomicU64>,
    busy: Arc<AtomicBool>,
}
impl Worker {
    fn start(&self, ui: &PlayerWindow, selected: Option<PathBuf>) {
        if self.busy.swap(true, Ordering::AcqRel) {
            ui.set_color_profile_notice("A profile selection is already open".into());
            return;
        }
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let worker = self.clone();
        let weak = ui.as_weak();
        ui.set_color_profile_notice("Choose a display ICC profile…".into());
        std::thread::spawn(move || {
            let selected = selected.or_else(|| {
                futures_lite::future::block_on(
                    rfd::AsyncFileDialog::new()
                        .set_title("Choose the ICC profile for your monitor")
                        .add_filter("RGB display profile", &["icc", "icm"])
                        .pick_file(),
                )
                .map(|file| file.path().to_path_buf())
            });
            let result = selected.map(|path| DisplayProfile::new(&path)).transpose();
            let _ = worker.sender.send(Selection { generation, result });
            let _ = weak.upgrade_in_event_loop(|ui| ui.invoke_color_profile_result_ready());
        });
    }
}
pub(super) type ProfileApply = Rc<dyn Fn(Option<DisplayProfile>) -> Result<(), String>>;
pub fn install(ui: &PlayerWindow, engine: &Option<Rc<Engine>>) {
    let engine = engine.clone();
    install_with_apply(
        ui,
        Rc::new(move |profile| {
            engine
                .as_ref()
                .ok_or_else(|| "Playback engine unavailable".to_owned())?
                .set_display_profile(profile)
        }),
    );
}
pub(super) fn install_with_apply(ui: &PlayerWindow, apply: ProfileApply) {
    let (sender, receiver) = mpsc::channel();
    let worker = Worker {
        sender,
        generation: Arc::new(AtomicU64::new(0)),
        busy: Arc::new(AtomicBool::new(false)),
    };
    let (weak, picker) = (ui.as_weak(), worker.clone());
    ui.on_choose_color_profile(move || {
        if let Some(ui) = weak.upgrade() {
            picker.start(&ui, None);
        }
    });
    let (weak, picker) = (ui.as_weak(), worker.clone());
    ui.on_load_color_profile(move |path| {
        if let Some(ui) = weak.upgrade() {
            picker.start(&ui, Some(PathBuf::from(path.as_str())));
        }
    });
    let (weak, playback, generation, busy) = (
        ui.as_weak(),
        apply.clone(),
        worker.generation.clone(),
        worker.busy.clone(),
    );
    ui.on_color_profile_result_ready(move || {
        let Some(ui) = weak.upgrade() else { return; };
        for selection in receiver.try_iter() {
            busy.store(false, Ordering::Release);
            if selection.generation != generation.load(Ordering::Acquire) { continue; }
            let notice = match selection.result {
                Ok(None) => "Profile selection cancelled".into(),
                Err(error) => error,
                Ok(Some(profile)) => {
                    let name = profile.path().file_name().unwrap_or_default().to_string_lossy().chars().take(64).collect::<String>();
                    playback(Some(profile)).map(|()| format!("Selected {name} for this session; choose the appropriate profile when changing monitors")).unwrap_or_else(|error| error)
                },
            };
            ui.set_color_profile_notice(notice.into());
        }
    });
    let (weak, generation) = (ui.as_weak(), worker.generation);
    ui.on_clear_color_profile(move || {
        generation.fetch_add(1, Ordering::AcqRel);
        if let Some(ui) = weak.upgrade() {
            ui.set_color_profile_notice(
                apply(None)
                    .map(|()| "Manual display profile cleared".to_owned())
                    .unwrap_or_else(|error| error)
                    .into(),
            );
        }
    });
}
