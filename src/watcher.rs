use notify::{Event, RecursiveMode, Result as NotifyResult, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

pub struct RepoWatcher {
    watcher: notify::RecommendedWatcher,
    watched_paths: Vec<PathBuf>,
    pub rx: mpsc::Receiver<NotifyResult<Event>>,
}

impl RepoWatcher {
    pub fn new() -> NotifyResult<Self> {
        let (tx, rx) = mpsc::channel();
        let watcher = notify::recommended_watcher(tx)?;
        Ok(Self {
            watcher,
            watched_paths: Vec::new(),
            rx,
        })
    }

    /// Start watching a repo's .git directory for changes.
    pub fn watch_repo(&mut self, repo_path: &Path) -> NotifyResult<()> {
        let git_dir = repo_path.join(".git");
        if git_dir.is_dir() && !self.watched_paths.contains(&git_dir) {
            self.watcher.watch(&git_dir, RecursiveMode::Recursive)?;
            self.watched_paths.push(git_dir);
        }
        Ok(())
    }

    /// Stop watching a repo.
    pub fn unwatch_repo(&mut self, repo_path: &Path) -> NotifyResult<()> {
        let git_dir = repo_path.join(".git");
        self.watcher.unwatch(&git_dir)?;
        self.watched_paths.retain(|p| p != &git_dir);
        Ok(())
    }

    /// Drain all pending events, returning paths of repos that changed.
    pub fn poll_changed_repos(&self) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        while let Ok(Ok(event)) = self.rx.try_recv() {
            for path in &event.paths {
                if let Some(repo_root) = find_repo_root(path) {
                    if !changed.contains(&repo_root) {
                        changed.push(repo_root);
                    }
                }
            }
        }
        changed
    }
}

/// Given a path inside a .git directory, find the repo root (parent of .git).
fn find_repo_root(path: &Path) -> Option<PathBuf> {
    let mut current = path;
    loop {
        if current.file_name().map(|n| n == ".git").unwrap_or(false) {
            return current.parent().map(|p| p.to_path_buf());
        }
        current = current.parent()?;
    }
}
