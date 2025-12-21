# LLM Inference Example

This example demonstrates local LLM inference using llama.cpp integration in CortexOS.

## Features

- Load GGUF format models
- Text completion
- Chat completion
- Code generation
- Tokenization

## Requirements

- A GGUF format model file (e.g., TinyLlama, CodeLlama, Mistral)
- Sufficient RAM for the model (varies by model size and quantization)
- Optional: CUDA/Metal support for GPU acceleration

## Downloading a Model

You can download GGUF models from Hugging Face. For example:

```bash
# Create a models directory
mkdir -p models

# Download TinyLlama (small, good for testing)
wget https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf \
  -O models/tinyllama.gguf
```

Other recommended models:
- **TinyLlama** (1.1B params): Fast, good for testing
- **CodeLlama** (7B/13B params): Specialized for code generation
- **Mistral** (7B params): General-purpose, high quality

## Building

```bash
# Build the example
cargo build --release --package llm-inference

# Or build with all workspace
cargo build --release
```

## Running

```bash
# Basic usage
cargo run --release --package llm-inference -- ./models/tinyllama.gguf

# Or run the built binary directly
./target/release/llm-inference ./models/tinyllama.gguf
```

## Example Output

```
=== CortexOS LLM Inference Demo ===
Model path: ./models/tinyllama.gguf
Loading model...
✓ Model loaded successfully

=== Tokenization Test ===
Text: "Hello, world!"
Tokens: [1, 15043, 29892, 3186, 29991] (count: 5)

=== Text Completion Test ===
Prompt: "Once upon a time"
Response: "there was a young girl named Alice who lived in a small village..."

=== Chat Completion Test ===
Chat messages:
  [System] You are a helpful assistant.
  [User] What is 2 + 2?
Assistant: "2 + 2 equals 4."

=== Code Generation Test ===
Prompt: "Write a Rust function that adds two numbers"
Generated code:
fn add(a: i32, b: i32) -> i32 {
    a + b
}

=== Cleanup ===
✓ Model unloaded

=== Demo Complete ===
```

## Configuration

The model configuration can be adjusted in `src/main.rs`:

- `context_size`: Maximum context window (default: 2048)
- `threads`: Number of CPU threads for inference (default: 4)
- `gpu_layers`: Number of layers to offload to GPU (default: 0 = CPU only)
- `temperature`: Sampling temperature for generation (default: 0.7)
- `max_tokens`: Maximum tokens to generate (default: 50)

## Performance Tips

1. **Use quantized models**: Q4_K_M or Q5_K_M offer good balance of size/quality
2. **GPU acceleration**: Set `gpu_layers` > 0 if you have CUDA/Metal support
3. **Adjust threads**: Match to your CPU cores for best performance
4. **Lower temperature**: Use 0.1-0.3 for code generation, 0.7-1.0 for creative text

## Troubleshooting

### "Model file not found"
Ensure the path to the GGUF file is correct and the file exists.

### "Out of memory"
Try a smaller model or increase your system's available RAM. Quantized models (Q4, Q5) use less memory.

### "No GPU support"
The default configuration uses CPU only. To enable GPU, ensure llama.cpp is built with CUDA or Metal support and set `gpu_layers` > 0.

## Next Steps

- Integrate with CortexOS agents for autonomous code generation
- Use for natural language understanding in the Grid protocol
- Implement fine-tuning for domain-specific tasks
- Add support for multi-modal models (vision + text)
