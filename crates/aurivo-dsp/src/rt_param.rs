//! Real-time-safe parameter plumbing shared by every DSP processor.
//!
//! Audio callbacks must never block. The UI owns an `Arc<Mutex<T>>` and writes
//! new parameters there; the audio thread owns an [`RtParam`] cache and uses
//! `try_lock` to refresh it. If the UI currently holds the lock, the audio
//! thread keeps rendering with the last known good values instead of stalling.
//!
//! This is a deliberate trade-off: a parameter change may be applied one audio
//! block late under contention, but latency stays bounded and predictable.

use std::sync::{Arc, Mutex};

/// A copy-on-poll snapshot of a shared parameter block.
#[derive(Debug)]
pub struct RtParam<T: Clone> {
    shared: Arc<Mutex<T>>,
    cached: T,
    refreshes: u64,
    skipped: u64,
}

impl<T: Clone> RtParam<T> {
    /// Wrap a shared parameter block. `initial` is used until the first
    /// successful poll, which means the audio thread never blocks at startup.
    pub fn new(shared: Arc<Mutex<T>>, initial: T) -> Self {
        Self {
            shared,
            cached: initial,
            refreshes: 0,
            skipped: 0,
        }
    }

    /// Attempt a non-blocking refresh and return the current local snapshot.
    pub fn poll(&mut self) -> &T {
        match self.shared.try_lock() {
            Ok(guard) => {
                self.cached = guard.clone();
                self.refreshes += 1;
            }
            Err(_) => self.skipped += 1,
        }
        &self.cached
    }

    /// Return the cached value without attempting a refresh. Useful when a
    /// processor needs the parameter more than once in the same block.
    pub fn cached(&self) -> &T {
        &self.cached
    }

    /// Number of successful parameter refreshes.
    pub fn refreshes(&self) -> u64 {
        self.refreshes
    }

    /// Number of polls that had to reuse the cached value.
    pub fn skipped(&self) -> u64 {
        self.skipped
    }

    /// Access the shared handle for UI-side updates and snapshots.
    pub fn shared(&self) -> &Arc<Mutex<T>> {
        &self.shared
    }

    /// Clone the shared handle for a UI controller.
    pub fn handle(&self) -> Arc<Mutex<T>> {
        self.shared.clone()
    }
}

/// Snapshot a shared parameter block from the UI thread.
///
/// Uses `try_lock` so a slow UI never waits on the audio thread either. If the
/// lock is contended, the caller-provided fallback is returned.
pub fn try_snapshot<T: Clone>(handle: &Arc<Mutex<T>>, fallback: T) -> T {
    handle.try_lock().map(|g| g.clone()).unwrap_or(fallback)
}

/// Update a shared parameter block from the UI thread without blocking.
/// Returns `true` when the update was applied.
pub fn try_update<T>(handle: &Arc<Mutex<T>>, value: T) -> bool {
    match handle.try_lock() {
        Ok(mut guard) => {
            *guard = value;
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[derive(Clone, Debug, PartialEq)]
    struct Params {
        gain: f32,
        mode: i32,
    }

    #[test]
    fn poll_reads_initial_and_updated_values() {
        let shared = Arc::new(Mutex::new(Params { gain: 1.0, mode: 0 }));
        let mut rt = RtParam::new(shared.clone(), Params { gain: 1.0, mode: 0 });
        assert_eq!(rt.poll(), &Params { gain: 1.0, mode: 0 });
        *shared.lock().unwrap() = Params { gain: 2.0, mode: 3 };
        assert_eq!(rt.poll(), &Params { gain: 2.0, mode: 3 });
        assert_eq!(rt.refreshes(), 2);
        assert_eq!(rt.skipped(), 0);
    }

    #[test]
    fn contended_lock_keeps_last_known_good_value() {
        let shared = Arc::new(Mutex::new(Params { gain: 1.0, mode: 0 }));
        let mut rt = RtParam::new(shared.clone(), Params { gain: 1.0, mode: 0 });
        let holder = shared.lock().unwrap();
        assert_eq!(rt.poll(), &Params { gain: 1.0, mode: 0 });
        assert_eq!(rt.skipped(), 1);
        drop(holder);
        *shared.lock().unwrap() = Params { gain: 9.0, mode: 1 };
        assert_eq!(rt.poll(), &Params { gain: 9.0, mode: 1 });
    }

    #[test]
    fn try_update_and_snapshot() {
        let shared = Arc::new(Mutex::new(Params { gain: 0.0, mode: 0 }));
        assert!(try_update(&shared, Params { gain: 4.0, mode: 2 }));
        assert_eq!(
            try_snapshot(&shared, Params { gain: -1.0, mode: -1 }),
            Params { gain: 4.0, mode: 2 }
        );
    }

    #[test]
    fn try_paths_never_block_on_a_held_lock() {
        let shared = Arc::new(Mutex::new(1u32));
        let holder = shared.lock().unwrap();
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            assert!(!try_update(&shared, 2));
            assert_eq!(try_snapshot(&shared, 7), 7);
        }
        assert!(start.elapsed() < Duration::from_millis(100));
        drop(holder);
    }

    #[test]
    fn concurrent_writer_and_reader_stay_finite() {
        let shared = Arc::new(Mutex::new(0.5f32));
        let mut rt = RtParam::new(shared.clone(), 0.5);
        let writer = thread::spawn(move || {
            for i in 0..5000 {
                *shared.lock().unwrap() = i as f32 / 5000.0;
            }
        });
        for _ in 0..5000 {
            assert!(rt.poll().is_finite());
        }
        writer.join().unwrap();
        assert!(rt.refreshes() + rt.skipped() >= 5000);
    }
}
