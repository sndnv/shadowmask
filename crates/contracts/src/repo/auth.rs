use domain::repository::AuthTokenRepository;
use domain::user::{ApiToken, ApiTokenId, AuthSession, AuthSessionId, Device, DeviceId, UserId};
use jiff::Timestamp;

fn at(secs: i64) -> Timestamp {
    Timestamp::from_second(secs).expect("valid timestamp")
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
    })
    .await
    .unwrap();
    let stored = repo.get_device(&device_id).await.unwrap().unwrap();
    assert_eq!(stored.name, "Living Room");
    assert_eq!(stored.created_at, at(5));
    assert_eq!(stored.last_seen, Some(at(10)));

    repo.upsert_device(Device {
        id: device_id.clone(),
        user: UserId("u1".into()),
        name: "Bedroom".into(),
        platform: "roku".into(),
        created_at: at(999),
        last_seen: None,
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

    repo.revoke_api_token(&ApiTokenId("tok1".into())).await.unwrap();
    assert!(repo.list_api_tokens(&u1).await.unwrap().is_empty());
    assert!(repo.find_api_token_by_hash("hash-1").await.unwrap().is_none());
    repo.revoke_api_token(&ApiTokenId("tok1".into())).await.unwrap();

    repo.delete_device(&device_id).await.unwrap();
    assert!(repo.list_devices(&u1).await.unwrap().is_empty());
    assert!(repo.get_device(&device_id).await.unwrap().is_none());
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
    repo.upsert_device(Device {
        id: DeviceId("dev3".into()),
        user: u1.clone(),
        name: "Kitchen".into(),
        platform: "roku".into(),
        created_at: at(60),
        last_seen: None,
    })
    .await
    .unwrap();
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

    repo.revoke_all_for_user(&u1).await.unwrap();

    assert!(repo.find_refresh(&doomed).await.unwrap().is_none());
    assert!(repo.list_devices(&u1).await.unwrap().is_empty());
    assert!(repo.list_api_tokens(&u1).await.unwrap().is_empty());
    assert!(repo.find_api_token_by_hash("hash-3").await.unwrap().is_none());

    let survivor = "signing one account out must leave every other account signed in";
    assert!(repo.find_refresh(&spared).await.unwrap().is_some(), "{survivor}");
    assert_eq!(repo.list_devices(&u2).await.unwrap().len(), 1, "{survivor}");
    assert_eq!(repo.list_api_tokens(&u2).await.unwrap().len(), 1, "{survivor}");
    assert!(repo.find_api_token_by_hash("hash-2").await.unwrap().is_some(), "{survivor}");
}
