use crate::session::{SessionId, StreamGeneration, StreamRegistration};

pub trait StreamRegistry {
    fn register(&self, session: SessionId, generation: StreamGeneration, entry: StreamRegistration);
    fn remove(&self, session: &SessionId);
}
