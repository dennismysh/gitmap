use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Recursively discover all git repositories under `root`.
/// Skips hidden directories (except checking for .git).
pub fn discover_repos(root: &Path) -> Vec<PathBuf> {
    let mut repos = Vec::new();

    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if e.depth() > 0 && name.starts_with('.') {
                return false;
            }
            true
        });

    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_dir() {
            continue;
        }
        if entry.path().join(".git").is_dir() {
            repos.push(entry.path().to_path_buf());
        }
    }

    repos
}
