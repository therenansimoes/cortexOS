//! Sharded Model Implementation
//! 
//! Loads only a portion of a model's layers for pipeline parallelism.
//! This enables TRUE distributed inference where each node holds different layers.

use candle_core::{DType, Device, Module, Tensor, IndexOp};
use candle_nn::{VarBuilder, linear_no_bias, RmsNorm, Activation};
use candle_transformers::models::llama::{Config as LlamaConfig, LlamaEosToks};
use std::path::{Path, PathBuf};
use std::fs;
use tracing::{debug, info, warn};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct LlamaConfigJson {
    hidden_size: usize,
    intermediate_size: usize,
    vocab_size: usize,
    num_hidden_layers: usize,
    num_attention_heads: usize,
    num_key_value_heads: usize,
    rms_norm_eps: f64,
    rope_theta: f32,
    max_position_embeddings: usize,
    bos_token_id: Option<u32>,
    eos_token_id: Option<serde_json::Value>,
    tie_word_embeddings: Option<bool>,
    use_flash_attn: Option<bool>,
}

impl From<LlamaConfigJson> for LlamaConfig {
    fn from(c: LlamaConfigJson) -> Self {
        let eos_token_id = match c.eos_token_id {
            Some(serde_json::Value::Number(n)) => n.as_u64().map(|v| LlamaEosToks::Single(v as u32)),
            Some(serde_json::Value::Array(arr)) => {
                let toks: Vec<u32> = arr.iter().filter_map(|v| v.as_u64().map(|n| n as u32)).collect();
                 Some(LlamaEosToks::Multiple(toks))
            },
            _ => None,
        };

        LlamaConfig {
            hidden_size: c.hidden_size,
            intermediate_size: c.intermediate_size,
            vocab_size: c.vocab_size,
            num_hidden_layers: c.num_hidden_layers,
            num_attention_heads: c.num_attention_heads,
            num_key_value_heads: c.num_key_value_heads,
            rms_norm_eps: c.rms_norm_eps,
            rope_theta: c.rope_theta,
            max_position_embeddings: c.max_position_embeddings,
            bos_token_id: c.bos_token_id,
            eos_token_id,
            rope_scaling: None,
            tie_word_embeddings: c.tie_word_embeddings.unwrap_or(false),
            use_flash_attn: c.use_flash_attn.unwrap_or(false),
        }
    }
}


/// Role of this node in the pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineRole {
    /// First node - handles embedding and first layers
    Head { start_layer: u32, end_layer: u32 },
    /// Middle node - processes intermediate layers
    Middle { start_layer: u32, end_layer: u32 },
    /// Last node - processes final layers and generates output
    Tail { start_layer: u32, end_layer: u32 },
    /// Single node - processes everything locally (Head + Tail)
    Single { start_layer: u32, end_layer: u32 },
}

impl PipelineRole {
    pub fn layer_range(&self) -> (u32, u32) {
        match self {
            PipelineRole::Head { start_layer, end_layer } => (*start_layer, *end_layer),
            PipelineRole::Middle { start_layer, end_layer } => (*start_layer, *end_layer),
            PipelineRole::Tail { start_layer, end_layer } => (*start_layer, *end_layer),
            PipelineRole::Single { start_layer, end_layer } => (*start_layer, *end_layer),
        }
    }
    
    pub fn is_head(&self) -> bool {
        matches!(self, PipelineRole::Head { .. } | PipelineRole::Single { .. })
    }
    
    pub fn is_tail(&self) -> bool {
        matches!(self, PipelineRole::Tail { .. } | PipelineRole::Single { .. })
    }
}

/// Configuration for a sharded model
#[derive(Debug, Clone)]
pub struct ShardConfig {
    /// Model directory path containing safetensors and config.json
    pub model_path: String,
    /// Total layers in the full model
    pub total_layers: u32,
    /// This node's role and layer range
    pub role: PipelineRole,
    /// Device to use (CPU/CUDA/Metal)
    pub device: Device,
    /// Data type (f32, f16, bf16)
    pub dtype: DType,
}

/// A sharded portion of an LLM
#[derive(Debug)]
pub struct ShardedLlama {
    config: ShardConfig,
    llama_config: LlamaConfig,
    
    // Only loaded if this is HEAD
    embedding: Option<candle_nn::Embedding>,
    
    // Only our assigned layers
    layers: Vec<TransformerBlock>,
    
    // Only loaded if this is TAIL
    norm: Option<RmsNorm>,
    lm_head: Option<candle_nn::Linear>,
    
    // Rotary embedding cache (shared across layers)
    rope: RotaryEmbedding,
}

#[derive(Debug)]
struct RotaryEmbedding {
    dim: usize,
    max_position_embeddings: usize,
    base: f32,
    cos: Tensor,
    sin: Tensor,
}

impl RotaryEmbedding {
    fn new(dim: usize, max_position_embeddings: usize, base: f32, device: &Device) -> Result<Self, candle_core::Error> {
        let inv_freq: Vec<_> = (0..dim)
            .step_by(2)
            .map(|i| 1f32 / base.powf(i as f32 / dim as f32))
            .collect();
        let inv_freq = Tensor::from_vec(inv_freq, (dim / 2,), device)?;
        let t = Tensor::arange(0u32, max_position_embeddings as u32, device)?
            .to_dtype(DType::F32)?
            .reshape((max_position_embeddings, 1))?;
        let freqs = t.matmul(&inv_freq.reshape((1, dim / 2))?)?;
        let cos = freqs.cos()?;
        let sin = freqs.sin()?;
        Ok(Self {
            dim,
            max_position_embeddings,
            base,
            cos,
            sin,
        })
    }

    fn forward(&self, x: &Tensor, pos: usize) -> Result<Tensor, candle_core::Error> {
        let (_b, seq_len, _h, _d) = x.dims4()?;
        let cos = self.cos.narrow(0, pos, seq_len)?;
        let sin = self.sin.narrow(0, pos, seq_len)?;
        candle_nn::rotary_emb::rope(x, &cos, &sin)
    }
}

/// A single transformer block
#[derive(Debug)]
pub struct TransformerBlock {
    layer_idx: u32,
    attention_norm: RmsNorm,
    ffn_norm: RmsNorm,
    // Attention weights
    wq: candle_nn::Linear,
    wk: candle_nn::Linear,
    wv: candle_nn::Linear,
    wo: candle_nn::Linear,
    // FFN weights
    w1: candle_nn::Linear, // Gate
    w2: candle_nn::Linear, // Down
    w3: candle_nn::Linear, // Up
    
    n_head: usize,
    n_kv_head: usize,
    head_dim: usize,
    span: tracing::Span,
}

impl ShardedLlama {
    /// Load a sharded portion of the model
    pub fn load(config: ShardConfig) -> Result<Self, ShardedModelError> {
        info!("🔧 Loading sharded model: role={:?}, layers={:?}", 
              config.role, config.role.layer_range());
        
        // Load model config
        let llama_config = load_llama_config(&config.model_path)?;
        
        // Find safetensors files
        let model_path = Path::new(&config.model_path);
        let files = if model_path.is_dir() {
            fs::read_dir(model_path)?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map_or(false, |e| e == "safetensors"))
                .collect::<Vec<_>>()
        } else {
            vec![PathBuf::from(&config.model_path)]
        };
        
        if files.is_empty() {
            return Err(ShardedModelError::ConfigError(format!("No .safetensors files found in {}", config.model_path)));
        }
        
        info!("  📂 Found {} weight files", files.len());
        
        // Load weights
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&files, config.dtype, &config.device)? };
        
        let (start, end) = config.role.layer_range();
        
        // Load RoPE
        let head_dim = llama_config.hidden_size / llama_config.num_attention_heads;
        let rope = RotaryEmbedding::new(
            head_dim, 
            llama_config.max_position_embeddings, 
            llama_config.rope_theta, 
            &config.device
        )?;
        
        // Load specific layers
        let mut layers = Vec::new();
        for layer_idx in start..=end {
            info!("  📦 Loading layer {}", layer_idx);
            layers.push(TransformerBlock::new(
                layer_idx, 
                &llama_config, 
                vb.pp(format!("model.layers.{}", layer_idx))
            )?);
        }
        
        // Load embedding only for HEAD
        let embedding = if config.role.is_head() {
            info!("  📦 Loading embedding layer");
            Some(candle_nn::embedding(
                llama_config.vocab_size, 
                llama_config.hidden_size, 
                vb.pp("model.embed_tokens")
            )?)
        } else {
            None
        };
        
        // Load output layers only for TAIL
        let (norm, lm_head) = if config.role.is_tail() {
            info!("  📦 Loading output norm and lm_head");
            (
                Some(candle_nn::rms_norm(
                    llama_config.hidden_size, 
                    llama_config.rms_norm_eps, 
                    vb.pp("model.norm")
                )?),
                Some(linear_no_bias(
                    llama_config.hidden_size, 
                    llama_config.vocab_size, 
                    vb.pp("lm_head")
                )?),
            )
        } else {
            (None, None)
        };
        
        info!("✅ Sharded model loaded: {} layers", layers.len());
        
        Ok(Self {
            config,
            llama_config,
            embedding,
            layers,
            norm,
            lm_head,
            rope,
        })
    }
    
    /// Process input through this shard's layers
    pub fn forward(&self, input: &Tensor) -> Result<Tensor, ShardedModelError> {
        let start_time = std::time::Instant::now();
        let (start_layer, end_layer) = self.config.role.layer_range();
        
        let mut hidden = if self.config.role.is_head() {
            // HEAD: Embed tokens first
            let embedding = self.embedding.as_ref()
                .ok_or(ShardedModelError::MissingLayer("embedding".to_string()))?;
            embedding.forward(input)?
        } else {
            // MIDDLE/TAIL: Input is already hidden states
            input.clone()
        };
        
        let (_b, seq_len, _h) = hidden.dims3()?;
        
        // Process through our layers
        for (i, layer) in self.layers.iter().enumerate() {
            // debug!("  🔄 Processing layer {} ({}/{})", layer.layer_idx, i + 1, self.layers.len());
            hidden = layer.forward(&hidden, &self.rope, 0)?; // pos=0 for non-causal/prefill
        }
        
        // TAIL: Apply final norm and lm_head
        if self.config.role.is_tail() {
            let norm = self.norm.as_ref()
                .ok_or(ShardedModelError::MissingLayer("norm".to_string()))?;
            hidden = norm.forward(&hidden)?;
            
            let lm_head = self.lm_head.as_ref()
                .ok_or(ShardedModelError::MissingLayer("lm_head".to_string()))?;
            
            // Extract last token logits
            let last_hidden = hidden.i((.., seq_len - 1, ..))?;
            hidden = lm_head.forward(&last_hidden)?;
        }
        
        let elapsed = start_time.elapsed().as_millis();
        info!("⚡ Shard forward pass: layers {}-{} in {}ms", start_layer, end_layer, elapsed);
        
        Ok(hidden)
    }
    
    /// Get model info
    pub fn info(&self) -> ShardInfo {
        let (start, end) = self.config.role.layer_range();
        ShardInfo {
            role: format!("{:?}", self.config.role),
            start_layer: start,
            end_layer: end,
            num_layers: self.layers.len() as u32,
            total_layers: self.llama_config.num_hidden_layers as u32,
            hidden_size: self.llama_config.hidden_size as u32,
            vocab_size: self.llama_config.vocab_size as u32,
        }
    }
}

impl TransformerBlock {
    fn new(layer_idx: u32, config: &LlamaConfig, vb: VarBuilder) -> Result<Self, ShardedModelError> {
        let hidden_size = config.hidden_size;
        let intermediate_size = config.intermediate_size;
        let n_head = config.num_attention_heads;
        let n_kv_head = config.num_key_value_heads;
        let head_dim = hidden_size / n_head;
        
        Ok(Self {
            layer_idx,
            attention_norm: candle_nn::rms_norm(hidden_size, config.rms_norm_eps, vb.pp("input_layernorm"))?,
            ffn_norm: candle_nn::rms_norm(hidden_size, config.rms_norm_eps, vb.pp("post_attention_layernorm"))?,
            wq: linear_no_bias(hidden_size, n_head * head_dim, vb.pp("self_attn.q_proj"))?,
            wk: linear_no_bias(hidden_size, n_kv_head * head_dim, vb.pp("self_attn.k_proj"))?,
            wv: linear_no_bias(hidden_size, n_kv_head * head_dim, vb.pp("self_attn.v_proj"))?,
            wo: linear_no_bias(n_head * head_dim, hidden_size, vb.pp("self_attn.o_proj"))?,
            w1: linear_no_bias(hidden_size, intermediate_size, vb.pp("mlp.gate_proj"))?,
            w2: linear_no_bias(intermediate_size, hidden_size, vb.pp("mlp.down_proj"))?,
            w3: linear_no_bias(hidden_size, intermediate_size, vb.pp("mlp.up_proj"))?,
            n_head,
            n_kv_head,
            head_dim,
            span: tracing::span!(tracing::Level::TRACE, "layer", idx = layer_idx),
        })
    }
    
    fn forward(&self, x: &Tensor, rope: &RotaryEmbedding, pos: usize) -> Result<Tensor, ShardedModelError> {
        let _enter = self.span.enter();
        let (b_sz, seq_len, _hidden_size) = x.dims3()?;
        
        // Attention block with residual
        let residual = x.clone();
        let x = self.attention_norm.forward(x)?;
        
        let q = self.wq.forward(&x)?;
        let k = self.wk.forward(&x)?;
        let v = self.wv.forward(&x)?;
        
        // Reshape for attention
        let q = q.reshape((b_sz, seq_len, self.n_head, self.head_dim))?.transpose(1, 2)?;
        let k = k.reshape((b_sz, seq_len, self.n_kv_head, self.head_dim))?.transpose(1, 2)?;
        let v = v.reshape((b_sz, seq_len, self.n_kv_head, self.head_dim))?.transpose(1, 2)?;
        
        // Apply RoPE
        let q = rope.forward(&q, pos)?;
        let k = rope.forward(&k, pos)?;
        
        // Repeat KV if needed (GQA)
        let k = repeat_kv(k, self.n_head / self.n_kv_head)?;
        let v = repeat_kv(v, self.n_head / self.n_kv_head)?;
        
        // Scaled dot-product attention
        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let attn_weights = (q.matmul(&k.transpose(2, 3)?)? * scale)?;
        let attn_weights = candle_nn::ops::softmax(&attn_weights, candle_core::D::Minus1)?;
        let attn_output = attn_weights.matmul(&v)?;
        
        let attn_output = attn_output
            .transpose(1, 2)?
            .reshape((b_sz, seq_len, self.n_head * self.head_dim))?;
            
        let attn_output = self.wo.forward(&attn_output)?;
        let x = (residual + attn_output)?;
        
        // FFN block with residual
        let residual = x.clone();
        let x = self.ffn_norm.forward(&x)?;
        
        // SwiGLU FFN
        let gate = self.w1.forward(&x)?;
        let gate = candle_nn::ops::silu(&gate)?;
        let up = self.w3.forward(&x)?;
        let ffn_output = (gate * up)?;
        let ffn_output = self.w2.forward(&ffn_output)?;
        
        let output = (residual + ffn_output)?;
        
        Ok(output)
    }
}

fn repeat_kv(x: Tensor, n_rep: usize) -> Result<Tensor, candle_core::Error> {
    if n_rep == 1 {
        Ok(x)
    } else {
        let (b, n_kv_head, seq_len, head_dim) = x.dims4()?;
        let x = x.unsqueeze(2)?.expand((b, n_kv_head, n_rep, seq_len, head_dim))?;
        x.reshape((b, n_kv_head * n_rep, seq_len, head_dim))
    }
}

fn load_llama_config(model_path: &str) -> Result<LlamaConfig, ShardedModelError> {
    let config_path = Path::new(model_path).join("config.json");
    if config_path.exists() {
        let file = fs::File::open(config_path)?;
        let config: LlamaConfigJson = serde_json::from_reader(file).map_err(|e| ShardedModelError::ConfigError(e.to_string()))?;
        Ok(config.into())
    } else {
        // Fallback for demo (Qwen 0.5B default)
        warn!("⚠️ No config.json found in {}, using default Qwen-0.5B config", model_path);
        Ok(LlamaConfig {
            hidden_size: 896,
            intermediate_size: 4864,
            vocab_size: 151936,
            num_hidden_layers: 24,
            num_attention_heads: 14,
            num_key_value_heads: 2,
            rms_norm_eps: 1e-6,
            rope_theta: 1000000.0,
            use_flash_attn: false,
            max_position_embeddings: 32768,
            bos_token_id: Some(1),
            eos_token_id: Some(LlamaEosToks::Single(2)),
            rope_scaling: None,
            tie_word_embeddings: false,
        })
    }
}

/// Info about a model shard
#[derive(Debug, Clone)]
pub struct ShardInfo {
    pub role: String,
    pub start_layer: u32,
    pub end_layer: u32,
    pub num_layers: u32,
    pub total_layers: u32,
    pub hidden_size: u32,
    pub vocab_size: u32,
}

/// Errors in sharded model operations
#[derive(Debug, thiserror::Error)]
pub enum ShardedModelError {
    #[error("Candle error: {0}")]
    CandleError(#[from] candle_core::Error),
    
    #[error("Missing layer: {0}")]
    MissingLayer(String),
    
    #[error("Config error: {0}")]
    ConfigError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
