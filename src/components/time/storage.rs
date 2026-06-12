//! Persistence for the time/scheduler module.
//!
//! Desktop-only (the app's default target). Writes through the shared
//! `utils::json_store` helpers to a file outside the project tree, so `dx serve`
//! doesn't rebuild-loop on every save. Path convention matches the rest of the
//! app: `directories::ProjectDirs::from("com", "gauss", "momentum-dioxus")`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::utils::json_store::{load_json, save_json};

use super::model::Event;

/// On-disk shape. Versioned so the format can evolve without silent breakage.
/// No `next_id` — event IDs are UUIDs generated at insert time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedState {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub events: Vec<Event>,
}

fn default_version() -> u32 {
    1
}

impl Default for SavedState {
    fn default() -> Self {
        SavedState {
            version: 1,
            events: Vec::new(),
        }
    }
}

/// `~/.local/share/momentum-dioxus/time.json` on Linux (platform-appropriate
/// elsewhere). Falls back to a bare relative path only if the OS dirs are
/// somehow unavailable.
fn time_path() -> PathBuf {
    if let Some(proj) = directories::ProjectDirs::from("com", "gauss", "momentum-dioxus") {
        let mut p = proj.data_dir().to_path_buf();
        p.push("time.json");
        return p;
    }
    PathBuf::from("time.json")
}

/// Load saved state, defaulting to empty when the file is missing or unreadable.
/// (First run has no file; that's not an error.)
pub fn load() -> SavedState {
    load_json::<SavedState>(time_path()).unwrap_or_default()
}

/// Best-effort save. Errors are swallowed here; if you want them surfaced into a
/// status `Signal`, call `save_json` directly and map with `err_to_string`.
pub fn save(state: &SavedState) {
    let _ = save_json(time_path(), state);
}
