//! Comment, Event, and Label types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::enums::EventType;

/// A comment on an issue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: i64,

    pub issue_id: String,

    pub author: String,

    pub text: String,

    pub created_at: DateTime<Utc>,
}

/// An audit trail entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: i64,

    pub issue_id: String,

    pub event_type: EventType,

    pub actor: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_value: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_value: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    pub created_at: DateTime<Utc>,
}

/// A label (tag) on an issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    pub issue_id: String,
    pub label: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_serde_roundtrip() {
        let c = Comment {
            id: 42,
            issue_id: "bd-abc".into(),
            author: "alice".into(),
            text: "Looks good to me".into(),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&c).unwrap();
        let back: Comment = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 42);
        assert_eq!(back.author, "alice");
    }

    #[test]
    fn event_serde_roundtrip() {
        let e = Event {
            id: 1,
            issue_id: "bd-abc".into(),
            event_type: EventType::StatusChanged,
            actor: "bob".into(),
            old_value: Some("open".into()),
            new_value: Some("closed".into()),
            comment: None,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&e).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(back.event_type, EventType::StatusChanged);
        assert_eq!(back.old_value, Some("open".into()));
    }

    #[test]
    fn label_serde() {
        let l = Label {
            issue_id: "bd-abc".into(),
            label: "tech-debt".into(),
        };
        let json = serde_json::to_string(&l).unwrap();
        let back: Label = serde_json::from_str(&json).unwrap();
        assert_eq!(back.label, "tech-debt");
    }
}
