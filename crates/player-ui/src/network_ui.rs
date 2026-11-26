//! Optional service jobs. No network work runs on the UI thread.
use crate::UiState;
use crate::{PlayerWindow, RadioRow, SubtitleRow};
use player_services::{CurlClient, HttpClient, RadioDirectory, SubtitleProvider, SubtitleResult};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::JoinHandle,
};

type Jobs = Rc<RefCell<Vec<JoinHandle<()>>>>;

pub struct SystemHttp;
impl HttpClient for SystemHttp {
    fn get(&self, url: &player_services::Url, cancel: &AtomicBool) -> Result<Vec<u8>, String> {
        CurlClient::system()?.get(url, cancel)
    }

    fn get_bounded(
        &self,
        url: &player_services::Url,
        max_bytes: usize,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, String> {
        CurlClient::system()?.get_bounded(url, max_bytes, cancel)
    }

    fn get_with_headers(
        &self,
        url: &player_services::Url,
        headers: &[(&str, &str)],
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, String> {
        CurlClient::system()?.get_with_headers(url, headers, cancel)
    }

    fn post_json(
        &self,
        url: &player_services::Url,
        headers: &[(&str, &str)],
        body: &[u8],
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, String> {
        CurlClient::system()?.post_json(url, headers, body, cancel)
    }
}

pub struct RadioController {
    active: Rc<RefCell<Option<Arc<AtomicBool>>>>,
    click_cancel: Arc<AtomicBool>,
    jobs: Jobs,
}

pub struct SubtitleController {
    active: Rc<RefCell<Option<Arc<AtomicBool>>>>,
    jobs: Jobs,
}

impl Drop for SubtitleController {
    fn drop(&mut self) {
        if let Some(active) = self.active.borrow().as_ref() {
            active.store(true, Ordering::Release);
        }
        for job in self.jobs.borrow_mut().drain(..) {
            let _ = job.join();
        }
    }
}
impl Drop for RadioController {
    fn drop(&mut self) {
        if let Some(active) = self.active.borrow().as_ref() {
            active.store(true, Ordering::Release);
        }
        self.click_cancel.store(true, Ordering::Release);
        for job in self.jobs.borrow_mut().drain(..) {
            let _ = job.join();
        }
    }
}
fn reap(jobs: &Jobs) {
    let mut jobs = jobs.borrow_mut();
    let mut index = 0;
    while index < jobs.len() {
        if jobs[index].is_finished() {
            let _ = jobs.swap_remove(index).join();
        } else {
            index += 1;
        }
    }
}

pub fn install_radio<C: HttpClient + 'static>(
    ui: &PlayerWindow,
    client: Arc<C>,
    directory: RadioDirectory,
) -> RadioController {
    let active: Rc<RefCell<Option<Arc<AtomicBool>>>> = Rc::new(RefCell::new(None));
    let sequence = Arc::new(AtomicU64::new(0));
    let jobs: Jobs = Rc::new(RefCell::new(Vec::new()));
    let click_cancel = Arc::new(AtomicBool::new(false));
    let clicking = Arc::new(AtomicBool::new(false));
    let (weak, current, generations) = (ui.as_weak(), active.clone(), sequence.clone());
    ui.on_cancel_services(move || {
        generations.fetch_add(1, Ordering::AcqRel);
        if let Some(old) = current.borrow_mut().take() {
            old.store(true, Ordering::Release);
        }
        if let Some(ui) = weak.upgrade() {
            ui.set_radio_busy(false);
            ui.set_radio_error("Search cancelled".into());
        }
    });
    let (weak, current, generations, tasks, http, api) = (
        ui.as_weak(),
        active.clone(),
        sequence.clone(),
        jobs.clone(),
        client.clone(),
        directory.clone(),
    );
    ui.on_radio_search(move |query| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        if let Some(old) = current.borrow_mut().take() {
            old.store(true, Ordering::Release);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *current.borrow_mut() = Some(cancel.clone());
        let request_id = generations.fetch_add(1, Ordering::AcqRel) + 1;
        ui.set_radio_busy(true);
        ui.set_radio_error("".into());
        ui.set_radio_stations(ModelRc::new(VecModel::from(Vec::<RadioRow>::new())));
        reap(&tasks);
        let (weak, generations, http, api) =
            (weak.clone(), generations.clone(), http.clone(), api.clone());
        tasks.borrow_mut().push(std::thread::spawn(move || {
            let result = api.search(http.as_ref(), query.as_str(), &cancel);
            let _ = weak.upgrade_in_event_loop(move |ui| {
                if generations.load(Ordering::Acquire) != request_id {
                    return;
                }
                ui.set_radio_busy(false);
                match result {
                    Ok(stations) => {
                        let rows: Vec<_> = stations
                            .into_iter()
                            .map(|station| RadioRow {
                                id: station.id.into(),
                                name: station.name.into(),
                                url: station.url.as_str().into(),
                                detail: format!(
                                    "{} · {} · {} kbps",
                                    station.country, station.codec, station.bitrate
                                )
                                .into(),
                            })
                            .collect();
                        if rows.is_empty() {
                            ui.set_radio_error("No matching stations".into());
                        }
                        ui.set_radio_stations(ModelRc::new(VecModel::from(rows)));
                    }
                    Err(error) => ui.set_radio_error(error.into()),
                }
            });
        }));
    });
    let (weak, tasks, http, api, cancel, in_flight) = (
        ui.as_weak(),
        jobs.clone(),
        client,
        directory,
        click_cancel.clone(),
        clicking,
    );
    ui.on_radio_play(move |url, id| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let row = ui
            .get_radio_stations()
            .iter()
            .find(|row| row.id == id && row.url == url);
        let Some(row) = row else {
            ui.set_radio_error("Station list changed; select a current result".into());
            return;
        };
        if player_services::public_http_url(url.as_str()).is_err() {
            ui.set_radio_error("Invalid station URL".into());
            return;
        }
        ui.set_active_radio_url(url.clone());
        ui.set_active_radio_name(row.name.clone());
        ui.invoke_open_url(url);
        ui.set_media_title(row.name);
        ui.set_panel("".into());
        if in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            reap(&tasks);
            let (http, api, cancel, done) =
                (http.clone(), api.clone(), cancel.clone(), in_flight.clone());
            tasks.borrow_mut().push(std::thread::spawn(move || {
                let _ = api.click(http.as_ref(), id.as_str(), &cancel);
                done.store(false, Ordering::Release);
            }));
        }
    });
    RadioController {
        active,
        click_cancel,
        jobs,
    }
}

pub fn install_subtitles<C: HttpClient + 'static>(
    ui: &PlayerWindow,
    client: Arc<C>,
    state: Rc<RefCell<UiState>>,
) -> SubtitleController {
    let active: Rc<RefCell<Option<Arc<AtomicBool>>>> = Rc::new(RefCell::new(None));
    let generations = Arc::new(AtomicU64::new(0));
    let jobs: Jobs = Rc::new(RefCell::new(Vec::new()));
    let current_results = Arc::new(std::sync::Mutex::new(Vec::<SubtitleResult>::new()));
    let active_key = Rc::new(RefCell::new(String::new()));
    let query_media = Rc::new(RefCell::new(None::<PathBuf>));

    let (weak, current, generation_cancel, results_cancel, key, searched_media) = (
        ui.as_weak(),
        active.clone(),
        generations.clone(),
        current_results.clone(),
        active_key.clone(),
        query_media.clone(),
    );
    ui.on_cancel_subtitle_jobs(move || {
        generation_cancel.fetch_add(1, Ordering::AcqRel);
        if let Some(old) = current.borrow_mut().take() {
            old.store(true, Ordering::Release);
        }
        *key.borrow_mut() = String::new();
        if let Ok(mut results) = results_cancel.lock() {
            results.clear();
        }
        *searched_media.borrow_mut() = None;
        if let Some(ui) = weak.upgrade() {
            ui.set_subtitle_busy(false);
            ui.set_subtitle_api_key(String::new().into());
            ui.set_subtitle_results(ModelRc::new(VecModel::from(Vec::<SubtitleRow>::new())));
        }
    });

    let (
        weak,
        current,
        generation_search,
        results_search,
        key,
        searched_media,
        tasks,
        http,
        search_state,
    ) = (
        ui.as_weak(),
        active.clone(),
        generations.clone(),
        current_results.clone(),
        active_key.clone(),
        query_media.clone(),
        jobs.clone(),
        client.clone(),
        state.clone(),
    );
    ui.on_subtitle_search(move |query, language, api_key| {
        let Some(ui) = weak.upgrade() else { return };
        let media_path = {
            let state = search_state.borrow();
            state
                .playlist
                .get(state.selected)
                .filter(|path| path.is_file())
                .cloned()
        };
        let Some(media_path) = media_path else {
            ui.set_subtitle_error("Open a local video file before searching for subtitles".into());
            return;
        };
        if let Some(old) = current.borrow_mut().take() {
            old.store(true, Ordering::Release);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *current.borrow_mut() = Some(cancel.clone());
        *key.borrow_mut() = api_key.to_string();
        *searched_media.borrow_mut() = Some(media_path);
        let request_id = generation_search.fetch_add(1, Ordering::AcqRel) + 1;
        ui.set_subtitle_busy(true);
        ui.set_subtitle_error("".into());
        if let Ok(mut results) = results_search.lock() {
            results.clear();
        }
        ui.set_subtitle_results(ModelRc::new(VecModel::from(Vec::<SubtitleRow>::new())));
        reap(&tasks);
        let (weak, generations, results, http, cancel) = (
            weak.clone(),
            generation_search.clone(),
            results_search.clone(),
            http.clone(),
            cancel.clone(),
        );
        tasks.borrow_mut().push(std::thread::spawn(move || {
            let result = SubtitleProvider::opensubtitles().search(
                http.as_ref(),
                api_key.as_str(),
                query.as_str(),
                language.as_str(),
                &cancel,
            );
            let _ = weak.upgrade_in_event_loop(move |ui| {
                if generations.load(Ordering::Acquire) != request_id {
                    return;
                }
                ui.set_subtitle_busy(false);
                match result {
                    Ok(found) => {
                        let rows: Vec<_> = found
                            .iter()
                            .map(|subtitle| SubtitleRow {
                                file_id: subtitle.file_id.clone().into(),
                                file_name: subtitle.file_name.clone().into(),
                                title: subtitle.title.clone().into(),
                                release: subtitle.release.clone().into(),
                                language: subtitle.language.clone().into(),
                                downloads: subtitle.download_count.to_string().into(),
                            })
                            .collect();
                        if rows.is_empty() {
                            ui.set_subtitle_error("No matching subtitles".into());
                        }
                        if let Ok(mut results) = results.lock() {
                            *results = found;
                        }
                        ui.set_subtitle_results(ModelRc::new(VecModel::from(rows)));
                    }
                    Err(error) => ui.set_subtitle_error(error.into()),
                }
            });
        }));
    });

    let (weak, current, generations, results, key, searched_media, tasks, http, state) = (
        ui.as_weak(),
        active.clone(),
        generations.clone(),
        current_results.clone(),
        active_key.clone(),
        query_media.clone(),
        jobs.clone(),
        client,
        state,
    );
    ui.on_subtitle_download(move |file_id| {
        let Some(ui) = weak.upgrade() else { return };
        let selected_result = results.lock().ok().and_then(|items| {
            items
                .iter()
                .find(|item| item.file_id == file_id.as_str())
                .cloned()
        });
        let Some(subtitle) = selected_result else {
            ui.set_subtitle_error("Subtitle results changed; search again".into());
            return;
        };
        let media = {
            let state = state.borrow();
            state
                .playlist
                .get(state.selected)
                .filter(|path| path.is_file())
                .cloned()
        };
        let Some(media) = media else {
            ui.set_subtitle_error("The current item is not a local media file".into());
            return;
        };
        if searched_media.borrow().as_ref() != Some(&media) {
            ui.set_subtitle_error("Media changed after the search; search again".into());
            return;
        }
        let Some(directory) = media.parent().map(Path::to_path_buf) else {
            ui.set_subtitle_error("Could not find the media folder".into());
            return;
        };
        let api_key = key.borrow().clone();
        if api_key.is_empty() {
            ui.set_subtitle_error("Enter an application API key and search again".into());
            return;
        }
        if let Some(old) = current.borrow_mut().take() {
            old.store(true, Ordering::Release);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *current.borrow_mut() = Some(cancel.clone());
        let request_id = generations.fetch_add(1, Ordering::AcqRel) + 1;
        ui.set_subtitle_busy(true);
        ui.set_subtitle_error("Downloading and validating subtitle…".into());
        reap(&tasks);
        let (weak, generations, http, key, cancel) = (
            weak.clone(),
            generations.clone(),
            http.clone(),
            api_key,
            cancel,
        );
        tasks.borrow_mut().push(std::thread::spawn(move || {
            let result = SubtitleProvider::opensubtitles().download_to(
                http.as_ref(),
                &key,
                &subtitle,
                &directory,
                &cancel,
            );
            let _ = weak.upgrade_in_event_loop(move |ui| {
                if generations.load(Ordering::Acquire) != request_id {
                    return;
                }
                ui.set_subtitle_busy(false);
                match result {
                    Ok(path) => {
                        if let Some(path) = path.to_str() {
                            ui.invoke_subtitle_added(path.into());
                            ui.set_subtitle_error(
                                "Subtitle downloaded and added to playback".into(),
                            );
                        } else {
                            ui.set_subtitle_error(
                                "Downloaded subtitle path is not valid Unicode".into(),
                            );
                        }
                    }
                    Err(error) => ui.set_subtitle_error(error.into()),
                }
            });
        }));
    });
    SubtitleController { active, jobs }
}
