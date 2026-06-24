use crate::profile::CapabilityProfile;

pub trait ProfileRegistry {
    fn resolve(&self, platform: &str) -> CapabilityProfile;
}
