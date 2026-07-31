use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::job::{JobCanceller, JobId};
use tokio::sync::watch;

#[derive(Clone, Default)]
pub struct CancelRegistry {
    inner: Arc<Mutex<HashMap<JobId, watch::Sender<bool>>>>,
}

impl CancelRegistry {
    pub fn register(&self, id: &JobId) -> watch::Receiver<bool> {
        let (tx, rx) = watch::channel(false);
        self.inner.lock().unwrap().insert(id.clone(), tx);
        rx
    }

    pub fn deregister(&self, id: &JobId) {
        self.inner.lock().unwrap().remove(id);
    }

    pub fn cancel(&self, id: &JobId) -> bool {
        match self.inner.lock().unwrap().get(id) {
            Some(tx) => {
                let _ = tx.send(true);
                true
            }
            None => false,
        }
    }
}

impl JobCanceller for CancelRegistry {
    fn request_cancel(&self, id: &JobId) -> bool {
        self.cancel(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_then_cancel_signals_receiver() {
        let reg = CancelRegistry::default();
        let id = JobId("j".into());
        let mut rx = reg.register(&id);
        assert!(!*rx.borrow());
        assert!(reg.cancel(&id));
        assert!(rx.changed().await.is_ok());
        assert!(*rx.borrow());
    }

    #[test]
    fn cancel_unknown_is_false() {
        let reg = CancelRegistry::default();
        assert!(!reg.cancel(&JobId("nope".into())));
    }

    #[test]
    fn deregister_removes_entry() {
        let reg = CancelRegistry::default();
        let id = JobId("j".into());
        let _rx = reg.register(&id);
        reg.deregister(&id);
        assert!(!reg.cancel(&id));
    }

    #[test]
    fn request_cancel_delegates_to_cancel() {
        let reg = CancelRegistry::default();
        let id = JobId("j".into());
        let _rx = reg.register(&id);
        assert!(JobCanceller::request_cancel(&reg, &id));
        assert!(!JobCanceller::request_cancel(&reg, &JobId("other".into())));
    }
}
