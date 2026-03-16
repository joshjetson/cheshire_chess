#![allow(dead_code)]

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct Progress {
    pub completed: HashSet<String>,
    pub last_theme: Option<String>,
    pub last_offset: usize,
    #[serde(skip)]
    save_path: PathBuf,
}

impl Progress {
    pub fn load(data_dir: &Path) -> Self {
        let save_path = data_dir.join("progress.json");
        let mut progress = if let Ok(content) = fs::read_to_string(&save_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        };
        progress.save_path = save_path;
        progress
    }

    pub fn save(&self) {
        if let Some(parent) = self.save_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&self.save_path, json);
        }
    }

    pub fn mark_completed(&mut self, puzzle_id: &str) {
        self.completed.insert(puzzle_id.to_string());
        self.save();
    }

    pub fn is_completed(&self, puzzle_id: &str) -> bool {
        self.completed.contains(puzzle_id)
    }

    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }

    pub fn update_last_position(&mut self, theme: &str, offset: usize) {
        self.last_theme = Some(theme.to_string());
        self.last_offset = offset;
        self.save();
    }
}
