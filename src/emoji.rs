use std::{fs, path::Path};

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
