use crate::PlayerWindow;
use player_render::Engine;
use player_services::ShaderFile;
use slint::{ComponentHandle, Model};
use std::{
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
};

struct ResultMessage {
    generation: u64,
    selection: Result<Option<Vec<ShaderFile>>, String>,
}
#[derive(Clone)]
struct Worker {
    sender: mpsc::Sender<ResultMessage>,
    generation: Arc<AtomicU64>,
    busy: Arc<AtomicBool>,
}
impl Worker {
    fn start(&self, ui: &PlayerWindow, chosen: Option<Vec<PathBuf>>) {
        if self.busy.swap(true, Ordering::AcqRel) {
            ui.set_shader_notice("A shader selection is already open".into());
            return;
        }
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let worker = self.clone();
        let weak = ui.as_weak();
        ui.set_shader_notice("Choose trusted GPU shader files…".into());
        std::thread::spawn(move || {
            let paths = chosen.or_else(|| {
                futures_lite::future::block_on(
                    rfd::AsyncFileDialog::new()
                        .set_title(
                            "Choose trusted mpv GLSL shaders (selection order is pipeline order)",
                        )
                        .add_filter("mpv user shaders", &["glsl", "hook"])
                        .pick_files(),
                )
                .map(|files| {
                    files
                        .into_iter()
                        .map(|file| file.path().to_path_buf())
                        .collect()
                })
            });
            let selection = paths
                .map(|paths| {
                    if paths.len() > 8 {
                        return Err("Choose at most eight shaders".into());
                    }
                    paths
                        .iter()
                        .map(|path| ShaderFile::new(path))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose();
            let _ = worker.sender.send(ResultMessage {
                generation,
                selection,
            });
            worker.busy.store(false, Ordering::Release);
            let _ = weak.upgrade_in_event_loop(|ui| ui.invoke_shader_result_ready());
        });
    }
}

/// Native file reads and dialogs run off the UI thread. Completion wakes the UI
/// once; no polling timer or extra idle/render wakeup is introduced.
pub fn install(ui: &PlayerWindow, engine: &Option<Rc<Engine>>) {
    let engine = engine.clone();
    install_with_apply(
        ui,
        Rc::new(move |shaders| {
            engine
                .as_ref()
                .ok_or_else(|| "Playback engine unavailable".to_owned())?
                .set_shaders(shaders)
        }),
    );
}

pub(super) type ShaderApply = Rc<dyn Fn(Vec<ShaderFile>) -> Result<(), String>>;
pub(super) fn install_with_apply(ui: &PlayerWindow, apply: ShaderApply) {
    let (sender, receiver) = mpsc::channel();
    let worker = Worker {
        sender,
        generation: Arc::new(AtomicU64::new(0)),
        busy: Arc::new(AtomicBool::new(false)),
    };
    let (weak, picker) = (ui.as_weak(), worker.clone());
    ui.on_choose_shaders(move || {
        if let Some(ui) = weak.upgrade() {
            picker.start(&ui, None);
        }
    });
    let (weak, selection) = (ui.as_weak(), worker.clone());
    ui.on_load_shader_files(move |paths| {
        if let Some(ui) = weak.upgrade() {
            selection.start(
                &ui,
                Some(
                    paths
                        .iter()
                        .map(|path| PathBuf::from(path.as_str()))
                        .collect(),
                ),
            );
        }
    });
    let (weak, playback, generation) = (ui.as_weak(), apply.clone(), worker.generation.clone());
    ui.on_shader_result_ready(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        for result in receiver.try_iter() {
            if result.generation != generation.load(Ordering::Acquire) {
                continue;
            }
            let notice = match result.selection {
                Ok(None) => "Shader selection cancelled".into(),
                Err(error) => error,
                Ok(Some(shaders)) => {
                    let count = shaders.len();
                    match playback(shaders) {
                        Ok(()) => format!(
                            "Applying {count} shader(s); renderer errors appear in playback status"
                        ),
                        Err(error) => error,
                    }
                }
            };
            ui.set_shader_notice(notice.into());
        }
    });
    let (weak, playback, generation) = (ui.as_weak(), apply, worker.generation);
    ui.on_clear_shaders(move || {
        generation.fetch_add(1, Ordering::AcqRel);
        if let Some(ui) = weak.upgrade() {
            let notice = playback(vec![])
                .map(|()| "Clearing shader stack".to_owned())
                .unwrap_or_else(|error| error);
            ui.set_shader_notice(notice.into());
        }
    });
}
