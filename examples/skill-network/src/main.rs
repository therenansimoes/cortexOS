use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, Level};

use async_trait::async_trait;
use cortex_grid::NodeId;
use cortex_reputation::{EigenTrust, Rating, SkillId, TrustGraph};
use cortex_skill::{
    LocalSkillRegistry, NetworkSkillRegistry, Result as SkillResult, Skill, SkillCapability,
    SkillInput, SkillMetadata, SkillOutput, SkillRouter, SkillTask,
};

/// A simple "math" skill for demonstration
struct MathSkill {
    metadata: SkillMetadata,
    specialization: String,
}

impl MathSkill {
    fn new(specialization: &str) -> Self {
        Self {
            metadata: SkillMetadata::new(
                "math.compute",
                "Math Computation",
                &format!("Mathematical computation ({} specialist)", specialization),
            ),
            specialization: specialization.to_string(),
        }
    }
}

#[async_trait]
impl Skill for MathSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    async fn execute(&self, input: SkillInput) -> SkillResult<SkillOutput> {
        let expr = input.get_text().unwrap_or_default();

        // Simulate computation (in real world, this would be actual math)
        let result = format!(
            "[{} specialist] Computed: {} = 42",
            self.specialization, expr
        );

        Ok(SkillOutput::new().with_text(&result))
    }
}

/// A "translation" skill
struct TranslationSkill {
    metadata: SkillMetadata,
    language: String,
}

impl TranslationSkill {
    fn new(language: &str) -> Self {
        Self {
            metadata: SkillMetadata::new(
                &format!("translate.{}", language),
                &format!("{} Translation", language),
                &format!("Translate text to {}", language),
            ),
            language: language.to_string(),
        }
    }
}

#[async_trait]
impl Skill for TranslationSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    async fn execute(&self, input: SkillInput) -> SkillResult<SkillOutput> {
        let text = input.get_text().unwrap_or_default();

        // Mock translation
        let result = format!("[Translated to {}]: {}", self.language, text);

        Ok(SkillOutput::new().with_text(&result))
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("🧠 CortexOS Skill Network Demo");
    info!("   Demonstrating decentralized AI with reputation-based routing");
    info!("");

    // Create 4 nodes with different specializations
    let node_alice = NodeId::random();
    let node_bob = NodeId::random();
    let node_carol = NodeId::random();
    let node_dave = NodeId::random();

    info!("Creating 4 specialized nodes:");
    info!("  🔢 Alice (Math expert):       {}", node_alice);
    info!("  🌐 Bob (Translation expert):  {}", node_bob);
    info!("  🔢 Carol (Math novice):       {}", node_carol);
    info!("  🌐 Dave (Translation novice): {}", node_dave);
    info!("");

    // Create trust graph from Alice's perspective
    let mut graph = TrustGraph::new(node_alice);
    // Add Bob as pre-trusted (bootstrap trust)
    graph.add_pre_trusted(node_bob);
    let trust_graph = Arc::new(RwLock::new(graph));

    // Create network skill registry
    let skill_registry = Arc::new(RwLock::new(NetworkSkillRegistry::new(node_alice)));

    // Register skills for each node
    {
        let mut registry = skill_registry.write().await;
        registry.register_node_skill(node_alice, SkillId::new("math.compute"));
        registry.register_node_skill(node_bob, SkillId::new("translate.spanish"));
        registry.register_node_skill(node_bob, SkillId::new("translate.french"));
        registry.register_node_skill(node_carol, SkillId::new("math.compute"));
        registry.register_node_skill(node_dave, SkillId::new("translate.spanish"));
    }

    info!("📊 Simulating reputation history...");
    info!("");

    // Simulate rating history
    {
        let graph = trust_graph.write().await;

        // Alice rates others based on past interactions
        // Bob is excellent at translation
        graph
            .rate(node_bob, "translate.spanish".into(), Rating::positive())
            .unwrap();
        graph
            .rate(node_bob, "translate.spanish".into(), Rating::positive())
            .unwrap();
        graph
            .rate(node_bob, "translate.spanish".into(), Rating::positive())
            .unwrap();
        graph
            .rate(node_bob, "translate.french".into(), Rating::positive())
            .unwrap();

        // Dave is mediocre at translation
        graph
            .rate(node_dave, "translate.spanish".into(), Rating::positive())
            .unwrap();
        graph
            .rate(node_dave, "translate.spanish".into(), Rating::negative())
            .unwrap();

        // Carol is not great at math
        graph
            .rate(node_carol, "math.compute".into(), Rating::negative())
            .unwrap();
        graph
            .rate(node_carol, "math.compute".into(), Rating::positive())
            .unwrap();
        graph
            .rate(node_carol, "math.compute".into(), Rating::negative())
            .unwrap();
    }

    // Show ratings
    info!("Current skill ratings:");
    {
        let graph = trust_graph.read().await;

        if let Some(rating) = graph.get_skill_rating(&node_bob, &"translate.spanish".into()) {
            info!(
                "  Bob (translate.spanish): +{} / -{} = {:.2}",
                rating.positive_count,
                rating.negative_count,
                rating.normalized_score()
            );
        }

        if let Some(rating) = graph.get_skill_rating(&node_dave, &"translate.spanish".into()) {
            info!(
                "  Dave (translate.spanish): +{} / -{} = {:.2}",
                rating.positive_count,
                rating.negative_count,
                rating.normalized_score()
            );
        }

        if let Some(rating) = graph.get_skill_rating(&node_carol, &"math.compute".into()) {
            info!(
                "  Carol (math.compute): +{} / -{} = {:.2}",
                rating.positive_count,
                rating.negative_count,
                rating.normalized_score()
            );
        }
    }
    info!("");

    // Compute global trust using EigenTrust
    info!("🔄 Computing global trust scores (EigenTrust)...");
    {
        let graph = trust_graph.read().await;
        let eigentrust = EigenTrust::new();
        eigentrust.update_graph(&graph);
    }
    info!("");

    // Create router
    let router = SkillRouter::new(
        node_alice,
        Arc::clone(&trust_graph),
        Arc::clone(&skill_registry),
    );

    // Route a translation task
    info!("📨 Routing task: 'Translate to Spanish'");
    let task = SkillTask::new(
        "translate.spanish".into(),
        SkillInput::new().with_text("Hello, world!"),
        node_alice,
    );

    match router.route(&task).await {
        Ok(decision) => {
            info!("   ✅ Routed to: {}", decision.node);
            info!("      Trust score: {:.2}", decision.trust_score.value());
            info!("      Skill score: {:.2}", decision.skill_score);
            info!("      Route score: {:.2}", decision.route_score);

            if decision.node == node_bob {
                info!("   📝 Correctly chose Bob (the expert)!");
            }
        }
        Err(e) => {
            info!("   ❌ Routing failed: {}", e);
        }
    }
    info!("");

    // Route a math task
    info!("📨 Routing task: 'Math computation'");
    let task = SkillTask::new(
        "math.compute".into(),
        SkillInput::new().with_text("2 + 2"),
        node_alice,
    );

    match router.route(&task).await {
        Ok(decision) => {
            info!("   ✅ Routed to: {}", decision.node);
            info!("      Skill score: {:.2}", decision.skill_score);

            // Show alternatives
            if !decision.alternatives.is_empty() {
                info!("   📋 Alternatives:");
                for (alt_node, score) in &decision.alternatives {
                    info!("      - {} (score: {:.2})", alt_node, score);
                }
            }
        }
        Err(e) => {
            info!("   ❌ Routing failed: {}", e);
        }
    }
    info!("");

    // Show skill distribution
    info!("📈 Network skill distribution:");
    {
        let registry = skill_registry.read().await;
        let dist = registry.skill_distribution();
        for (skill, count) in dist {
            info!("   {}: {} nodes", skill, count);
        }
    }
    info!("");

    // Demonstrate finding top nodes
    info!("🏆 Top nodes for 'translate.spanish':");
    {
        let graph = trust_graph.read().await;
        let top = graph.top_nodes_for_skill(&"translate.spanish".into(), 5);
        for (i, (node, rating)) in top.iter().enumerate() {
            info!(
                "   {}. {} (score: {:.2}, ratings: {})",
                i + 1,
                node,
                rating.normalized_score(),
                rating.total_ratings()
            );
        }
    }
    info!("");

    info!("✅ Demo complete!");
    info!("");
    info!("Key concepts demonstrated:");
    info!("  • Nodes specialize in different skills");
    info!("  • Nodes rate each other after interactions");
    info!("  • EigenTrust computes global reputation");
    info!("  • Router picks best node based on skill + trust");
    info!("  • Network self-organizes around competence");
}
