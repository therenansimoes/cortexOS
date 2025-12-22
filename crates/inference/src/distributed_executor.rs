//! Distributed Pipeline Executor
//! 
//! Coordinates TRUE distributed inference across multiple nodes.
//! Each node runs a portion of the model, passing hidden states to the next node.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock, oneshot};
use tracing::{debug, error, info, warn};
use candle_core::{Device, Tensor, DType};

use crate::sharded_model::{ShardedLlama, ShardConfig, PipelineRole, ShardedModelError};
use crate::tensor_transport::{
    InferenceMessage, InferenceMetadata, SerializedTensor, TensorTransport, TensorTransportError,
};

/// A node in the distributed inference pipeline
#[derive(Debug, Clone)]
pub struct PipelineNode {
    pub node_id: String,
    pub address: String,
    pub role: PipelineRole,
    pub is_local: bool,
}

/// Configuration for distributed inference
#[derive(Debug, Clone)]
pub struct DistributedConfig {
    /// This node's ID
    pub node_id: String,
    /// This node's listening address for tensor streams
    pub listen_addr: String,
    /// Model to use
    pub model_name: String,
    /// Total layers in the model
    pub total_layers: u32,
    /// Layers per node
    pub layers_per_node: u32,
}

/// Manages distributed inference across the pipeline
pub struct DistributedExecutor {
    config: DistributedConfig,
    /// Our local model shard
    shard: Arc<RwLock<Option<ShardedLlama>>>,
    /// Pipeline topology
    pipeline: Arc<RwLock<Vec<PipelineNode>>>,
    /// Pending inference requests
    pending: Arc<RwLock<HashMap<String, PendingInference>>>,
    /// Transport for sending/receiving tensors
    transport: Arc<TensorTransport>,
}

/// A pending inference request waiting for completion
struct PendingInference {
    task_id: String,
    input_tokens: Vec<u32>,
    response_tx: oneshot::Sender<InferenceResult>,
}

/// Result of distributed inference
#[derive(Debug, Clone)]
pub struct InferenceResult {
    pub task_id: String,
    pub success: bool,
    pub tokens: Vec<u32>,
    pub text: String,
    pub total_time_ms: u64,
    pub nodes_used: Vec<String>,
    pub per_node_time_ms: Vec<(String, u64)>,
}

impl DistributedExecutor {
    pub fn new(config: DistributedConfig) -> Self {
        Self {
            transport: Arc::new(TensorTransport::new(&config.listen_addr)),
            config,
            shard: Arc::new(RwLock::new(None)),
            pipeline: Arc::new(RwLock::new(Vec::new())),
            pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Initialize this node with its role in the pipeline
    pub async fn initialize(&self, role: PipelineRole) -> Result<(), ExecutorError> {
        info!("🚀 Initializing distributed executor with role: {:?}", role);
        
        let shard_config = ShardConfig {
            model_path: self.config.model_name.clone(),
            total_layers: self.config.total_layers,
            role,
            device: Device::Cpu,
            dtype: DType::F32,
        };
        
        let shard = ShardedLlama::load(shard_config)?;
        
        *self.shard.write().await = Some(shard);
        
        info!("✅ Executor initialized");
        Ok(())
    }
    
    /// Set the pipeline topology
    pub async fn set_pipeline(&self, nodes: Vec<PipelineNode>) {
        info!("🔗 Setting pipeline: {} nodes", nodes.len());
        for (i, node) in nodes.iter().enumerate() {
            info!("  [{}] {} ({:?}) @ {}", 
                  i, &node.node_id[..8], node.role, node.address);
        }
        *self.pipeline.write().await = nodes;
    }
    
    /// Start listening for incoming tensor streams
    pub async fn start_server(&self) -> Result<(), ExecutorError> {
        let listener = TcpListener::bind(&self.config.listen_addr).await
            .map_err(|e| ExecutorError::NetworkError(e.to_string()))?;
        
        info!("🎧 Tensor server listening on {}", self.config.listen_addr);
        
        let shard = Arc::clone(&self.shard);
        let pipeline = Arc::clone(&self.pipeline);
        let pending = Arc::clone(&self.pending);
        let transport = Arc::clone(&self.transport);
        let node_id = self.config.node_id.clone();
        
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        debug!("📥 Incoming connection from {}", addr);
                        
                        let shard = Arc::clone(&shard);
                        let pipeline = Arc::clone(&pipeline);
                        let pending = Arc::clone(&pending);
                        let transport = Arc::clone(&transport);
                        let node_id = node_id.clone();
                        
                        tokio::spawn(async move {
                            if let Err(e) = Self::handle_connection(
                                stream, shard, pipeline, pending, transport, node_id
                            ).await {
                                error!("❌ Connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("❌ Accept error: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Handle an incoming tensor stream connection
    async fn handle_connection(
        mut stream: TcpStream,
        shard: Arc<RwLock<Option<ShardedLlama>>>,
        pipeline: Arc<RwLock<Vec<PipelineNode>>>,
        pending: Arc<RwLock<HashMap<String, PendingInference>>>,
        transport: Arc<TensorTransport>,
        node_id: String,
    ) -> Result<(), ExecutorError> {
        let message = TensorTransport::receive_tensor(&mut stream).await?;
        
        match message {
            InferenceMessage::HiddenState { task_id, layer_idx, tensor, metadata } => {
                info!("📥 Received hidden state for task {} (layer {})", &task_id[..8], layer_idx);
                
                let start = std::time::Instant::now();
                
                // Deserialize tensor
                let device = Device::Cpu;
                let hidden = tensor.to_tensor(&device)?;
                
                // Process through our layers
                let shard_guard = shard.read().await;
                let shard = shard_guard.as_ref()
                    .ok_or(ExecutorError::NotInitialized)?;
                
                let output = shard.forward(&hidden)?;
                let info = shard.info();
                
                let processing_time = start.elapsed().as_millis() as u64;
                
                // Check if we're the tail
                let pipeline_guard = pipeline.read().await;
                let our_idx = pipeline_guard.iter()
                    .position(|n| n.node_id == node_id);
                
                if let Some(idx) = our_idx {
                    let is_last = idx == pipeline_guard.len() - 1;
                    
                    if is_last {
                        // We're the tail - generate final output
                        info!("🎯 TAIL: Generating final output for task {}", &task_id[..8]);
                        
                        // In real impl, we'd sample from logits and decode tokens
                        // For now, return placeholder
                        let response = InferenceMessage::FinalOutput {
                            task_id: task_id.clone(),
                            tokens: vec![1, 2, 3], // Placeholder
                            text: format!("[Distributed inference complete! Processed through {} nodes]", pipeline_guard.len()),
                            total_time_ms: processing_time,
                        };
                        
                        // Send response back
                        Self::send_response(&mut stream, response).await?;
                    } else {
                        // Forward to next node
                        let next_node = &pipeline_guard[idx + 1];
                        info!("➡️ Forwarding to next node: {} @ {}", 
                              &next_node.node_id[..8], next_node.address);
                        
                        let serialized = SerializedTensor::from_tensor(&output)?;
                        let forward_msg = InferenceMessage::HiddenState {
                            task_id: task_id.clone(),
                            layer_idx: info.end_layer,
                            tensor: serialized,
                            metadata: InferenceMetadata {
                                model_name: metadata.model_name,
                                total_layers: metadata.total_layers,
                                current_layer: info.end_layer,
                                sequence_length: metadata.sequence_length,
                                batch_size: metadata.batch_size,
                            },
                        };
                        
                        // Forward to next node
                        transport.send_tensor(&next_node.address, forward_msg).await?;
                        
                        // Send acknowledgment
                        let response = InferenceMessage::ProcessResponse {
                            task_id,
                            end_layer: info.end_layer,
                            tensor: SerializedTensor::from_tensor(&output)?,
                            processing_time_ms: processing_time,
                        };
                        Self::send_response(&mut stream, response).await?;
                    }
                }
            }
            
            InferenceMessage::ProcessRequest { task_id, .. } => {
                info!("📋 Received process request for task {}", &task_id[..8]);
                // Handle direct process requests
            }
            
            _ => {
                warn!("⚠️ Unexpected message type");
            }
        }
        
        Ok(())
    }
    
    async fn send_response(stream: &mut TcpStream, message: InferenceMessage) -> Result<(), ExecutorError> {
        use tokio::io::AsyncWriteExt;
        
        let data = bincode::serialize(&message)
            .map_err(|e| ExecutorError::SerializationError(e.to_string()))?;
        
        let len = data.len() as u64;
        stream.write_all(&len.to_le_bytes()).await
            .map_err(|e| ExecutorError::NetworkError(e.to_string()))?;
        stream.write_all(&data).await
            .map_err(|e| ExecutorError::NetworkError(e.to_string()))?;
        stream.flush().await
            .map_err(|e| ExecutorError::NetworkError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Run distributed inference from HEAD node
    pub async fn infer(&self, input_text: &str) -> Result<InferenceResult, ExecutorError> {
        let task_id = blake3::hash(input_text.as_bytes()).to_hex().to_string();
        info!("🚀 Starting distributed inference: task={}", &task_id[..8]);
        
        let start = std::time::Instant::now();
        
        // Get pipeline
        let pipeline = self.pipeline.read().await;
        if pipeline.is_empty() {
            return Err(ExecutorError::NoPipeline);
        }
        
        // We should be the HEAD
        let shard_guard = self.shard.read().await;
        let shard = shard_guard.as_ref()
            .ok_or(ExecutorError::NotInitialized)?;
        
        let info = shard.info();
        if !info.role.contains("Head") {
            return Err(ExecutorError::NotHead);
        }
        
        // Tokenize input (simplified - just use bytes as tokens)
        let tokens: Vec<u32> = input_text.bytes().map(|b| b as u32).collect();
        let device = Device::Cpu;
        
        // Create input tensor
        let input = Tensor::from_vec(tokens.clone(), (1, tokens.len()), &device)?;
        
        // Process through our layers
        info!("🔧 HEAD: Processing layers 0-{}", info.end_layer);
        let hidden = shard.forward(&input)?;
        
        // If we're the only node, generate output directly
        if pipeline.len() == 1 {
            let total_time = start.elapsed().as_millis() as u64;
            return Ok(InferenceResult {
                task_id,
                success: true,
                tokens: tokens.clone(),
                text: format!("[Single node inference - {} layers in {}ms]", info.num_layers, total_time),
                total_time_ms: total_time,
                nodes_used: vec![self.config.node_id.clone()],
                per_node_time_ms: vec![(self.config.node_id.clone(), total_time)],
            });
        }
        
        // Forward to next node
        let next_node = &pipeline[1];
        info!("➡️ Forwarding hidden states to {} @ {}", 
              &next_node.node_id[..8], next_node.address);
        
        let serialized = SerializedTensor::from_tensor(&hidden)?;
        let metadata = InferenceMetadata {
            model_name: self.config.model_name.clone(),
            total_layers: self.config.total_layers,
            current_layer: info.end_layer,
            sequence_length: tokens.len(),
            batch_size: 1,
        };
        
        // Send and wait for final response
        let result = self.transport.forward_and_wait(
            &next_node.address,
            &task_id,
            &hidden,
            metadata,
        ).await;
        
        let total_time = start.elapsed().as_millis() as u64;
        
        match result {
            Ok(_output) => {
                Ok(InferenceResult {
                    task_id,
                    success: true,
                    tokens,
                    text: format!("[Distributed inference complete through {} nodes in {}ms]", 
                                  pipeline.len(), total_time),
                    total_time_ms: total_time,
                    nodes_used: pipeline.iter().map(|n| n.node_id.clone()).collect(),
                    per_node_time_ms: vec![], // Would track per-node timing
                })
            }
            Err(e) => {
                Err(ExecutorError::InferenceError(e.to_string()))
            }
        }
    }
    
    /// Get status of the distributed executor
    pub async fn status(&self) -> ExecutorStatus {
        let shard = self.shard.read().await;
        let pipeline = self.pipeline.read().await;
        
        ExecutorStatus {
            initialized: shard.is_some(),
            shard_info: shard.as_ref().map(|s| s.info()),
            pipeline_nodes: pipeline.len(),
            listen_addr: self.config.listen_addr.clone(),
        }
    }
}

/// Status of the distributed executor
#[derive(Debug)]
pub struct ExecutorStatus {
    pub initialized: bool,
    pub shard_info: Option<crate::sharded_model::ShardInfo>,
    pub pipeline_nodes: usize,
    pub listen_addr: String,
}

/// Errors in distributed execution
#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("Shard error: {0}")]
    ShardError(#[from] ShardedModelError),
    
    #[error("Tensor transport error: {0}")]
    TransportError(#[from] TensorTransportError),
    
    #[error("Candle error: {0}")]
    CandleError(#[from] candle_core::Error),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Executor not initialized")]
    NotInitialized,
    
    #[error("No pipeline configured")]
    NoPipeline,
    
    #[error("This node is not HEAD - cannot initiate inference")]
    NotHead,
    
    #[error("Inference error: {0}")]
    InferenceError(String),
}

