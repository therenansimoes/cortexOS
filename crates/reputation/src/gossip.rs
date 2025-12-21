use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info};

use cortex_grid::NodeId;

use crate::error::Result;
use crate::rating::RatingRecord;
use crate::trust::TrustGraph;

/// Messages for reputation gossip protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GossipMessage {
    /// Announce a new rating
    NewRating(RatingRecord),

    /// Request ratings for a node+skill
    RequestRatings { node: NodeId, skill: String },

    /// Response with ratings
    RatingsResponse { ratings: Vec<RatingRecord> },

    /// Request top nodes for a skill
    RequestTopNodes { skill: String, limit: usize },

    /// Response with top nodes
    TopNodesResponse {
        skill: String,
        nodes: Vec<(NodeId, f32)>, // node, score
    },

    /// Sync request - send ratings newer than timestamp
    SyncRequest { since_timestamp: u64 },

    /// Sync response
    SyncResponse { ratings: Vec<RatingRecord> },
}

/// Gossip protocol for propagating reputation data
pub struct ReputationGossip {
    my_id: NodeId,
    graph: Arc<RwLock<TrustGraph>>,
    outbound_tx: mpsc::Sender<(NodeId, GossipMessage)>,
    seen_ratings: Arc<RwLock<HashSet<[u8; 32]>>>,
    running: Arc<RwLock<bool>>,
}

impl ReputationGossip {
    pub fn new(
        my_id: NodeId,
        graph: Arc<RwLock<TrustGraph>>,
    ) -> (Self, mpsc::Receiver<(NodeId, GossipMessage)>) {
        let (tx, rx) = mpsc::channel(256);

        (
            Self {
                my_id,
                graph,
                outbound_tx: tx,
                seen_ratings: Arc::new(RwLock::new(HashSet::new())),
                running: Arc::new(RwLock::new(false)),
            },
            rx,
        )
    }

    /// Start the gossip protocol
    pub async fn start(&self) -> Result<()> {
        *self.running.write().await = true;
        info!("Reputation gossip started for {}", self.my_id);
        Ok(())
    }

    /// Stop the gossip protocol
    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("Reputation gossip stopped");
    }

    /// Handle incoming gossip message
    pub async fn handle_message(
        &self,
        from: NodeId,
        msg: GossipMessage,
    ) -> Result<Option<GossipMessage>> {
        match msg {
            GossipMessage::NewRating(record) => self.handle_new_rating(from, record).await,
            GossipMessage::RequestRatings { node, skill } => {
                self.handle_request_ratings(node, skill).await
            }
            GossipMessage::RequestTopNodes { skill, limit } => {
                self.handle_request_top_nodes(skill, limit).await
            }
            GossipMessage::SyncRequest { since_timestamp } => {
                self.handle_sync_request(since_timestamp).await
            }
            GossipMessage::RatingsResponse { ratings } => {
                self.handle_ratings_response(ratings).await
            }
            GossipMessage::TopNodesResponse { .. } => Ok(None),
            GossipMessage::SyncResponse { ratings } => self.handle_sync_response(ratings).await,
        }
    }

    async fn handle_new_rating(
        &self,
        _from: NodeId,
        record: RatingRecord,
    ) -> Result<Option<GossipMessage>> {
        let hash = record.hash();

        // Check if already seen
        {
            let mut seen = self.seen_ratings.write().await;
            if seen.contains(&hash) {
                debug!("Already seen rating, skipping");
                return Ok(None);
            }
            seen.insert(hash);
        }

        // Record the rating
        self.graph.write().await.record_rating(record.clone())?;
        debug!(
            "Recorded rating from {} for {} on skill {}",
            record.rater, record.ratee, record.skill
        );

        // Propagate to others (will be handled by caller)
        Ok(None)
    }

    async fn handle_request_ratings(
        &self,
        node: NodeId,
        skill: String,
    ) -> Result<Option<GossipMessage>> {
        let graph = self.graph.read().await;
        let history = graph.history();

        let ratings: Vec<_> = history
            .into_iter()
            .filter(|r| r.ratee == node && r.skill.as_str() == skill)
            .collect();

        Ok(Some(GossipMessage::RatingsResponse { ratings }))
    }

    async fn handle_request_top_nodes(
        &self,
        skill: String,
        limit: usize,
    ) -> Result<Option<GossipMessage>> {
        let graph = self.graph.read().await;
        let skill_id: crate::rating::SkillId = skill.clone().into();
        let top = graph.top_nodes_for_skill(&skill_id, limit);

        let nodes: Vec<_> = top
            .into_iter()
            .map(|(node, rating)| (node, rating.normalized_score()))
            .collect();

        Ok(Some(GossipMessage::TopNodesResponse { skill, nodes }))
    }

    async fn handle_sync_request(&self, since_timestamp: u64) -> Result<Option<GossipMessage>> {
        let graph = self.graph.read().await;
        let history = graph.history();

        let ratings: Vec<_> = history
            .into_iter()
            .filter(|r| r.timestamp > since_timestamp)
            .collect();

        Ok(Some(GossipMessage::SyncResponse { ratings }))
    }

    async fn handle_ratings_response(
        &self,
        ratings: Vec<RatingRecord>,
    ) -> Result<Option<GossipMessage>> {
        let graph = self.graph.write().await;
        for record in ratings {
            let _ = graph.record_rating(record);
        }
        Ok(None)
    }

    async fn handle_sync_response(
        &self,
        ratings: Vec<RatingRecord>,
    ) -> Result<Option<GossipMessage>> {
        self.handle_ratings_response(ratings).await
    }

    /// Broadcast a new rating to the network
    pub async fn broadcast_rating(&self, record: RatingRecord, targets: Vec<NodeId>) -> Result<()> {
        let msg = GossipMessage::NewRating(record);
        for target in targets {
            if target != self.my_id {
                let _ = self.outbound_tx.send((target, msg.clone())).await;
            }
        }
        Ok(())
    }

    /// Request sync from a peer
    pub async fn request_sync(&self, peer: NodeId, since_timestamp: u64) -> Result<()> {
        let msg = GossipMessage::SyncRequest { since_timestamp };
        let _ = self.outbound_tx.send((peer, msg)).await;
        Ok(())
    }

    /// Query top nodes for a skill
    pub async fn query_top_nodes(&self, peer: NodeId, skill: &str, limit: usize) -> Result<()> {
        let msg = GossipMessage::RequestTopNodes {
            skill: skill.to_string(),
            limit,
        };
        let _ = self.outbound_tx.send((peer, msg)).await;
        Ok(())
    }
}

impl Clone for ReputationGossip {
    fn clone(&self) -> Self {
        Self {
            my_id: self.my_id,
            graph: Arc::clone(&self.graph),
            outbound_tx: self.outbound_tx.clone(),
            seen_ratings: Arc::clone(&self.seen_ratings),
            running: Arc::clone(&self.running),
        }
    }
}
