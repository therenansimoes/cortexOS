//! CortexOS Distributed Inference Engine
//! 
//! TRUE distributed AI inference with:
//! - **Tensor Parallelism**: Split weight matrices across nodes
//! - **Pipeline Parallelism**: Split layers, pass hidden states between nodes  
//! - **Model Sharding**: Each node holds only PART of the model weights
//! 
//! ## Architecture
//! 
//! ```text
//! Node 1 (HEAD):       Embedding + Layers 0-7   → Process input, send hidden state
//! Node 2 (MIDDLE):     Layers 8-15              → Receive hidden, process, send
//! Node 3 (MIDDLE):     Layers 16-23             → Receive hidden, process, send
//! Node 4 (TAIL):       Layers 24-31 + LM Head   → Receive hidden, generate output
//! ```
//! 
//! ## Usage
//! 
//! ```rust,ignore
//! use cortex_inference::{DistributedExecutor, DistributedConfig, PipelineRole};
//! 
//! // Create executor
//! let config = DistributedConfig {
//!     node_id: "node1".to_string(),
//!     listen_addr: "0.0.0.0:9001".to_string(),
//!     model_name: "qwen2.5-0.5b".to_string(),
//!     total_layers: 24,
//!     layers_per_node: 8,
//! };
//! 
//! let executor = DistributedExecutor::new(config);
//! 
//! // Initialize as HEAD
//! executor.initialize(PipelineRole::Head { 
//!     start_layer: 0, 
//!     end_layer: 7 
//! }).await?;
//! 
//! // Start tensor server
//! executor.start_server().await?;
//! 
//! // Run inference
//! let result = executor.infer("Hello world").await?;
//! ```

pub mod tensor_transport;
pub mod sharded_model;
pub mod distributed_executor;

pub use tensor_transport::{
    SerializedTensor, 
    InferenceMessage, 
    InferenceMetadata, 
    TensorTransport,
    TensorTransportError,
};

pub use sharded_model::{
    ShardedLlama,
    ShardConfig,
    PipelineRole,
    ShardInfo,
    ShardedModelError,
};

pub use distributed_executor::{
    DistributedExecutor,
    DistributedConfig,
    PipelineNode,
    InferenceResult,
    ExecutorStatus,
    ExecutorError,
};

/// Calculate optimal layer distribution for N nodes
pub fn calculate_layer_distribution(total_layers: u32, num_nodes: u32) -> Vec<(u32, u32)> {
    let layers_per_node = total_layers / num_nodes;
    let remainder = total_layers % num_nodes;
    
    let mut distribution = Vec::new();
    let mut current = 0;
    
    for i in 0..num_nodes {
        let extra = if i < remainder { 1 } else { 0 };
        let layers = layers_per_node + extra;
        let end = current + layers - 1;
        distribution.push((current, end));
        current = end + 1;
    }
    
    distribution
}

/// Estimate equivalent model size for distributed network
pub fn estimate_equivalent_params(nodes: usize, params_per_node_b: f32) -> f32 {
    // With pipeline parallelism, each node handles different layers
    // So the total is additive (not just redundancy)
    nodes as f32 * params_per_node_b
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_layer_distribution() {
        // 24 layers, 3 nodes = 8 layers each
        let dist = calculate_layer_distribution(24, 3);
        assert_eq!(dist, vec![(0, 7), (8, 15), (16, 23)]);
        
        // 24 layers, 4 nodes = 6 layers each
        let dist = calculate_layer_distribution(24, 4);
        assert_eq!(dist, vec![(0, 5), (6, 11), (12, 17), (18, 23)]);
        
        // 25 layers, 3 nodes = 9, 8, 8
        let dist = calculate_layer_distribution(25, 3);
        assert_eq!(dist, vec![(0, 8), (9, 16), (17, 24)]);
    }
    
    #[test]
    fn test_equivalent_params() {
        // 5 nodes × 0.5B = 2.5B equivalent
        assert_eq!(estimate_equivalent_params(5, 0.5), 2.5);
        
        // 100 nodes × 0.5B = 50B equivalent
        assert_eq!(estimate_equivalent_params(100, 0.5), 50.0);
    }
}
