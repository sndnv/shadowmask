use std::future::Future;

use crate::error::WalkError;
use crate::library::WalkedEntry;

pub trait SourceWalker {
    fn walk(&self, root: &str) -> impl Future<Output = Result<Vec<WalkedEntry>, WalkError>> + Send;
}
