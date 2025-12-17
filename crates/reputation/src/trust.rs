use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use cortex_grid::NodeId;

use crate::rating::{RatingRecord, SkillId, SkillRating};
use crate::error::{ReputationError, Result};

/// Trust score for a node (0.0 to 1.0)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TrustScore(f32);

impl TrustScore {
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    pub fn value(&self) -> f32 {
        self.0
    }

    pub fn is_trusted(&self) -> bool {
        self.0 > 0.5
    }

    pub fn is_highly_trusted(&self) -> bool {
        self.0 > 0.8
    }
}

impl Default for TrustScore {
    fn default() -> Self {
        Self(0.5) // neutral trust
    }
}

/// Local view of trust relationships
#[derive(Debug, Clone)]
pub struct TrustGraph {
    /// My node ID
    my_id: NodeId,
    /// Direct trust I have in other nodes (per skill)
    direct_trust: DashMap<(NodeId, SkillId), TrustScore>,
    /// Aggregated ratings for each node+skill
    skill_ratings: DashMap<(NodeId, SkillId), SkillRating>,
    /// All rating records we've seen
    rating_history: Arc<RwLock<Vec<RatingRecord>>>,
    /// Global trust scores (computed via EigenTrust)
    global_trust: DashMap<NodeId, TrustScore>,
    /// Pre-trusted nodes (bootstrap trust)
    pre_trusted: HashSet<NodeId>,
}

impl TrustGraph {
    pub fn new(my_id: NodeId) -> Self {
        Self {
            my_id,
            direct_trust: DashMap::new(),
            skill_ratings: DashMap::new(),
            rating_history: Arc::new(RwLock::new(Vec::new())),
            global_trust: DashMap::new(),
            pre_trusted: HashSet::new(),
        }
    }

    /// Add a pre-trusted node (e.g., known good actors)
    pub fn add_pre_trusted(&mut self, node: NodeId) {
        self.pre_trusted.insert(node);
        self.global_trust.insert(node, TrustScore::new(0.9));
    }

    /// Record a rating (from any node)
    pub fn record_rating(&self, record: RatingRecord) -> Result<()> {
        if record.rater == record.ratee {
            return Err(ReputationError::SelfRatingNotAllowed);
        }

        // Get or create skill rating
        let key = (record.ratee, record.skill.clone());
        let rater_trust = self.get_trust(&record.rater).value();

        {
            let mut entry = self.skill_ratings
                .entry(key.clone())
                .or_insert_with(|| SkillRating::new(record.skill.clone(), record.ratee));
            entry.add_weighted_rating(record.rating, rater_trust);
        }

        // Store in history
        self.rating_history.write().push(record);

        Ok(())
    }

    /// Rate another node for a skill (as myself)
    pub fn rate(&self, target: NodeId, skill: SkillId, rating: crate::rating::Rating) -> Result<RatingRecord> {
        let record = RatingRecord::new(self.my_id, target, skill, rating);
        self.record_rating(record.clone())?;
        Ok(record)
    }

    /// Get my direct trust in a node for a skill
    pub fn get_direct_trust(&self, node: &NodeId, skill: &SkillId) -> TrustScore {
        self.direct_trust
            .get(&(*node, skill.clone()))
            .map(|v| *v)
            .unwrap_or_default()
    }

    /// Get global trust for a node (skill-agnostic)
    pub fn get_trust(&self, node: &NodeId) -> TrustScore {
        if self.pre_trusted.contains(node) {
            return TrustScore::new(0.9);
        }
        self.global_trust
            .get(node)
            .map(|v| *v)
            .unwrap_or_default()
    }

    /// Get skill rating for a node
    pub fn get_skill_rating(&self, node: &NodeId, skill: &SkillId) -> Option<SkillRating> {
        self.skill_ratings
            .get(&(*node, skill.clone()))
            .map(|v| v.clone())
    }

    /// Find top N nodes for a skill
    pub fn top_nodes_for_skill(&self, skill: &SkillId, limit: usize) -> Vec<(NodeId, SkillRating)> {
        let mut results: Vec<_> = self.skill_ratings
            .iter()
            .filter(|entry| &entry.key().1 == skill)
            .map(|entry| (entry.key().0, entry.value().clone()))
            .collect();

        results.sort_by(|a, b| {
            b.1.normalized_score()
                .partial_cmp(&a.1.normalized_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(limit);
        results
    }

    /// Get all skills a node has been rated for
    pub fn skills_for_node(&self, node: &NodeId) -> Vec<SkillRating> {
        self.skill_ratings
            .iter()
            .filter(|entry| &entry.key().0 == node)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get all known skills in the network
    pub fn known_skills(&self) -> Vec<SkillId> {
        let mut skills: HashSet<SkillId> = HashSet::new();
        for entry in self.skill_ratings.iter() {
            skills.insert(entry.key().1.clone());
        }
        skills.into_iter().collect()
    }

    /// Get rating history
    pub fn history(&self) -> Vec<RatingRecord> {
        self.rating_history.read().clone()
    }
}

/// EigenTrust algorithm for computing global trust
/// Based on: "The EigenTrust Algorithm for Reputation Management in P2P Networks"
pub struct EigenTrust {
    /// Convergence threshold
    epsilon: f32,
    /// Maximum iterations
    max_iterations: usize,
    /// Pre-trusted peers weight
    alpha: f32,
}

impl EigenTrust {
    pub fn new() -> Self {
        Self {
            epsilon: 0.001,
            max_iterations: 20,
            alpha: 0.1, // 10% weight to pre-trusted peers
        }
    }

    /// Compute global trust scores from rating history
    pub fn compute(&self, graph: &TrustGraph) -> HashMap<NodeId, TrustScore> {
        let history = graph.history();
        if history.is_empty() {
            return HashMap::new();
        }

        // Build normalized local trust matrix
        let mut nodes: HashSet<NodeId> = HashSet::new();
        let mut local_trust: HashMap<(NodeId, NodeId), f32> = HashMap::new();

        for record in &history {
            nodes.insert(record.rater);
            nodes.insert(record.ratee);

            // Aggregate ratings: positive = +1, negative = -1
            let key = (record.rater, record.ratee);
            let current = local_trust.entry(key).or_insert(0.0);
            *current += record.rating.value();
        }

        // Normalize to [0, 1] and ensure row sums = 1
        let node_list: Vec<_> = nodes.iter().cloned().collect();
        let n = node_list.len();
        let node_index: HashMap<_, _> = node_list.iter().enumerate().map(|(i, n)| (*n, i)).collect();

        // Build normalized trust matrix C
        let mut c_matrix: Vec<Vec<f32>> = vec![vec![0.0; n]; n];
        for (i, rater) in node_list.iter().enumerate() {
            let mut row_sum = 0.0;
            for ratee in &node_list {
                let val = local_trust.get(&(*rater, *ratee)).copied().unwrap_or(0.0);
                let normalized = val.max(0.0); // Only positive trust
                row_sum += normalized;
                if let Some(&j) = node_index.get(ratee) {
                    c_matrix[i][j] = normalized;
                }
            }
            if row_sum > 0.0 {
                for j in 0..n {
                    c_matrix[i][j] /= row_sum;
                }
            }
        }

        // Pre-trusted distribution
        let mut p: Vec<f32> = vec![0.0; n];
        let pre_trusted_count = graph.pre_trusted.len().max(1);
        for pt in &graph.pre_trusted {
            if let Some(&i) = node_index.get(pt) {
                p[i] = 1.0 / pre_trusted_count as f32;
            }
        }
        if graph.pre_trusted.is_empty() {
            // Uniform distribution if no pre-trusted
            for i in 0..n {
                p[i] = 1.0 / n as f32;
            }
        }

        // Power iteration
        let mut t: Vec<f32> = p.clone();
        for _ in 0..self.max_iterations {
            let mut new_t: Vec<f32> = vec![0.0; n];

            // t' = (1 - alpha) * C^T * t + alpha * p
            for j in 0..n {
                let mut sum = 0.0;
                for i in 0..n {
                    sum += c_matrix[i][j] * t[i];
                }
                new_t[j] = (1.0 - self.alpha) * sum + self.alpha * p[j];
            }

            // Check convergence
            let diff: f32 = t.iter().zip(&new_t).map(|(a, b)| (a - b).abs()).sum();
            t = new_t;

            if diff < self.epsilon {
                break;
            }
        }

        // Convert to TrustScore map
        let mut result = HashMap::new();
        for (i, score) in t.into_iter().enumerate() {
            result.insert(node_list[i], TrustScore::new(score * n as f32)); // Scale by n
        }

        result
    }

    /// Update graph with computed trust scores
    pub fn update_graph(&self, graph: &TrustGraph) {
        let scores = self.compute(graph);
        for (node, score) in scores {
            graph.global_trust.insert(node, score);
        }
    }
}

impl Default for EigenTrust {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rating::Rating;

    #[tokio::test]
    async fn test_trust_graph() {
        let my_id = NodeId::random();
        let other = NodeId::random();
        let graph = TrustGraph::new(my_id);

        let record = graph.rate(other, "coding".into(), Rating::positive()).unwrap();
        assert!(record.rating.is_positive());

        let rating = graph.get_skill_rating(&other, &"coding".into());
        assert!(rating.is_some());
        assert_eq!(rating.unwrap().positive_count, 1);
    }

    #[tokio::test]
    async fn test_top_nodes() {
        let my_id = NodeId::random();
        let graph = TrustGraph::new(my_id);

        let node_a = NodeId::random();
        let node_b = NodeId::random();

        // Node A gets 3 positive ratings
        for _ in 0..3 {
            graph.rate(node_a, "rust".into(), Rating::positive()).unwrap();
        }

        // Node B gets 1 positive, 2 negative
        graph.rate(node_b, "rust".into(), Rating::positive()).unwrap();
        graph.rate(node_b, "rust".into(), Rating::negative()).unwrap();
        graph.rate(node_b, "rust".into(), Rating::negative()).unwrap();

        let top = graph.top_nodes_for_skill(&"rust".into(), 10);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, node_a); // A should be first
    }
}
