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

#[test]
fn audit_scanner_accuracy() {
    let config = Config::load();

    if config.tracked_repos.is_empty() {
        eprintln!("No tracked repos configured — skipping audit");
        return;
    }

    let mut total_discrepancies = 0;

    for repo_path in &config.tracked_repos {
        let repo_name = repo_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| repo_path.display().to_string());

        let identity = match scanner::detect_identity(repo_path) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("  SKIP {}: failed to detect identity: {}", repo_name, e);
                continue;
            }
        };

        let gitmap_stats = match scanner::scan_repo(repo_path, &identity, None) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  SKIP {}: scan_repo failed: {}", repo_name, e);
                continue;
            }
        };

        let cli_stats = git_cli_stats(repo_path, &identity);

        // Collect all dates from both sources
        let mut all_dates: Vec<NaiveDate> = gitmap_stats
            .keys()
            .chain(cli_stats.keys())
            .copied()
            .collect();
        all_dates.sort();
        all_dates.dedup();

        let mut repo_discrepancies = 0;

        eprintln!("\n=== {} ({}) ===", repo_name, repo_path.display());
        eprintln!("  Identity: {} <{}>", identity.name, identity.email);
        eprintln!(
            "  gitmap total days: {}, git CLI total days: {}",
            gitmap_stats.len(),
            cli_stats.len()
        );

        for date in &all_dates {
            let gm = gitmap_stats.get(date);
            let cli = cli_stats.get(date);

            let (gm_commits, gm_ins, gm_del) = gm
                .map(|s| (s.commits, s.insertions, s.deletions))
                .unwrap_or((0, 0, 0));
            let (cli_commits, cli_ins, cli_del) = cli
                .map(|s| (s.commits, s.insertions, s.deletions))
                .unwrap_or((0, 0, 0));

            if gm_commits != cli_commits || gm_ins != cli_ins || gm_del != cli_del {
                repo_discrepancies += 1;
                eprintln!(
                    "  MISMATCH {}: gitmap({} commits, +{} -{}) vs cli({} commits, +{} -{})",
                    date, gm_commits, gm_ins, gm_del, cli_commits, cli_ins, cli_del
                );
            }
        }

        if repo_discrepancies == 0 {
            eprintln!("  ALL MATCH ({} days compared)", all_dates.len());
        } else {
            eprintln!(
                "  {} discrepancies out of {} days",
                repo_discrepancies,
                all_dates.len()
            );
        }
        total_discrepancies += repo_discrepancies;
    }

    eprintln!("\n=== SUMMARY: {} total discrepancies ===", total_discrepancies);
}
