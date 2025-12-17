use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info};

use cortex_grid::NodeId;
use cortex_reputation::SkillId;

use crate::definition::{Skill, SkillMetadata};

/// Registry of locally available skills
pub struct LocalSkillRegistry {
    skills: HashMap<SkillId, Arc<dyn Skill>>,
}

impl LocalSkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Register a skill
    pub fn register(&mut self, skill: Arc<dyn Skill>) {
        let id = skill.metadata().id.clone();
        info!("Registered local skill: {}", id);
        self.skills.insert(id, skill);
    }

    /// Get a skill by ID
    pub fn get(&self, id: &SkillId) -> Option<Arc<dyn Skill>> {
        self.skills.get(id).cloned()
    }

    /// List all skill IDs
    pub fn list_skills(&self) -> Vec<SkillId> {
        self.skills.keys().cloned().collect()
    }

    /// List all skill metadata
    pub fn list_metadata(&self) -> Vec<SkillMetadata> {
        self.skills.values().map(|s| s.metadata().clone()).collect()
    }

    /// Check if we have a skill
    pub fn has_skill(&self, id: &SkillId) -> bool {
        self.skills.contains_key(id)
    }
}

impl Default for LocalSkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Network-wide skill registry (who has what)
pub struct NetworkSkillRegistry {
    /// node -> skills they have
    node_skills: DashMap<NodeId, HashSet<SkillId>>,
    /// skill -> nodes that have it
    skill_nodes: DashMap<SkillId, HashSet<NodeId>>,
    /// My skills
    my_id: NodeId,
    my_skills: HashSet<SkillId>,
}

impl NetworkSkillRegistry {
    pub fn new(my_id: NodeId) -> Self {
        Self {
            node_skills: DashMap::new(),
            skill_nodes: DashMap::new(),
            my_id,
            my_skills: HashSet::new(),
        }
    }

    /// Register that a node has a skill
    pub fn register_node_skill(&self, node: NodeId, skill: SkillId) {
        // Update node -> skills
        self.node_skills
            .entry(node)
            .or_insert_with(HashSet::new)
            .insert(skill.clone());

        // Update skill -> nodes
        self.skill_nodes
            .entry(skill.clone())
            .or_insert_with(HashSet::new)
            .insert(node);

        debug!("Registered node {} has skill {}", node, skill);
    }

    /// Register multiple skills for a node
    pub fn register_node_skills(&self, node: NodeId, skills: Vec<SkillId>) {
        for skill in skills {
            self.register_node_skill(node, skill);
        }
    }

    /// Register my own skill
    pub fn register_my_skill(&mut self, skill: SkillId) {
        self.my_skills.insert(skill.clone());
        self.register_node_skill(self.my_id, skill);
    }

    /// Get all nodes that have a skill
    pub fn nodes_with_skill(&self, skill: &SkillId) -> Vec<NodeId> {
        self.skill_nodes
            .get(skill)
            .map(|nodes| nodes.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all skills a node has
    pub fn skills_of_node(&self, node: &NodeId) -> Vec<SkillId> {
        self.node_skills
            .get(node)
            .map(|skills| skills.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all known skills
    pub fn all_skills(&self) -> Vec<SkillId> {
        self.skill_nodes.iter().map(|e| e.key().clone()).collect()
    }

    /// Get all known nodes
    pub fn all_nodes(&self) -> Vec<NodeId> {
        self.node_skills.iter().map(|e| *e.key()).collect()
    }

    /// Remove a node (when they go offline)
    pub fn remove_node(&self, node: &NodeId) {
        if let Some((_, skills)) = self.node_skills.remove(node) {
            for skill in skills {
                if let Some(mut nodes) = self.skill_nodes.get_mut(&skill) {
                    nodes.remove(node);
                }
            }
        }
    }

    /// Count of nodes for each skill
    pub fn skill_distribution(&self) -> HashMap<SkillId, usize> {
        self.skill_nodes
            .iter()
            .map(|e| (e.key().clone(), e.value().len()))
            .collect()
    }
}

/// Message for announcing skills
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SkillAnnouncement {
    /// I have these skills
    Announce {
        node: NodeId,
        skills: Vec<SkillMetadata>,
    },
    /// I no longer have this skill
    Withdraw {
        node: NodeId,
        skill: SkillId,
    },
    /// Request: what skills does this node have?
    Query {
        node: NodeId,
    },
    /// Response to query
    QueryResponse {
        node: NodeId,
        skills: Vec<SkillMetadata>,
    },
    /// Request: who has this skill?
    WhoHas {
        skill: SkillId,
    },
    /// Response: these nodes have it
    WhoHasResponse {
        skill: SkillId,
        nodes: Vec<NodeId>,
    },
}
