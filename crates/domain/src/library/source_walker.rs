use std::future::Future;

use crate::error::WalkError;
use crate::library::Walked;

pub trait SourceWalker {
    fn walk(&self, root: &str) -> impl Future<Output = Result<Walked, WalkError>> + Send;
}
