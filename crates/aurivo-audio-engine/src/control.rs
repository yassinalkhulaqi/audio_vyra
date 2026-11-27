//! Control-change mapping for external controllers.
//!
//! A binding maps an incoming `(source, channel, controller)` triple to an
//! engine parameter. Values are normalized to 0..1 by the sender and scaled
//! into the target's min/max range. The module deliberately has no MIDI
//! dependency: a future `midir` integration only has to feed
//! [`ControlMap::apply`] with messages.

use serde::{Deserialize, Serialize};

pub const MAX_BINDINGS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlSource {
    pub channel: u8,
    pub controller: u8,
}

impl ControlSource {
    pub fn new(channel: u8, controller: u8) -> Self {
        Self { channel: channel.min(15), controller: controller.min(127) }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlMessage {
    pub source: ControlSource,
    pub value: f32,
}

impl ControlMessage {
    pub fn new(channel: u8, controller: u8, value: f32) -> Self {
        Self {
            source: ControlSource::new(channel, controller),
            value: if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlBinding {
    pub channel: u8,
    pub controller: u8,
    pub target: String,
    pub min: f32,
    pub max: f32,
    pub enabled: bool,
}

impl ControlBinding {
    pub fn new(channel: u8, controller: u8, target: &str, min: f32, max: f32) -> Self {
        let (min, max) = if min.is_finite() && max.is_finite() && max > min {
            (min, max)
        } else {
            (0.0, 1.0)
        };
        Self {
            channel: channel.min(15),
            controller: controller.min(127),
            target: target.to_string(),
            min,
            max,
            enabled: true,
        }
    }

    pub fn matches(&self, source: ControlSource) -> bool {
        self.channel == source.channel && self.controller == source.controller
    }

    pub fn scale(&self, value: f32) -> f32 {
        let value = if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 };
        self.min + (self.max - self.min) * value
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ControlMap {
    bindings: Vec<ControlBinding>,
}

impl ControlMap {
    pub fn new() -> Self { Self::default() }

    pub fn add(&mut self, binding: ControlBinding) -> bool {
        if self.bindings.len() >= MAX_BINDINGS {
            return false;
        }
        let duplicate = self.bindings.iter().any(|b| {
            b.channel == binding.channel
                && b.controller == binding.controller
                && b.target == binding.target
        });
        if duplicate {
            return false;
        }
        self.bindings.push(binding);
        true
    }

    pub fn remove_exact(&mut self, channel: u8, controller: u8) -> bool {
        let before = self.bindings.len();
        self.bindings.retain(|b| !(b.channel == channel && b.controller == controller));
        self.bindings.len() != before
    }

    pub fn remove_target(&mut self, target: &str) -> bool {
        let before = self.bindings.len();
        self.bindings.retain(|b| b.target != target);
        self.bindings.len() != before
    }

    pub fn clear(&mut self) { self.bindings.clear(); }

    pub fn len(&self) -> usize { self.bindings.len() }
    pub fn is_empty(&self) -> bool { self.bindings.is_empty() }
    pub fn bindings(&self) -> &[ControlBinding] { &self.bindings }

    pub fn set_enabled(&mut self, channel: u8, controller: u8, enabled: bool) -> bool {
        if let Some(binding) = self.bindings.iter_mut().find(|b| b.channel == channel && b.controller == controller) {
            binding.enabled = enabled;
            true
        } else {
            false
        }
    }

    pub fn apply(&self, message: &ControlMessage) -> Vec<(String, f32)> {
        self.bindings
            .iter()
            .filter(|b| b.enabled && b.matches(message.source))
            .map(|b| (b.target.clone(), b.scale(message.value)))
            .collect()
    }

    pub fn find(&self, channel: u8, controller: u8) -> Option<&ControlBinding> {
        self.bindings.iter().find(|b| b.channel == channel && b.controller == controller)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_values_are_clamped() {
        let source = ControlSource::new(99, 250);
        assert_eq!(source.channel, 15);
        assert_eq!(source.controller, 127);
    }

    #[test]
    fn binding_scales_to_range() {
        let binding = ControlBinding::new(0, 7, "gain_db", -24.0, 24.0);
        assert!((binding.scale(0.0) + 24.0).abs() < 1e-6);
        assert!((binding.scale(0.5)).abs() < 1e-6);
        assert!((binding.scale(1.0) - 24.0).abs() < 1e-6);
    }

    #[test]
    fn invalid_ranges_fall_back_to_unit() {
        let binding = ControlBinding::new(0, 1, "x", 10.0, 0.0);
        assert!((binding.min - 0.0).abs() < 1e-6);
        assert!((binding.max - 1.0).abs() < 1e-6);
    }

    #[test]
    fn apply_returns_matching_bindings_only() {
        let mut map = ControlMap::new();
        map.add(ControlBinding::new(0, 1, "gain_db", -12.0, 12.0));
        map.add(ControlBinding::new(0, 2, "pitch", -24.0, 24.0));
        let msg = ControlMessage::new(0, 1, 1.0);
        let applied = map.apply(&msg);
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].0, "gain_db");
        assert!((applied[0].1 - 12.0).abs() < 1e-6);
    }

    #[test]
    fn disabled_bindings_are_ignored() {
        let mut map = ControlMap::new();
        map.add(ControlBinding::new(0, 1, "gain_db", -12.0, 12.0));
        map.set_enabled(0, 1, false);
        assert!(map.apply(&ControlMessage::new(0, 1, 0.5)).is_empty());
    }

    #[test]
    fn one_controller_can_drive_multiple_targets() {
        let mut map = ControlMap::new();
        map.add(ControlBinding::new(0, 1, "gain_db", -12.0, 12.0));
        map.add(ControlBinding::new(0, 1, "pitch", -24.0, 24.0));
        assert_eq!(map.len(), 2);
        assert_eq!(map.apply(&ControlMessage::new(0, 1, 0.5)).len(), 2);
        assert!(!map.add(ControlBinding::new(0, 1, "pitch", -24.0, 24.0)));
    }

    #[test]
    fn map_is_bounded() {
        let mut map = ControlMap::new();
        for i in 0..MAX_BINDINGS + 10 {
            map.add(ControlBinding::new(0, i as u8, "gain_db", -12.0, 12.0));
        }
        assert_eq!(map.len(), MAX_BINDINGS);
    }

    #[test]
    fn removal_by_target() {
        let mut map = ControlMap::new();
        map.add(ControlBinding::new(0, 1, "gain_db", -12.0, 12.0));
        map.add(ControlBinding::new(0, 2, "gain_db", -12.0, 12.0));
        assert!(map.remove_target("gain_db"));
        assert!(map.is_empty());
    }

    #[test]
    fn non_finite_messages_are_zero() {
        let msg = ControlMessage::new(0, 1, f32::NAN);
        assert_eq!(msg.value, 0.0);
    }
}
