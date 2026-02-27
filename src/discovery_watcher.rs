use notify::{Event, EventKind, RecursiveMode, Result as NotifyResult, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

const DEBOUNCE_SECS: u64 = 3;

pub struct DiscoveryWatcher {
    watcher: notify::RecommendedWatcher,
    rx: mpsc::Receiver<NotifyResult<Event>>,
    watched_roots: Vec<PathBuf>,
    /// Paths pending debounce: path -> first seen timestamp
    pending: HashMap<PathBuf, Instant>,
}

impl DiscoveryWatcher {
    pub fn new() -> NotifyResult<Self> {
        let (tx, rx) = mpsc::channel();
        let watcher = notify::recommended_watcher(tx)?;
        Ok(Self {
            watcher,
            rx,
            watched_roots: Vec::new(),
            pending: HashMap::new(),
        })
    }

    /// Start watching a root directory for new subdirectories.
    pub fn watch_root(&mut self, root: &Path) -> NotifyResult<()> {
        if !self.watched_roots.contains(&root.to_path_buf()) {
            self.watcher.watch(root, RecursiveMode::NonRecursive)?;
            self.watched_roots.push(root.to_path_buf());
        }
        Ok(())
    }

    /// Stop watching a root directory.
    pub fn unwatch_root(&mut self, root: &Path) -> NotifyResult<()> {
        self.watcher.unwatch(root)?;
        self.watched_roots.retain(|p| p != root);
        Ok(())
    }

    /// Poll for newly discovered git repos. Call this from the UI event loop.
    /// Returns repo root paths (parent of .git) that are ready (debounce elapsed).
    pub fn poll_new_repos(&mut self) -> Vec<PathBuf> {
        // Drain FSEvents into pending map
        while let Ok(Ok(event)) = self.rx.try_recv() {
            if matches!(event.kind, EventKind::Create(_)) {
                for path in event.paths {
                    if path.is_dir() && !self.pending.contains_key(&path) {
                        self.pending.insert(path, Instant::now());
                    }
                }
            }
        }

        // Check debounced entries for .git
        let now = Instant::now();
        let debounce = std::time::Duration::from_secs(DEBOUNCE_SECS);
        let mut ready = Vec::new();
        let mut done = Vec::new();

        for (path, first_seen) in &self.pending {
            if now.duration_since(*first_seen) >= debounce {
                done.push(path.clone());
                if path.join(".git").is_dir() {
                    ready.push(path.clone());
                }
            }
        }

        for path in done {
            self.pending.remove(&path);
        }

        ready
    }
}
