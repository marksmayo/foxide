//! Optional service jobs. No network work runs on the UI thread.
use crate::{PlayerWindow, RadioRow};
use player_services::{CurlClient, HttpClient, RadioDirectory};
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
