use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use cortex_grid::NodeId;
use cortex_reputation::{SkillId, TrustGraph, TrustScore};

use crate::error::{Result, SkillError};
use crate::registry::NetworkSkillRegistry;
use crate::task::SkillTask;

/// Decision on where to route a task
#[derive(Debug, Clone)]
pub struct RouteDecision {
    /// Selected node
    pub node: NodeId,
    /// Node's trust score for this skill
    pub trust_score: TrustScore,
    /// Node's skill rating (normalized)
    pub skill_score: f32,
    /// Combined routing score
    pub route_score: f32,
    /// Alternative nodes (in order of preference)
    pub alternatives: Vec<(NodeId, f32)>,
}

/// Routes tasks to the best node based on skill + reputation
pub struct SkillRouter {
    my_id: NodeId,
    trust_graph: Arc<RwLock<TrustGraph>>,
    skill_registry: Arc<RwLock<NetworkSkillRegistry>>,
    /// Weight for trust vs skill rating (0.0 = only skill, 1.0 = only trust)
    trust_weight: f32,
}

impl SkillRouter {
    pub fn new(
        my_id: NodeId,
        trust_graph: Arc<RwLock<TrustGraph>>,
        skill_registry: Arc<RwLock<NetworkSkillRegistry>>,
    ) -> Self {
        Self {
            my_id,
            trust_graph,
            skill_registry,
            trust_weight: 0.3, // 30% trust, 70% skill rating
        }
    }

    pub fn with_trust_weight(mut self, weight: f32) -> Self {
        self.trust_weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Find the best node to execute a task
    pub async fn route(&self, task: &SkillTask) -> Result<RouteDecision> {
        let skill = &task.skill;
        let min_trust = task.min_trust;

        // Get all nodes that can execute this skill
        let candidates = self.skill_registry.read().await.nodes_with_skill(skill);

        if candidates.is_empty() {
            return Err(SkillError::NoCapableNode(skill.to_string()));
        }

        let trust_graph = self.trust_graph.read().await;
        let mut scored: Vec<(NodeId, f32, TrustScore, f32)> = Vec::new();

        for node in candidates {
            // Don't route to self (unless no other option)
            if node == self.my_id && scored.len() > 0 {
                continue;
            }

            let trust = trust_graph.get_trust(&node);

            // Filter by minimum trust
            if trust.value() < min_trust {
                debug!(
                    "Node {} filtered out: trust {:.2} < min {:.2}",
                    node,
                    trust.value(),
                    min_trust
                );
                continue;
            }

            let skill_rating = trust_graph
                .get_skill_rating(&node, skill)
                .map(|sr| sr.normalized_score())
                .unwrap_or(0.0);

            // Combined score
            let combined = self.trust_weight * trust.value()
                + (1.0 - self.trust_weight) * (skill_rating + 1.0) / 2.0;

            scored.push((node, combined, trust, skill_rating));
        }

        if scored.is_empty() {
            return Err(SkillError::NoCapableNode(format!(
                "{} (no nodes meet min_trust {})",
                skill, min_trust
            )));
        }

        // Sort by combined score (descending)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let best = &scored[0];
        let alternatives: Vec<_> = scored.iter().skip(1).map(|(n, s, _, _)| (*n, *s)).collect();

        info!(
            "Routed task {} to node {} (trust: {:.2}, skill: {:.2}, combined: {:.2})",
            task.id,
            best.0,
            best.2.value(),
            best.3,
            best.1
        );

        Ok(RouteDecision {
            node: best.0,
            trust_score: best.2,
            skill_score: best.3,
            route_score: best.1,
            alternatives,
        })
    }

    /// Route with fallback: try alternatives if primary fails
    pub async fn route_with_fallback(
        &self,
        task: &SkillTask,
        failed_nodes: &[NodeId],
    ) -> Result<RouteDecision> {
        let mut decision = self.route(task).await?;

        // If best node has failed, try alternatives
        if failed_nodes.contains(&decision.node) {
            for (alt_node, alt_score) in &decision.alternatives {
                if !failed_nodes.contains(alt_node) {
                    warn!(
                        "Primary node {} failed, falling back to {}",
                        decision.node, alt_node
                    );
                    decision.node = *alt_node;
                    decision.route_score = *alt_score;
                    return Ok(decision);
                }
            }
            return Err(SkillError::NoCapableNode(format!(
                "{} (all capable nodes have failed)",
                task.skill
            )));
        }

        Ok(decision)
    }

    /// Find multiple nodes for parallel/redundant execution
    pub async fn route_multi(&self, task: &SkillTask, count: usize) -> Result<Vec<RouteDecision>> {
        let decision = self.route(task).await?;

        let mut results = vec![decision.clone()];

        for (alt_node, alt_score) in decision.alternatives.into_iter().take(count - 1) {
            let trust_graph = self.trust_graph.read().await;
            let trust = trust_graph.get_trust(&alt_node);
            let skill_score = trust_graph
                .get_skill_rating(&alt_node, &task.skill)
                .map(|sr| sr.normalized_score())
                .unwrap_or(0.0);

            results.push(RouteDecision {
                node: alt_node,
                trust_score: trust,
                skill_score,
                route_score: alt_score,
                alternatives: Vec::new(),
            });
        }

        Ok(results)
    }
}
