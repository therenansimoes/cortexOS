use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use cortex_grid::NodeId;

/// A skill identifier - human-readable tag
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillId(pub String);

impl SkillId {
    pub fn new(name: &str) -> Self {
        Self(name.to_lowercase().trim().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SkillId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for SkillId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for SkillId {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

/// Rating value: -1.0 (terrible) to +1.0 (excellent)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rating(f32);

impl Rating {
    pub fn new(value: f32) -> Option<Self> {
        if (-1.0..=1.0).contains(&value) {
            Some(Self(value))
        } else {
            None
        }
    }

    pub fn positive() -> Self {
        Self(1.0)
    }

    pub fn negative() -> Self {
        Self(-1.0)
    }

    pub fn neutral() -> Self {
        Self(0.0)
    }

    pub fn value(&self) -> f32 {
        self.0
    }

    pub fn is_positive(&self) -> bool {
        self.0 > 0.0
    }

    pub fn is_negative(&self) -> bool {
        self.0 < 0.0
    }
}

impl Default for Rating {
    fn default() -> Self {
        Self::neutral()
    }
}

/// A rating record: who rated whom, for which skill, and when
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingRecord {
    /// Who gave the rating
    pub rater: NodeId,
    /// Who received the rating
    pub ratee: NodeId,
    /// Which skill is being rated
    pub skill: SkillId,
    /// The rating value
    pub rating: Rating,
    /// When the rating was given
    pub timestamp: u64,
    /// Optional context (e.g., task ID that prompted the rating)
    pub context: Option<String>,
    /// Signature from rater (for verification)
    pub signature: Option<Vec<u8>>,
}

impl RatingRecord {
    pub fn new(rater: NodeId, ratee: NodeId, skill: SkillId, rating: Rating) -> Self {
        Self {
            rater,
            ratee,
            skill,
            rating,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            context: None,
            signature: None,
        }
    }

    pub fn with_context(mut self, context: &str) -> Self {
        self.context = Some(context.to_string());
        self
    }

    /// Compute hash for deduplication
    pub fn hash(&self) -> [u8; 32] {
        let data = format!(
            "{}:{}:{}:{}",
            hex::encode(self.rater.as_bytes()),
            hex::encode(self.ratee.as_bytes()),
            self.skill.as_str(),
            self.timestamp
        );
        *blake3::hash(data.as_bytes()).as_bytes()
    }
}

/// Aggregated skill rating for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRating {
    pub skill: SkillId,
    pub node: NodeId,
    /// Number of positive ratings
    pub positive_count: u32,
    /// Number of negative ratings
    pub negative_count: u32,
    /// Weighted score (considering rater trust)
    pub weighted_score: f32,
    /// Last updated timestamp
    pub updated_at: u64,
}

impl SkillRating {
    pub fn new(skill: SkillId, node: NodeId) -> Self {
        Self {
            skill,
            node,
            positive_count: 0,
            negative_count: 0,
            weighted_score: 0.0,
            updated_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Add a rating (unweighted)
    pub fn add_rating(&mut self, rating: Rating) {
        if rating.is_positive() {
            self.positive_count += 1;
        } else if rating.is_negative() {
            self.negative_count += 1;
        }
        self.update_timestamp();
    }

    /// Add a weighted rating (considering rater's trust)
    pub fn add_weighted_rating(&mut self, rating: Rating, rater_trust: f32) {
        let weight = rater_trust.clamp(0.0, 1.0);
        self.weighted_score += rating.value() * weight;

        if rating.is_positive() {
            self.positive_count += 1;
        } else if rating.is_negative() {
            self.negative_count += 1;
        }
        self.update_timestamp();
    }

    fn update_timestamp(&mut self) {
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Total number of ratings
    pub fn total_ratings(&self) -> u32 {
        self.positive_count + self.negative_count
    }

    /// Simple ratio: positive / total
    pub fn approval_ratio(&self) -> f32 {
        let total = self.total_ratings();
        if total == 0 {
            return 0.5; // neutral if no ratings
        }
        self.positive_count as f32 / total as f32
    }

    /// Normalized weighted score
    pub fn normalized_score(&self) -> f32 {
        let total = self.total_ratings();
        if total == 0 {
            return 0.0;
        }
        (self.weighted_score / total as f32).clamp(-1.0, 1.0)
    }
}

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rating_bounds() {
        assert!(Rating::new(0.5).is_some());
        assert!(Rating::new(1.0).is_some());
        assert!(Rating::new(-1.0).is_some());
        assert!(Rating::new(1.5).is_none());
        assert!(Rating::new(-1.5).is_none());
    }

    #[test]
    fn test_skill_rating() {
        let skill = SkillId::new("rust-programming");
        let node = NodeId::random();
        let mut sr = SkillRating::new(skill, node);

        sr.add_rating(Rating::positive());
        sr.add_rating(Rating::positive());
        sr.add_rating(Rating::negative());

        assert_eq!(sr.positive_count, 2);
        assert_eq!(sr.negative_count, 1);
        assert!((sr.approval_ratio() - 0.666).abs() < 0.01);
    }
}
