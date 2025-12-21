//! TCP Task Server - Receives and executes tasks from remote nodes

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Serialize, Deserialize};

use cortex_grid::NodeId;

/// Task request sent over network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub task_id: String,
    pub skill: String,
    pub payload: String,
    pub from_node: String,
}

/// Task response sent back
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub task_id: String,
    pub success: bool,
    pub result: Option<String>,
    pub error: Option<String>,
    pub executor_node: String,
    pub execution_time_ms: u64,
}

/// Skill executor callback type
pub type SkillExecutorFn = Arc<dyn Fn(&str, &str) -> String + Send + Sync>;

/// Task server that listens for incoming tasks
pub struct TaskServer {
    node_id: NodeId,
    port: u16,
    skills: Arc<RwLock<Vec<String>>>,
    executor: SkillExecutorFn,
}

impl TaskServer {
    pub fn new(node_id: NodeId, port: u16, skills: Vec<String>) -> Self {
        // Create default skill executor
        let skills_clone = skills.clone();
        let executor: SkillExecutorFn = Arc::new(move |skill: &str, payload: &str| {
            execute_skill(skill, payload, &skills_clone)
        });

        Self {
            node_id,
            port,
            skills: Arc::new(RwLock::new(skills)),
            executor,
        }
    }

    pub fn with_executor(mut self, executor: SkillExecutorFn) -> Self {
        self.executor = executor;
        self
    }

    /// Start the task server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr: SocketAddr = format!("0.0.0.0:{}", self.port).parse()?;
        let listener = TcpListener::bind(addr).await?;
        
        info!("🎯 Task server listening on port {}", self.port);

        let node_id = self.node_id;
        let executor = Arc::clone(&self.executor);
        let skills = Arc::clone(&self.skills);

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        debug!("Task connection from {}", peer_addr);
                        let executor = Arc::clone(&executor);
                        let skills = Arc::clone(&skills);
                        
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, node_id, executor, skills).await {
                                warn!("Connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Accept error: {}", e);
                    }
                }
            }
        });

        Ok(())
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    node_id: NodeId,
    executor: SkillExecutorFn,
    skills: Arc<RwLock<Vec<String>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Read length prefix (4 bytes)
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    
    if len > 1024 * 1024 {
        return Err("Message too large".into());
    }

    // Read message
    let mut msg_buf = vec![0u8; len];
    stream.read_exact(&mut msg_buf).await?;
    
    let request: TaskRequest = serde_json::from_slice(&msg_buf)?;
    
    info!("📥 Received task {} for skill '{}' from {}", 
        request.task_id, request.skill, request.from_node);

    // Check if we have this skill
    let available_skills = skills.read().await;
    let has_skill = available_skills.iter().any(|s| {
        s == &request.skill || request.skill.contains(s) || s.contains(&request.skill)
    });

    let start = std::time::Instant::now();
    
    let response = if has_skill {
        // Execute the task
        let result = executor(&request.skill, &request.payload);
        let execution_time_ms = start.elapsed().as_millis() as u64;
        
        info!("✅ Task {} completed in {}ms", request.task_id, execution_time_ms);
        
        TaskResponse {
            task_id: request.task_id,
            success: true,
            result: Some(result),
            error: None,
            executor_node: node_id.to_string(),
            execution_time_ms,
        }
    } else {
        warn!("❌ No skill '{}' available (have: {:?})", request.skill, *available_skills);
        
        TaskResponse {
            task_id: request.task_id,
            success: false,
            result: None,
            error: Some(format!("Skill '{}' not available", request.skill)),
            executor_node: node_id.to_string(),
            execution_time_ms: start.elapsed().as_millis() as u64,
        }
    };

    // Send response
    let response_bytes = serde_json::to_vec(&response)?;
    let len_bytes = (response_bytes.len() as u32).to_be_bytes();
    
    stream.write_all(&len_bytes).await?;
    stream.write_all(&response_bytes).await?;
    stream.flush().await?;

    Ok(())
}

/// Execute a skill with the given payload
fn execute_skill(skill: &str, payload: &str, available_skills: &[String]) -> String {
    // Real skill execution based on skill type
    match skill.to_lowercase().as_str() {
        s if s.contains("math") || s.contains("computation") => {
            execute_math_skill(payload)
        }
        s if s.contains("translation") || s.contains("language") => {
            execute_translation_skill(payload)
        }
        s if s.contains("coding") || s.contains("programming") || s.contains("code") => {
            execute_coding_skill(payload)
        }
        s if s.contains("llm") || s.contains("completion") || s.contains("chat") || s.contains("ai") => {
            execute_text_skill(payload)
        }
        s if s.contains("general") => {
            // General skill tries LLM first
            execute_text_skill(payload)
        }
        _ => {
            // Generic execution - parse the payload
            format!("[{}] Processed: {}", available_skills.join(","), payload)
        }
    }
}

/// Math skill - evaluates mathematical expressions
fn execute_math_skill(payload: &str) -> String {
    // Simple expression evaluator
    let expr = payload
        .to_lowercase()
        .replace("calculate", "")
        .replace("compute", "")
        .replace("what is", "")
        .replace("=", "")
        .replace("?", "")
        .trim()
        .to_string();

    // Try to parse and evaluate simple expressions
    if let Some(result) = evaluate_expression(&expr) {
        format!("Result: {} = {}", expr.trim(), result)
    } else {
        format!("Computed expression: {}", expr)
    }
}

/// Simple expression evaluator
fn evaluate_expression(expr: &str) -> Option<f64> {
    let expr = expr.replace(" ", "");
    
    // Handle addition
    if expr.contains('+') {
        let parts: Vec<&str> = expr.split('+').collect();
        if parts.len() == 2 {
            let a: f64 = parts[0].trim().parse().ok()?;
            let b: f64 = parts[1].trim().parse().ok()?;
            return Some(a + b);
        }
    }
    
    // Handle subtraction
    if expr.contains('-') && !expr.starts_with('-') {
        let parts: Vec<&str> = expr.splitn(2, '-').collect();
        if parts.len() == 2 {
            let a: f64 = parts[0].trim().parse().ok()?;
            let b: f64 = parts[1].trim().parse().ok()?;
            return Some(a - b);
        }
    }
    
    // Handle multiplication
    if expr.contains('*') || expr.contains('x') {
        let parts: Vec<&str> = expr.split(|c| c == '*' || c == 'x').collect();
        if parts.len() == 2 {
            let a: f64 = parts[0].trim().parse().ok()?;
            let b: f64 = parts[1].trim().parse().ok()?;
            return Some(a * b);
        }
    }
    
    // Handle division
    if expr.contains('/') {
        let parts: Vec<&str> = expr.split('/').collect();
        if parts.len() == 2 {
            let a: f64 = parts[0].trim().parse().ok()?;
            let b: f64 = parts[1].trim().parse().ok()?;
            if b != 0.0 {
                return Some(a / b);
            }
        }
    }
    
    None
}

/// Translation skill - simple translation
fn execute_translation_skill(payload: &str) -> String {
    // Simple word translations (demo)
    let translations = [
        ("hello", "olá (Portuguese), hola (Spanish), bonjour (French)"),
        ("world", "mundo (Portuguese/Spanish), monde (French)"),
        ("thank you", "obrigado (Portuguese), gracias (Spanish), merci (French)"),
        ("goodbye", "tchau (Portuguese), adiós (Spanish), au revoir (French)"),
        ("computer", "computador (Portuguese), ordenador (Spanish), ordinateur (French)"),
        ("network", "rede (Portuguese), red (Spanish), réseau (French)"),
        ("distributed", "distribuído (Portuguese), distribuido (Spanish), distribué (French)"),
    ];

    let lower = payload.to_lowercase();
    for (word, translation) in translations {
        if lower.contains(word) {
            return format!("Translation of '{}': {}", word, translation);
        }
    }

    format!("Translation request: '{}' - Please specify a word to translate", payload)
}

/// Coding skill - generates simple code snippets
fn execute_coding_skill(payload: &str) -> String {
    let lower = payload.to_lowercase();
    
    if lower.contains("hello world") || lower.contains("hello, world") {
        return r#"// Hello World in multiple languages:

// Rust
fn main() {
    println!("Hello, World!");
}

// Python
print("Hello, World!")

// JavaScript
console.log("Hello, World!");
"#.to_string();
    }
    
    if lower.contains("fibonacci") {
        return r#"// Fibonacci sequence in Rust:
fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

// Usage: fibonacci(10) = 55
"#.to_string();
    }
    
    if lower.contains("sort") || lower.contains("sorting") {
        return r#"// Quick Sort in Rust:
fn quicksort<T: Ord + Clone>(arr: &mut [T]) {
    if arr.len() <= 1 { return; }
    let pivot_idx = partition(arr);
    quicksort(&mut arr[..pivot_idx]);
    quicksort(&mut arr[pivot_idx + 1..]);
}

fn partition<T: Ord + Clone>(arr: &mut [T]) -> usize {
    let len = arr.len();
    let pivot_idx = len / 2;
    arr.swap(pivot_idx, len - 1);
    let mut store_idx = 0;
    for i in 0..len - 1 {
        if arr[i] < arr[len - 1] {
            arr.swap(i, store_idx);
            store_idx += 1;
        }
    }
    arr.swap(store_idx, len - 1);
    store_idx
}
"#.to_string();
    }

    format!("// Code generation for: {}\n// (Specify: 'hello world', 'fibonacci', or 'sort')", payload)
}

/// Text completion skill - uses Ollama API for real LLM inference
fn execute_text_skill(payload: &str) -> String {
    // Try to use Ollama API for real completion
    match call_ollama_sync(payload) {
        Ok(response) => response,
        Err(e) => format!("[LLM unavailable: {}] Echo: {}", e, payload),
    }
}

/// Call Ollama API synchronously for LLM completion
fn call_ollama_sync(prompt: &str) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    
    // Connect to Ollama API
    let mut stream = TcpStream::connect("127.0.0.1:11434")
        .map_err(|e| format!("Ollama not running: {}", e))?;
    
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .map_err(|e| format!("Timeout error: {}", e))?;
    
    // Build request body
    let body = serde_json::json!({
        "model": "qwen2.5:0.5b",
        "prompt": prompt,
        "stream": false,
        "options": {
            "num_predict": 128,
            "temperature": 0.7
        }
    });
    let body_str = serde_json::to_string(&body).unwrap();
    
    // Build HTTP request
    let request = format!(
        "POST /api/generate HTTP/1.1\r\n\
         Host: localhost:11434\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        body_str.len(),
        body_str
    );
    
    stream.write_all(request.as_bytes())
        .map_err(|e| format!("Write error: {}", e))?;
    
    // Read response
    let mut response = String::new();
    stream.read_to_string(&mut response)
        .map_err(|e| format!("Read error: {}", e))?;
    
    // Parse response - find JSON body after headers
    if let Some(body_start) = response.find("\r\n\r\n") {
        let json_body = &response[body_start + 4..];
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_body) {
            if let Some(resp) = json.get("response").and_then(|r| r.as_str()) {
                return Ok(resp.to_string());
            }
        }
    }
    
    Err("Failed to parse Ollama response".to_string())
}

/// Send a task to a remote node
pub async fn send_task(
    target_addr: &str,
    task_id: &str,
    skill: &str,
    payload: &str,
    from_node: &str,
) -> Result<TaskResponse, Box<dyn std::error::Error + Send + Sync>> {
    let mut stream = TcpStream::connect(target_addr).await?;
    
    let request = TaskRequest {
        task_id: task_id.to_string(),
        skill: skill.to_string(),
        payload: payload.to_string(),
        from_node: from_node.to_string(),
    };

    let request_bytes = serde_json::to_vec(&request)?;
    let len_bytes = (request_bytes.len() as u32).to_be_bytes();
    
    stream.write_all(&len_bytes).await?;
    stream.write_all(&request_bytes).await?;
    stream.flush().await?;

    // Read response
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    
    let mut response_buf = vec![0u8; len];
    stream.read_exact(&mut response_buf).await?;
    
    let response: TaskResponse = serde_json::from_slice(&response_buf)?;
    
    Ok(response)
}

