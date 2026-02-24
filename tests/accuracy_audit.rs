use chrono::NaiveDate;
use gitmap::config::Config;
use gitmap::scanner::{self, DayStats, GitIdentity};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Parse `git log` output into per-day stats matching scan_repo's structure.
/// Uses: git log --author="name" --author="email" --format="COMMIT %H %ad" --date=short --numstat
fn git_cli_stats(repo_path: &Path, identity: &GitIdentity) -> HashMap<NaiveDate, DayStats> {
    let output = Command::new("git")
        .args([
            "log",
            "--all",
            &format!("--author={}", identity.name),
            &format!("--author={}", identity.email),
            "--format=COMMIT %H %ad",
            "--date=short",
            "--numstat",
        ])
        .current_dir(repo_path)
        .output()
        .expect("git log failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut stats: HashMap<NaiveDate, DayStats> = HashMap::new();
    let mut current_date: Option<NaiveDate> = None;

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("COMMIT ") {
            // Format: "COMMIT <hash> <YYYY-MM-DD>"
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(date) = NaiveDate::parse_from_str(parts[1], "%Y-%m-%d") {
                    current_date = Some(date);
                    let entry = stats.entry(date).or_insert(DayStats {
                        commits: 0,
                        insertions: 0,
                        deletions: 0,
                    });
                    entry.commits += 1;
                }
            }
        } else if let Some(date) = current_date {
            // numstat line: "<insertions>\t<deletions>\t<filename>"
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let ins: u32 = parts[0].parse().unwrap_or(0); // "-" for binary
                let del: u32 = parts[1].parse().unwrap_or(0);
                if let Some(entry) = stats.get_mut(&date) {
                    entry.insertions += ins;
                    entry.deletions += del;
                }
            }
        }
    }

    stats
}
