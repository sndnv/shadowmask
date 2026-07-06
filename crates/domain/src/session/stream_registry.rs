use crate::session::{SessionId, StreamRegistration};

pub trait StreamRegistry {
    fn register(&self, session: SessionId, entry: StreamRegistration);
    fn remove(&self, session: &SessionId);
}
