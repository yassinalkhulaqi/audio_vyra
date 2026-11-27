//! Buffered-audio telemetry used to estimate monitoring latency.
//!
//! The input callback writes mono samples into a ring buffer and the output
//! callback consumes them. The number of buffered samples divided by the
//! sample rate is a good approximation of the delay through the app (not
//! counting device/OS buffers). The value is published atomically so the UI
//! can poll it without touching the audio thread.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct LatencyTracker {
    buffered_ms: AtomicU32,
    peak_buffered_ms: AtomicU32,
}

impl LatencyTracker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Publish the current buffer occupancy.
    pub fn report(&self, buffered_samples: usize, sample_rate: u32) {
        let ms = milliseconds(buffered_samples, sample_rate);
        self.buffered_ms.store(ms, Ordering::Relaxed);
        self.peak_buffered_ms.fetch_max(ms, Ordering::Relaxed);
    }

    pub fn buffered_ms(&self) -> u32 {
        self.buffered_ms.load(Ordering::Relaxed)
    }

    pub fn peak_ms(&self) -> u32 {
        self.peak_buffered_ms.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.buffered_ms.store(0, Ordering::Relaxed);
        self.peak_buffered_ms.store(0, Ordering::Relaxed);
    }
}

pub fn milliseconds(samples: usize, sample_rate: u32) -> u32 {
    if sample_rate == 0 {
        return 0;
    }
    ((samples as f64 / sample_rate as f64) * 1000.0).round().max(0.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_samples_to_milliseconds() {
        assert_eq!(milliseconds(48000, 48000), 1000);
        assert_eq!(milliseconds(2400, 48000), 50);
        assert_eq!(milliseconds(100, 0), 0);
    }

    #[test]
    fn tracks_current_and_peak() {
        let tracker = LatencyTracker::new();
        tracker.report(4800, 48000);
        assert_eq!(tracker.buffered_ms(), 100);
        assert_eq!(tracker.peak_ms(), 100);
        tracker.report(480, 48000);
        assert_eq!(tracker.buffered_ms(), 10);
        assert_eq!(tracker.peak_ms(), 100);
    }

    #[test]
    fn reset_clears_peak() {
        let tracker = LatencyTracker::new();
        tracker.report(9600, 48000);
        tracker.reset();
        assert_eq!(tracker.buffered_ms(), 0);
        assert_eq!(tracker.peak_ms(), 0);
    }

    #[test]
    fn very_large_buffer_does_not_overflow() {
        let tracker = LatencyTracker::new();
        tracker.report(usize::MAX, 1);
        assert!(tracker.buffered_ms() > 0);
    }
}
