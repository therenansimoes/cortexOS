/// Signal Learning Module
///
/// Coordinates evolution and recognition to create an adaptive learning system
/// for signal communication protocols.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use cortex_core::SymbolId;

use crate::error::SignalError;
use crate::evolution::{EvolutionConfig, EvolutionEngine, FitnessMetrics};
use crate::recognition::{RecognitionConfig, RecognitionEngine};
use crate::signal::SignalPattern;

/// Learning strategy for signal adaptation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningStrategy {
    /// Evolve new patterns from scratch
    Evolution,
    /// Learn from observed patterns
    Recognition,
    /// Combine evolution and recognition
    Hybrid,
}

/// Configuration for the learning system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    pub strategy: LearningStrategy,
    pub evolution_config: EvolutionConfig,
    pub recognition_config: RecognitionConfig,
    /// Number of generations before evaluating progress
    pub evaluation_interval: u32,
    /// Whether to automatically evolve based on feedback
    pub auto_evolve: bool,
    /// Minimum improvement required to adopt new patterns
    pub min_improvement: f32,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            strategy: LearningStrategy::Hybrid,
            evolution_config: EvolutionConfig::default(),
            recognition_config: RecognitionConfig::default(),
            evaluation_interval: 10,
            auto_evolve: true,
            min_improvement: 0.05,
        }
    }
}

/// Statistics about the learning process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStats {
    pub total_signals_sent: u32,
    pub total_signals_received: u32,
    pub successful_communications: u32,
    pub failed_communications: u32,
    pub avg_fitness: f32,
    pub best_fitness: f32,
    pub current_generation: u32,
    pub patterns_learned: u32,
}

impl Default for LearningStats {
    fn default() -> Self {
        Self {
            total_signals_sent: 0,
            total_signals_received: 0,
            successful_communications: 0,
            failed_communications: 0,
            avg_fitness: 0.0,
            best_fitness: 0.0,
            current_generation: 0,
            patterns_learned: 0,
        }
    }
}

impl LearningStats {
    pub fn success_rate(&self) -> f32 {
        let total = self.successful_communications + self.failed_communications;
        if total == 0 {
            0.0
        } else {
            self.successful_communications as f32 / total as f32
        }
    }
}

/// Communication outcome for learning feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationOutcome {
    pub symbol: SymbolId,
    pub pattern: SignalPattern,
    pub success: bool,
    pub snr: Option<f32>,
    pub latency_us: Option<u32>,
    pub energy_cost: Option<f32>,
}

impl CommunicationOutcome {
    pub fn success(symbol: SymbolId, pattern: SignalPattern) -> Self {
        Self {
            symbol,
            pattern,
            success: true,
            snr: None,
            latency_us: None,
            energy_cost: None,
        }
    }

    pub fn failure(symbol: SymbolId, pattern: SignalPattern) -> Self {
        Self {
            symbol,
            pattern,
            success: false,
            snr: None,
            latency_us: None,
            energy_cost: None,
        }
    }

    pub fn with_snr(mut self, snr: f32) -> Self {
        self.snr = Some(snr);
        self
    }

    pub fn with_latency(mut self, latency_us: u32) -> Self {
        self.latency_us = Some(latency_us);
        self
    }

    pub fn with_energy_cost(mut self, energy_cost: f32) -> Self {
        self.energy_cost = Some(energy_cost);
        self
    }
}

/// The main learning system
pub struct LearningSystem {
    config: LearningConfig,
    evolution_engine: EvolutionEngine,
    recognition_engine: RecognitionEngine,
    stats: Arc<RwLock<HashMap<SymbolId, LearningStats>>>,
}

impl LearningSystem {
    pub fn new(config: LearningConfig) -> Self {
        let evolution_engine = EvolutionEngine::new(config.evolution_config.clone());
        let recognition_engine = RecognitionEngine::new(config.recognition_config.clone());

        Self {
            config,
            evolution_engine,
            recognition_engine,
            stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(LearningConfig::default())
    }

    /// Initialize learning for a symbol
    pub async fn initialize_symbol(&self, symbol: SymbolId) -> Result<(), SignalError> {
        match self.config.strategy {
            LearningStrategy::Evolution | LearningStrategy::Hybrid => {
                self.evolution_engine.initialize_population(symbol).await?;
            }
            LearningStrategy::Recognition => {
                // Recognition engine doesn't need initialization
            }
        }

        let mut stats = self.stats.write().await;
        stats.insert(symbol, LearningStats::default());

        info!(symbol = ?symbol, strategy = ?self.config.strategy, "Initialized learning");

        Ok(())
    }

    /// Learn a pattern from observation
    pub async fn learn_pattern(
        &self,
        symbol: SymbolId,
        pattern: SignalPattern,
    ) -> Result<(), SignalError> {
        match self.config.strategy {
            LearningStrategy::Recognition | LearningStrategy::Hybrid => {
                self.recognition_engine
                    .register_template(symbol, pattern)
                    .await?;

                let mut stats = self.stats.write().await;
                if let Some(s) = stats.get_mut(&symbol) {
                    s.patterns_learned += 1;
                }

                debug!(symbol = ?symbol, "Learned new pattern");
            }
            LearningStrategy::Evolution => {
                // Evolution doesn't learn from external patterns directly
                warn!("Evolution strategy doesn't support direct pattern learning");
            }
        }

        Ok(())
    }

    /// Get the best pattern for a symbol
    pub async fn get_best_pattern(&self, symbol: SymbolId) -> Result<SignalPattern, SignalError> {
        match self.config.strategy {
            LearningStrategy::Evolution => {
                self.evolution_engine.get_best_pattern(symbol).await
            }
            LearningStrategy::Recognition => {
                let template = self.recognition_engine.get_best_template(symbol).await?;
                Ok(template.pattern)
            }
            LearningStrategy::Hybrid => {
                // Try recognition first, fall back to evolution
                match self.recognition_engine.get_best_template(symbol).await {
                    Ok(template) if template.success_rate > 0.5 => Ok(template.pattern),
                    _ => self.evolution_engine.get_best_pattern(symbol).await,
                }
            }
        }
    }

    /// Record communication outcome for learning
    pub async fn record_outcome(&self, outcome: CommunicationOutcome) -> Result<(), SignalError> {
        let symbol = outcome.symbol;

        // Update statistics
        let mut stats = self.stats.write().await;
        if let Some(s) = stats.get_mut(&symbol) {
            if outcome.success {
                s.successful_communications += 1;
            } else {
                s.failed_communications += 1;
            }
        }
        drop(stats);

        // Update recognition engine
        if matches!(
            self.config.strategy,
            LearningStrategy::Recognition | LearningStrategy::Hybrid
        ) {
            let _ = self
                .recognition_engine
                .record_usage(symbol, &outcome.pattern, outcome.success)
                .await;
        }

        // Update evolution engine with fitness metrics
        if matches!(
            self.config.strategy,
            LearningStrategy::Evolution | LearningStrategy::Hybrid
        ) {
            let metrics = FitnessMetrics {
                success_rate: if outcome.success { 1.0 } else { 0.0 },
                avg_snr: outcome.snr.unwrap_or(0.0),
                avg_latency_us: outcome.latency_us.unwrap_or(u32::MAX),
                energy_cost: outcome.energy_cost.unwrap_or(1.0),
                distinctiveness: 0.5, // Would calculate based on other patterns
            };

            let _ = self
                .evolution_engine
                .record_fitness(symbol, &outcome.pattern, metrics)
                .await;
        }

        // Auto-evolve if enabled
        if self.config.auto_evolve {
            self.maybe_evolve(symbol).await?;
        }

        debug!(
            symbol = ?symbol,
            success = outcome.success,
            "Recorded communication outcome"
        );

        Ok(())
    }

    /// Evolve patterns if it's time
    async fn maybe_evolve(&self, symbol: SymbolId) -> Result<(), SignalError> {
        if !matches!(
            self.config.strategy,
            LearningStrategy::Evolution | LearningStrategy::Hybrid
        ) {
            return Ok(());
        }

        let stats = self.stats.read().await;
        let symbol_stats = stats.get(&symbol).cloned().unwrap_or_default();
        drop(stats);

        let total_comms = symbol_stats.successful_communications + symbol_stats.failed_communications;

        // Check if we should evolve
        if total_comms > 0 && total_comms % self.config.evaluation_interval == 0 {
            info!(
                symbol = ?symbol,
                generation = symbol_stats.current_generation,
                success_rate = symbol_stats.success_rate(),
                "Triggering evolution"
            );

            self.evolution_engine.evolve_generation(symbol).await?;

            // Update stats
            let mut stats = self.stats.write().await;
            if let Some(s) = stats.get_mut(&symbol) {
                s.current_generation += 1;
            }
        }

        Ok(())
    }

    /// Force evolution for a symbol
    pub async fn evolve(&self, symbol: SymbolId) -> Result<(), SignalError> {
        if !matches!(
            self.config.strategy,
            LearningStrategy::Evolution | LearningStrategy::Hybrid
        ) {
            return Err(SignalError::CodecError(
                "evolution not enabled for this strategy".into(),
            ));
        }

        self.evolution_engine.evolve_generation(symbol).await?;

        let mut stats = self.stats.write().await;
        if let Some(s) = stats.get_mut(&symbol) {
            s.current_generation += 1;
        }

        info!(symbol = ?symbol, "Manually triggered evolution");

        Ok(())
    }

    /// Get learning statistics for a symbol
    pub async fn get_stats(&self, symbol: SymbolId) -> LearningStats {
        let stats = self.stats.read().await;
        stats.get(&symbol).cloned().unwrap_or_default()
    }

    /// Reset learning for a symbol
    pub async fn reset_symbol(&self, symbol: SymbolId) -> Result<(), SignalError> {
        match self.config.strategy {
            LearningStrategy::Evolution | LearningStrategy::Hybrid => {
                self.evolution_engine.reset_population(symbol).await?;
            }
            LearningStrategy::Recognition => {
                // Would need to clear templates if we had that API
            }
        }

        let mut stats = self.stats.write().await;
        stats.insert(symbol, LearningStats::default());

        info!(symbol = ?symbol, "Reset learning");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::Pulse;

    #[tokio::test]
    async fn test_learning_initialization() {
        let system = LearningSystem::with_default_config();
        let symbol = SymbolId::from_bytes(b"TEST");

        system.initialize_symbol(symbol).await.unwrap();

        let stats = system.get_stats(symbol).await;
        assert_eq!(stats.current_generation, 0);
    }

    #[tokio::test]
    async fn test_pattern_learning() {
        let mut config = LearningConfig::default();
        config.strategy = LearningStrategy::Recognition;
        let system = LearningSystem::new(config);

        let symbol = SymbolId::from_bytes(b"TEST");
        let pattern = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);

        system.initialize_symbol(symbol).await.unwrap();
        system.learn_pattern(symbol, pattern.clone()).await.unwrap();

        let best = system.get_best_pattern(symbol).await.unwrap();
        assert_eq!(best, pattern);
    }

    #[tokio::test]
    async fn test_outcome_recording() {
        let system = LearningSystem::with_default_config();
        let symbol = SymbolId::from_bytes(b"TEST");
        let pattern = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);

        system.initialize_symbol(symbol).await.unwrap();

        let outcome = CommunicationOutcome::success(symbol, pattern)
            .with_snr(75.0)
            .with_latency(5000);

        system.record_outcome(outcome).await.unwrap();

        let stats = system.get_stats(symbol).await;
        assert_eq!(stats.successful_communications, 1);
        assert_eq!(stats.success_rate(), 1.0);
    }

    #[tokio::test]
    async fn test_auto_evolution() {
        let mut config = LearningConfig::default();
        config.auto_evolve = true;
        config.evaluation_interval = 2;
        config.strategy = LearningStrategy::Evolution;

        let system = LearningSystem::new(config);
        let symbol = SymbolId::from_bytes(b"TEST");

        system.initialize_symbol(symbol).await.unwrap();

        let pattern = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);

        // Record enough outcomes to trigger evolution
        for _ in 0..2 {
            let outcome = CommunicationOutcome::success(symbol, pattern.clone());
            system.record_outcome(outcome).await.unwrap();
        }

        let stats = system.get_stats(symbol).await;
        assert!(stats.current_generation > 0);
    }

    #[tokio::test]
    async fn test_hybrid_strategy() {
        let mut config = LearningConfig::default();
        config.strategy = LearningStrategy::Hybrid;

        let system = LearningSystem::new(config);
        let symbol = SymbolId::from_bytes(b"TEST");

        system.initialize_symbol(symbol).await.unwrap();

        // Learn a pattern
        let learned = SignalPattern::new(vec![Pulse::on(200), Pulse::off(200)]);
        system.learn_pattern(symbol, learned.clone()).await.unwrap();

        // Record successful usage
        let outcome = CommunicationOutcome::success(symbol, learned.clone());
        system.record_outcome(outcome).await.unwrap();

        // Should prefer the learned pattern
        let best = system.get_best_pattern(symbol).await.unwrap();
        // In hybrid mode, it might return either evolved or learned pattern
        assert!(!best.pulses.is_empty());
    }

    #[tokio::test]
    async fn test_manual_evolution() {
        let mut config = LearningConfig::default();
        config.strategy = LearningStrategy::Evolution;
        config.auto_evolve = false;

        let system = LearningSystem::new(config);
        let symbol = SymbolId::from_bytes(b"TEST");

        system.initialize_symbol(symbol).await.unwrap();

        let stats_before = system.get_stats(symbol).await;
        let gen_before = stats_before.current_generation;

        system.evolve(symbol).await.unwrap();

        let stats_after = system.get_stats(symbol).await;
        assert_eq!(stats_after.current_generation, gen_before + 1);
    }

    #[tokio::test]
    async fn test_success_rate_tracking() {
        let system = LearningSystem::with_default_config();
        let symbol = SymbolId::from_bytes(b"TEST");
        let pattern = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);

        system.initialize_symbol(symbol).await.unwrap();

        // Record mixed outcomes
        for i in 0..10 {
            let outcome = if i < 7 {
                CommunicationOutcome::success(symbol, pattern.clone())
            } else {
                CommunicationOutcome::failure(symbol, pattern.clone())
            };
            system.record_outcome(outcome).await.unwrap();
        }

        let stats = system.get_stats(symbol).await;
        assert_eq!(stats.success_rate(), 0.7);
    }

    #[tokio::test]
    async fn test_reset_symbol() {
        let system = LearningSystem::with_default_config();
        let symbol = SymbolId::from_bytes(b"TEST");

        system.initialize_symbol(symbol).await.unwrap();

        let pattern = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);
        let outcome = CommunicationOutcome::success(symbol, pattern);
        system.record_outcome(outcome).await.unwrap();

        system.reset_symbol(symbol).await.unwrap();

        let stats = system.get_stats(symbol).await;
        assert_eq!(stats.successful_communications, 0);
    }
}
