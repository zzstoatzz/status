use crate::db::StatusFromDb;
use chrono::{Duration, Utc};
use rand::prelude::*;

/// Generate dummy status data for development testing
pub fn generate_dummy_statuses(count: usize) -> Vec<StatusFromDb> {
    let mut rng = thread_rng();
    let mut statuses = Vec::new();

    // Sample data pools
    let emojis = vec![
        "🚀", "💭", "☕", "🎨", "📚", "🎵", "🏃", "😴", "🍕", "💻", "🌟", "🔥", "✨", "🌙", "☀️",
        "🌈", "⚡", "🎯", "🎮", "📝",
    ];

    let texts = [
        Some("working on something cool"),
        Some("deep in flow state"),
        Some("taking a break"),
        Some("debugging..."),
        Some("shipping it"),
        None,
        Some("coffee time"),
        Some("in a meeting"),
        Some("focused"),
        None,
        Some("learning rust"),
        Some("reading docs"),
    ];

    let handles = [
        "alice.test",
        "bob.test",
        "charlie.test",
        "dana.test",
        "eve.test",
        "frank.test",
        "grace.test",
        "henry.test",
        "iris.test",
        "jack.test",
        "karen.test",
        "leo.test",
    ];

    let now = Utc::now();

    for i in 0..count {
        // Generate random timestamps going back up to 48 hours
        let hours_ago = rng.gen_range(0..48);
        let minutes_ago = rng.gen_range(0..60);
        let started_at = now - Duration::hours(hours_ago) - Duration::minutes(minutes_ago);

        // Random chance of having an expiration
        let expires_at = if rng.gen_bool(0.3) {
            // 30% chance of having expiration
            let expire_hours = rng.gen_range(1..24);
            Some(started_at + Duration::hours(expire_hours))
        } else {
            None
        };

        let mut status = StatusFromDb::new(
            format!("at://did:plc:dummy{}/xyz.statusphere.status/dummy{}", i, i),
            format!("did:plc:dummy{}", i % handles.len()),
            emojis.choose(&mut rng).unwrap().to_string(),
        );

        status.text = texts.choose(&mut rng).unwrap().map(|s| s.to_string());
        status.started_at = started_at;
        status.expires_at = expires_at;
        status.indexed_at = started_at;
        status.handle = Some(handles[i % handles.len()].to_string());

        statuses.push(status);
    }

    // Sort by started_at desc (newest first)
    statuses.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    statuses
}

/// Check if dev mode is requested via query parameter
pub fn is_dev_mode_requested(query: &str) -> bool {
    let query_lower = query.to_lowercase();
    query_lower.contains("dev=true") || query.contains("dev=1")
}
