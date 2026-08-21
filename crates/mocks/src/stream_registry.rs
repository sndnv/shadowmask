use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::session::{SessionId, StreamRegistration, StreamRegistry};

#[derive(Default)]
struct State {
    registrations: HashMap<SessionId, StreamRegistration>,
    removed: Vec<SessionId>,
}

#[derive(Clone, Default)]
pub struct MockStreamRegistry {
    state: Arc<Mutex<State>>,
}

impl MockStreamRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn registration(&self, session: &SessionId) -> Option<StreamRegistration> {
        self.state
            .lock()
            .unwrap()
            .registrations
            .get(session)
            .cloned()
    }

    pub fn removed(&self) -> Vec<SessionId> {
        self.state.lock().unwrap().removed.clone()
    }
}

impl StreamRegistry for MockStreamRegistry {
    fn register(&self, session: SessionId, entry: StreamRegistration) {
        self.state
            .lock()
            .unwrap()
            .registrations
            .insert(session, entry);
    }

    fn remove(&self, session: &SessionId) {
        let mut state = self.state.lock().unwrap();
        state.registrations.remove(session);
        state.removed.push(session.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use domain::session::DeliveryMode;

    fn entry() -> StreamRegistration {
        StreamRegistration {
            mode: DeliveryMode::Transcode,
            output_dir: PathBuf::from("/mock/cache/s1"),
            direct_path: None,
            bandwidth: 1_000_000,
            subtitle: None,
        }
    }

    #[test]
    fn register_then_remove() {
        let registry = MockStreamRegistry::new();
        let session = SessionId("s1".to_owned());
        assert!(registry.registration(&session).is_none());
        registry.register(session.clone(), entry());
        assert_eq!(
            registry.registration(&session).unwrap().mode,
            DeliveryMode::Transcode
        );
        registry.remove(&session);
        assert!(registry.registration(&session).is_none());
        assert_eq!(registry.removed(), vec![session]);
    }
}
