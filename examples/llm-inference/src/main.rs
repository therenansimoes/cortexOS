use cortex_inference::{ChatMessage, GenerationParams, LlamaModel, Model, ModelConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Check for model path argument
    let model_path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!(
            "Usage: {} <path-to-gguf-model>",
            std::env::args().next().unwrap()
        );
        eprintln!(
            "Example: {} ./models/tinyllama-1.1b-chat.Q4_K_M.gguf",
            std::env::args().next().unwrap()
        );
        std::process::exit(1);
    });

    println!("=== CortexOS LLM Inference Demo ===");
    println!("Model path: {}", model_path);

    // Create model config
    let config = ModelConfig::new(&model_path)
        .with_context_size(2048)
        .with_threads(4)
        .with_gpu_layers(0); // Use CPU only

    // Create and load model
    let mut model = LlamaModel::new("demo-model", config);

    println!("Loading model...");
    match model.load().await {
        Ok(_) => println!("✓ Model loaded successfully"),
        Err(e) => {
            eprintln!("✗ Failed to load model: {}", e);
            std::process::exit(1);
        }
    }

    // Test tokenization
    println!("\n=== Tokenization Test ===");
    let test_text = "Hello, world!";
    match model.tokenize(test_text) {
        Ok(tokens) => {
            println!("Text: \"{}\"", test_text);
            println!("Tokens: {:?} (count: {})", tokens, tokens.len());
        }
        Err(e) => eprintln!("Tokenization error: {}", e),
    }

    // Test text completion
    println!("\n=== Text Completion Test ===");
    let prompt = "Once upon a time";
    let params = GenerationParams {
        max_tokens: 50,
        temperature: 0.7,
        ..Default::default()
    };

    println!("Prompt: \"{}\"", prompt);
    print!("Response: ");
    match model.complete(prompt, &params).await {
        Ok(response) => println!("\"{}\"", response),
        Err(e) => eprintln!("Completion error: {}", e),
    }

    // Test chat completion
    println!("\n=== Chat Completion Test ===");
    let messages = vec![
        ChatMessage::system("You are a helpful assistant."),
        ChatMessage::user("What is 2 + 2?"),
    ];

    println!("Chat messages:");
    for msg in &messages {
        println!("  [{:?}] {}", msg.role, msg.content);
    }
    print!("Assistant: ");
    match model.chat(&messages, &params).await {
        Ok(response) => println!("\"{}\"", response),
        Err(e) => eprintln!("Chat error: {}", e),
    }

    // Test code generation
    println!("\n=== Code Generation Test ===");
    let code_prompt = "Write a Rust function that adds two numbers";
    let code_params = GenerationParams {
        max_tokens: 100,
        temperature: 0.2, // Lower temperature for code
        ..Default::default()
    };

    println!("Prompt: \"{}\"", code_prompt);
    print!("Generated code:\n");
    match model.complete(code_prompt, &code_params).await {
        Ok(code) => println!("{}", code),
        Err(e) => eprintln!("Code generation error: {}", e),
    }

    // Unload model
    println!("\n=== Cleanup ===");
    model.unload().await?;
    println!("✓ Model unloaded");

    println!("\n=== Demo Complete ===");
    Ok(())
}
