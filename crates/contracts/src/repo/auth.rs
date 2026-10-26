use std::net::Ipv4Addr;

use domain::repository::AuthTokenRepository;
use domain::user::{
    ApiToken, ApiTokenId, AuthSession, AuthSessionId, Device, DeviceId, DeviceTarget, PendingLink,
    Role, TargetKind, UserId,
};
use jiff::Timestamp;

fn at(secs: i64) -> Timestamp {
    Timestamp::from_second(secs).expect("valid timestamp")
}

fn device(id: &str, user: &UserId, name: &str, created_at: i64) -> Device {
    Device {
        id: DeviceId(id.into()),
        user: user.clone(),
        name: name.into(),
        platform: "roku".into(),
        created_at: at(created_at),
        last_seen: None,
        target: None,
    }
}

fn roku_target(reported_at: i64) -> DeviceTarget {
    DeviceTarget {
        kind: TargetKind::Roku,
        addresses: vec![Ipv4Addr::new(192, 168, 1, 40), Ipv4Addr::new(10, 0, 0, 2)],
        port: 8060,
        app_id: Some("dev".into()),
        reported_at: at(reported_at),
    }
}

pub async fn auth_token_repository_contract<R: AuthTokenRepository>(repo: R) {
    let device_id = DeviceId("dev1".into());
    assert!(repo.get_device(&device_id).await.unwrap().is_none());

    repo.upsert_device(Device {
        id: device_id.clone(),
        user: UserId("u1".into()),
        name: "Living Room".into(),
        platform: "roku".into(),
        created_at: at(5),
        last_seen: Some(at(10)),
        target: None,
    })
    .await
    .unwrap();
    let stored = repo.get_device(&device_id).await.unwrap().unwrap();
    assert_eq!(stored.name, "Living Room");
    assert_eq!(stored.created_at, at(5));
    assert_eq!(stored.last_seen, Some(at(10)));
    assert!(stored.target.is_none());

    repo.upsert_device(Device {
        id: device_id.clone(),
        user: UserId("u1".into()),
        name: "Bedroom".into(),
        platform: "roku".into(),
        created_at: at(999),
        last_seen: None,
        target: None,
    })
    .await
    .unwrap();
    let updated = repo.get_device(&device_id).await.unwrap().unwrap();
    assert_eq!(updated.name, "Bedroom");
    assert_eq!(updated.created_at, at(5));
    assert!(updated.last_seen.is_none());

    assert!(repo.find_api_token_by_hash("hash-1").await.unwrap().is_none());
    repo.store_api_token(ApiToken {
        id: ApiTokenId("tok1".into()),
        user: UserId("u1".into()),
        device: device_id.clone(),
        token_hash: "hash-1".into(),
        created_at: at(20),
        last_used_at: None,
    })
    .await
    .unwrap();
    let token = repo.find_api_token_by_hash("hash-1").await.unwrap().unwrap();
    assert_eq!(token.id, ApiTokenId("tok1".into()));
    assert_eq!(token.device, device_id);
    assert_eq!(token.created_at, at(20));
    assert!(token.last_used_at.is_none());

    repo.touch_api_token(&ApiTokenId("tok1".into()), at(40)).await.unwrap();
    let touched = repo.find_api_token_by_hash("hash-1").await.unwrap().unwrap();
    assert_eq!(touched.created_at, at(20));
    assert_eq!(touched.last_used_at, Some(at(40)));

    repo.upsert_device(Device {
        id: DeviceId("dev2".into()),
        user: UserId("u2".into()),
        name: "Other".into(),
        platform: "web".into(),
        created_at: at(15),
        last_seen: None,
        target: None,
    })
    .await
    .unwrap();
    repo.store_api_token(ApiToken {
        id: ApiTokenId("tok2".into()),
        user: UserId("u2".into()),
        device: DeviceId("dev2".into()),
        token_hash: "hash-2".into(),
        created_at: at(30),
        last_used_at: None,
    })
    .await
    .unwrap();

    let u1 = UserId("u1".into());
    let devices = repo.list_devices(&u1).await.unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].id, device_id);
    let tokens = repo.list_api_tokens(&u1).await.unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].id, ApiTokenId("tok1".into()));

    repo.put_device_target(&device_id, &roku_target(70)).await.unwrap();
    assert_eq!(repo.get_device(&device_id).await.unwrap().unwrap().target, Some(roku_target(70)));
    assert_eq!(repo.list_devices(&u1).await.unwrap()[0].target, Some(roku_target(70)));
    let android = DeviceTarget {
        kind: TargetKind::Android,
        addresses: vec![Ipv4Addr::new(172, 16, 0, 9)],
        port: 7979,
        app_id: None,
        reported_at: at(80),
    };
    repo.put_device_target(&device_id, &android).await.unwrap();
    assert_eq!(
        repo.get_device(&device_id).await.unwrap().unwrap().target,
        Some(android),
        "a second report replaces the first"
    );
    repo.delete_device_target(&device_id).await.unwrap();
    assert!(repo.get_device(&device_id).await.unwrap().unwrap().target.is_none());
    repo.delete_device_target(&device_id).await.unwrap();
    repo.put_device_target(&device_id, &roku_target(90)).await.unwrap();
    repo.put_device_target(&DeviceId("dev2".into()), &roku_target(100)).await.unwrap();

    repo.revoke_api_token(&ApiTokenId("tok1".into())).await.unwrap();
    assert!(repo.list_api_tokens(&u1).await.unwrap().is_empty());
    assert!(repo.find_api_token_by_hash("hash-1").await.unwrap().is_none());
    assert!(
        repo.get_device(&device_id).await.unwrap().unwrap().target.is_none(),
        "revoking a device's token deletes its target report"
    );
    assert_eq!(
        repo.get_device(&DeviceId("dev2".into())).await.unwrap().unwrap().target,
        Some(roku_target(100)),
        "revoking one token leaves every other device's target report"
    );
    repo.revoke_api_token(&ApiTokenId("tok1".into())).await.unwrap();

    repo.delete_device(&device_id).await.unwrap();
    assert!(repo.list_devices(&u1).await.unwrap().is_empty());
    assert!(repo.get_device(&device_id).await.unwrap().is_none());
    repo.delete_device(&device_id).await.unwrap();
    repo.upsert_device(device("dev1", &u1, "Bedroom", 95)).await.unwrap();
    assert!(
        repo.get_device(&device_id).await.unwrap().unwrap().target.is_none(),
        "deleting a device deletes its target report"
    );
    repo.delete_device(&device_id).await.unwrap();

    let u2 = UserId("u2".into());
    assert_eq!(repo.list_devices(&u2).await.unwrap().len(), 1);
    assert_eq!(repo.list_api_tokens(&u2).await.unwrap().len(), 1);

    let doomed = AuthSessionId("sess1".into());
    let spared = AuthSessionId("sess2".into());
    for (jti, user) in [(&doomed, &u1), (&spared, &u2)] {
        repo.store_refresh(AuthSession {
            id: jti.clone(),
            user: user.clone(),
            refresh_token_hash: format!("refresh-{}", user.0),
            issued_at: at(50),
            expires_at: at(3600),
        })
        .await
        .unwrap();
    }
    repo.upsert_device(device("dev3", &u1, "Kitchen", 60)).await.unwrap();
    repo.put_device_target(&DeviceId("dev3".into()), &roku_target(110)).await.unwrap();
    repo.store_api_token(ApiToken {
        id: ApiTokenId("tok3".into()),
        user: u1.clone(),
        device: DeviceId("dev3".into()),
        token_hash: "hash-3".into(),
        created_at: at(60),
        last_used_at: None,
    })
    .await
    .unwrap();
    for (code, user) in [("DOOMED", &u1), ("SPARED", &u2)] {
        let link = PendingLink {
            code: code.into(),
            user: user.clone(),
            role: Role::Player,
            expires_at: at(4000),
        };
        repo.store_link_code(link, at(60)).await.unwrap();
    }

    repo.revoke_all_for_user(&u1).await.unwrap();

    assert!(repo.find_refresh(&doomed).await.unwrap().is_none());
    assert!(repo.list_link_codes(&u1, at(60)).await.unwrap().is_empty());
    assert!(repo.list_devices(&u1).await.unwrap().is_empty());
    assert!(repo.list_api_tokens(&u1).await.unwrap().is_empty());
    assert!(repo.find_api_token_by_hash("hash-3").await.unwrap().is_none());
    repo.upsert_device(device("dev3", &u1, "Kitchen", 120)).await.unwrap();
    assert!(
        repo.get_device(&DeviceId("dev3".into())).await.unwrap().unwrap().target.is_none(),
        "signing an account out everywhere deletes its devices' target reports"
    );

    let survivor = "signing one account out must leave every other account signed in";
    assert!(repo.find_refresh(&spared).await.unwrap().is_some(), "{survivor}");
    assert_eq!(repo.list_link_codes(&u2, at(60)).await.unwrap().len(), 1, "{survivor}");
    assert_eq!(repo.list_devices(&u2).await.unwrap().len(), 1, "{survivor}");
    assert_eq!(
        repo.list_devices(&u2).await.unwrap()[0].target,
        Some(roku_target(100)),
        "{survivor}"
    );
    assert_eq!(repo.list_api_tokens(&u2).await.unwrap().len(), 1, "{survivor}");
    assert!(repo.find_api_token_by_hash("hash-2").await.unwrap().is_some(), "{survivor}");
}
