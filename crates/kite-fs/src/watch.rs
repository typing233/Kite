use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc::UnboundedSender;

pub struct FsWatcher {
    _watcher: RecommendedWatcher,
}

impl FsWatcher {
    pub fn new(path: &Path, sender: UnboundedSender<Vec<std::path::PathBuf>>) -> notify::Result<Self> {
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let paths: Vec<_> = event.paths;
                    if !paths.is_empty() {
                        let _ = sender.send(paths);
                    }
                }
            },
            Config::default(),
        )?;

        watcher.watch(path, RecursiveMode::NonRecursive)?;

        Ok(Self { _watcher: watcher })
    }

    pub fn watch_new_path(&mut self, _path: &Path) -> notify::Result<()> {
        // The watcher is recreated when directory changes, this is a placeholder
        // for the full implementation that would unwatch old + watch new
        Ok(())
    }
}
