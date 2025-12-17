use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::codebook::Codebook;
use crate::error::EmitError;
use crate::signal::{Channel, Signal, SignalPattern};

#[async_trait]
pub trait Emitter: Send + Sync {
    fn channel(&self) -> Channel;
    async fn emit(&self, pattern: &SignalPattern) -> Result<(), EmitError>;
    async fn emit_signal(&self, signal: &Signal, codebook: &Codebook) -> Result<(), EmitError>;
}

pub struct MockEmitter {
    channel: Channel,
    emit_count: AtomicUsize,
    emitted_patterns: Arc<Mutex<Vec<SignalPattern>>>,
    should_fail: bool,
}

impl MockEmitter {
    pub fn new(channel: Channel) -> Self {
        Self {
            channel,
            emit_count: AtomicUsize::new(0),
            emitted_patterns: Arc::new(Mutex::new(Vec::new())),
            should_fail: false,
        }
    }

    pub fn failing(channel: Channel) -> Self {
        Self {
            channel,
            emit_count: AtomicUsize::new(0),
            emitted_patterns: Arc::new(Mutex::new(Vec::new())),
            should_fail: true,
        }
    }

    pub fn emit_count(&self) -> usize {
        self.emit_count.load(Ordering::SeqCst)
    }

    pub async fn emitted_patterns(&self) -> Vec<SignalPattern> {
        self.emitted_patterns.lock().await.clone()
    }
}

#[async_trait]
impl Emitter for MockEmitter {
    fn channel(&self) -> Channel {
        self.channel.clone()
    }

    async fn emit(&self, pattern: &SignalPattern) -> Result<(), EmitError> {
        if self.should_fail {
            return Err(EmitError::HardwareError("mock failure".into()));
        }

        self.emit_count.fetch_add(1, Ordering::SeqCst);
        self.emitted_patterns.lock().await.push(pattern.clone());
        debug!(
            channel = ?self.channel,
            pulses = pattern.pulse_count(),
            "MockEmitter: emitted pattern"
        );
        Ok(())
    }

    async fn emit_signal(&self, signal: &Signal, codebook: &Codebook) -> Result<(), EmitError> {
        let pattern = codebook.encode(signal.symbol)?;
        self.emit(pattern).await
    }
}

pub struct ConsoleEmitter {
    channel: Channel,
    name: String,
}

impl ConsoleEmitter {
    pub fn new(channel: Channel, name: impl Into<String>) -> Self {
        Self {
            channel,
            name: name.into(),
        }
    }
}

#[async_trait]
impl Emitter for ConsoleEmitter {
    fn channel(&self) -> Channel {
        self.channel.clone()
    }

    async fn emit(&self, pattern: &SignalPattern) -> Result<(), EmitError> {
        let visual: String = pattern
            .pulses
            .iter()
            .map(|p| {
                let symbol = if p.on { '█' } else { '░' };
                let width = (p.duration_us / 100).max(1) as usize;
                std::iter::repeat(symbol).take(width).collect::<String>()
            })
            .collect();

        info!(
            emitter = %self.name,
            channel = ?self.channel,
            duration_us = pattern.total_duration_us(),
            "[{}] {}",
            self.name,
            visual
        );

        println!(
            "[{:?}] {} | {} ({}µs)",
            self.channel,
            self.name,
            visual,
            pattern.total_duration_us()
        );

        Ok(())
    }

    async fn emit_signal(&self, signal: &Signal, codebook: &Codebook) -> Result<(), EmitError> {
        let pattern = codebook.encode(signal.symbol)?;
        println!(
            "[{:?}] {} emitting symbol: {:?}",
            self.channel, self.name, signal.symbol
        );
        self.emit(pattern).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::Pulse;

    #[tokio::test]
    async fn test_mock_emitter() {
        let emitter = MockEmitter::new(Channel::Light);
        let pattern = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);

        emitter.emit(&pattern).await.unwrap();
        assert_eq!(emitter.emit_count(), 1);

        let emitted = emitter.emitted_patterns().await;
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0], pattern);
    }

    #[tokio::test]
    async fn test_failing_emitter() {
        let emitter = MockEmitter::failing(Channel::Audio);
        let pattern = SignalPattern::new(vec![Pulse::on(100)]);

        let result = emitter.emit(&pattern).await;
        assert!(result.is_err());
    }
}
