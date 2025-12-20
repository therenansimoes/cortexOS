/// Signal Evolution Framework
///
/// This module implements an adaptive communication protocol evolution system.
/// Signals can evolve through reinforcement learning to optimize communication
/// effectiveness across different channels and environmental conditions.

use std::collections::HashMap;
use std::sync::Arc;

use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

use cortex_core::SymbolId;

use crate::error::SignalError;
use crate::signal::{Pulse, SignalPattern};

/// Configuration for the evolution algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    /// Population size for each generation
    pub population_size: usize,
    /// Mutation rate (0.0 to 1.0)
    pub mutation_rate: f32,
    /// Maximum pattern length in pulses
    pub max_pattern_length: usize,
    /// Minimum pattern length in pulses
    pub min_pattern_length: usize,
    /// Pulse duration range (min, max) in microseconds
    pub pulse_duration_range: (u32, u32),
    /// Number of elite patterns to preserve each generation
    pub elite_count: usize,
    /// Fitness threshold for accepting a pattern
    pub fitness_threshold: f32,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            population_size: 50,
            mutation_rate: 0.2,
            max_pattern_length: 20,
            min_pattern_length: 2,
            pulse_duration_range: (50, 1000),
            elite_count: 5,
            fitness_threshold: 0.7,
        }
    }
}

/// Fitness metrics for a signal pattern
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FitnessMetrics {
    /// Success rate (0.0 to 1.0)
    pub success_rate: f32,
    /// Average signal-to-noise ratio
    pub avg_snr: f32,
    /// Average latency in microseconds
    pub avg_latency_us: u32,
    /// Energy efficiency (lower is better)
    pub energy_cost: f32,
    /// Pattern distinctiveness (higher is better)
    pub distinctiveness: f32,
}

impl FitnessMetrics {
    pub fn new() -> Self {
        Self {
            success_rate: 0.0,
            avg_snr: 0.0,
            avg_latency_us: 0,
            energy_cost: 1.0,
            distinctiveness: 0.0,
        }
    }

    /// Calculate overall fitness score (0.0 to 1.0)
    pub fn fitness_score(&self) -> f32 {
        let success_weight = 0.4;
        let snr_weight = 0.2;
        let latency_weight = 0.15;
        let energy_weight = 0.15;
        let distinct_weight = 0.1;

        let snr_score = (self.avg_snr / 100.0).clamp(0.0, 1.0);
        let latency_score = 1.0 - (self.avg_latency_us as f32 / 1_000_000.0).clamp(0.0, 1.0);
        let energy_score = 1.0 - self.energy_cost.clamp(0.0, 1.0);

        (self.success_rate * success_weight)
            + (snr_score * snr_weight)
            + (latency_score * latency_weight)
            + (energy_score * energy_weight)
            + (self.distinctiveness * distinct_weight)
    }
}

impl Default for FitnessMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// An evolved signal pattern with fitness tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolvedPattern {
    pub pattern: SignalPattern,
    pub generation: u32,
    pub fitness: FitnessMetrics,
    pub usage_count: u32,
}

impl EvolvedPattern {
    pub fn new(pattern: SignalPattern, generation: u32) -> Self {
        Self {
            pattern,
            generation,
            fitness: FitnessMetrics::new(),
            usage_count: 0,
        }
    }

    pub fn update_fitness(&mut self, metrics: FitnessMetrics) {
        self.fitness = metrics;
    }

    pub fn increment_usage(&mut self) {
        self.usage_count += 1;
    }
}

/// The main evolution engine
pub struct EvolutionEngine {
    config: EvolutionConfig,
    population: Arc<RwLock<HashMap<SymbolId, Vec<EvolvedPattern>>>>,
    generation: Arc<RwLock<u32>>,
}

impl EvolutionEngine {
    pub fn new(config: EvolutionConfig) -> Self {
        Self {
            config,
            population: Arc::new(RwLock::new(HashMap::new())),
            generation: Arc::new(RwLock::new(0)),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(EvolutionConfig::default())
    }

    /// Initialize a population for a symbol
    pub async fn initialize_population(&self, symbol: SymbolId) -> Result<(), SignalError> {
        let mut population = self.population.write().await;
        let generation = *self.generation.read().await;

        if population.contains_key(&symbol) {
            return Err(SignalError::InvalidPattern(
                "population already initialized".into(),
            ));
        }

        let mut patterns = Vec::new();
        for _ in 0..self.config.population_size {
            let pattern = self.generate_random_pattern();
            patterns.push(EvolvedPattern::new(pattern, generation));
        }

        population.insert(symbol, patterns);
        info!(
            symbol = ?symbol,
            population_size = self.config.population_size,
            "Initialized evolution population"
        );

        Ok(())
    }

    /// Reset/reinitialize a population for a symbol
    pub async fn reset_population(&self, symbol: SymbolId) -> Result<(), SignalError> {
        let mut population = self.population.write().await;
        let generation = *self.generation.read().await;

        let mut patterns = Vec::new();
        for _ in 0..self.config.population_size {
            let pattern = self.generate_random_pattern();
            patterns.push(EvolvedPattern::new(pattern, generation));
        }

        population.insert(symbol, patterns);
        info!(
            symbol = ?symbol,
            population_size = self.config.population_size,
            "Reset evolution population"
        );

        Ok(())
    }

    /// Generate a random signal pattern
    fn generate_random_pattern(&self) -> SignalPattern {
        let mut rng = rand::thread_rng();
        let length = rng.gen_range(self.config.min_pattern_length..=self.config.max_pattern_length);

        let pulses: Vec<Pulse> = (0..length)
            .map(|_| {
                let on = rng.gen_bool(0.5);
                let duration = rng.gen_range(
                    self.config.pulse_duration_range.0..=self.config.pulse_duration_range.1,
                );
                Pulse::new(on, duration)
            })
            .collect();

        SignalPattern::new(pulses)
    }

    /// Mutate a pattern
    fn mutate_pattern(&self, pattern: &SignalPattern) -> SignalPattern {
        let mut rng = rand::thread_rng();
        let mut pulses = pattern.pulses.clone();

        // Randomly mutate pulses
        for pulse in &mut pulses {
            if rng.gen::<f32>() < self.config.mutation_rate {
                // Mutate duration
                let delta = rng.gen_range(-100..=100);
                pulse.duration_us = (pulse.duration_us as i32 + delta)
                    .clamp(
                        self.config.pulse_duration_range.0 as i32,
                        self.config.pulse_duration_range.1 as i32,
                    ) as u32;
            }
            if rng.gen::<f32>() < self.config.mutation_rate / 2.0 {
                // Flip on/off
                pulse.on = !pulse.on;
            }
        }

        // Occasionally add or remove pulses
        if rng.gen::<f32>() < self.config.mutation_rate && pulses.len() < self.config.max_pattern_length {
            let on = rng.gen_bool(0.5);
            let duration = rng.gen_range(
                self.config.pulse_duration_range.0..=self.config.pulse_duration_range.1,
            );
            pulses.insert(rng.gen_range(0..=pulses.len()), Pulse::new(on, duration));
        } else if rng.gen::<f32>() < self.config.mutation_rate && pulses.len() > self.config.min_pattern_length {
            pulses.remove(rng.gen_range(0..pulses.len()));
        }

        SignalPattern::new(pulses)
    }

    /// Crossover two patterns
    fn crossover_patterns(&self, parent1: &SignalPattern, parent2: &SignalPattern) -> SignalPattern {
        let mut rng = rand::thread_rng();
        let mut pulses = Vec::new();

        let max_len = parent1.pulses.len().max(parent2.pulses.len());
        for i in 0..max_len {
            let pulse = if rng.gen_bool(0.5) {
                parent1.pulses.get(i).or_else(|| parent2.pulses.get(i))
            } else {
                parent2.pulses.get(i).or_else(|| parent1.pulses.get(i))
            };

            if let Some(p) = pulse {
                pulses.push(*p);
            }
        }

        // Ensure within length constraints
        if pulses.len() > self.config.max_pattern_length {
            pulses.truncate(self.config.max_pattern_length);
        }

        SignalPattern::new(pulses)
    }

    /// Update fitness for a pattern after usage
    pub async fn record_fitness(
        &self,
        symbol: SymbolId,
        pattern: &SignalPattern,
        metrics: FitnessMetrics,
    ) -> Result<(), SignalError> {
        let mut population = self.population.write().await;

        if let Some(patterns) = population.get_mut(&symbol) {
            for evolved in patterns.iter_mut() {
                if evolved.pattern == *pattern {
                    evolved.update_fitness(metrics);
                    evolved.increment_usage();
                    debug!(
                        symbol = ?symbol,
                        fitness = metrics.fitness_score(),
                        usage_count = evolved.usage_count,
                        "Updated pattern fitness"
                    );
                    return Ok(());
                }
            }
        }

        Err(SignalError::InvalidPattern("pattern not found in population".into()))
    }

    /// Evolve the population for a symbol
    pub async fn evolve_generation(&self, symbol: SymbolId) -> Result<(), SignalError> {
        let mut population = self.population.write().await;
        let mut generation = self.generation.write().await;

        let patterns = population
            .get_mut(&symbol)
            .ok_or_else(|| SignalError::InvalidPattern("population not initialized".into()))?;

        // Sort by fitness
        patterns.sort_by(|a, b| {
            b.fitness
                .fitness_score()
                .partial_cmp(&a.fitness.fitness_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Keep elite patterns
        let mut new_population = patterns[..self.config.elite_count.min(patterns.len())].to_vec();

        // Generate new patterns through crossover and mutation
        while new_population.len() < self.config.population_size {
            let parent1_idx = rand::thread_rng().gen_range(0..self.config.elite_count.min(patterns.len()));
            let parent2_idx = rand::thread_rng().gen_range(0..self.config.elite_count.min(patterns.len()));

            let parent1 = &patterns[parent1_idx].pattern;
            let parent2 = &patterns[parent2_idx].pattern;

            let mut child = self.crossover_patterns(parent1, parent2);
            child = self.mutate_pattern(&child);

            new_population.push(EvolvedPattern::new(child, *generation + 1));
        }

        *patterns = new_population;
        *generation += 1;

        info!(
            symbol = ?symbol,
            generation = *generation,
            population_size = patterns.len(),
            "Evolved new generation"
        );

        Ok(())
    }

    /// Get the best pattern for a symbol
    pub async fn get_best_pattern(&self, symbol: SymbolId) -> Result<SignalPattern, SignalError> {
        let population = self.population.read().await;

        let patterns = population
            .get(&symbol)
            .ok_or_else(|| SignalError::InvalidPattern("population not initialized".into()))?;

        patterns
            .iter()
            .max_by(|a, b| {
                a.fitness
                    .fitness_score()
                    .partial_cmp(&b.fitness.fitness_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| p.pattern.clone())
            .ok_or_else(|| SignalError::InvalidPattern("no patterns available".into()))
    }

    /// Get current generation number
    pub async fn current_generation(&self) -> u32 {
        *self.generation.read().await
    }

    /// Calculate distinctiveness of a pattern compared to others
    pub fn calculate_distinctiveness(
        &self,
        pattern: &SignalPattern,
        other_patterns: &[SignalPattern],
    ) -> f32 {
        if other_patterns.is_empty() {
            return 1.0;
        }

        let mut total_distance = 0.0;
        for other in other_patterns {
            total_distance += self.pattern_distance(pattern, other);
        }

        (total_distance / other_patterns.len() as f32).clamp(0.0, 1.0)
    }

    /// Calculate distance between two patterns (normalized)
    fn pattern_distance(&self, p1: &SignalPattern, p2: &SignalPattern) -> f32 {
        let len_diff = (p1.pulses.len() as i32 - p2.pulses.len() as i32).abs() as f32;
        let max_len = p1.pulses.len().max(p2.pulses.len()) as f32;

        if max_len == 0.0 {
            return 0.0;
        }

        let mut pulse_diff = 0.0;
        for (i, pulse1) in p1.pulses.iter().enumerate() {
            if let Some(pulse2) = p2.pulses.get(i) {
                let on_diff = if pulse1.on != pulse2.on { 1.0 } else { 0.0 };
                let duration_diff = (pulse1.duration_us as f32 - pulse2.duration_us as f32).abs()
                    / self.config.pulse_duration_range.1 as f32;
                pulse_diff += on_diff + duration_diff;
            } else {
                pulse_diff += 2.0; // Missing pulse
            }
        }

        ((len_diff / max_len) + (pulse_diff / (max_len * 2.0))) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_evolution_initialization() {
        let engine = EvolutionEngine::with_default_config();
        let symbol = SymbolId::from_bytes(b"TEST_SIGNAL");

        engine.initialize_population(symbol).await.unwrap();

        // Should be able to get best pattern
        let pattern = engine.get_best_pattern(symbol).await.unwrap();
        assert!(!pattern.pulses.is_empty());
    }

    #[tokio::test]
    async fn test_fitness_recording() {
        let engine = EvolutionEngine::with_default_config();
        let symbol = SymbolId::from_bytes(b"TEST_SIGNAL");

        engine.initialize_population(symbol).await.unwrap();
        let pattern = engine.get_best_pattern(symbol).await.unwrap();

        let metrics = FitnessMetrics {
            success_rate: 0.9,
            avg_snr: 75.0,
            avg_latency_us: 5000,
            energy_cost: 0.3,
            distinctiveness: 0.8,
        };

        engine.record_fitness(symbol, &pattern, metrics).await.unwrap();

        // Fitness should be recorded
        let updated_pattern = engine.get_best_pattern(symbol).await.unwrap();
        assert_eq!(updated_pattern, pattern);
    }

    #[tokio::test]
    async fn test_evolution_generation() {
        let engine = EvolutionEngine::with_default_config();
        let symbol = SymbolId::from_bytes(b"TEST_SIGNAL");

        engine.initialize_population(symbol).await.unwrap();
        let gen0 = engine.current_generation().await;

        // Record some fitness to create selection pressure
        let pattern = engine.get_best_pattern(symbol).await.unwrap();
        let metrics = FitnessMetrics {
            success_rate: 0.95,
            avg_snr: 80.0,
            avg_latency_us: 3000,
            energy_cost: 0.2,
            distinctiveness: 0.9,
        };
        engine.record_fitness(symbol, &pattern, metrics).await.unwrap();

        // Evolve
        engine.evolve_generation(symbol).await.unwrap();

        let gen1 = engine.current_generation().await;
        assert_eq!(gen1, gen0 + 1);
    }

    #[test]
    fn test_fitness_score_calculation() {
        let metrics = FitnessMetrics {
            success_rate: 0.9,
            avg_snr: 75.0,
            avg_latency_us: 5000,
            energy_cost: 0.3,
            distinctiveness: 0.8,
        };

        let score = metrics.fitness_score();
        assert!(score > 0.5 && score < 1.0);
    }

    #[test]
    fn test_pattern_mutation() {
        let config = EvolutionConfig::default();
        let engine = EvolutionEngine::new(config);

        let original = SignalPattern::new(vec![
            Pulse::on(100),
            Pulse::off(100),
            Pulse::on(100),
        ]);

        let mutated = engine.mutate_pattern(&original);

        // Pattern should exist but may differ
        assert!(!mutated.pulses.is_empty());
    }

    #[test]
    fn test_pattern_crossover() {
        let config = EvolutionConfig::default();
        let max_len = config.max_pattern_length;
        let engine = EvolutionEngine::new(config);

        let parent1 = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);
        let parent2 = SignalPattern::new(vec![Pulse::off(150), Pulse::on(150)]);

        let child = engine.crossover_patterns(&parent1, &parent2);

        assert!(!child.pulses.is_empty());
        assert!(child.pulses.len() <= max_len);
    }

    #[test]
    fn test_pattern_distance() {
        let config = EvolutionConfig::default();
        let engine = EvolutionEngine::new(config);

        let p1 = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);
        let p2 = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);
        let p3 = SignalPattern::new(vec![Pulse::off(200), Pulse::on(200)]);

        let dist_same = engine.pattern_distance(&p1, &p2);
        let dist_diff = engine.pattern_distance(&p1, &p3);

        assert!(dist_same < dist_diff);
    }

    #[test]
    fn test_distinctiveness_calculation() {
        let config = EvolutionConfig::default();
        let engine = EvolutionEngine::new(config);

        let pattern = SignalPattern::new(vec![Pulse::on(100), Pulse::off(100)]);
        let similar = SignalPattern::new(vec![Pulse::on(110), Pulse::off(90)]);
        let different = SignalPattern::new(vec![Pulse::off(500), Pulse::on(500), Pulse::off(500)]);

        let distinct_similar = engine.calculate_distinctiveness(&pattern, &[similar]);
        let distinct_different = engine.calculate_distinctiveness(&pattern, &[different]);

        assert!(distinct_different > distinct_similar);
    }
}
