use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeRange {
    Week1,
    Weeks2,
    Days30,
    Months3,
    Months6,
    Months12,
}

impl TimeRange {
    pub fn label(&self) -> &'static str {
        match self {
            TimeRange::Week1 => "1 week",
            TimeRange::Weeks2 => "2 weeks",
            TimeRange::Days30 => "30 days",
            TimeRange::Months3 => "3 months",
            TimeRange::Months6 => "6 months",
            TimeRange::Months12 => "12 months",
        }
    }

    pub fn days(&self) -> i64 {
        match self {
            TimeRange::Week1 => 7,
            TimeRange::Weeks2 => 14,
            TimeRange::Days30 => 30,
            TimeRange::Months3 => 90,
            TimeRange::Months6 => 180,
            TimeRange::Months12 => 365,
        }
    }

    pub fn all() -> &'static [TimeRange] {
        &[
            TimeRange::Week1,
            TimeRange::Weeks2,
            TimeRange::Days30,
            TimeRange::Months3,
            TimeRange::Months6,
            TimeRange::Months12,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    Year,
    Rolling,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IconColor {
    Green,
    Blue,
    Purple,
    Orange,
    Pink,
    Bumblebee,
    Lemon,
}

impl Default for IconColor {
    fn default() -> Self {
        IconColor::Green
    }
}

impl IconColor {
    pub fn label(&self) -> &'static str {
        match self {
            IconColor::Green => "Green",
            IconColor::Blue => "Blue",
            IconColor::Purple => "Purple",
            IconColor::Orange => "Orange",
            IconColor::Pink => "Pink",
            IconColor::Bumblebee => "Bumblebee",
            IconColor::Lemon => "Lemon",
        }
    }

    pub fn all() -> &'static [IconColor] {
        &[
            IconColor::Green,
            IconColor::Blue,
            IconColor::Purple,
            IconColor::Orange,
            IconColor::Pink,
            IconColor::Bumblebee,
            IconColor::Lemon,
        ]
    }

    /// Representative RGB color for the settings UI swatch
    pub fn swatch_rgb(&self) -> [u8; 3] {
        match self {
            IconColor::Green => [57, 211, 83],
            IconColor::Blue => [88, 166, 255],
            IconColor::Purple => [124, 58, 237],
            IconColor::Orange => [249, 115, 22],
            IconColor::Pink => [236, 72, 153],
            IconColor::Bumblebee => [245, 158, 11],
            IconColor::Lemon => [234, 212, 30],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub tracked_repos: Vec<PathBuf>,
    pub auto_discover_roots: Vec<PathBuf>,
    pub accent_color: String,
    pub time_range: TimeRange,
    pub data_mode: DataMode,
    pub selected_year: i32,
    pub view_mode: ViewMode,
    pub auto_update: bool,
    #[serde(default)]
    pub last_updated_version: Option<String>,
    #[serde(default)]
    pub untracked_repos: Vec<PathBuf>,
    #[serde(default)]
    pub icon_color: IconColor,
    #[serde(default)]
    pub colored_tray_icon: bool,
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
            view_mode: ViewMode::Year,
            auto_update: false,
            last_updated_version: None,
            untracked_repos: Vec::new(),
            icon_color: IconColor::default(),
            colored_tray_icon: false,
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
