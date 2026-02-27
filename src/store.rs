use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::scanner::DayStats;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitStore {
    stats: HashMap<NaiveDate, DayStats>,
}

impl Default for CommitStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CommitStore {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }

    pub fn stats(&self) -> &HashMap<NaiveDate, DayStats> {
        &self.stats
    }

    pub fn get(&self, date: NaiveDate) -> Option<&DayStats> {
        self.stats.get(&date)
    }

    pub fn merge(&mut self, new_stats: HashMap<NaiveDate, DayStats>) {
        for (date, new) in new_stats {
            let entry = self.stats.entry(date).or_insert(DayStats {
                commits: 0,
                insertions: 0,
                deletions: 0,
            });
            *entry = entry.merge(&new);
        }
    }

    pub fn most_recent_date(&self) -> Option<NaiveDate> {
        self.stats.keys().max().copied()
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }

    pub fn load_from(path: &Path) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
