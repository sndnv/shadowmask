use std::net::Ipv4Addr;

use jiff::Timestamp;

use crate::error::DeviceTargetError;

const MAX_ADDRESSES: usize = 4;
const MAX_APP_ID_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Roku,
    Android,
}

impl TargetKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "roku" => Some(Self::Roku),
            "android" => Some(Self::Android),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Roku => "roku",
            Self::Android => "android",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceTarget {
    pub kind: TargetKind,
    pub addresses: Vec<Ipv4Addr>,
    pub port: u16,
    pub app_id: Option<String>,
    pub reported_at: Timestamp,
}

impl DeviceTarget {
    pub fn new(
        kind: &str,
        addresses: &[String],
        port: i64,
        app_id: Option<String>,
        reported_at: Timestamp,
    ) -> Result<Self, DeviceTargetError> {
        let kind =
            TargetKind::parse(kind).ok_or_else(|| DeviceTargetError::Kind(kind.to_owned()))?;
        if addresses.is_empty() || addresses.len() > MAX_ADDRESSES {
            return Err(DeviceTargetError::AddressCount(addresses.len()));
        }
        let addresses =
            addresses.iter().map(|value| private_address(value)).collect::<Result<_, _>>()?;
        let port =
            u16::try_from(port).ok().filter(|p| *p > 0).ok_or(DeviceTargetError::Port(port))?;
        let app_id = match (kind, app_id) {
            (TargetKind::Roku, Some(id)) if valid_app_id(&id) => Some(id),
            (TargetKind::Roku, Some(id)) => return Err(DeviceTargetError::AppId(id)),
            (TargetKind::Roku, None) => return Err(DeviceTargetError::MissingAppId),
            (TargetKind::Android, Some(_)) => return Err(DeviceTargetError::UnexpectedAppId),
            (TargetKind::Android, None) => None,
        };
        Ok(Self { kind, addresses, port, app_id, reported_at })
    }
}

fn private_address(value: &str) -> Result<Ipv4Addr, DeviceTargetError> {
    value
        .parse::<Ipv4Addr>()
        .ok()
        .filter(Ipv4Addr::is_private)
        .ok_or_else(|| DeviceTargetError::Address(value.to_owned()))
}

fn valid_app_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= MAX_APP_ID_LEN && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addresses(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| (*v).to_owned()).collect()
    }

    fn roku(values: &[&str]) -> Result<DeviceTarget, DeviceTargetError> {
        DeviceTarget::new(
            "roku",
            &addresses(values),
            8060,
            Some("dev".into()),
            Timestamp::UNIX_EPOCH,
        )
    }

    #[test]
    fn a_roku_report_keeps_every_field() {
        let target = roku(&["192.168.1.40", "10.0.0.7"]).unwrap();
        assert_eq!(target.kind, TargetKind::Roku);
        assert_eq!(
            target.addresses,
            vec![Ipv4Addr::new(192, 168, 1, 40), Ipv4Addr::new(10, 0, 0, 7)]
        );
        assert_eq!(target.port, 8060);
        assert_eq!(target.app_id.as_deref(), Some("dev"));
        assert_eq!(target.reported_at, Timestamp::UNIX_EPOCH);
    }

    #[test]
    fn an_android_report_has_no_app_id() {
        let target =
            DeviceTarget::new("android", &addresses(&["172.16.0.2"]), 7979, None, Timestamp::now())
                .unwrap();
        assert_eq!(target.kind, TargetKind::Android);
        assert_eq!(target.app_id, None);
    }

    #[test]
    fn one_to_four_addresses_are_accepted() {
        assert!(roku(&["10.0.0.1"]).is_ok());
        assert!(roku(&["10.0.0.1", "10.0.0.2", "10.0.0.3", "10.0.0.4"]).is_ok());
        assert_eq!(roku(&[]).unwrap_err(), DeviceTargetError::AddressCount(0));
        assert_eq!(
            roku(&["10.0.0.1", "10.0.0.2", "10.0.0.3", "10.0.0.4", "10.0.0.5"]).unwrap_err(),
            DeviceTargetError::AddressCount(5)
        );
    }

    #[test]
    fn only_private_ipv4_addresses_are_accepted() {
        for accepted in ["10.255.255.255", "172.16.0.1", "172.31.255.254", "192.168.0.1"] {
            assert!(roku(&[accepted]).is_ok(), "{accepted}");
        }
        for refused in [
            "172.15.255.255",
            "172.32.0.1",
            "8.8.8.8",
            "127.0.0.1",
            "169.254.1.1",
            "0.0.0.0",
            "fd00::1",
            "abc",
            "",
        ] {
            assert_eq!(
                roku(&["10.0.0.1", refused]).unwrap_err(),
                DeviceTargetError::Address(refused.to_owned())
            );
        }
    }

    #[test]
    fn the_port_is_one_to_65535() {
        let at = |port| {
            DeviceTarget::new("android", &addresses(&["10.0.0.1"]), port, None, Timestamp::now())
        };
        assert_eq!(at(1).unwrap().port, 1);
        assert_eq!(at(65535).unwrap().port, 65535);
        assert_eq!(at(0).unwrap_err(), DeviceTargetError::Port(0));
        assert_eq!(at(65536).unwrap_err(), DeviceTargetError::Port(65536));
        assert_eq!(at(-1).unwrap_err(), DeviceTargetError::Port(-1));
    }

    #[test]
    fn a_roku_needs_a_plain_app_id() {
        let with = |app_id: Option<&str>| {
            DeviceTarget::new(
                "roku",
                &addresses(&["10.0.0.1"]),
                8060,
                app_id.map(str::to_owned),
                Timestamp::now(),
            )
        };
        assert!(with(Some("12345")).is_ok());
        assert!(with(Some(&"a".repeat(32))).is_ok());
        assert_eq!(with(None).unwrap_err(), DeviceTargetError::MissingAppId);
        for refused in ["", "dev/../x", "dev?x=1", "dév", &"a".repeat(33)] {
            assert_eq!(
                with(Some(refused)).unwrap_err(),
                DeviceTargetError::AppId(refused.to_owned())
            );
        }
    }

    #[test]
    fn an_android_report_refuses_an_app_id() {
        assert_eq!(
            DeviceTarget::new(
                "android",
                &addresses(&["10.0.0.1"]),
                7979,
                Some("dev".into()),
                Timestamp::now()
            )
            .unwrap_err(),
            DeviceTargetError::UnexpectedAppId
        );
    }

    #[test]
    fn an_unknown_kind_is_refused() {
        assert_eq!(
            DeviceTarget::new("desktop", &addresses(&["10.0.0.1"]), 1, None, Timestamp::now())
                .unwrap_err(),
            DeviceTargetError::Kind("desktop".into())
        );
    }

    #[test]
    fn every_kind_names_itself_for_the_wire() {
        for kind in [TargetKind::Roku, TargetKind::Android] {
            assert_eq!(TargetKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(TargetKind::parse("Roku"), None);
    }
}
