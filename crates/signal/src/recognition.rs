/// Signal Recognition Module
///
/// Provides pattern matching and signal recognition capabilities for
/// evolved and learned signal patterns.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use cortex_core::SymbolId;

use crate::error::SignalError;
use crate::signal::SignalPattern;

/// Confidence level for a recognition match
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MatchConfidence {
    /// Confidence score (0.0 to 1.0)
    pub score: f32,
    /// Distance metric (lower is better)
    pub distance: f32,
}

impl MatchConfidence {
    pub fn new(score: f32, distance: f32) -> Self {
        Self { score, distance }
    }

    pub fn is_strong(&self, threshold: f32) -> bool {
        self.score >= threshold
    }
}

/// A recognized signal with its matched symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognizedSignal {
    pub symbol: SymbolId,
    pub pattern: SignalPattern,
    pub confidence: MatchConfidence,
    pub timestamp_us: u64,
}

impl RecognizedSignal {
    pub fn new(
        symbol: SymbolId,
        pattern: SignalPattern,
        confidence: MatchConfidence,
    ) -> Self {
        Self {
            symbol,
            pattern,
            confidence,
            timestamp_us: 0, // Would be set by caller with actual timestamp
        }
    }

    pub fn with_timestamp(mut self, timestamp_us: u64) -> Self {
        self.timestamp_us = timestamp_us;
        self
    }
}

/// Configuration for signal recognition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognitionConfig {
    /// Minimum confidence threshold for recognition
    pub min_confidence: f32,
    /// Maximum pattern distance to consider a match
    pub max_distance: f32,
    /// Enable fuzzy matching
    pub fuzzy_matching: bool,
    /// Tolerance for duration variations (percentage)
    pub duration_tolerance: f32,
}

impl Default for RecognitionConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            max_distance: 0.3,
            fuzzy_matching: true,
            duration_tolerance: 0.15,
        }
    }
}

/// Template for a learned signal pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalTemplate {
    pub symbol: SymbolId,
    pub pattern: SignalPattern,
    pub usage_count: u32,
    pub success_rate: f32,
}

impl SignalTemplate {
    pub fn new(symbol: SymbolId, pattern: SignalPattern) -> Self {
        Self {
            symbol,
            pattern,
            usage_count: 0,
            success_rate: 0.0,
        }
    }

    pub fn record_usage(&mut self, success: bool) {
        self.usage_count += 1;
        // Calculate weighted average: (previous_total_successes + current) / total_count
        let prev_total_successes = self.success_rate * (self.usage_count - 1) as f32;
        let current_success = if success { 1.0 } else { 0.0 };
        self.success_rate = (prev_total_successes + current_success) / self.usage_count as f32;
    }
}

/// The main signal recognition engine
pub struct RecognitionEngine {
    config: RecognitionConfig,
    templates: Arc<RwLock<HashMap<SymbolId, Vec<SignalTemplate>>>>,
}

impl RecognitionEngine {
    pub fn new(config: RecognitionConfig) -> Self {
        Self {
            config,
            templates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(RecognitionConfig::default())
    }

    /// Register a new signal template
    pub async fn register_template(
        &self,
        symbol: SymbolId,
        pattern: SignalPattern,
    ) -> Result<(), SignalError> {
        let mut templates = self.templates.write().await;

        let template = SignalTemplate::new(symbol, pattern);

        templates
            .entry(symbol)
            .or_insert_with(Vec::new)
            .push(template);

        debug!(symbol = ?symbol, "Registered new signal template");

        Ok(())
    }

    /// Recognize a signal pattern
    pub async fn recognize(
        &self,
        pattern: &SignalPattern,
    ) -> Result<RecognizedSignal, SignalError> {
        let templates = self.templates.read().await;

        let mut best_match: Option<(SymbolId, MatchConfidence, SignalPattern)> = None;

        for (symbol, template_list) in templates.iter() {
            for template in template_list {
                let confidence = self.calculate_match(pattern, &template.pattern);

                if confidence.score >= self.config.min_confidence
                    && confidence.distance <= self.config.max_distance
                {
                    if best_match.is_none()
                        || confidence.score > best_match.as_ref().unwrap().1.score
                    {
                        best_match = Some((*symbol, confidence, template.pattern.clone()));
                    }
                }
            }
        }

        if let Some((symbol, confidence, matched_pattern)) = best_match {
            debug!(
                symbol = ?symbol,
                confidence = confidence.score,
                "Recognized signal pattern"
            );

            Ok(RecognizedSignal::new(symbol, matched_pattern, confidence))
        } else {
            warn!("No matching signal template found");
            Err(SignalError::InvalidPattern(
                "no matching template found".into(),
            ))
        }
    }

    /// Calculate match confidence between two patterns
    fn calculate_match(&self, pattern1: &SignalPattern, pattern2: &SignalPattern) -> MatchConfidence {
        let distance = self.pattern_distance(pattern1, pattern2);
        let score = 1.0 - distance.clamp(0.0, 1.0);

        MatchConfidence::new(score, distance)
    }

    /// Calculate normalized distance between two patterns
    fn pattern_distance(&self, p1: &SignalPattern, p2: &SignalPattern) -> f32 {
        // Length difference component
        let len_diff = (p1.pulses.len() as i32 - p2.pulses.len() as i32).abs() as f32;
        let max_len = p1.pulses.len().max(p2.pulses.len()).max(1) as f32;
        let len_distance = len_diff / max_len;

        // Pulse-wise comparison
        let mut pulse_distance = 0.0;
        let min_len = p1.pulses.len().min(p2.pulses.len());

        for i in 0..min_len {
            let pulse1 = &p1.pulses[i];
            let pulse2 = &p2.pulses[i];

            // On/off state difference
            let state_diff = if pulse1.on != pulse2.on { 1.0 } else { 0.0 };

            // Duration difference with tolerance
            let duration_diff = if self.config.fuzzy_matching {
                let tolerance = pulse2.duration_us as f32 * self.config.duration_tolerance;
                let diff = (pulse1.duration_us as i32 - pulse2.duration_us as i32).abs() as f32;
                if diff <= tolerance {
                    0.0
                } else {
                    (diff - tolerance) / pulse2.duration_us as f32
                }
            } else {
                (pulse1.duration_us as f32 - pulse2.duration_us as f32).abs()
                    / pulse2.duration_us.max(1) as f32
            };

            pulse_distance += (state_diff + duration_diff.min(1.0)) / 2.0;
        }

        // Normalize pulse distance
        let avg_pulse_distance = if min_len > 0 {
            pulse_distance / min_len as f32
        } else {
            1.0
        };

        // Combine length and pulse distances
        (len_distance * 0.3 + avg_pulse_distance * 0.7).clamp(0.0, 1.0)
    }

    /// Update template statistics after usage
    pub async fn record_usage(
        &self,
        symbol: SymbolId,
        pattern: &SignalPattern,
        success: bool,
    ) -> Result<(), SignalError> {
        let mut templates = self.templates.write().await;

        if let Some(template_list) = templates.get_mut(&symbol) {
            // Find the matching template
            for template in template_list.iter_mut() {
                if self.patterns_match(&template.pattern, pattern) {
                    template.record_usage(success);
                    debug!(
                        symbol = ?symbol,
                        success_rate = template.success_rate,
                        usage_count = template.usage_count,
                        "Updated template statistics"
                    );
                    return Ok(());
                }
            }
        }

        Err(SignalError::UnknownSymbol(format!("{:?}", symbol)))
    }

    /// Check if two patterns match closely
    fn patterns_match(&self, p1: &SignalPattern, p2: &SignalPattern) -> bool {
        let distance = self.pattern_distance(p1, p2);
        distance <= self.config.max_distance
    }

    /// Get all templates for a symbol
    pub async fn get_templates(&self, symbol: SymbolId) -> Vec<SignalTemplate> {
        let templates = self.templates.read().await;
        templates.get(&symbol).cloned().unwrap_or_default()
    }

    /// Get the best template for a symbol (highest success rate)
    pub async fn get_best_template(&self, symbol: SymbolId) -> Result<SignalTemplate, SignalError> {
        let templates = self.templates.read().await;

        templates
            .get(&symbol)
            .and_then(|list| {
                list.iter()
                    .max_by(|a, b| {
                        a.success_rate
                            .partial_cmp(&b.success_rate)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .cloned()
            })
            .ok_or_else(|| SignalError::UnknownSymbol(format!("{:?}", symbol)))
    }

    /// Remove poor performing templates
    pub async fn prune_templates(&self, min_success_rate: f32, min_usage_count: u32) {
        let mut templates = self.templates.write().await;

        for (symbol, template_list) in templates.iter_mut() {
            template_list.retain(|t| {
                t.usage_count < min_usage_count || t.success_rate >= min_success_rate
            });
            debug!(
                symbol = ?symbol,
                remaining = template_list.len(),
                "Pruned low-performing templates"
            );
        }
    }

    /// Get total number of registered templates
    pub async fn template_count(&self) -> usize {
        let templates = self.templates.read().await;
        templates.values().map(|v| v.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::Pulse;

    #[tokio::test]
    async fn test_template_registration() {
        let engine = RecognitionEngine::with_default_config();
        let symbol = SymbolId::from_bytes(b"TEST_SIGNAL");
        let pattern = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);

        engine.register_template(symbol, pattern.clone()).await.unwrap();

        let templates = engine.get_templates(symbol).await;
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].pattern, pattern);
    }

    #[tokio::test]
    async fn test_exact_recognition() {
        let engine = RecognitionEngine::with_default_config();
        let symbol = SymbolId::from_bytes(b"TEST_SIGNAL");
        let pattern = SignalPattern::new(vec![
            Pulse::on(100),
            Pulse::off(100),
            Pulse::on(100),
        ]);

        engine.register_template(symbol, pattern.clone()).await.unwrap();

        let recognized = engine.recognize(&pattern).await.unwrap();
        assert_eq!(recognized.symbol, symbol);
        assert!(recognized.confidence.score > 0.95);
    }

    #[tokio::test]
    async fn test_fuzzy_recognition() {
        let mut config = RecognitionConfig::default();
        config.min_confidence = 0.6;
        let engine = RecognitionEngine::new(config);

        let symbol = SymbolId::from_bytes(b"TEST_SIGNAL");
        let template_pattern = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);
        let similar_pattern = SignalPattern::new(vec![Pulse::on(110), Pulse::off(90)]);

        engine.register_template(symbol, template_pattern).await.unwrap();

        let recognized = engine.recognize(&similar_pattern).await.unwrap();
        assert_eq!(recognized.symbol, symbol);
        assert!(recognized.confidence.score > 0.6);
    }

    #[tokio::test]
    async fn test_no_match() {
        let engine = RecognitionEngine::with_default_config();
        let symbol = SymbolId::from_bytes(b"TEST_SIGNAL");
        let template = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);
        let different = SignalPattern::new(vec![
            Pulse::off(500),
            Pulse::on(500),
            Pulse::off(500),
        ]);

        engine.register_template(symbol, template).await.unwrap();

        let result = engine.recognize(&different).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_usage_tracking() {
        let engine = RecognitionEngine::with_default_config();
        let symbol = SymbolId::from_bytes(b"TEST_SIGNAL");
        let pattern = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);

        engine.register_template(symbol, pattern.clone()).await.unwrap();

        // Record successful usage
        engine.record_usage(symbol, &pattern, true).await.unwrap();
        engine.record_usage(symbol, &pattern, true).await.unwrap();
        engine.record_usage(symbol, &pattern, false).await.unwrap();

        let template = engine.get_best_template(symbol).await.unwrap();
        assert_eq!(template.usage_count, 3);
        assert!((template.success_rate - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_template_pruning() {
        let engine = RecognitionEngine::with_default_config();
        let symbol = SymbolId::from_bytes(b"TEST_SIGNAL");

        // Add two templates with very different patterns
        let good_pattern = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);
        let bad_pattern = SignalPattern::new(vec![
            Pulse::off(500),
            Pulse::on(500),
            Pulse::off(500),
            Pulse::on(500),
        ]);

        engine.register_template(symbol, good_pattern.clone()).await.unwrap();
        engine.register_template(symbol, bad_pattern.clone()).await.unwrap();

        // Record usage - good pattern succeeds, bad pattern fails
        engine.record_usage(symbol, &good_pattern, true).await.unwrap();
        engine.record_usage(symbol, &good_pattern, true).await.unwrap();
        engine.record_usage(symbol, &bad_pattern, false).await.unwrap();
        engine.record_usage(symbol, &bad_pattern, false).await.unwrap();

        // Prune - should remove bad pattern (success_rate=0.0 < 0.5)
        engine.prune_templates(0.5, 2).await;

        let templates = engine.get_templates(symbol).await;
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].pattern, good_pattern);
    }

    #[test]
    fn test_match_confidence() {
        let confidence = MatchConfidence::new(0.8, 0.2);
        assert!(confidence.is_strong(0.7));
        assert!(!confidence.is_strong(0.9));
    }

    #[test]
    fn test_pattern_distance_calculation() {
        let engine = RecognitionEngine::with_default_config();

        let p1 = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);
        let p2 = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);
        let p3 = SignalPattern::new(vec![Pulse::off(200), Pulse::on(200)]);

        let dist_same = engine.pattern_distance(&p1, &p2);
        let dist_diff = engine.pattern_distance(&p1, &p3);

        assert!(dist_same < 0.1);
        assert!(dist_diff > dist_same);
    }
}
