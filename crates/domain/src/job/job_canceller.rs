use crate::job::JobId;

pub trait JobCanceller: Send + Sync {
    fn request_cancel(&self, id: &JobId) -> bool;
}
