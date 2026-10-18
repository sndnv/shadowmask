use std::sync::{Arc, Mutex};

use tokio::sync::Notify;

#[derive(Default)]
struct Gates {
    reached: Notify,
    released: Notify,
}

#[derive(Clone, Default)]
pub struct Hold(Arc<Gates>);

impl Hold {
    pub async fn reached(&self) {
        self.0.reached.notified().await;
    }

    pub fn release(&self) {
        self.0.released.notify_one();
    }
}

#[derive(Clone, Default)]
pub struct NextHold(Arc<Mutex<Option<Hold>>>);

impl NextHold {
    pub fn arm(&self) -> Hold {
        let hold = Hold::default();
        *self.0.lock().unwrap() = Some(hold.clone());
        hold
    }

    pub async fn pass(&self) {
        let hold = self.0.lock().unwrap().take();
        if let Some(hold) = hold {
            hold.0.reached.notify_one();
            hold.0.released.notified().await;
        }
    }
}
