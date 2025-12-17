use cortex_core::SymbolId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Channel {
    Light,
    Audio,
    Ble,
    Vibration,
    Radio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pulse {
    pub on: bool,
    pub duration_us: u32,
}

impl Pulse {
    pub fn new(on: bool, duration_us: u32) -> Self {
        Self { on, duration_us }
    }

    pub fn on(duration_us: u32) -> Self {
        Self::new(true, duration_us)
    }

    pub fn off(duration_us: u32) -> Self {
        Self::new(false, duration_us)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalPattern {
    pub pulses: Vec<Pulse>,
}

impl SignalPattern {
    pub fn new(pulses: Vec<Pulse>) -> Self {
        Self { pulses }
    }

    pub fn empty() -> Self {
        Self { pulses: Vec::new() }
    }

    pub fn total_duration_us(&self) -> u64 {
        self.pulses.iter().map(|p| p.duration_us as u64).sum()
    }

    pub fn pulse_count(&self) -> usize {
        self.pulses.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signal {
    pub symbol: SymbolId,
    pub pattern: SignalPattern,
    pub channel: Channel,
}

impl Signal {
    pub fn new(symbol: SymbolId, pattern: SignalPattern, channel: Channel) -> Self {
        Self {
            symbol,
            pattern,
            channel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pulse_creation() {
        let on_pulse = Pulse::on(1000);
        assert!(on_pulse.on);
        assert_eq!(on_pulse.duration_us, 1000);

        let off_pulse = Pulse::off(500);
        assert!(!off_pulse.on);
        assert_eq!(off_pulse.duration_us, 500);
    }

    #[test]
    fn test_pattern_duration() {
        let pattern = SignalPattern::new(vec![
            Pulse::on(1000),
            Pulse::off(500),
            Pulse::on(1000),
        ]);
        assert_eq!(pattern.total_duration_us(), 2500);
        assert_eq!(pattern.pulse_count(), 3);
    }
}
