use cortex_agent::prelude::*;
use cortex_agent::builtin::compiler::{CodeGenRequest, CodeGenResponse};
use cortex_agent::{EventBusHandle, GraphStoreHandle, IntentionManager};
use tokio::sync::mpsc;
use tracing::{info, Level};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 CortexOS Compiler Agent Demo");
    info!("   Demonstrating AI-assisted code generation");
    info!("");

    // Setup infrastructure
    let event_bus = EventBusHandle::new(100);
    let graph = GraphStoreHandle::new();
    let intentions = IntentionManager::new();
    let (agent_spawn_tx, _agent_spawn_rx) = mpsc::channel(10);

    // Create compiler agent
    let mut compiler = CompilerAgent::new()
        .with_name("code-generator");

    let mut ctx = AgentContext::new(
        event_bus.clone(),
        graph.clone(),
        intentions.clone(),
        agent_spawn_tx,
    );

    // Subscribe to compiler responses BEFORE initializing
    let mut response_subscription = event_bus.subscribe();
    
    // Spawn a task to listen for responses
    tokio::spawn(async move {
        while let Ok(event) = response_subscription.recv().await {
            if event.kind == "compiler.response" {
                match serde_json::from_slice::<CodeGenResponse>(&event.payload) {
                    Ok(response) => {
                        info!("\n📝 Generated Code ({})", response.language);
                        info!("Quality Score: {:.1}%", response.quality_score);
                        info!("Compilation Success: {}", response.compilation_success);
                        if !response.validation_notes.is_empty() {
                            info!("Notes:");
                            for note in &response.validation_notes {
                                info!("  - {}", note);
                            }
                        }
                        info!("\nCode:\n{}", "=".repeat(60));
                        info!("{}", response.code);
                        info!("{}\n", "=".repeat(60));
                    }
                    Err(e) => {
                        info!("Failed to parse response: {}", e);
                    }
                }
            }
        }
    });

    // Initialize the agent
    compiler.init(&mut ctx).await.expect("Failed to initialize compiler");

    info!("✅ Compiler agent initialized\n");

    // Example 1: Generate Rust HTTP server
    info!("📋 Request 1: Generate Rust HTTP server");
    let request1 = CodeGenRequest {
        task_description: "Create an HTTP server that responds with 'Hello, World!'".to_string(),
        language: "rust".to_string(),
        context: Some("Using standard library only".to_string()),
        constraints: vec!["Must handle errors gracefully".to_string()],
    };

    let event1 = Event::new(
        "compiler.generate",
        serde_json::to_vec(&request1).unwrap(),
    );
    compiler.on_event(&event1, &mut ctx).await.expect("Failed to process request 1");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Example 2: Generate Python data processor
    info!("\n📋 Request 2: Generate Python data processor");
    let request2 = CodeGenRequest {
        task_description: "Create a function to process CSV data and calculate statistics".to_string(),
        language: "python".to_string(),
        context: None,
        constraints: vec![
            "Include error handling for file not found".to_string(),
            "Calculate mean, median, and standard deviation".to_string(),
        ],
    };

    let event2 = Event::new(
        "compiler.generate",
        serde_json::to_vec(&request2).unwrap(),
    );
    compiler.on_event(&event2, &mut ctx).await.expect("Failed to process request 2");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Example 3: Generate JavaScript API client
    info!("\n📋 Request 3: Generate JavaScript API client");
    let request3 = CodeGenRequest {
        task_description: "Create an API client for fetching user data".to_string(),
        language: "javascript".to_string(),
        context: Some("Uses fetch API".to_string()),
        constraints: vec!["Must handle network errors".to_string()],
    };

    let event3 = Event::new(
        "compiler.generate",
        serde_json::to_vec(&request3).unwrap(),
    );
    compiler.on_event(&event3, &mut ctx).await.expect("Failed to process request 3");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Show statistics
    info!("\n📊 Compiler Agent Statistics");
    info!("   Total Requests: {}", compiler.total_requests());
    info!("   Successful Compilations: {}", compiler.successful_compilations());
    info!("   Success Rate: {:.1}%", compiler.success_rate());
    info!("   Average Quality Score: {:.1}%", compiler.average_quality_score());

    // Shutdown
    compiler.shutdown(&mut ctx).await.expect("Failed to shutdown");
    info!("\n🛑 Demo completed");
}
