use std::net::Ipv4Addr;

use domain::repository::AuthTokenRepository;
use domain::user::{Device, DeviceId, DeviceShare, DeviceTarget, TargetKind, UserId};
use jiff::Timestamp;

fn at(secs: i64) -> Timestamp {
    Timestamp::from_second(secs).expect("valid timestamp")
}

fn user(id: &str) -> UserId {
    UserId(id.into())
}

fn device(id: &str, owner: &str) -> Device {
    Device {
        id: DeviceId(id.into()),
        user: user(owner),
        name: id.into(),
        platform: "roku".into(),
        created_at: at(1),
        last_seen: None,
        target: None,
    }
}

fn share(device: &str, with: &str, created_at: i64) -> DeviceShare {
    DeviceShare { device: DeviceId(device.into()), user: user(with), created_at: at(created_at) }
}

async fn shared_with<R: AuthTokenRepository>(repo: &R, id: &str) -> Vec<String> {
    let devices = repo.list_shared_devices(&user(id)).await.unwrap();
    devices.into_iter().map(|device| device.id.0).collect()
}

pub async fn device_share_contract<R: AuthTokenRepository>(repo: R) {
    for (id, owner) in [("tv", "owner"), ("phone", "owner"), ("elsewhere", "other")] {
        repo.upsert_device(device(id, owner)).await.unwrap();
    }
    let target = DeviceTarget {
        kind: TargetKind::Roku,
        addresses: vec![Ipv4Addr::new(192, 168, 1, 40)],
        port: 8060,
        app_id: Some("dev".into()),
        reported_at: at(5),
    };
    repo.put_device_target(&DeviceId("tv".into()), &target).await.unwrap();

    repo.share_device(share("tv", "friend", 10)).await.unwrap();
    repo.share_device(share("tv", "friend", 20)).await.unwrap();
    repo.share_device(share("phone", "friend", 30)).await.unwrap();
    repo.share_device(share("tv", "other", 40)).await.unwrap();
    repo.share_device(share("elsewhere", "friend", 50)).await.unwrap();

    assert_eq!(
        repo.list_device_shares(&user("owner")).await.unwrap(),
        [share("phone", "friend", 30), share("tv", "friend", 10), share("tv", "other", 40)],
        "sharing again keeps the first share"
    );
    assert_eq!(shared_with(&repo, "friend").await, ["elsewhere", "phone", "tv"]);
    let listed = repo.list_shared_devices(&user("friend")).await.unwrap();
    assert_eq!(listed[2].target, Some(target), "a shared device carries its target report");
    assert_eq!(listed[2].user, user("owner"));
    assert!(shared_with(&repo, "owner").await.is_empty(), "nothing is shared with the owner");

    repo.unshare_device(&DeviceId("tv".into()), &user("other")).await.unwrap();
    repo.unshare_device(&DeviceId("tv".into()), &user("other")).await.unwrap();
    assert!(shared_with(&repo, "other").await.is_empty());
    assert_eq!(shared_with(&repo, "friend").await, ["elsewhere", "phone", "tv"]);

    repo.delete_device(&DeviceId("phone".into())).await.unwrap();
    repo.upsert_device(device("phone", "owner")).await.unwrap();
    assert_eq!(
        repo.list_device_shares(&user("owner")).await.unwrap(),
        [share("tv", "friend", 10)],
        "deleting a device deletes its shares"
    );

    repo.purge_user(&user("friend")).await.unwrap();
    assert!(repo.list_device_shares(&user("owner")).await.unwrap().is_empty());
    assert!(
        repo.list_device_shares(&user("other")).await.unwrap().is_empty(),
        "deleting a user deletes every share made to them"
    );

    repo.share_device(share("tv", "other", 60)).await.unwrap();
    repo.share_device(share("elsewhere", "owner", 70)).await.unwrap();
    repo.revoke_all_for_user(&user("owner")).await.unwrap();
    assert!(shared_with(&repo, "other").await.is_empty(), "the owner's devices went");
    assert_eq!(shared_with(&repo, "owner").await, ["elsewhere"], "shares to the owner stay");

    repo.share_device(share("elsewhere", "friend", 80)).await.unwrap();
    repo.purge_user(&user("other")).await.unwrap();
    assert!(shared_with(&repo, "owner").await.is_empty(), "the owner of a device purged");
    assert!(shared_with(&repo, "friend").await.is_empty());
}
