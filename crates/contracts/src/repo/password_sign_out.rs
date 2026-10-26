use std::net::Ipv4Addr;

use domain::repository::AuthTokenRepository;
use domain::user::{
    ApiToken, ApiTokenId, AuthSession, AuthSessionId, Device, DeviceId, DeviceShare, DeviceTarget,
    PendingLink, Role, TargetKind, UserId,
};
use jiff::Timestamp;

fn at(secs: i64) -> Timestamp {
    Timestamp::from_second(secs).expect("valid timestamp")
}

fn user(id: &str) -> UserId {
    UserId(id.into())
}

fn session(jti: &str, owner: &str) -> AuthSession {
    AuthSession {
        id: AuthSessionId(jti.into()),
        user: user(owner),
        refresh_token_hash: format!("refresh-{jti}"),
        issued_at: at(50),
        expires_at: at(3600),
    }
}

pub async fn password_sign_out_contract<R: AuthTokenRepository>(repo: R) {
    let target = DeviceTarget {
        kind: TargetKind::Roku,
        addresses: vec![Ipv4Addr::new(192, 168, 1, 40)],
        port: 8060,
        app_id: Some("dev".into()),
        reported_at: at(5),
    };
    repo.upsert_device(Device {
        id: DeviceId("tv".into()),
        user: user("owner"),
        name: "Living Room".into(),
        platform: "roku".into(),
        created_at: at(1),
        last_seen: None,
        target: None,
    })
    .await
    .unwrap();
    repo.put_device_target(&DeviceId("tv".into()), &target).await.unwrap();
    repo.store_api_token(ApiToken {
        id: ApiTokenId("tok".into()),
        user: user("owner"),
        device: DeviceId("tv".into()),
        token_hash: "hash-tv".into(),
        created_at: at(1),
        last_used_at: None,
    })
    .await
    .unwrap();
    let share =
        DeviceShare { device: DeviceId("tv".into()), user: user("friend"), created_at: at(9) };
    repo.share_device(share.clone()).await.unwrap();
    for (jti, owner) in [("first", "owner"), ("second", "owner"), ("friends", "friend")] {
        repo.store_refresh(session(jti, owner)).await.unwrap();
    }
    for (code, owner) in [("OWNERCODE", "owner"), ("FRIENDCODE", "friend")] {
        let link = PendingLink {
            code: code.into(),
            user: user(owner),
            role: Role::Player,
            expires_at: at(4000),
        };
        repo.store_link_code(link, at(10)).await.unwrap();
    }

    repo.revoke_refresh_and_link_codes(&user("owner")).await.unwrap();

    for jti in ["first", "second"] {
        assert!(repo.find_refresh(&AuthSessionId(jti.into())).await.unwrap().is_none());
    }
    assert!(
        repo.list_link_codes(&user("owner"), at(10)).await.unwrap().is_empty(),
        "a code made before the change cannot link a device after it"
    );
    assert_eq!(repo.list_link_codes(&user("friend"), at(10)).await.unwrap().len(), 1);
    let kept = "a password sign-out leaves the user's linked devices";
    let devices = repo.list_devices(&user("owner")).await.unwrap();
    assert_eq!(devices.len(), 1, "{kept}");
    assert_eq!(devices[0].target, Some(target), "{kept}");
    assert!(repo.find_api_token_by_hash("hash-tv").await.unwrap().is_some(), "{kept}");
    assert_eq!(repo.list_device_shares(&user("owner")).await.unwrap(), [share], "{kept}");
    assert!(
        repo.find_refresh(&AuthSessionId("friends".into())).await.unwrap().is_some(),
        "another user's sign-ins stay"
    );
}
