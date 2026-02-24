use chrono::NaiveDate;
use git2::{Repository, Sort};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayStats {
    pub commits: u32,
    pub insertions: u32,
    pub deletions: u32,
}

impl DayStats {
    pub fn merge(&self, other: &DayStats) -> DayStats {
        DayStats {
            commits: self.commits + other.commits,
            insertions: self.insertions + other.insertions,
            deletions: self.deletions + other.deletions,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitIdentity {
    pub name: String,
    pub email: String,
}

/// Detect the git user.name and user.email configured for a repo.
pub fn detect_identity(repo_path: &Path) -> Result<GitIdentity, git2::Error> {
    let repo = Repository::open(repo_path)?;
    let config = repo.config()?;
    let name = config.get_string("user.name").unwrap_or_default();
    let email = config.get_string("user.email").unwrap_or_default();
    Ok(GitIdentity { name, email })
}

/// Scan a git repository and return daily commit stats.
///
/// If `since` is provided, only commits after that date are included.
/// Commits are filtered to only those matching the given identity (by name or email).
pub fn scan_repo(
    repo_path: &Path,
    identity: &GitIdentity,
    since: Option<NaiveDate>,
) -> Result<HashMap<NaiveDate, DayStats>, git2::Error> {
    let repo = Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME)?;

    let mut stats: HashMap<NaiveDate, DayStats> = HashMap::new();

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        let git_time = commit.time();
        let timestamp = git_time.seconds();
        let offset_minutes = git_time.offset_minutes();
        let offset = chrono::FixedOffset::east_opt(offset_minutes as i32 * 60)
            .unwrap_or_else(|| chrono::FixedOffset::east_opt(0).unwrap());
        let date = chrono::DateTime::from_timestamp(timestamp, 0)
            .map(|dt| dt.with_timezone(&offset).date_naive())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

        if let Some(since_date) = since {
            if date <= since_date {
                break;
            }
        }

        let author = commit.author();
        let author_name = author.name().unwrap_or("");
        let author_email = author.email().unwrap_or("");

        if author_name != identity.name && author_email != identity.email {
            continue;
        }

        // Skip line stats for merge commits (parent_count > 1) to avoid
        // double-counting: the constituent commits already account for the
        // same line changes. Merge commits still count toward commit totals.
        let (insertions, deletions) = if commit.parent_count() > 1 {
            (0, 0)
        } else if commit.parent_count() == 1 {
            let parent = commit.parent(0)?;
            let parent_tree = parent.tree()?;
            let commit_tree = commit.tree()?;
            let mut diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)?;
            let mut find_opts = git2::DiffFindOptions::new();
            find_opts.renames(true);
            diff.find_similar(Some(&mut find_opts))?;
            let diff_stats = diff.stats()?;
            (diff_stats.insertions() as u32, diff_stats.deletions() as u32)
        } else {
            // Initial commit (no parents) — diff against empty tree
            let commit_tree = commit.tree()?;
            let diff = repo.diff_tree_to_tree(None, Some(&commit_tree), None)?;
            let diff_stats = diff.stats()?;
            (diff_stats.insertions() as u32, diff_stats.deletions() as u32)
        };

        let entry = stats.entry(date).or_insert(DayStats {
            commits: 0,
            insertions: 0,
            deletions: 0,
        });
        entry.commits += 1;
        entry.insertions += insertions;
        entry.deletions += deletions;
    }

    Ok(stats)
}
