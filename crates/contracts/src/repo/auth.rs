use domain::repository::AuthTokenRepository;
use domain::user::{ApiToken, ApiTokenId, Device, DeviceId, UserId};
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
        last_seen: Some(at(10)),
    })
    .await
    .unwrap();
    let stored = repo.get_device(&device_id).await.unwrap().unwrap();
    assert_eq!(stored.name, "Living Room");
    assert_eq!(stored.last_seen, Some(at(10)));

    repo.upsert_device(Device {
        id: device_id.clone(),
        user: UserId("u1".into()),
        name: "Bedroom".into(),
        platform: "roku".into(),
        last_seen: None,
    })
    .await
    .unwrap();
    let updated = repo.get_device(&device_id).await.unwrap().unwrap();
    assert_eq!(updated.name, "Bedroom");
    assert!(updated.last_seen.is_none());

    assert!(
        repo.find_api_token_by_hash("hash-1")
            .await
            .unwrap()
            .is_none()
    );
    repo.store_api_token(ApiToken {
        id: ApiTokenId("tok1".into()),
        user: UserId("u1".into()),
        device: device_id.clone(),
        token_hash: "hash-1".into(),
        created_at: at(20),
    })
    .await
    .unwrap();
    let token = repo
        .find_api_token_by_hash("hash-1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(token.id, ApiTokenId("tok1".into()));
    assert_eq!(token.device, device_id);
    assert_eq!(token.created_at, at(20));

    repo.upsert_device(Device {
        id: DeviceId("dev2".into()),
        user: UserId("u2".into()),
        name: "Other".into(),
        platform: "web".into(),
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

    repo.revoke_api_token(&ApiTokenId("tok1".into()))
        .await
        .unwrap();
    assert!(repo.list_api_tokens(&u1).await.unwrap().is_empty());
    assert!(
        repo.find_api_token_by_hash("hash-1")
            .await
            .unwrap()
            .is_none()
    );
    repo.revoke_api_token(&ApiTokenId("tok1".into()))
        .await
        .unwrap();

    repo.delete_device(&device_id).await.unwrap();
    assert!(repo.list_devices(&u1).await.unwrap().is_empty());
    assert!(repo.get_device(&device_id).await.unwrap().is_none());
    repo.delete_device(&device_id).await.unwrap();

    let u2 = UserId("u2".into());
    assert_eq!(repo.list_devices(&u2).await.unwrap().len(), 1);
    assert_eq!(repo.list_api_tokens(&u2).await.unwrap().len(), 1);
}
