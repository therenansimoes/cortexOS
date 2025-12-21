use serde::{Deserialize, Serialize};

use crate::types::{EventId, NodeId, PrivacyLevel, Tag, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtNode {
    pub id: NodeId,
    pub content: ThoughtContent,
    pub created_at: Timestamp,
    pub tags: Vec<Tag>,
    pub privacy: PrivacyLevel,
}

impl ThoughtNode {
    pub fn new(content: ThoughtContent) -> Self {
        Self {
            id: NodeId::new(),
            content,
            created_at: Timestamp::now(),
            tags: Vec::new(),
            privacy: PrivacyLevel::Private,
        }
    }

    pub fn with_tags(mut self, tags: Vec<Tag>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_privacy(mut self, privacy: PrivacyLevel) -> Self {
        self.privacy = privacy;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThoughtContent {
    Perception {
        event_id: EventId,
        summary: String,
    },
    Intention {
        goal: String,
        status: IntentionStatus,
    },
    Action {
        description: String,
        outcome: Option<Outcome>,
    },
    Memory {
        text: String,
    },
    Concept {
        name: String,
        definition: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentionStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub success: bool,
    pub description: String,
    pub timestamp: Timestamp,
}

impl Outcome {
    pub fn success(description: impl Into<String>) -> Self {
        Self {
            success: true,
            description: description.into(),
            timestamp: Timestamp::now(),
        }
    }

    pub fn failure(description: impl Into<String>) -> Self {
        Self {
            success: false,
            description: description.into(),
            timestamp: Timestamp::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub relation: Relation,
    pub weight: f32,
}

impl ThoughtEdge {
    pub fn new(from: NodeId, to: NodeId, relation: Relation) -> Self {
        Self {
            from,
            to,
            relation,
            weight: 1.0,
        }
    }

    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relation {
    Causes,
    Contradicts,
    RemindsOf,
    PartOf,
    LeadsTo,
    Supports,
}

impl Relation {
    pub fn is_associative(&self) -> bool {
        matches!(self, Relation::RemindsOf | Relation::Supports)
    }

    pub fn is_causal(&self) -> bool {
        matches!(self, Relation::Causes | Relation::LeadsTo)
    }
}
