use std::future::Future;

pub trait FsEventStream: Send {
    fn next_change(&mut self) -> impl Future<Output = Option<Vec<String>>> + Send;
}
