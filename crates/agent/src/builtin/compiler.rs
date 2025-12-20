use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::context::AgentContext;
use crate::error::AgentError;
use crate::traits::Agent;
use crate::types::{AgentId, CapabilitySet, Event, ThoughtContent};

/// Request for code generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenRequest {
    pub task_description: String,
    pub language: String,
    pub context: Option<String>,
    pub constraints: Vec<String>,
}

/// Response from code generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenResponse {
    pub code: String,
    pub language: String,
    pub quality_score: f64,
    pub compilation_success: bool,
    pub validation_notes: Vec<String>,
}

/// Quality metrics for generated code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeQualityMetrics {
    pub syntax_valid: bool,
    pub has_documentation: bool,
    pub has_error_handling: bool,
    pub follows_conventions: bool,
    pub overall_score: f64,
}

impl CodeQualityMetrics {
    pub fn calculate_score(&mut self) {
        let mut score = 0.0;
        let mut total = 0.0;

        if self.syntax_valid {
            score += 40.0;
        }
        total += 40.0;

        if self.has_documentation {
            score += 20.0;
        }
        total += 20.0;

        if self.has_error_handling {
            score += 20.0;
        }
        total += 20.0;

        if self.follows_conventions {
            score += 20.0;
        }
        total += 20.0;

        self.overall_score = if total > 0.0 { score / total * 100.0 } else { 0.0 };
    }
}

/// Compiler Agent for AI-assisted code generation
pub struct CompilerAgent {
    id: AgentId,
    name: String,
    capabilities: CapabilitySet,
    
    // Statistics
    total_requests: u64,
    successful_compilations: u64,
    average_quality_score: f64,
    
    // Configuration
    enable_validation: bool,
    enable_compilation: bool,
    supported_languages: Vec<String>,
}

impl CompilerAgent {
    pub fn new() -> Self {
        Self {
            id: AgentId::new(),
            name: "compiler".to_string(),
            capabilities: CapabilitySet::new()
                .with_capability("code-generation")
                .with_capability("compilation")
                .with_capability("code-validation")
                .with_capability("syntax-checking"),
            total_requests: 0,
            successful_compilations: 0,
            average_quality_score: 0.0,
            enable_validation: true,
            enable_compilation: true,
            supported_languages: vec![
                "rust".to_string(),
                "python".to_string(),
                "javascript".to_string(),
                "typescript".to_string(),
            ],
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_validation(mut self, enable: bool) -> Self {
        self.enable_validation = enable;
        self
    }

    pub fn with_compilation(mut self, enable: bool) -> Self {
        self.enable_compilation = enable;
        self
    }

    pub fn add_language(mut self, language: impl Into<String>) -> Self {
        self.supported_languages.push(language.into());
        self
    }

    pub fn total_requests(&self) -> u64 {
        self.total_requests
    }

    pub fn successful_compilations(&self) -> u64 {
        self.successful_compilations
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_compilations as f64 / self.total_requests as f64) * 100.0
        }
    }

    pub fn average_quality_score(&self) -> f64 {
        self.average_quality_score
    }

    /// Generate code using LLM
    async fn generate_code(&self, request: &CodeGenRequest) -> Result<String, AgentError> {
        debug!(
            language = %request.language,
            task = %request.task_description,
            "Generating code"
        );

        // Build prompt for LLM
        let _prompt = self.build_code_prompt(request);

        // In a real implementation, this would call an LLM
        // For now, generate a basic template based on language
        let code = self.generate_template(&request.language, &request.task_description)?;

        Ok(code)
    }

    fn build_code_prompt(&self, request: &CodeGenRequest) -> String {
        let mut prompt = format!(
            "Generate {} code for the following task:\n\n{}\n\n",
            request.language, request.task_description
        );

        if let Some(context) = &request.context {
            prompt.push_str(&format!("Context:\n{}\n\n", context));
        }

        if !request.constraints.is_empty() {
            prompt.push_str("Constraints:\n");
            for constraint in &request.constraints {
                prompt.push_str(&format!("- {}\n", constraint));
            }
            prompt.push('\n');
        }

        prompt.push_str("Requirements:\n");
        prompt.push_str("- Include proper error handling\n");
        prompt.push_str("- Add documentation comments\n");
        prompt.push_str("- Follow language conventions\n");
        prompt.push_str("- Ensure code is production-ready\n");

        prompt
    }

    fn generate_template(&self, language: &str, task: &str) -> Result<String, AgentError> {
        let code = match language.to_lowercase().as_str() {
            "rust" => self.generate_rust_template(task),
            "python" => self.generate_python_template(task),
            "javascript" | "typescript" => self.generate_js_template(task),
            _ => {
                return Err(AgentError::Internal(format!(
                    "Unsupported language: {}",
                    language
                )))
            }
        };

        Ok(code)
    }

    fn generate_rust_template(&self, task: &str) -> String {
        format!(
            r#"/// {}
pub fn execute() -> Result<(), Box<dyn std::error::Error>> {{
    // TODO: Implement task logic
    
    Ok(())
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_execute() {{
        let result = execute();
        assert!(result.is_ok());
    }}
}}"#,
            task
        )
    }

    fn generate_python_template(&self, task: &str) -> String {
        format!(
            r#""""
{}
"""

def execute():
    """Execute the task"""
    # TODO: Implement task logic
    pass


if __name__ == "__main__":
    execute()
"#,
            task
        )
    }

    fn generate_js_template(&self, task: &str) -> String {
        format!(
            r#"/**
 * {}
 */
function execute() {{
    // TODO: Implement task logic
}}

module.exports = {{ execute }};
"#,
            task
        )
    }

    /// Validate generated code
    /// 
    /// Note: This is a basic heuristic validation. For production use,
    /// this should be replaced with actual syntax parsers and linters
    /// for each language (e.g., syn for Rust, ast for Python).
    fn validate_code(&self, code: &str, language: &str) -> CodeQualityMetrics {
        let mut metrics = CodeQualityMetrics {
            syntax_valid: false,
            has_documentation: false,
            has_error_handling: false,
            follows_conventions: false,
            overall_score: 0.0,
        };

        // Basic syntax validation - checks structure, not actual parsing
        // In production, integrate with language-specific parsers
        metrics.syntax_valid = self.basic_syntax_check(code, language);

        // Check for documentation (more robust pattern matching)
        metrics.has_documentation = self.check_documentation(code, language);

        // Check for error handling patterns (context-aware)
        metrics.has_error_handling = self.check_error_handling(code, language);

        // Check for basic conventions
        metrics.follows_conventions = self.check_conventions(code, language);

        metrics.calculate_score();
        metrics
    }

    fn basic_syntax_check(&self, code: &str, language: &str) -> bool {
        if code.is_empty() || code.len() < 10 {
            return false;
        }

        match language.to_lowercase().as_str() {
            "rust" => {
                // Check for basic Rust structure
                (code.contains("fn ") || code.contains("pub fn "))
                    && code.contains('{')
                    && code.contains('}')
            }
            "python" => {
                // Check for basic Python structure
                code.contains("def ") && code.contains(':')
            }
            "javascript" | "typescript" => {
                // Check for basic JS structure
                (code.contains("function ") || code.contains("const ") || code.contains("=>"))
                    && code.contains('{')
                    && code.contains('}')
            }
            _ => code.len() > 10,
        }
    }

    fn check_documentation(&self, code: &str, language: &str) -> bool {
        match language.to_lowercase().as_str() {
            "rust" => {
                // Look for doc comments at start of lines
                code.lines().any(|line| {
                    let trimmed = line.trim();
                    trimmed.starts_with("///") || trimmed.starts_with("//!")
                })
            }
            "python" => {
                // Look for docstrings (triple quotes)
                code.contains(r#"""""#) || code.contains("'''")
            }
            "javascript" | "typescript" => {
                // Look for JSDoc comments
                code.lines().any(|line| {
                    let trimmed = line.trim();
                    trimmed.starts_with("/**") || trimmed.starts_with("*")
                })
            }
            _ => false,
        }
    }

    fn check_error_handling(&self, code: &str, language: &str) -> bool {
        match language.to_lowercase().as_str() {
            "rust" => {
                // Check for Result type or error handling operators
                // Avoid false positives from comments by checking actual code context
                let has_result = code.contains("-> Result<");
                let has_question = code.lines().any(|line| {
                    let trimmed = line.trim();
                    !trimmed.starts_with("//") && trimmed.contains('?')
                });
                has_result || has_question
            }
            "python" => {
                // Look for try-except blocks
                code.lines().any(|line| {
                    let trimmed = line.trim();
                    trimmed.starts_with("try:") || trimmed.starts_with("except")
                })
            }
            "javascript" | "typescript" => {
                // Look for try-catch blocks
                code.contains("try") && code.contains("catch")
            }
            _ => false,
        }
    }

    fn check_conventions(&self, code: &str, language: &str) -> bool {
        match language.to_lowercase().as_str() {
            "rust" => {
                // Check for function definitions and visibility modifiers
                code.contains("fn ") || code.contains("pub ")
            }
            "python" => {
                // Check for function definitions
                code.contains("def ")
            }
            "javascript" | "typescript" => {
                // Check for modern JS patterns
                code.contains("function ") || code.contains("const ") || code.contains("=>")
            }
            _ => false,
        }
    }

    /// Simulate compilation check
    fn check_compilation(&self, _code: &str, language: &str) -> bool {
        // In a real implementation, this would invoke actual compilers
        // For now, just check if language is supported
        self.supported_languages
            .iter()
            .any(|l| l.eq_ignore_ascii_case(language))
    }

    /// Process code generation request
    async fn process_request(
        &mut self,
        request: CodeGenRequest,
        ctx: &mut AgentContext,
    ) -> Result<CodeGenResponse, AgentError> {
        self.total_requests += 1;

        info!(
            agent = %self.name,
            language = %request.language,
            "Processing code generation request"
        );

        // Generate code
        let code = self.generate_code(&request).await?;

        // Validate if enabled
        let quality_score = if self.enable_validation {
            let metrics = self.validate_code(&code, &request.language);
            debug!(
                score = metrics.overall_score,
                "Code quality metrics calculated"
            );
            metrics.overall_score
        } else {
            100.0
        };

        // Check compilation if enabled
        let compilation_success = if self.enable_compilation {
            self.check_compilation(&code, &request.language)
        } else {
            true
        };

        if compilation_success {
            self.successful_compilations += 1;
        }

        // Update average quality score
        let n = self.total_requests as f64;
        self.average_quality_score = 
            (self.average_quality_score * (n - 1.0) + quality_score) / n;

        let mut validation_notes = Vec::new();
        if quality_score < 80.0 {
            validation_notes.push(format!(
                "Quality score ({:.1}%) is below target of 80%",
                quality_score
            ));
        }
        if !compilation_success {
            validation_notes.push("Compilation check failed".to_string());
        }

        let response = CodeGenResponse {
            code: code.clone(),
            language: request.language.clone(),
            quality_score,
            compilation_success,
            validation_notes,
        };

        // Store in thought graph
        let thought = ThoughtContent::new(
            "code-generation",
            serde_json::to_vec(&serde_json::json!({
                "request": request,
                "response": response,
                "timestamp": crate::types::Timestamp::now().0,
            }))
            .unwrap_or_default(),
        );
        ctx.add_thought(thought).await?;

        info!(
            quality = quality_score,
            compilation = compilation_success,
            "Code generation completed"
        );

        Ok(response)
    }
}

impl Default for CompilerAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for CompilerAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    async fn init(&mut self, ctx: &mut AgentContext) -> Result<(), AgentError> {
        info!(
            agent_id = %self.id,
            agent_name = %self.name,
            "CompilerAgent initialized"
        );

        ctx.emit_event(
            "compiler.started",
            serde_json::to_vec(&serde_json::json!({
                "agent_id": self.id.to_string(),
                "agent_name": self.name,
                "supported_languages": self.supported_languages,
            }))
            .unwrap_or_default(),
        )
        .await?;

        Ok(())
    }

    async fn on_event(&mut self, event: &Event, ctx: &mut AgentContext) -> Result<(), AgentError> {
        // Handle code generation requests
        if event.kind.starts_with("compiler.generate") {
            match serde_json::from_slice::<CodeGenRequest>(&event.payload) {
                Ok(request) => {
                    debug!("Received code generation request");
                    
                    match self.process_request(request, ctx).await {
                        Ok(response) => {
                            ctx.emit_event(
                                "compiler.response",
                                serde_json::to_vec(&response).unwrap_or_default(),
                            )
                            .await?;
                        }
                        Err(e) => {
                            warn!(error = %e, "Code generation failed");
                            ctx.emit_event(
                                "compiler.error",
                                serde_json::to_vec(&serde_json::json!({
                                    "error": e.to_string(),
                                }))
                                .unwrap_or_default(),
                            )
                            .await?;
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Failed to parse code generation request");
                }
            }
        }

        Ok(())
    }

    async fn tick(&mut self, _ctx: &mut AgentContext) -> Result<(), AgentError> {
        // Compiler agent is reactive, no periodic tasks
        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut AgentContext) -> Result<(), AgentError> {
        info!(
            agent_id = %self.id,
            total_requests = self.total_requests,
            success_rate = self.success_rate(),
            avg_quality = self.average_quality_score,
            "CompilerAgent shutting down"
        );

        ctx.emit_event(
            "compiler.stopped",
            serde_json::to_vec(&serde_json::json!({
                "agent_id": self.id.to_string(),
                "agent_name": self.name,
                "total_requests": self.total_requests,
                "successful_compilations": self.successful_compilations,
                "success_rate": self.success_rate(),
                "average_quality_score": self.average_quality_score,
            }))
            .unwrap_or_default(),
        )
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_agent_creation() {
        let agent = CompilerAgent::new();
        assert_eq!(agent.name(), "compiler");
        assert!(agent.capabilities().has("code-generation"));
        assert!(agent.capabilities().has("compilation"));
    }

    #[test]
    fn test_quality_metrics() {
        let mut metrics = CodeQualityMetrics {
            syntax_valid: true,
            has_documentation: true,
            has_error_handling: true,
            follows_conventions: true,
            overall_score: 0.0,
        };

        metrics.calculate_score();
        assert_eq!(metrics.overall_score, 100.0);
    }

    #[test]
    fn test_quality_metrics_partial() {
        let mut metrics = CodeQualityMetrics {
            syntax_valid: true,
            has_documentation: false,
            has_error_handling: true,
            follows_conventions: false,
            overall_score: 0.0,
        };

        metrics.calculate_score();
        assert_eq!(metrics.overall_score, 60.0);
    }

    #[test]
    fn test_rust_template_generation() {
        let agent = CompilerAgent::new();
        let code = agent.generate_rust_template("Test task");
        assert!(code.contains("pub fn execute"));
        assert!(code.contains("Result"));
        assert!(code.contains("#[cfg(test)]"));
    }

    #[test]
    fn test_python_template_generation() {
        let agent = CompilerAgent::new();
        let code = agent.generate_python_template("Test task");
        assert!(code.contains("def execute"));
        assert!(code.contains("\"\"\""));
    }

    #[test]
    fn test_code_validation_rust() {
        let agent = CompilerAgent::new();
        let code = r#"
            /// Test function
            pub fn test() -> Result<(), Box<dyn std::error::Error>> {
                Ok(())
            }
        "#;
        let metrics = agent.validate_code(code, "rust");
        assert!(metrics.syntax_valid);
        assert!(metrics.has_documentation);
        assert!(metrics.has_error_handling);
        assert!(metrics.follows_conventions);
    }

    #[test]
    fn test_compilation_check() {
        let agent = CompilerAgent::new();
        assert!(agent.check_compilation("code", "rust"));
        assert!(agent.check_compilation("code", "python"));
        assert!(!agent.check_compilation("code", "fortran"));
    }

    #[test]
    fn test_success_rate() {
        let mut agent = CompilerAgent::new();
        assert_eq!(agent.success_rate(), 0.0);
        
        agent.total_requests = 10;
        agent.successful_compilations = 9;
        assert_eq!(agent.success_rate(), 90.0);
    }

    #[test]
    fn test_custom_name() {
        let agent = CompilerAgent::new().with_name("custom-compiler");
        assert_eq!(agent.name(), "custom-compiler");
    }

    #[test]
    fn test_add_language() {
        let agent = CompilerAgent::new().add_language("go");
        assert!(agent.supported_languages.contains(&"go".to_string()));
    }
}
