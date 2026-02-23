use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeRange {
    Days30,
    Months3,
    Months6,
    Months12,
}

impl TimeRange {
    pub fn label(&self) -> &'static str {
        match self {
            TimeRange::Days30 => "30 days",
            TimeRange::Months3 => "3 months",
            TimeRange::Months6 => "6 months",
            TimeRange::Months12 => "12 months",
        }
    }

    pub fn all() -> &'static [TimeRange] {
        &[
            TimeRange::Days30,
            TimeRange::Months3,
            TimeRange::Months6,
            TimeRange::Months12,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataMode {
    Commits,
    LinesChanged,
}

impl DataMode {
    pub fn label(&self) -> &'static str {
        match self {
            DataMode::Commits => "Commits",
            DataMode::LinesChanged => "Lines changed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub tracked_repos: Vec<PathBuf>,
    pub auto_discover_roots: Vec<PathBuf>,
    pub accent_color: String,
    pub time_range: TimeRange,
    pub data_mode: DataMode,
    pub selected_year: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tracked_repos: Vec::new(),
            auto_discover_roots: Vec::new(),
            accent_color: "#39d353".to_string(),
            time_range: TimeRange::Months12,
            data_mode: DataMode::Commits,
            selected_year: chrono::Local::now().year(),
        }
    }
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gitmap")
}

fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        if path.exists() {
            let contents = std::fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&contents).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)
    }
}
