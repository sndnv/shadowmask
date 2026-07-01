use std::path::Path;

use notify::{RecursiveMode, Watcher};
use tokio::sync::mpsc::unbounded_channel;

use crate::scan::{FsEventSource, NotifyFsEventStream};

#[derive(Debug, Default, Clone, Copy)]
pub struct NotifyFsEventSource;

impl FsEventSource for NotifyFsEventSource {
    type Stream = NotifyFsEventStream;

    fn watch(&self, roots: &[String]) -> notify::Result<NotifyFsEventStream> {
        let (tx, rx) = unbounded_channel::<Vec<String>>();
        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
                if let Ok(event) = result {
                    let paths: Vec<String> = event
                        .paths
                        .iter()
                        .filter_map(|path| path.to_str().map(str::to_owned))
                        .collect();
                    if !paths.is_empty() {
                        let _ = tx.send(paths);
                    }
                }
            })?;
        for root in roots {
            watcher.watch(Path::new(root), RecursiveMode::Recursive)?;
        }
        Ok(NotifyFsEventStream::new(watcher, rx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::FsEventStream;
    use std::fs;
    use std::time::Duration;

    #[tokio::test]
    async fn observes_a_real_file_change() {
        let dir = tempfile::tempdir().unwrap();
        let mut stream = NotifyFsEventSource
            .watch(&[dir.path().to_str().unwrap().to_owned()])
            .expect("watcher should start");

        fs::write(dir.path().join("new.mkv"), b"data").unwrap();

        let change = tokio::time::timeout(Duration::from_secs(10), stream.next_change())
            .await
            .expect("an fs event should arrive")
            .expect("stream should yield a change");
        assert!(!change.is_empty());
    }

    #[tokio::test]
    async fn watching_a_missing_root_errors() {
        let result = NotifyFsEventSource.watch(&["/no/such/shadowmask/root".to_owned()]);
        assert!(result.is_err());
    }
}
