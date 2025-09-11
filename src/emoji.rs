use once_cell::sync::OnceCell;
use std::{collections::HashSet, fs, path::Path, sync::Arc};

use crate::config::Config;

/// Ensure the runtime emoji directory exists, and seed it from the bundled
/// `static/emojis` on first run if the runtime directory is empty.
pub fn init_runtime_dir(config: &Config) {
    let runtime_emoji_dir = &config.emoji_dir;
    let bundled_emoji_dir = "static/emojis";

    if let Err(e) = fs::create_dir_all(runtime_emoji_dir) {
        log::warn!(
            "Failed to ensure emoji directory exists at {}: {}",
            runtime_emoji_dir,
            e
        );
        return;
    }

    let should_seed = runtime_emoji_dir != bundled_emoji_dir
        && fs::read_dir(runtime_emoji_dir)
            .map(|mut it| it.next().is_none())
            .unwrap_or(false);

    if !should_seed {
        return;
    }

    if !Path::new(bundled_emoji_dir).exists() {
        return;
    }

    match fs::read_dir(bundled_emoji_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    let dest = Path::new(runtime_emoji_dir).join(name);
                    if path.is_file() {
                        if let Err(err) = fs::copy(&path, &dest) {
                            log::warn!("Failed to seed emoji {:?} -> {:?}: {}", path, dest, err);
                        }
                    }
                }
            }
            log::info!(
                "Seeded emoji directory {} from {}",
                runtime_emoji_dir,
                bundled_emoji_dir
            );
        }
        Err(err) => log::warn!(
            "Failed to read bundled emoji directory {}: {}",
            bundled_emoji_dir,
            err
        ),
    }
}

#[allow(dead_code)]
static BUILTIN_SLUGS: OnceCell<Arc<HashSet<String>>> = OnceCell::new();

#[allow(dead_code)]
async fn load_builtin_slugs_inner() -> Arc<HashSet<String>> {
    // Fetch emoji data and collect first short_name as slug
    let url = "https://cdn.jsdelivr.net/npm/emoji-datasource@15.1.0/emoji.json";
    let client = reqwest::Client::new();
    let mut set = HashSet::new();
    if let Ok(resp) = client.get(url).send().await {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(arr) = json.as_array() {
                for item in arr {
                    if let Some(shorts) = item.get("short_names").and_then(|v| v.as_array()) {
                        if let Some(first) = shorts.first().and_then(|v| v.as_str()) {
                            set.insert(first.to_lowercase());
                        }
                    } else if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        // Fallback: slugify the name
                        let slug: String = name
                            .chars()
                            .map(|c| {
                                if c.is_ascii_alphanumeric() {
                                    c.to_ascii_lowercase()
                                } else {
                                    '-'
                                }
                            })
                            .collect::<String>()
                            .trim_matches('-')
                            .to_string();
                        if !slug.is_empty() {
                            set.insert(slug);
                        }
                    }
                }
            }
        }
    }
    Arc::new(set)
}

#[allow(dead_code)]
pub async fn is_builtin_slug(name: &str) -> bool {
    let name = name.to_lowercase();
    if let Some(cache) = BUILTIN_SLUGS.get() {
        return cache.contains(&name);
    }
    let set = load_builtin_slugs_inner().await;
    let contains = set.contains(&name);
    let _ = BUILTIN_SLUGS.set(set);
    contains
}
