use crate::{
    native::Engine,
    thumbnail_cache::{LatestRequest, ThumbnailKey},
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

const THUMBNAIL_INTERVAL_SECONDS: f64 = 2.0;
const ENGINE_POLL: Duration = Duration::from_millis(80);
type WakeupCallback = Arc<dyn Fn() + Send + Sync>;
type SharedWakeup = Arc<Mutex<Option<WakeupCallback>>>;

#[derive(Clone, Debug)]
pub(crate) struct ThumbnailRequest {
    pub(crate) key: ThumbnailKey,
    pub(crate) media: Arc<str>,
    pub(crate) position: f64,
}

enum Work {
    Request(ThumbnailRequest),
    Clear,
}

pub struct ThumbnailController {
    queue: Arc<LatestRequest<Work>>,
    current: Arc<Mutex<Option<ThumbnailRequest>>>,
    engine: Arc<Mutex<Option<Arc<Engine>>>>,
    wakeup: SharedWakeup,
    stop: Arc<AtomicBool>,
}

impl ThumbnailController {
    pub fn new() -> Self {
        let queue = Arc::new(LatestRequest::new());
        let current = Arc::new(Mutex::new(None));
        let engine = Arc::new(Mutex::new(None));
        let wakeup = Arc::new(Mutex::new(None::<WakeupCallback>));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_queue = queue.clone();
        let worker_current = current.clone();
        let worker_engine = engine.clone();
        let worker_wakeup = wakeup.clone();
        let worker_stop = stop.clone();
        let _ = thread::Builder::new()
            .name("foxide-thumbnail-worker".into())
            .spawn(move || {
                thumbnail_worker(
                    worker_queue,
                    worker_current,
                    worker_engine,
                    worker_wakeup,
                    worker_stop,
                )
            });
        Self {
            queue,
            current,
            engine,
            wakeup,
            stop,
        }
    }

    pub fn request(&self, media: String, generation: u64, position: f64, duration: f64) -> bool {
        if media.is_empty()
            || media.contains("://")
            || !position.is_finite()
            || !duration.is_finite()
            || duration <= 0.0
        {
            self.clear();
            return false;
        }
        let position = position.clamp(0.0, duration);
        let bucket = (position / THUMBNAIL_INTERVAL_SECONDS).floor() as u64;
        let request = ThumbnailRequest {
            key: ThumbnailKey::new(Arc::from(media.as_str()), generation, bucket),
            media: media.into(),
            position: (bucket as f64 * THUMBNAIL_INTERVAL_SECONDS).min(duration),
        };
        if let Ok(mut current) = self.current.lock() {
            *current = Some(request.clone());
        } else {
            return false;
        }
        self.queue.submit(Work::Request(request))
    }

    pub fn clear(&self) {
        if let Ok(mut current) = self.current.lock() {
            *current = None;
        }
        let _ = self.queue.submit(Work::Clear);
    }

    pub(crate) fn current_request(&self) -> Option<ThumbnailRequest> {
        self.current.lock().ok().and_then(|current| current.clone())
    }

    pub(crate) fn preview_engine(&self) -> Option<Arc<Engine>> {
        self.engine.lock().ok().and_then(|engine| engine.clone())
    }

    pub(crate) fn set_wakeup(&self, wakeup: WakeupCallback) {
        if let Ok(mut slot) = self.wakeup.lock() {
            *slot = Some(wakeup.clone());
        }
        if let Some(engine) = self.preview_engine() {
            engine.set_snapshot_notifier(wakeup);
        }
    }
}

impl Default for ThumbnailController {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ThumbnailController {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.queue.close();
    }
}

fn thumbnail_worker(
    queue: Arc<LatestRequest<Work>>,
    current: Arc<Mutex<Option<ThumbnailRequest>>>,
    engine_slot: Arc<Mutex<Option<Arc<Engine>>>>,
    wakeup: SharedWakeup,
    stop: Arc<AtomicBool>,
) {
    let mut engine: Option<Arc<Engine>> = None;
    let mut loading_path = None::<Arc<str>>;
    let mut last_seek = None::<ThumbnailKey>;

    while !stop.load(Ordering::Acquire) {
        let Some(work) = queue.take_timeout(ENGINE_POLL) else {
            continue;
        };
        let Work::Request(mut request) = work else {
            continue;
        };
        if engine.is_none() {
            let Ok(created) = Engine::new_thumbnail() else {
                continue;
            };
            let created = Arc::new(created);
            let _ = created.command(vec!["set".into(), "pause".into(), "yes".into()]);
            if let Some(callback) = wakeup.lock().ok().and_then(|slot| slot.clone()) {
                created.set_snapshot_notifier(callback);
            }
            if let Ok(mut shared) = engine_slot.lock() {
                *shared = Some(created.clone());
            }
            engine = Some(created);
        }

        while !stop.load(Ordering::Acquire) {
            if let Some(new_work) = queue.take_timeout(ENGINE_POLL) {
                match new_work {
                    Work::Request(next) => request = next,
                    Work::Clear => break,
                }
            }
            let Some(engine) = engine.as_ref() else {
                break;
            };
            let Some(snapshot) = engine.snapshot() else {
                continue;
            };
            if snapshot.error.is_empty()
                && snapshot.loaded
                && snapshot.path == request.media.as_ref()
            {
                if last_seek.as_ref() != Some(&request.key) {
                    if engine
                        .command(vec![
                            "seek".into(),
                            request.position.to_string(),
                            "absolute+exact".into(),
                        ])
                        .is_err()
                    {
                        break;
                    }
                    last_seek = Some(request.key.clone());
                }
                if (snapshot.position - request.position).abs() <= 0.75 {
                    if let Some(callback) = wakeup.lock().ok().and_then(|slot| slot.clone()) {
                        callback();
                    }
                    break;
                }
            } else if snapshot.requested_path.as_ref() != request.media.as_ref()
                && loading_path.as_deref() != Some(request.media.as_ref())
            {
                if engine
                    .command(vec![
                        "loadfile".into(),
                        request.media.to_string(),
                        "replace".into(),
                    ])
                    .is_err()
                {
                    break;
                }
                loading_path = Some(request.media.clone());
                last_seek = None;
            } else if !snapshot.error.is_empty() {
                break;
            }
            if current
                .lock()
                .ok()
                .and_then(|value| value.as_ref().map(|value| value.key.clone()))
                .as_ref()
                != Some(&request.key)
            {
                if let Some(latest) = current.lock().ok().and_then(|value| value.clone()) {
                    request = latest;
                } else {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_requests_are_bucketed_and_clamped_to_media_duration() {
        let controller = ThumbnailController::new();

        assert!(controller.request("clip.mkv".into(), 3, 7.9, 10.0));
        let request = controller.current_request().unwrap();

        assert_eq!(request.key, ThumbnailKey::new(Arc::from("clip.mkv"), 3, 3));
        assert_eq!(request.position, 6.0);
    }

    #[test]
    fn invalid_hover_request_clears_the_previous_preview() {
        let controller = ThumbnailController::new();
        assert!(controller.request("clip.mkv".into(), 3, 4.0, 10.0));

        assert!(!controller.request("https://example.test/clip".into(), 3, 5.0, 10.0));
        assert!(controller.current_request().is_none());
    }

    #[test]
    fn reload_generation_changes_the_thumbnail_identity() {
        let controller = ThumbnailController::new();
        assert!(controller.request("clip.mkv".into(), 3, 4.0, 10.0));
        let first = controller.current_request().unwrap();

        assert!(controller.request("clip.mkv".into(), 4, 4.0, 10.0));
        let reloaded = controller.current_request().unwrap();

        assert_ne!(first.key, reloaded.key);
    }
}
