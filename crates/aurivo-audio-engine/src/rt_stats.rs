//! Lock-contention counters for the real-time audio path.
//!
//! Every callback uses non-blocking locks. When a lock is held by the UI the
//! callback skips that stage instead of waiting; these counters make those
//! skips visible so regressions can be diagnosed from the UI or CI.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct RtStats {
    chain: AtomicU64,
    analyzer: AtomicU64,
    recorder: AtomicU64,
    resampler: AtomicU64,
}

impl RtStats {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn bump_chain(&self) {
        self.chain.fetch_add(1, Ordering::Relaxed);
    }

    pub fn bump_analyzer(&self) {
        self.analyzer.fetch_add(1, Ordering::Relaxed);
    }

    pub fn bump_recorder(&self) {
        self.recorder.fetch_add(1, Ordering::Relaxed);
    }

    pub fn bump_resampler(&self) {
        self.resampler.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> RtSnapshot {
        RtSnapshot {
            chain: self.chain.load(Ordering::Relaxed),
            analyzer: self.analyzer.load(Ordering::Relaxed),
            recorder: self.recorder.load(Ordering::Relaxed),
            resampler: self.resampler.load(Ordering::Relaxed),
        }
    }

    pub fn reset(&self) {
        self.chain.store(0, Ordering::Relaxed);
        self.analyzer.store(0, Ordering::Relaxed);
        self.recorder.store(0, Ordering::Relaxed);
        self.resampler.store(0, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RtSnapshot {
    pub chain: u64,
    pub analyzer: u64,
    pub recorder: u64,
    pub resampler: u64,
}

impl RtSnapshot {
    pub fn total(&self) -> u64 {
        self.chain + self.analyzer + self.recorder + self.resampler
    }

    pub fn as_array(&self) -> [u64; 4] {
        [self.chain, self.analyzer, self.recorder, self.resampler]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn counters_are_independent() {
        let stats = RtStats::new();
        stats.bump_chain();
        stats.bump_chain();
        stats.bump_analyzer();
        let snap = stats.snapshot();
        assert_eq!(snap.chain, 2);
        assert_eq!(snap.analyzer, 1);
        assert_eq!(snap.total(), 3);
        assert_eq!(snap.as_array(), [2, 1, 0, 0]);
    }

    #[test]
    fn reset_clears_all_counters() {
        let stats = RtStats::new();
        stats.bump_chain();
        stats.bump_recorder();
        stats.reset();
        assert_eq!(stats.snapshot().total(), 0);
    }

    #[test]
    fn concurrent_bumps_are_counted() {
        let stats = RtStats::new();
        let mut handles = Vec::new();
        for _ in 0..4 {
            let stats = stats.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    stats.bump_resampler();
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(stats.snapshot().resampler, 4000);
    }
}
