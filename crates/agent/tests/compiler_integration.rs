use cortex_agent::prelude::*;
use cortex_agent::builtin::compiler::{CodeGenRequest, CodeGenResponse};
use cortex_agent::{EventBusHandle, GraphStoreHandle, IntentionManager};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_compiler_agent_integration() {
    // Setup infrastructure
    let event_bus = EventBusHandle::new(100);
    let graph = GraphStoreHandle::new();
    let intentions = IntentionManager::new();
    let (agent_spawn_tx, _agent_spawn_rx) = mpsc::channel(10);

    // Create and initialize compiler agent
    let mut compiler = CompilerAgent::new();
    let mut ctx = AgentContext::new(
        event_bus.clone(),
        graph.clone(),
        intentions.clone(),
        agent_spawn_tx,
    );

    // Subscribe before init to avoid event bus errors
    let mut subscription = event_bus.subscribe();

    // Initialize should succeed
    assert!(compiler.init(&mut ctx).await.is_ok());

    // Create a code generation request
    let request = CodeGenRequest {
        task_description: "Create a test function".to_string(),
        language: "rust".to_string(),
        context: None,
        constraints: vec![],
    };

    // Send request
    let event = Event::new(
        "compiler.generate",
        serde_json::to_vec(&request).unwrap(),
    );

    assert!(compiler.on_event(&event, &mut ctx).await.is_ok());

    // Should receive a response event
    let mut received_response = false;
    for _ in 0..10 {
        if let Ok(event) = subscription.try_recv() {
            if event.kind == "compiler.response" {
                let response: CodeGenResponse = serde_json::from_slice(&event.payload).unwrap();
                assert!(!response.code.is_empty());
                assert_eq!(response.language, "rust");
                assert!(response.quality_score >= 0.0 && response.quality_score <= 100.0);
                received_response = true;
                break;
            }
        }
    }

    assert!(received_response, "Should receive compiler.response event");

    // Verify statistics
    assert_eq!(compiler.total_requests(), 1);
    assert_eq!(compiler.successful_compilations(), 1);
    assert_eq!(compiler.success_rate(), 100.0);

    // Shutdown should succeed
    assert!(compiler.shutdown(&mut ctx).await.is_ok());
}

#[tokio::test]
async fn test_compiler_agent_multiple_languages() {
    let event_bus = EventBusHandle::new(100);
    let graph = GraphStoreHandle::new();
    let intentions = IntentionManager::new();
    let (agent_spawn_tx, _agent_spawn_rx) = mpsc::channel(10);

    let mut compiler = CompilerAgent::new();
    let mut ctx = AgentContext::new(
        event_bus.clone(),
        graph.clone(),
        intentions.clone(),
        agent_spawn_tx,
    );

    let _subscription = event_bus.subscribe();
    compiler.init(&mut ctx).await.unwrap();

    // Test multiple languages
    let languages = vec!["rust", "python", "javascript"];
    
    for lang in languages {
        let request = CodeGenRequest {
            task_description: format!("Create a {} function", lang),
            language: lang.to_string(),
            context: None,
            constraints: vec![],
        };

        let event = Event::new(
            "compiler.generate",
            serde_json::to_vec(&request).unwrap(),
        );

        assert!(compiler.on_event(&event, &mut ctx).await.is_ok());
    }

    // Verify all were processed
    assert_eq!(compiler.total_requests(), 3);
    assert_eq!(compiler.success_rate(), 100.0);
}

#[tokio::test]
async fn test_compiler_agent_unsupported_language() {
    let event_bus = EventBusHandle::new(100);
    let graph = GraphStoreHandle::new();
    let intentions = IntentionManager::new();
    let (agent_spawn_tx, _agent_spawn_rx) = mpsc::channel(10);

    let mut compiler = CompilerAgent::new();
    let mut ctx = AgentContext::new(
        event_bus.clone(),
        graph.clone(),
        intentions.clone(),
        agent_spawn_tx,
    );

    let mut error_subscription = event_bus.subscribe();
    compiler.init(&mut ctx).await.unwrap();

    // Request unsupported language
    let request = CodeGenRequest {
        task_description: "Test".to_string(),
        language: "fortran".to_string(),
        context: None,
        constraints: vec![],
    };

    let event = Event::new(
        "compiler.generate",
        serde_json::to_vec(&request).unwrap(),
    );

    compiler.on_event(&event, &mut ctx).await.unwrap();

    // Should receive error event
    let mut received_error = false;
    for _ in 0..10 {
        if let Ok(event) = error_subscription.try_recv() {
            if event.kind == "compiler.error" {
                received_error = true;
                break;
            }
        }
    }

    assert!(received_error, "Should receive compiler.error event for unsupported language");
}

#[tokio::test]
async fn test_compiler_agent_capabilities() {
    let compiler = CompilerAgent::new();
    
    assert!(compiler.capabilities().has("code-generation"));
    assert!(compiler.capabilities().has("compilation"));
    assert!(compiler.capabilities().has("code-validation"));
    assert!(compiler.capabilities().has("syntax-checking"));
}

#[tokio::test]
async fn test_compiler_agent_custom_configuration() {
    let compiler = CompilerAgent::new()
        .with_name("custom-compiler")
        .with_validation(false)
        .with_compilation(false)
        .add_language("go");
    
    assert_eq!(compiler.name(), "custom-compiler");
}
