use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::emitter::Emitter;
use crate::error::NegotiationError;
use crate::receiver::Receiver;
use crate::signal::Channel;

#[derive(Debug, Clone)]
pub struct ChannelQuality {
    pub snr: f32,
    pub latency_us: u32,
    pub packet_loss: f32,
    pub available: bool,
}

impl Default for ChannelQuality {
    fn default() -> Self {
        Self {
            snr: 0.0,
            latency_us: u32::MAX,
            packet_loss: 1.0,
            available: false,
        }
    }
}

impl ChannelQuality {
    pub fn score(&self) -> f32 {
        if !self.available {
            return 0.0;
        }
        let snr_score = self.snr.clamp(0.0, 100.0) / 100.0;
        let latency_score = 1.0 - (self.latency_us as f32 / 1_000_000.0).clamp(0.0, 1.0);
        let loss_score = 1.0 - self.packet_loss.clamp(0.0, 1.0);

        (snr_score * 0.4) + (latency_score * 0.3) + (loss_score * 0.3)
    }
}

pub struct ChannelNegotiator {
    qualities: Arc<RwLock<HashMap<Channel, ChannelQuality>>>,
    priority: Vec<Channel>,
    min_snr: f32,
    max_latency_us: u32,
}

impl Default for ChannelNegotiator {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelNegotiator {
    pub fn new() -> Self {
        Self {
            qualities: Arc::new(RwLock::new(HashMap::new())),
            priority: vec![
                Channel::Ble,
                Channel::Radio,
                Channel::Light,
                Channel::Audio,
                Channel::Vibration,
            ],
            min_snr: 10.0,
            max_latency_us: 100_000,
        }
    }

    pub fn with_priority(mut self, priority: Vec<Channel>) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_min_snr(mut self, min_snr: f32) -> Self {
        self.min_snr = min_snr;
        self
    }

    pub fn with_max_latency(mut self, max_latency_us: u32) -> Self {
        self.max_latency_us = max_latency_us;
        self
    }

    pub async fn update_quality(&self, channel: Channel, quality: ChannelQuality) {
        let mut qualities = self.qualities.write().await;
        debug!(channel = ?channel, snr = quality.snr, "Updated channel quality");
        qualities.insert(channel, quality);
    }

    pub async fn mark_unavailable(&self, channel: Channel) {
        let mut qualities = self.qualities.write().await;
        if let Some(q) = qualities.get_mut(&channel) {
            q.available = false;
            warn!(channel = ?channel, "Marked channel as unavailable");
        }
    }

    pub async fn mark_available(&self, channel: Channel, quality: ChannelQuality) {
        let mut qualities = self.qualities.write().await;
        qualities.insert(channel, ChannelQuality { available: true, ..quality });
    }

    pub async fn best_channel(&self) -> Result<Channel, NegotiationError> {
        let qualities = self.qualities.read().await;

        let mut best: Option<(Channel, f32)> = None;

        for channel in &self.priority {
            if let Some(quality) = qualities.get(channel) {
                if !quality.available {
                    continue;
                }
                if quality.snr < self.min_snr {
                    continue;
                }
                if quality.latency_us > self.max_latency_us {
                    continue;
                }

                let score = quality.score();
                if best.as_ref().map_or(true, |&(_, best_score)| score > best_score) {
                    best = Some((channel.clone(), score));
                }
            }
        }

        best.map(|(c, _)| c)
            .ok_or(NegotiationError::NoChannelsAvailable)
    }

    pub async fn select_with_fallback(
        &self,
        preferred: Channel,
    ) -> Result<Channel, NegotiationError> {
        let qualities = self.qualities.read().await;

        if let Some(quality) = qualities.get(&preferred) {
            if quality.available && quality.snr >= self.min_snr {
                return Ok(preferred);
            }
        }

        drop(qualities);
        self.best_channel().await
    }

    pub async fn probe_channel<E: Emitter, R: Receiver>(
        &self,
        emitter: &E,
        _receiver: &R,
    ) -> ChannelQuality {
        let channel = emitter.channel();
        let start = std::time::Instant::now();

        let test_pattern = crate::signal::SignalPattern::new(vec![
            crate::signal::Pulse::on(100),
            crate::signal::Pulse::off(100),
        ]);

        match emitter.emit(&test_pattern).await {
            Ok(_) => {
                let latency = start.elapsed().as_micros() as u32;
                ChannelQuality {
                    snr: 50.0, // Would need actual hardware measurement
                    latency_us: latency,
                    packet_loss: 0.0,
                    available: true,
                }
            }
            Err(_) => {
                warn!(channel = ?channel, "Channel probe failed");
                ChannelQuality::default()
            }
        }
    }

    pub async fn available_channels(&self) -> Vec<Channel> {
        let qualities = self.qualities.read().await;
        qualities
            .iter()
            .filter(|(_, q)| q.available)
            .map(|(c, _)| c.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_quality_score() {
        let quality = ChannelQuality {
            snr: 50.0,
            latency_us: 10_000,
            packet_loss: 0.1,
            available: true,
        };
        let score = quality.score();
        assert!(score > 0.0 && score < 1.0);
    }

    #[tokio::test]
    async fn test_best_channel_selection() {
        let negotiator = ChannelNegotiator::new();

        negotiator
            .update_quality(
                Channel::Light,
                ChannelQuality {
                    snr: 30.0,
                    latency_us: 5000,
                    packet_loss: 0.05,
                    available: true,
                },
            )
            .await;

        negotiator
            .update_quality(
                Channel::Ble,
                ChannelQuality {
                    snr: 60.0,
                    latency_us: 2000,
                    packet_loss: 0.01,
                    available: true,
                },
            )
            .await;

        let best = negotiator.best_channel().await.unwrap();
        assert_eq!(best, Channel::Ble);
    }

    #[tokio::test]
    async fn test_fallback_on_unavailable() {
        let negotiator = ChannelNegotiator::new();

        negotiator
            .update_quality(
                Channel::Light,
                ChannelQuality {
                    snr: 30.0,
                    latency_us: 5000,
                    packet_loss: 0.05,
                    available: true,
                },
            )
            .await;

        let result = negotiator.select_with_fallback(Channel::Ble).await.unwrap();
        assert_eq!(result, Channel::Light);
    }

    #[tokio::test]
    async fn test_no_channels_available() {
        let negotiator = ChannelNegotiator::new();
        let result = negotiator.best_channel().await;
        assert!(matches!(result, Err(NegotiationError::NoChannelsAvailable)));
    }
}
