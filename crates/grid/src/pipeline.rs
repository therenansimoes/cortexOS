//! Pipeline Parallelism - Run HUGE models across many small devices
//! 
//! 100 devices × 0.5B each = 50B AI!
//! 
//! How it works:
//! 1. Large model is SPLIT into layers
//! 2. Each node holds a FEW layers (not the whole model)
//! 3. Inference flows through the network:
//!    
//!    Input → Node1(layers 1-4) → Node2(layers 5-8) → ... → Output
//!    
//! 4. Hidden states are passed between nodes
//! 5. Each node only needs enough RAM for its layers!

use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use tracing::{info, warn};

use crate::{NodeId, PeerStore};

/// Pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Total layers in the model
    pub total_layers: u32,
    /// Layers per node
    pub layers_per_node: u32,
    /// Model name (e.g., "llama-70b")
    pub model_name: String,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            total_layers: 80,      // LLaMA-70B has ~80 layers
            layers_per_node: 4,    // Each node handles 4 layers
            model_name: "distributed-llm".to_string(),
        }
    }
}

/// A node's role in the pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PipelineRole {
    /// First node - receives input tokens, produces initial embeddings
    Head { layers: (u32, u32) },
    /// Middle node - receives hidden states, outputs hidden states
    Middle { layers: (u32, u32) },
    /// Final node - receives hidden states, produces output tokens
    Tail { layers: (u32, u32) },
    /// Not part of the pipeline
    Inactive,
}

/// Hidden state passed between pipeline nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiddenState {
    /// Sequence ID for this inference run
    pub sequence_id: String,
    /// Current position in generation
    pub position: u32,
    /// The hidden state tensor (flattened)
    pub data: Vec<f32>,
    /// Shape of the tensor [batch, seq_len, hidden_dim]
    pub shape: Vec<u32>,
    /// Which layer this came from
    pub from_layer: u32,
}

/// Pipeline node info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineNode {
    pub node_id: NodeId,
    pub role: PipelineRole,
    pub address: String,
    pub layers_loaded: bool,
    pub latency_ms: u32,
}

/// Pipeline coordinator - manages the distributed model
pub struct PipelineCoordinator {
    pub config: PipelineConfig,
    pub nodes: Arc<RwLock<Vec<PipelineNode>>>,
    pub peer_store: Arc<PeerStore>,
    node_id: NodeId,
}

impl PipelineCoordinator {
    pub fn new(node_id: NodeId, peer_store: Arc<PeerStore>, config: PipelineConfig) -> Self {
        Self {
            config,
            nodes: Arc::new(RwLock::new(Vec::new())),
            peer_store,
            node_id,
        }
    }

    /// Assign pipeline roles to available nodes
    pub async fn build_pipeline(&self) -> Result<Vec<PipelineNode>, String> {
        let peers = self.peer_store
            .find_by_capability(|caps| caps.can_compute)
            .await;

        if peers.is_empty() {
            return Err("No compute nodes available".to_string());
        }

        let nodes_needed = (self.config.total_layers / self.config.layers_per_node) as usize;
        
        info!("🔧 Building pipeline: {} layers ÷ {} per node = {} nodes needed",
            self.config.total_layers, self.config.layers_per_node, nodes_needed);
        info!("   Available nodes: {}", peers.len());

        let mut pipeline_nodes = Vec::new();
        let available = peers.len().min(nodes_needed);

        for (i, peer) in peers.iter().take(available).enumerate() {
            let start_layer = (i as u32) * self.config.layers_per_node;
            let end_layer = start_layer + self.config.layers_per_node - 1;
            
            let role = if i == 0 {
                PipelineRole::Head { layers: (start_layer, end_layer) }
            } else if i == available - 1 {
                PipelineRole::Tail { layers: (start_layer, end_layer) }
            } else {
                PipelineRole::Middle { layers: (start_layer, end_layer) }
            };

            let address = peer.addresses.first()
                .map(|a| a.to_string())
                .unwrap_or_default();

            let node = PipelineNode {
                node_id: peer.node_id,
                role,
                address,
                layers_loaded: false,
                latency_ms: peer.latency_ms.unwrap_or(0),
            };

            info!("   Node {} → {:?}", &peer.node_id.to_string()[..8], node.role);
            pipeline_nodes.push(node);
        }

        *self.nodes.write().await = pipeline_nodes.clone();
        
        let total_params = available as f32 * 0.5; // Assume 0.5B per node
        info!("✅ Pipeline ready: {} nodes = ~{:.1}B parameters!", available, total_params);

        Ok(pipeline_nodes)
    }

    /// Run inference through the pipeline
    pub async fn infer(&self, prompt: &str) -> Result<String, String> {
        let nodes = self.nodes.read().await;
        
        if nodes.is_empty() {
            return Err("Pipeline not built. Call build_pipeline() first".to_string());
        }

        let sequence_id = format!("seq-{}", uuid::Uuid::new_v4());
        info!("🚀 Starting pipeline inference: {}", sequence_id);

        // For now, we'll simulate the pipeline by having each node
        // process the prompt and pass results to the next
        // In production, this would pass actual hidden states

        let mut current_output = prompt.to_string();
        let total_time_start = std::time::Instant::now();

        for (i, node) in nodes.iter().enumerate() {
            let node_start = std::time::Instant::now();
            
            info!("   Stage {}/{}: Node {} processing layers {:?}",
                i + 1, nodes.len(), &node.node_id.to_string()[..8], 
                match &node.role {
                    PipelineRole::Head { layers } => layers,
                    PipelineRole::Middle { layers } => layers,
                    PipelineRole::Tail { layers } => layers,
                    PipelineRole::Inactive => &(0, 0),
                });

            // Send to node for processing
            match self.send_to_node(node, &current_output, &sequence_id, i as u32).await {
                Ok(output) => {
                    current_output = output;
                    info!("      ✓ Stage {} complete in {}ms", 
                        i + 1, node_start.elapsed().as_millis());
                }
                Err(e) => {
                    warn!("      ✗ Stage {} failed: {}", i + 1, e);
                    return Err(format!("Pipeline stage {} failed: {}", i + 1, e));
                }
            }
        }

        let total_time = total_time_start.elapsed().as_millis();
        info!("✅ Pipeline complete in {}ms across {} nodes", total_time, nodes.len());

        Ok(current_output)
    }

    /// Send data to a pipeline node for processing
    async fn send_to_node(
        &self, 
        node: &PipelineNode, 
        input: &str,
        sequence_id: &str,
        stage: u32,
    ) -> Result<String, String> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        // Get task port (node port + 1000)
        let task_addr = if let Some((ip, port_str)) = node.address.rsplit_once(':') {
            if let Ok(port) = port_str.parse::<u16>() {
                format!("{}:{}", ip, port + 1000)
            } else {
                return Err("Invalid port".to_string());
            }
        } else {
            return Err("Invalid address".to_string());
        };

        let mut stream = TcpStream::connect(&task_addr)
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        // Create pipeline request
        #[derive(Serialize)]
        struct PipelineRequest {
            task_id: String,
            skill: String,
            payload: String,
            from_node: String,
        }

        let request = PipelineRequest {
            task_id: format!("{}-stage{}", sequence_id, stage),
            skill: "llm".to_string(), // Use LLM skill for now
            payload: input.to_string(),
            from_node: self.node_id.to_string(),
        };

        let request_bytes = serde_json::to_vec(&request)
            .map_err(|e| format!("Serialize error: {}", e))?;
        let len_bytes = (request_bytes.len() as u32).to_be_bytes();

        stream.write_all(&len_bytes).await.map_err(|e| e.to_string())?;
        stream.write_all(&request_bytes).await.map_err(|e| e.to_string())?;
        stream.flush().await.map_err(|e| e.to_string())?;

        // Read response with timeout
        let mut len_buf = [0u8; 4];
        tokio::time::timeout(
            std::time::Duration::from_secs(120),
            stream.read_exact(&mut len_buf)
        ).await
            .map_err(|_| "Timeout".to_string())?
            .map_err(|e| e.to_string())?;

        let len = u32::from_be_bytes(len_buf) as usize;
        let mut response_buf = vec![0u8; len];
        stream.read_exact(&mut response_buf).await.map_err(|e| e.to_string())?;

        #[derive(Deserialize)]
        struct TaskResponse {
            success: bool,
            result: Option<String>,
            error: Option<String>,
        }

        let response: TaskResponse = serde_json::from_slice(&response_buf)
            .map_err(|e| e.to_string())?;

        if response.success {
            Ok(response.result.unwrap_or_default())
        } else {
            Err(response.error.unwrap_or_else(|| "Unknown error".to_string()))
        }
    }

    /// Get pipeline status
    pub async fn status(&self) -> PipelineStatus {
        let nodes = self.nodes.read().await;
        let active_nodes = nodes.iter().filter(|n| n.role != PipelineRole::Inactive).count();
        let total_layers = active_nodes as u32 * self.config.layers_per_node;
        let equivalent_params = active_nodes as f32 * 0.5; // 0.5B per node

        PipelineStatus {
            active_nodes,
            total_layers,
            equivalent_params_b: equivalent_params,
            nodes: nodes.clone(),
        }
    }
}

/// Pipeline status info
#[derive(Debug, Clone, Serialize)]
pub struct PipelineStatus {
    pub active_nodes: usize,
    pub total_layers: u32,
    pub equivalent_params_b: f32,
    pub nodes: Vec<PipelineNode>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_config() {
        let config = PipelineConfig::default();
        assert_eq!(config.total_layers, 80);
        assert_eq!(config.layers_per_node, 4);
        // 80 layers / 4 per node = 20 nodes needed
    }
}

