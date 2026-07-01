use crate::scan::FsEventStream;

pub trait FsEventSource: Send + Sync {
    type Stream: FsEventStream;

    fn watch(&self, roots: &[String]) -> notify::Result<Self::Stream>;
}
