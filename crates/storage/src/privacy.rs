use crate::error::StoreError;
use crate::graph::ThoughtNode;
use crate::types::{Event, PrivacyLevel};

pub struct PrivacyFilter {
    pub allowed_levels: Vec<PrivacyLevel>,
}

impl Default for PrivacyFilter {
    fn default() -> Self {
        Self {
            allowed_levels: vec![
                PrivacyLevel::Private,
                PrivacyLevel::Shareable,
                PrivacyLevel::Public,
            ],
        }
    }
}

impl PrivacyFilter {
    pub fn public_only() -> Self {
        Self {
            allowed_levels: vec![PrivacyLevel::Public],
        }
    }

    pub fn shareable() -> Self {
        Self {
            allowed_levels: vec![PrivacyLevel::Shareable, PrivacyLevel::Public],
        }
    }

    pub fn all() -> Self {
        Self::default()
    }

    pub fn allows(&self, level: &PrivacyLevel) -> bool {
        self.allowed_levels.contains(level)
    }

    pub fn filter_events(&self, events: Vec<Event>) -> Vec<Event> {
        events
            .into_iter()
            .filter(|e| self.allows(&e.privacy))
            .collect()
    }

    pub fn filter_nodes(&self, nodes: Vec<ThoughtNode>) -> Vec<ThoughtNode> {
        nodes
            .into_iter()
            .filter(|n| self.allows(&n.privacy))
            .collect()
    }

    pub fn check_event(&self, event: &Event) -> Result<(), StoreError> {
        if self.allows(&event.privacy) {
            Ok(())
        } else {
            Err(StoreError::PrivacyViolation(format!(
                "Event {} has privacy level {:?}, which is not allowed",
                event.id.0, event.privacy
            )))
        }
    }

    pub fn check_node(&self, node: &ThoughtNode) -> Result<(), StoreError> {
        if self.allows(&node.privacy) {
            Ok(())
        } else {
            Err(StoreError::PrivacyViolation(format!(
                "Node {} has privacy level {:?}, which is not allowed",
                node.id.0, node.privacy
            )))
        }
    }
}

pub trait PrivacyAware {
    fn privacy_level(&self) -> PrivacyLevel;

    fn is_shareable(&self) -> bool {
        self.privacy_level().can_share()
    }

    fn is_public(&self) -> bool {
        self.privacy_level().is_public()
    }
}

impl PrivacyAware for Event {
    fn privacy_level(&self) -> PrivacyLevel {
        self.privacy
    }
}

impl PrivacyAware for ThoughtNode {
    fn privacy_level(&self) -> PrivacyLevel {
        self.privacy
    }
}

pub fn redact_for_sharing<T: PrivacyAware + Clone>(items: &[T], filter: &PrivacyFilter) -> Vec<T> {
    items
        .iter()
        .filter(|item| filter.allows(&item.privacy_level()))
        .cloned()
        .collect()
}
