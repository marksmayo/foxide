//! Optional service jobs. No network work runs on the UI thread.
use crate::{PlayerWindow, RadioRow, WebDavRow};
use player_services::{CurlClient, HttpClient, RadioDirectory, WebDavCredentials, WebDavDirectory};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::{
    cell::RefCell,
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

    fn propfind(
        &self,
        url: &player_services::Url,
        body: &[u8],
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, String> {
        CurlClient::system()?.propfind(url, body, cancel)
    }

    fn propfind_authenticated(
        &self,
        url: &player_services::Url,
        body: &[u8],
        credentials: Option<&WebDavCredentials>,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, String> {
        CurlClient::system()?.propfind_authenticated(url, body, credentials, cancel)
    }
}

pub struct WebDavController {
    active: Arc<RefCell<Option<Arc<AtomicBool>>>>,
    generation: Arc<AtomicU64>,
    jobs: Jobs,
}

impl Drop for WebDavController {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        if let Some(cancel) = self.active.borrow().as_ref() {
            cancel.store(true, Ordering::Release);
        }
        for job in self.jobs.borrow_mut().drain(..) {
            let _ = job.join();
        }
    }
}

pub fn install_webdav(ui: &PlayerWindow) -> WebDavController {
    install_webdav_with_client(ui, Arc::new(SystemHttp))
}

pub fn install_webdav_with_client<C: HttpClient + 'static>(
    ui: &PlayerWindow,
    client: Arc<C>,
) -> WebDavController {
    let active = Arc::new(RefCell::new(None::<Arc<AtomicBool>>));
    let generation = Arc::new(AtomicU64::new(0));
    let jobs: Jobs = Rc::new(RefCell::new(Vec::new()));

    let (weak, current, generations) = (ui.as_weak(), active.clone(), generation.clone());
    ui.on_webdav_cancel(move || {
        let was_active = if let Some(cancel) = current.borrow_mut().take() {
            cancel.store(true, Ordering::Release);
            true
        } else {
            false
        };
        generations.fetch_add(1, Ordering::AcqRel);
        if was_active && let Some(ui) = weak.upgrade() {
            ui.set_webdav_busy(false);
            ui.set_webdav_status("Browse cancelled".into());
        }
    });

    let (weak, current, generations, tasks, http) = (
        ui.as_weak(),
        active.clone(),
        generation.clone(),
        jobs.clone(),
        client,
    );
    ui.on_webdav_browse(move |text| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        if let Some(old) = current.borrow_mut().take() {
            old.store(true, Ordering::Release);
        }
        let request_id = generations.fetch_add(1, Ordering::AcqRel) + 1;
        let directory = match WebDavDirectory::new(text.as_str()) {
            Ok(directory) => directory,
            Err(error) => {
                ui.set_webdav_busy(false);
                ui.set_webdav_entries(ModelRc::new(VecModel::from(Vec::<WebDavRow>::new())));
                ui.set_webdav_status(error.into());
                return;
            }
        };
        let username = ui.get_webdav_user();
        let password = ui.get_webdav_password();
        let credentials = match (username.is_empty(), password.is_empty()) {
            (true, true) => None,
            (false, false) => match WebDavCredentials::new(username.as_str(), password.as_str()) {
                Ok(credentials) => Some(credentials),
                Err(error) => {
                    ui.set_webdav_busy(false);
                    ui.set_webdav_entries(ModelRc::new(VecModel::from(Vec::<WebDavRow>::new())));
                    ui.set_webdav_status(error.into());
                    return;
                }
            },
            _ => {
                ui.set_webdav_busy(false);
                ui.set_webdav_entries(ModelRc::new(VecModel::from(Vec::<WebDavRow>::new())));
                ui.set_webdav_status(
                    "Enter both a username and password, or leave both blank".into(),
                );
                return;
            }
        };
        let cancel = Arc::new(AtomicBool::new(false));
        *current.borrow_mut() = Some(cancel.clone());
        ui.set_webdav_url(text.clone());
        ui.set_webdav_busy(true);
        ui.set_webdav_status("Connecting to WebDAV…".into());
        ui.set_webdav_entries(ModelRc::new(VecModel::from(Vec::<WebDavRow>::new())));
        reap(&tasks);
        let (weak, generations, http) = (weak.clone(), generations.clone(), http.clone());
        tasks.borrow_mut().push(std::thread::spawn(move || {
            let result =
                directory.browse_with_credentials(http.as_ref(), &cancel, credentials.as_ref());
            let _ = weak.upgrade_in_event_loop(move |ui| {
                if generations.load(Ordering::Acquire) != request_id {
                    return;
                }
                ui.set_webdav_busy(false);
                match result {
                    Ok(entries) => {
                        let count = entries.len();
                        let rows = entries
                            .into_iter()
                            .map(|entry| WebDavRow {
                                name: entry.name.into(),
                                href: entry.href.to_string().into(),
                                collection: entry.is_collection,
                                detail: if entry.is_collection {
                                    "Folder".into()
                                } else {
                                    entry
                                        .size_bytes
                                        .map(format_bytes)
                                        .unwrap_or_default()
                                        .into()
                                },
                            })
                            .collect::<Vec<_>>();
                        ui.set_webdav_entries(ModelRc::new(VecModel::from(rows)));
                        ui.set_webdav_status(if count == 0 {
                            "This folder is empty".into()
                        } else {
                            format!("{count} item(s)").into()
                        });
                    }
                    Err(error) => ui.set_webdav_status(error.into()),
                }
            });
        }));
    });

    let weak = ui.as_weak();
    ui.on_webdav_open(move |text| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let result = WebDavDirectory::new(ui.get_webdav_url().as_str())
            .and_then(|directory| {
                player_services::public_http_url(text.as_str()).map(|url| (directory, url))
            })
            .and_then(|(directory, url)| {
                if !directory.contains_item(&url) {
                    return Err("WebDAV item is outside the current collection".to_owned());
                }
                Ok((directory, url))
            });
        match result {
            Ok((_directory, url)) => {
                ui.set_webdav_password("".into());
                ui.invoke_open_url(url.to_string().into());
                ui.set_panel("".into());
            }
            Err(error) => ui.set_webdav_status(error.into()),
        }
    });

    WebDavController {
        active,
        generation,
        jobs,
    }
}

fn format_bytes(size: u64) -> String {
    if size < 1024 {
        format!("{size} B")
    } else if size < 1024 * 1024 {
        format!("{:.1} KiB", size as f64 / 1024.0)
    } else {
        format!("{:.1} MiB", size as f64 / (1024.0 * 1024.0))
    }
}

pub struct RadioController {
    active: Rc<RefCell<Option<Arc<AtomicBool>>>>,
    click_cancel: Arc<AtomicBool>,
    jobs: Jobs,
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
