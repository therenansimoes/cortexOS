use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

use crate::codebook::Codebook;
use crate::error::{DecodeError, ReceiveError};
use crate::signal::{Channel, Signal, SignalPattern};

#[async_trait]
pub trait Receiver: Send + Sync {
    fn channel(&self) -> Channel;
    async fn receive(&self) -> Result<SignalPattern, ReceiveError>;
    async fn decode(&self, codebook: &Codebook) -> Result<Signal, DecodeError>;
}

pub struct MockReceiver {
    channel: Channel,
    patterns: Arc<Mutex<Vec<SignalPattern>>>,
    should_fail: bool,
}

impl MockReceiver {
    pub fn new(channel: Channel) -> Self {
        Self {
            channel,
            patterns: Arc::new(Mutex::new(Vec::new())),
            should_fail: false,
        }
    }

    pub fn failing(channel: Channel) -> Self {
        Self {
            channel,
            patterns: Arc::new(Mutex::new(Vec::new())),
            should_fail: true,
        }
    }

    pub async fn queue_pattern(&self, pattern: SignalPattern) {
        self.patterns.lock().await.push(pattern);
    }

    pub async fn queue_patterns(&self, patterns: Vec<SignalPattern>) {
        let mut queue = self.patterns.lock().await;
        for p in patterns {
            queue.push(p);
        }
    }

    pub async fn pending_count(&self) -> usize {
        self.patterns.lock().await.len()
    }
}

#[async_trait]
impl Receiver for MockReceiver {
    fn channel(&self) -> Channel {
        self.channel.clone()
    }

    async fn receive(&self) -> Result<SignalPattern, ReceiveError> {
        if self.should_fail {
            return Err(ReceiveError::HardwareError("mock failure".into()));
        }

        let mut patterns = self.patterns.lock().await;
        if patterns.is_empty() {
            return Err(ReceiveError::Timeout);
        }

        let pattern = patterns.remove(0);
        debug!(
            channel = ?self.channel,
            pulses = pattern.pulse_count(),
            "MockReceiver: received pattern"
        );
        Ok(pattern)
    }

    async fn decode(&self, codebook: &Codebook) -> Result<Signal, DecodeError> {
        let pattern = self.receive().await?;
        let symbol = codebook.decode(&pattern)?;

        Ok(Signal::new(symbol, pattern, self.channel.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebook::StandardSymbol;
    use crate::signal::Pulse;

    #[tokio::test]
    async fn test_mock_receiver() {
        let receiver = MockReceiver::new(Channel::Light);
        let pattern = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);

        receiver.queue_pattern(pattern.clone()).await;
        assert_eq!(receiver.pending_count().await, 1);

        let received = receiver.receive().await.unwrap();
        assert_eq!(received, pattern);
        assert_eq!(receiver.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_receiver_timeout() {
        let receiver = MockReceiver::new(Channel::Audio);
        let result = receiver.receive().await;
        assert!(matches!(result, Err(ReceiveError::Timeout)));
    }

    #[tokio::test]
    async fn test_decode_signal() {
        let codebook = Codebook::new();
        let receiver = MockReceiver::new(Channel::Ble);

        let ack_pattern = codebook.encode(StandardSymbol::Ack.to_symbol_id()).unwrap();
        receiver.queue_pattern(ack_pattern.clone()).await;

        let signal = receiver.decode(&codebook).await.unwrap();
        assert_eq!(signal.symbol, StandardSymbol::Ack.to_symbol_id());
        assert_eq!(signal.channel, Channel::Ble);
    }
}
