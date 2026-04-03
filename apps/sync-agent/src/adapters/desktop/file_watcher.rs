//! Driving adapter: OS filesystem events → application sync. Constructed from the composition root.
#![allow(dead_code)]

use crate::application::services::file_service::FileServiceError;
use notify_debouncer_mini::{
    new_debouncer,
    notify::{Error as NotifyError, RecommendedWatcher, RecursiveMode},
    DebouncedEvent, DebounceEventResult, Debouncer,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(400);

pub struct DesktopFileWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
}

impl DesktopFileWatcher {
    /// Watches `sync_root` recursively with [`DEFAULT_DEBOUNCE`].
    pub fn new<F>(sync_root: PathBuf, on_sync: F) -> Result<Self, NotifyError>
    where
        F: Fn(&str) -> Result<(), FileServiceError> + Send + Sync + 'static,
    {
        Self::with_debounce(sync_root, DEFAULT_DEBOUNCE, on_sync)
    }

    pub fn with_debounce<F>(
        sync_root: PathBuf,
        debounce: Duration,
        on_sync: F,
    ) -> Result<Self, NotifyError>
    where
        F: Fn(&str) -> Result<(), FileServiceError> + Send + Sync + 'static,
    {
        let on_sync = Arc::new(on_sync);
        let mut debouncer = new_debouncer(debounce, {
            let on_sync = Arc::clone(&on_sync);
            move |res: DebounceEventResult| Self::dispatch_debounced(&on_sync, res)
        })?;
        debouncer
            .watcher()
            .watch(sync_root.as_path(), RecursiveMode::Recursive)?;
        Ok(Self {
            _debouncer: debouncer,
        })
    }

    fn dispatch_debounced<F>(on_sync: &Arc<F>, res: DebounceEventResult)
    where
        F: Fn(&str) -> Result<(), FileServiceError> + Send + Sync,
    {
        match res {
            Ok(events) => Self::apply_debounced_events(on_sync, events),
            Err(err) => eprintln!("sync-agent: filesystem watch error: {err:?}"),
        }
    }

    fn apply_debounced_events<F>(on_sync: &Arc<F>, events: Vec<DebouncedEvent>)
    where
        F: Fn(&str) -> Result<(), FileServiceError> + Send + Sync,
    {
        let mut seen = HashSet::new();
        for event in events {
            if !seen.insert(event.path.clone()) {
                continue;
            }
            if !should_sync_path(event.path.as_path()) {
                continue;
            }
            let Some(path_str) = normalize_path_for_repo(event.path.as_path()) else {
                continue;
            };
            if let Err(err) = on_sync(&path_str) {
                eprintln!("sync-agent: sync failed for {path_str}: {err:?}");
            }
        }
    }
}

pub(crate) fn should_sync_path(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.is_file())
        .unwrap_or(false)
}

pub(crate) fn normalize_path_for_repo(path: &Path) -> Option<String> {
    let s = path.to_string_lossy();
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn normalize_path_for_repo_non_empty() {
        let p = Path::new(r"C:\foo\bar.txt");
        assert_eq!(
            normalize_path_for_repo(p).as_deref(),
            Some("C:/foo/bar.txt")
        );
    }

    #[test]
    fn should_sync_path_file_vs_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file_path = dir.path().join("a.txt");
        fs::write(&file_path, b"x").expect("write");
        assert!(should_sync_path(&file_path));
        assert!(!should_sync_path(dir.path()));
    }
}
