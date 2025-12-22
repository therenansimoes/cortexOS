//! Tensor Transport Layer
//! 
//! Serializes and sends tensors between nodes for distributed inference.
//! This is the core of TRUE distributed AI - passing hidden states between nodes.

use candle_core::{DType, Device, Tensor};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, info, warn};

/// Serialized tensor format for network transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedTensor {
    /// Shape of the tensor (e.g., [batch, seq_len, hidden_dim])
    pub shape: Vec<usize>,
    /// Data type (f32, f16, bf16)
    pub dtype: String,
    /// Raw tensor data as bytes
    pub data: Vec<u8>,
    /// Checksum for integrity verification
    pub checksum: [u8; 32],
}

impl SerializedTensor {
    /// Serialize a Candle tensor for network transmission
    pub fn from_tensor(tensor: &Tensor) -> Result<Self, TensorTransportError> {
        let shape = tensor.dims().to_vec();
        let dtype = format!("{:?}", tensor.dtype());
        
        // Get raw data based on dtype
        let data: Vec<u8> = match tensor.dtype() {
            DType::F32 => {
                let values: Vec<f32> = tensor.flatten_all()?.to_vec1()?;
                let mut bytes = Vec::with_capacity(values.len() * 4);
                for v in values {
                    bytes.extend_from_slice(&v.to_le_bytes());
                }
                bytes
            }
            DType::F16 => {
                let values: Vec<half::f16> = tensor.flatten_all()?.to_vec1()?;
                let mut bytes = Vec::with_capacity(values.len() * 2);
                for v in values {
                    bytes.extend_from_slice(&v.to_le_bytes());
                }
                bytes
            }
            DType::BF16 => {
                let values: Vec<half::bf16> = tensor.flatten_all()?.to_vec1()?;
                let mut bytes = Vec::with_capacity(values.len() * 2);
                for v in values {
                    bytes.extend_from_slice(&v.to_le_bytes());
                }
                bytes
            }
            _ => return Err(TensorTransportError::UnsupportedDtype(dtype)),
        };
        
        // Calculate checksum
        let checksum = *blake3::hash(&data).as_bytes();
        
        Ok(Self {
            shape,
            dtype,
            data,
            checksum,
        })
    }
    
    /// Deserialize back to a Candle tensor
    pub fn to_tensor(&self, device: &Device) -> Result<Tensor, TensorTransportError> {
        // Verify checksum
        let computed_checksum = blake3::hash(&self.data);
        if computed_checksum.as_bytes() != &self.checksum {
            return Err(TensorTransportError::ChecksumMismatch);
        }
        
        // Reconstruct tensor based on dtype
        let tensor = match self.dtype.as_str() {
            "F32" => {
                let values: Vec<f32> = self.data
                    .chunks(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                Tensor::from_vec(values, self.shape.as_slice(), device)?
            }
            "F16" => {
                let values: Vec<half::f16> = self.data
                    .chunks(2)
                    .map(|chunk| half::f16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect();
                Tensor::from_vec(values, self.shape.as_slice(), device)?
            }
            "BF16" => {
                let values: Vec<half::bf16> = self.data
                    .chunks(2)
                    .map(|chunk| half::bf16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect();
                Tensor::from_vec(values, self.shape.as_slice(), device)?
            }
            _ => return Err(TensorTransportError::UnsupportedDtype(self.dtype.clone())),
        };
        
        Ok(tensor)
    }
    
    /// Serialize to bytes for network transmission
    pub fn to_bytes(&self) -> Result<Vec<u8>, TensorTransportError> {
        bincode::serialize(self).map_err(|e| TensorTransportError::SerializationError(e.to_string()))
    }
    
    /// Deserialize from network bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, TensorTransportError> {
        bincode::deserialize(bytes).map_err(|e| TensorTransportError::SerializationError(e.to_string()))
    }
}

/// Message types for distributed inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InferenceMessage {
    /// Send hidden states to next node in pipeline
    HiddenState {
        task_id: String,
        layer_idx: u32,
        tensor: SerializedTensor,
        metadata: InferenceMetadata,
    },
    /// Request for a node to process hidden states
    ProcessRequest {
        task_id: String,
        start_layer: u32,
        end_layer: u32,
    },
    /// Response with processed hidden states
    ProcessResponse {
        task_id: String,
        end_layer: u32,
        tensor: SerializedTensor,
        processing_time_ms: u64,
    },
    /// Final output from tail node
    FinalOutput {
        task_id: String,
        tokens: Vec<u32>,
        text: String,
        total_time_ms: u64,
    },
    /// Error during processing
    Error {
        task_id: String,
        error: String,
    },
}

/// Metadata about the inference request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceMetadata {
    pub model_name: String,
    pub total_layers: u32,
    pub current_layer: u32,
    pub sequence_length: usize,
    pub batch_size: usize,
}

/// Transport for sending/receiving tensors over TCP
pub struct TensorTransport {
    local_addr: String,
}

impl TensorTransport {
    pub fn new(local_addr: &str) -> Self {
        Self {
            local_addr: local_addr.to_string(),
        }
    }
    
    /// Send a tensor to another node
    pub async fn send_tensor(
        &self,
        target_addr: &str,
        message: InferenceMessage,
    ) -> Result<(), TensorTransportError> {
        let start = std::time::Instant::now();
        
        // Serialize message
        let data = bincode::serialize(&message)
            .map_err(|e| TensorTransportError::SerializationError(e.to_string()))?;
        
        // Connect to target node
        let mut stream = TcpStream::connect(target_addr).await
            .map_err(|e| TensorTransportError::ConnectionError(e.to_string()))?;
        
        // Send length prefix (8 bytes) + data
        let len = data.len() as u64;
        stream.write_all(&len.to_le_bytes()).await
            .map_err(|e| TensorTransportError::SendError(e.to_string()))?;
        stream.write_all(&data).await
            .map_err(|e| TensorTransportError::SendError(e.to_string()))?;
        stream.flush().await
            .map_err(|e| TensorTransportError::SendError(e.to_string()))?;
        
        let elapsed = start.elapsed().as_millis();
        debug!("📤 Sent {} bytes to {} in {}ms", data.len(), target_addr, elapsed);
        
        Ok(())
    }
    
    /// Receive a tensor message (blocking read)
    pub async fn receive_tensor(
        stream: &mut TcpStream,
    ) -> Result<InferenceMessage, TensorTransportError> {
        // Read length prefix
        let mut len_buf = [0u8; 8];
        stream.read_exact(&mut len_buf).await
            .map_err(|e| TensorTransportError::ReceiveError(e.to_string()))?;
        let len = u64::from_le_bytes(len_buf) as usize;
        
        // Read message data
        let mut data = vec![0u8; len];
        stream.read_exact(&mut data).await
            .map_err(|e| TensorTransportError::ReceiveError(e.to_string()))?;
        
        // Deserialize
        let message: InferenceMessage = bincode::deserialize(&data)
            .map_err(|e| TensorTransportError::SerializationError(e.to_string()))?;
        
        debug!("📥 Received {} bytes", len);
        
        Ok(message)
    }
    
    /// Send hidden state and wait for response
    pub async fn forward_and_wait(
        &self,
        target_addr: &str,
        task_id: &str,
        hidden_state: &Tensor,
        metadata: InferenceMetadata,
    ) -> Result<Tensor, TensorTransportError> {
        let serialized = SerializedTensor::from_tensor(hidden_state)?;
        
        let message = InferenceMessage::HiddenState {
            task_id: task_id.to_string(),
            layer_idx: metadata.current_layer,
            tensor: serialized,
            metadata: metadata.clone(),
        };
        
        // Connect
        let mut stream = TcpStream::connect(target_addr).await
            .map_err(|e| TensorTransportError::ConnectionError(e.to_string()))?;
        
        // Send request
        let data = bincode::serialize(&message)
            .map_err(|e| TensorTransportError::SerializationError(e.to_string()))?;
        let len = data.len() as u64;
        stream.write_all(&len.to_le_bytes()).await
            .map_err(|e| TensorTransportError::SendError(e.to_string()))?;
        stream.write_all(&data).await
            .map_err(|e| TensorTransportError::SendError(e.to_string()))?;
        stream.flush().await
            .map_err(|e| TensorTransportError::SendError(e.to_string()))?;
        
        // Wait for response
        let response = Self::receive_tensor(&mut stream).await?;
        
        match response {
            InferenceMessage::ProcessResponse { tensor, .. } => {
                tensor.to_tensor(&Device::Cpu)
            }
            InferenceMessage::Error { error, .. } => {
                Err(TensorTransportError::RemoteError(error))
            }
            _ => Err(TensorTransportError::UnexpectedMessage),
        }
    }
}

/// Errors that can occur during tensor transport
#[derive(Debug, thiserror::Error)]
pub enum TensorTransportError {
    #[error("Candle error: {0}")]
    CandleError(#[from] candle_core::Error),
    
    #[error("Unsupported dtype: {0}")]
    UnsupportedDtype(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Send error: {0}")]
    SendError(String),
    
    #[error("Receive error: {0}")]
    ReceiveError(String),
    
    #[error("Checksum mismatch - data corrupted")]
    ChecksumMismatch,
    
    #[error("Remote error: {0}")]
    RemoteError(String),
    
    #[error("Unexpected message type")]
    UnexpectedMessage,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tensor_serialization_roundtrip() {
        let device = Device::Cpu;
        let original = Tensor::randn(0f32, 1.0, (2, 4, 8), &device).unwrap();
        
        let serialized = SerializedTensor::from_tensor(&original).unwrap();
        let restored = serialized.to_tensor(&device).unwrap();
        
        assert_eq!(original.dims(), restored.dims());
    }
}

