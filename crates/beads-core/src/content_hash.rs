//! Deterministic content hashing for issues.
//!
//! Produces a SHA-256 hex digest over all substantive fields (excluding ID,
//! timestamps, and compaction metadata) so that identical content produces
//! identical hashes across all clones.

use sha2::{Digest, Sha256};

use crate::issue::Issue;

/// Separator byte written between fields.
const SEP: u8 = 0;

/// Computes a deterministic content hash for an issue.
///
/// Uses all substantive fields (excluding ID, timestamps, and compaction
/// metadata) to ensure that identical content produces identical hashes.
/// The algorithm mirrors the Go implementation field-for-field.
pub fn compute_content_hash(issue: &Issue) -> String {
    let mut h = Sha256::new();

    // Core fields in stable order.
    write_str(&mut h, &issue.title);
    write_str(&mut h, &issue.description);
    write_str(&mut h, &issue.design);
    write_str(&mut h, &issue.acceptance_criteria);
    write_str(&mut h, &issue.notes);
    write_str(&mut h, &issue.spec_id);
    write_str(&mut h, issue.status.as_str());
    write_int(&mut h, issue.priority);
    write_str(&mut h, issue.issue_type.as_str());
    write_str(&mut h, &issue.assignee);
    write_str(&mut h, &issue.owner);
    write_str(&mut h, &issue.created_by);

    // Optional fields.
    write_str_opt(&mut h, issue.external_ref.as_deref());
    write_str(&mut h, &issue.source_system);
    write_flag(&mut h, issue.pinned, "pinned");
    // Include metadata in content hash.
    if let Some(ref meta) = issue.metadata {
        write_str(&mut h, meta.get());
    } else {
        h.update([SEP]);
    }
    write_flag(&mut h, issue.is_template, "template");

    // Bonded molecules.
    for br in &issue.bonded_from {
        write_str(&mut h, &br.source_id);
        write_str(&mut h, &br.bond_type);
        write_str(&mut h, &br.bond_point);
    }

    // HOP entity tracking.
    write_entity_ref(&mut h, issue.creator.as_ref());

    // HOP validations.
    for v in &issue.validations {
        write_entity_ref(&mut h, v.validator.as_ref());
        write_str(&mut h, &v.outcome);
        write_str(&mut h, &v.timestamp.to_rfc3339());
        write_f32_opt(&mut h, v.score);
    }

    // HOP aggregate quality score and crystallizes.
    write_f32_opt(&mut h, issue.quality_score);
    write_flag(&mut h, issue.crystallizes, "crystallizes");

    // Gate fields.
    write_str(&mut h, &issue.await_type);
    write_str(&mut h, &issue.await_id);
    write_duration(&mut h, issue.timeout);
    for waiter in &issue.waiters {
        write_str(&mut h, waiter);
    }

    // Slot fields.
    write_str(&mut h, &issue.holder);

    // Agent identity fields.
    write_str(&mut h, &issue.hook_bead);
    write_str(&mut h, &issue.role_bead);
    write_str(&mut h, issue.agent_state.as_str());
    write_str(&mut h, &issue.role_type);
    write_str(&mut h, &issue.rig);

    // Molecule type.
    write_str(&mut h, issue.mol_type.as_str());

    // Work type.
    write_str(&mut h, issue.work_type.as_str());

    // Event fields.
    write_str(&mut h, &issue.event_kind);
    write_str(&mut h, &issue.actor);
    write_str(&mut h, &issue.target);
    write_str(&mut h, &issue.payload);

    format!("{:x}", h.finalize())
}

// -- helper writers --------------------------------------------------------

fn write_str(h: &mut Sha256, s: &str) {
    h.update(s.as_bytes());
    h.update([SEP]);
}

fn write_int(h: &mut Sha256, n: i32) {
    h.update(n.to_string().as_bytes());
    h.update([SEP]);
}

fn write_str_opt(h: &mut Sha256, s: Option<&str>) {
    if let Some(s) = s {
        h.update(s.as_bytes());
    }
    h.update([SEP]);
}

fn write_f32_opt(h: &mut Sha256, v: Option<f32>) {
    if let Some(v) = v {
        // Use Go-compatible %f format (6 decimal places).
        h.update(format!("{:.6}", v).as_bytes());
    }
    h.update([SEP]);
}

fn write_duration(h: &mut Sha256, d: Option<std::time::Duration>) {
    // Go stores Duration as nanoseconds (int64).
    let ns = d.map(|d| d.as_nanos() as i64).unwrap_or(0);
    h.update(ns.to_string().as_bytes());
    h.update([SEP]);
}

fn write_flag(h: &mut Sha256, b: bool, label: &str) {
    if b {
        h.update(label.as_bytes());
    }
    h.update([SEP]);
}

fn write_entity_ref(h: &mut Sha256, e: Option<&crate::entity::EntityRef>) {
    if let Some(e) = e {
        write_str(h, &e.name);
        write_str(h, &e.platform);
        write_str(h, &e.org);
        write_str(h, &e.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::IssueBuilder;

    #[test]
    fn content_hash_deterministic() {
        let issue = IssueBuilder::new("Test issue")
            .description("A description")
            .priority(2)
            .build();

        let hash1 = compute_content_hash(&issue);
        let hash2 = compute_content_hash(&issue);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn content_hash_differs_on_change() {
        let issue1 = IssueBuilder::new("Title A").build();
        let issue2 = IssueBuilder::new("Title B").build();
        assert_ne!(compute_content_hash(&issue1), compute_content_hash(&issue2));
    }

    #[test]
    fn content_hash_ignores_id_and_timestamps() {
        let mut issue1 = IssueBuilder::new("Same content").build();
        let mut issue2 = IssueBuilder::new("Same content").build();

        issue1.id = "bd-aaa".into();
        issue2.id = "bd-bbb".into();
        issue1.created_at = chrono::Utc::now();
        issue2.created_at = chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        assert_eq!(compute_content_hash(&issue1), compute_content_hash(&issue2));
    }
}
