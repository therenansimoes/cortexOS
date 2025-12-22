//! Smart Work Distribution
//! 
//! Splits inference tasks across peers based on their REAL capacity.
//! More powerful devices get more layers to process.

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::device::DeviceCapabilities;
use crate::task_queue::TensorChunk;

/// A peer's contribution to the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerWork {
    pub node_id: String,
    pub address: String,
    pub capacity_score: u32,
    pub max_layers: u32,
    /// Assigned layers for this task
    pub assigned_layers: (u32, u32),
    /// Chunk to send
    pub chunk: Option<TensorChunk>,
}

/// Work distribution plan for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkPlan {
    pub task_id: String,
    pub total_layers: u32,
    pub peers: Vec<PeerWork>,
    /// Total network capacity (sum of all scores)
    pub total_capacity: u32,
    /// Equivalent model size in billions
    pub equivalent_params_b: f32,
}

impl WorkPlan {
    /// Get a summary of the work distribution
    pub fn summary(&self) -> String {
        let peer_summary: Vec<String> = self.peers.iter()
            .map(|p| format!(
                "{}...: layers {}-{} (score: {})",
                &p.node_id[..8.min(p.node_id.len())],
                p.assigned_layers.0,
                p.assigned_layers.1,
                p.capacity_score
            ))
            .collect();
        
        format!(
            "Task {} | {} peers | {}B equivalent\n{}",
            &self.task_id[..8],
            self.peers.len(),
            self.equivalent_params_b,
            peer_summary.join("\n")
        )
    }
}

/// Distributes work across peers based on their real capacity
pub struct WorkDistributor;

impl WorkDistributor {
    /// Create a work plan that distributes layers proportionally to capacity
    /// 
    /// More powerful peers get more layers!
    pub fn distribute(
        task_id: &str,
        total_layers: u32,
        peers: &[(String, String, DeviceCapabilities)], // (node_id, address, caps)
    ) -> WorkPlan {
        if peers.is_empty() {
            return WorkPlan {
                task_id: task_id.to_string(),
                total_layers,
                peers: vec![],
                total_capacity: 0,
                equivalent_params_b: 0.0,
            };
        }
        
        // Calculate total capacity
        let total_capacity: u32 = peers.iter()
            .map(|(_, _, caps)| caps.capacity_score)
            .sum();
        
        info!("📊 Distributing {} layers across {} peers (total capacity: {})",
              total_layers, peers.len(), total_capacity);
        
        let mut peer_works = Vec::new();
        let mut current_layer = 0u32;
        let mut remaining_layers = total_layers;
        
        for (i, (node_id, address, caps)) in peers.iter().enumerate() {
            let is_last = i == peers.len() - 1;
            
            // Calculate proportional share
            let share = if is_last {
                // Last peer gets remaining layers
                remaining_layers
            } else {
                // Proportional to capacity
                let proportion = caps.capacity_score as f32 / total_capacity as f32;
                let layers = (total_layers as f32 * proportion).round() as u32;
                
                // Ensure at least 1 layer, and respect device max
                layers.max(1).min(caps.max_layers).min(remaining_layers)
            };
            
            let end_layer = current_layer + share - 1;
            
            debug!("  Node {}: {} layers (score: {}, max: {})",
                   &node_id[..8.min(node_id.len())], share, caps.capacity_score, caps.max_layers);
            
            peer_works.push(PeerWork {
                node_id: node_id.clone(),
                address: address.clone(),
                capacity_score: caps.capacity_score,
                max_layers: caps.max_layers,
                assigned_layers: (current_layer, end_layer),
                chunk: None,
            });
            
            current_layer = end_layer + 1;
            remaining_layers = remaining_layers.saturating_sub(share);
        }
        
        // Calculate equivalent model size
        // Each node contributing 0.5B params per batch of layers they process
        let params_per_layer = 0.5 / 24.0; // 0.5B model / 24 layers
        let equivalent_params_b = total_layers as f32 * params_per_layer;
        
        let plan = WorkPlan {
            task_id: task_id.to_string(),
            total_layers,
            peers: peer_works,
            total_capacity,
            equivalent_params_b,
        };
        
        info!("✅ Work plan created: {}", plan.summary());
        
        plan
    }
    
    /// Redistribute work when a peer fails
    pub fn redistribute_failed(
        plan: &WorkPlan,
        failed_node: &str,
        remaining_peers: &[(String, String, DeviceCapabilities)],
    ) -> WorkPlan {
        // Find the failed peer's layers
        let failed_peer = plan.peers.iter()
            .find(|p| p.node_id == failed_node);
        
        let failed_layers = failed_peer
            .map(|p| p.assigned_layers.1 - p.assigned_layers.0 + 1)
            .unwrap_or(0);
        
        info!("⚠️ Redistributing {} layers from failed node {}", 
              failed_layers, &failed_node[..8.min(failed_node.len())]);
        
        // Create new plan without the failed peer
        Self::distribute(&plan.task_id, plan.total_layers, remaining_peers)
    }
}

/// Response assembly - joins processed chunks back together
pub struct ResponseJoiner;

impl ResponseJoiner {
    /// Join tensor chunks back into a single response
    /// Like assembling torrent pieces!
    pub fn join_chunks(chunks: Vec<Vec<u8>>) -> Vec<u8> {
        // Simple concatenation for now
        // In real implementation, would properly merge tensor data
        let total_size: usize = chunks.iter().map(|c| c.len()).sum();
        let mut result = Vec::with_capacity(total_size);
        
        for chunk in chunks {
            result.extend(chunk);
        }
        
        result
    }
    
    /// Join tensor results with proper layer ordering
    pub fn join_layer_outputs(
        outputs: &mut [(u32, Vec<u8>)], // (start_layer, data)
    ) -> Vec<u8> {
        // Sort by layer index
        outputs.sort_by_key(|(layer, _)| *layer);
        
        // Concatenate in order
        let total_size: usize = outputs.iter().map(|(_, d)| d.len()).sum();
        let mut result = Vec::with_capacity(total_size);
        
        for (layer_idx, data) in outputs.iter() {
            debug!("📦 Joining output from layer {}: {} bytes", layer_idx, data.len());
            result.extend(data);
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn mock_caps(score: u32, max_layers: u32) -> DeviceCapabilities {
        DeviceCapabilities {
            device_type: crate::device::DeviceType::Desktop,
            cpu: crate::device::CpuInfo {
                model: "Test".to_string(),
                cores: 4,
                threads: 8,
                frequency_mhz: 2400,
                arch: "x86_64".to_string(),
            },
            memory: crate::device::MemoryInfo {
                total_mb: 8192,
                available_mb: 4096,
                used_mb: 4096,
            },
            gpu: None,
            storage: crate::device::StorageInfo {
                free_mb: 100000,
                is_ssd: true,
            },
            network_mbps: 100,
            capacity_score: score,
            max_layers,
            can_inference: true,
        }
    }
    
    #[test]
    fn test_proportional_distribution() {
        let peers = vec![
            ("node1".to_string(), "addr1".to_string(), mock_caps(10, 20)),
            ("node2".to_string(), "addr2".to_string(), mock_caps(30, 50)),
            ("node3".to_string(), "addr3".to_string(), mock_caps(60, 80)),
        ];
        
        let plan = WorkDistributor::distribute("task123", 24, &peers);
        
        // Total should be 24 layers
        let total: u32 = plan.peers.iter()
            .map(|p| p.assigned_layers.1 - p.assigned_layers.0 + 1)
            .sum();
        assert_eq!(total, 24);
        
        // node3 should have the most layers (highest score)
        let node3 = plan.peers.iter().find(|p| p.node_id == "node3").unwrap();
        let node3_layers = node3.assigned_layers.1 - node3.assigned_layers.0 + 1;
        
        let node1 = plan.peers.iter().find(|p| p.node_id == "node1").unwrap();
        let node1_layers = node1.assigned_layers.1 - node1.assigned_layers.0 + 1;
        
        assert!(node3_layers > node1_layers);
        
        println!("Plan:\n{}", plan.summary());
    }
    
    #[test]
    fn test_single_peer() {
        let peers = vec![
            ("node1".to_string(), "addr1".to_string(), mock_caps(50, 30)),
        ];
        
        let plan = WorkDistributor::distribute("task456", 24, &peers);
        
        assert_eq!(plan.peers.len(), 1);
        let assigned = plan.peers[0].assigned_layers;
        assert_eq!(assigned.0, 0);
        assert_eq!(assigned.1, 23); // All 24 layers
    }
}

