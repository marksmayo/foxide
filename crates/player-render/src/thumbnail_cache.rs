use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

pub(crate) struct LatestRequest<T> {
    latest: Mutex<Option<T>>,
    closed: Mutex<bool>,
    wake: Condvar,
}

impl<T> LatestRequest<T> {
    pub(crate) fn new() -> Self {
        Self {
            latest: Mutex::new(None),
            closed: Mutex::new(false),
            wake: Condvar::new(),
        }
    }

    pub(crate) fn submit(&self, request: T) -> bool {
        let Ok(closed) = self.closed.lock() else {
            return false;
        };
        if *closed {
            return false;
        }
        let Ok(mut latest) = self.latest.lock() else {
            return false;
        };
        *latest = Some(request);
        self.wake.notify_one();
        true
    }

    pub(crate) fn take_timeout(&self, timeout: Duration) -> Option<T> {
        let Ok(mut latest) = self.latest.lock() else {
            return None;
        };
        if latest.is_none() {
            let Ok((next, _)) = self.wake.wait_timeout(latest, timeout) else {
                return None;
            };
            latest = next;
        }
        latest.take()
    }

    pub(crate) fn close(&self) {
        if let Ok(mut closed) = self.closed.lock() {
            *closed = true;
        }
        self.wake.notify_all();
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ThumbnailKey {
    media: Arc<str>,
    generation: u64,
    bucket: u64,
}

impl ThumbnailKey {
    pub(crate) fn new(media: Arc<str>, generation: u64, bucket: u64) -> Self {
        Self {
            media,
            generation,
            bucket,
        }
    }
}

pub(crate) struct ThumbnailCache {
    slots: Vec<Option<ThumbnailKey>>,
    entries: HashMap<ThumbnailKey, usize>,
    ready: HashSet<ThumbnailKey>,
    least_recently_used: VecDeque<usize>,
}

pub(crate) struct SlotReservation {
    slot: usize,
}

impl SlotReservation {
    pub(crate) fn slot(&self) -> usize {
        self.slot
    }
}

impl ThumbnailCache {
    pub(crate) fn new(byte_budget: usize, bytes_per_thumbnail: usize) -> Self {
        let capacity = byte_budget
            .checked_div(bytes_per_thumbnail)
            .unwrap_or_default();
        Self {
            slots: vec![None; capacity],
            entries: HashMap::with_capacity(capacity),
            ready: HashSet::with_capacity(capacity),
            least_recently_used: VecDeque::with_capacity(capacity),
        }
    }

    pub(crate) fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub(crate) fn reserve(&mut self, key: ThumbnailKey) -> Option<SlotReservation> {
        if self.slots.is_empty() {
            return None;
        }
        if let Some(&slot) = self.entries.get(&key) {
            self.least_recently_used
                .retain(|&candidate| candidate != slot);
            self.least_recently_used.push_back(slot);
            return Some(SlotReservation { slot });
        }

        let slot = if let Some(slot) = self.slots.iter().position(Option::is_none) {
            slot
        } else {
            let slot = self.least_recently_used.pop_front()?;
            let previous = self.slots[slot].take();
            if let Some(previous) = previous.as_ref() {
                self.entries.remove(previous);
                self.ready.remove(previous);
            }
            slot
        };

        self.slots[slot] = Some(key.clone());
        self.entries.insert(key, slot);
        self.least_recently_used.push_back(slot);
        Some(SlotReservation { slot })
    }

    #[cfg(test)]
    pub(crate) fn slot_for(&self, key: &ThumbnailKey) -> Option<usize> {
        self.entries.get(key).copied()
    }

    pub(crate) fn is_ready(&self, key: &ThumbnailKey) -> bool {
        self.ready.contains(key)
    }

    pub(crate) fn mark_ready(&mut self, key: &ThumbnailKey) {
        if self.entries.contains_key(key) {
            self.ready.insert(key.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn key(media: &str, generation: u64, bucket: u64) -> ThumbnailKey {
        ThumbnailKey::new(Arc::<str>::from(media), generation, bucket)
    }

    #[test]
    fn cache_capacity_is_bounded_by_the_gpu_byte_budget() {
        let cache = ThumbnailCache::new(16 * 1024 * 1024, 256 * 144 * 4);

        assert_eq!(cache.capacity(), 113);
        assert!(cache.capacity() * 256 * 144 * 4 <= 16 * 1024 * 1024);
    }

    #[test]
    fn reserve_reuses_a_cached_slot_and_refreshes_its_lru_position() {
        let mut cache = ThumbnailCache::new(3 * 100, 100);
        let a = key("a.mkv", 1, 0);
        let b = key("b.mkv", 1, 0);
        let c = key("c.mkv", 1, 0);

        let a_slot = cache.reserve(a.clone()).unwrap().slot();
        cache.reserve(b.clone());
        cache.reserve(c);
        assert_eq!(cache.reserve(a.clone()).unwrap().slot(), a_slot);

        let next = cache.reserve(key("d.mkv", 1, 0)).unwrap();
        assert_eq!(next.slot(), cache.slot_for(&key("d.mkv", 1, 0)).unwrap());
        assert_ne!(a_slot, next.slot());
        assert!(!cache.is_ready(&key("d.mkv", 1, 0)));
        assert_ne!(cache.reserve(b).unwrap().slot(), a_slot);
    }

    #[test]
    fn only_rendered_slots_are_marked_ready_and_eviction_clears_readiness() {
        let mut cache = ThumbnailCache::new(100, 100);
        let a = key("a.mkv", 1, 0);
        cache.reserve(a.clone());
        assert!(!cache.is_ready(&a));
        cache.mark_ready(&a);
        assert!(cache.is_ready(&a));

        cache.reserve(key("b.mkv", 1, 0));
        assert!(!cache.is_ready(&a));
    }

    #[test]
    fn reloading_the_same_path_uses_a_different_generation() {
        let mut cache = ThumbnailCache::new(200, 100);
        let first = cache.reserve(key("episode.mkv", 4, 2)).unwrap();
        let next = cache.reserve(key("episode.mkv", 5, 2)).unwrap();

        assert_ne!(first.slot(), next.slot());
    }

    #[test]
    fn zero_budget_has_no_thumbnail_slots() {
        let mut cache = ThumbnailCache::new(0, 100);
        assert_eq!(cache.capacity(), 0);
        assert!(
            cache
                .reserve(ThumbnailKey::new(Arc::from("media"), 1, 0))
                .is_none()
        );
    }

    #[test]
    fn pending_hover_requests_are_coalesced_to_the_latest_position() {
        let requests = LatestRequest::new();
        assert!(requests.submit(1));
        assert!(requests.submit(2));
        assert!(requests.submit(3));

        assert_eq!(requests.take_timeout(Duration::ZERO), Some(3));
        assert_eq!(requests.take_timeout(Duration::ZERO), None);
    }

    #[test]
    fn closing_the_request_queue_rejects_new_hover_work() {
        let requests = LatestRequest::<u8>::new();
        requests.close();

        assert!(!requests.submit(1));
        assert_eq!(requests.take_timeout(Duration::ZERO), None);
    }
}
