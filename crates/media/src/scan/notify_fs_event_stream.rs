use notify::RecommendedWatcher;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::scan::FsEventStream;

pub struct NotifyFsEventStream {
    _watcher: RecommendedWatcher,
    rx: UnboundedReceiver<Vec<String>>,
}

impl NotifyFsEventStream {
    pub(crate) fn new(watcher: RecommendedWatcher, rx: UnboundedReceiver<Vec<String>>) -> Self {
        Self { _watcher: watcher, rx }
    }
}

impl FsEventStream for NotifyFsEventStream {
    async fn next_change(&mut self) -> Option<Vec<String>> {
        self.rx.recv().await
    }
}
