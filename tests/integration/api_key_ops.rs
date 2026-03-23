use better_auth_core::adapters::{ApiKeyOps, UserOps};
use better_auth_core::entity::{AuthApiKey, AuthUser};
use better_auth_core::types::{CreateApiKey, CreateUser, UpdateApiKey};
use chrono::{Duration, Utc};

use crate::common::test_adapter;

async fn create_test_user(
    adapter: &better_auth_diesel_sqlite::DieselSqliteAdapter,
    email: &str,
) -> String {
    let user = adapter
        .create_user(CreateUser::new().with_email(email))
        .await
        .expect("create_user failed");
    user.id().to_string()
}

fn make_api_key(user_id: &str, hash: &str) -> CreateApiKey {
    CreateApiKey {
        user_id: user_id.to_string(),
        name: Some("test-key".to_string()),
        prefix: Some("sk_test".to_string()),
        key_hash: hash.to_string(),
        start: Some("sk_te".to_string()),
        expires_at: None,
        remaining: Some(100),
        rate_limit_enabled: false,
        rate_limit_time_window: None,
        rate_limit_max: None,
        refill_interval: None,
        refill_amount: None,
        permissions: None,
        metadata: None,
        enabled: true,
    }
}

#[tokio::test]
async fn create_and_get_api_key_by_id() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "apikey-user@example.com").await;

    let key = adapter
        .create_api_key(make_api_key(&user_id, "hash_abc123"))
        .await
        .expect("create_api_key failed");

    assert!(!key.id().is_empty());
    assert_eq!(key.name(), Some("test-key"));
    assert_eq!(key.prefix(), Some("sk_test"));
    assert_eq!(key.key_hash(), "hash_abc123");
    assert_eq!(key.user_id(), &user_id);
    assert!(key.enabled());
    assert_eq!(key.remaining(), Some(100));

    let found = adapter
        .get_api_key_by_id(key.id())
        .await
        .expect("get_api_key_by_id failed")
        .expect("api key not found");
    assert_eq!(found.id(), key.id());
    assert_eq!(found.key_hash(), "hash_abc123");
}

#[tokio::test]
async fn get_api_key_by_hash() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "apikey-hash@example.com").await;

    adapter
        .create_api_key(make_api_key(&user_id, "unique_hash_xyz"))
        .await
        .expect("create_api_key failed");

    let found = adapter
        .get_api_key_by_hash("unique_hash_xyz")
        .await
        .expect("get_api_key_by_hash failed")
        .expect("api key not found by hash");
    assert_eq!(found.key_hash(), "unique_hash_xyz");
    assert_eq!(found.user_id(), &user_id);

    let not_found = adapter
        .get_api_key_by_hash("nonexistent_hash")
        .await
        .expect("get_api_key_by_hash failed");
    assert!(not_found.is_none());
}

#[tokio::test]
async fn list_api_keys_by_user() {
    let adapter = test_adapter().await;
    let user_a = create_test_user(&adapter, "apikey-list-a@example.com").await;
    let user_b = create_test_user(&adapter, "apikey-list-b@example.com").await;

    for i in 0..3 {
        adapter
            .create_api_key(make_api_key(&user_a, &format!("hash_a_{i}")))
            .await
            .expect("create_api_key failed");
    }
    adapter
        .create_api_key(make_api_key(&user_b, "hash_b_0"))
        .await
        .expect("create_api_key failed");

    let keys_a = adapter
        .list_api_keys_by_user(&user_a)
        .await
        .expect("list_api_keys_by_user failed");
    assert_eq!(keys_a.len(), 3);

    let keys_b = adapter
        .list_api_keys_by_user(&user_b)
        .await
        .expect("list_api_keys_by_user failed");
    assert_eq!(keys_b.len(), 1);
}

#[tokio::test]
async fn update_api_key_disable() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "apikey-update@example.com").await;

    let key = adapter
        .create_api_key(make_api_key(&user_id, "hash_update"))
        .await
        .expect("create_api_key failed");

    assert!(key.enabled());

    let updated = adapter
        .update_api_key(
            key.id(),
            UpdateApiKey {
                enabled: Some(false),
                name: Some("renamed-key".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("update_api_key failed");

    assert!(!updated.enabled());
    assert_eq!(updated.name(), Some("renamed-key"));
    assert_eq!(updated.id(), key.id());
}

#[tokio::test]
async fn delete_api_key() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "apikey-delete@example.com").await;

    let key = adapter
        .create_api_key(make_api_key(&user_id, "hash_delete"))
        .await
        .expect("create_api_key failed");

    adapter
        .delete_api_key(key.id())
        .await
        .expect("delete_api_key failed");

    let found = adapter
        .get_api_key_by_id(key.id())
        .await
        .expect("get_api_key_by_id failed");
    assert!(found.is_none());
}

#[tokio::test]
async fn delete_expired_api_keys() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "apikey-expired@example.com").await;

    // Create an already-expired key
    let past = (Utc::now() - Duration::hours(1)).to_rfc3339();
    let mut expired_input = make_api_key(&user_id, "hash_expired");
    expired_input.expires_at = Some(past);
    adapter
        .create_api_key(expired_input)
        .await
        .expect("create expired api_key failed");

    // Create a non-expiring key
    adapter
        .create_api_key(make_api_key(&user_id, "hash_no_expiry"))
        .await
        .expect("create non-expiring api_key failed");

    // Create a key that expires in the future
    let future = (Utc::now() + Duration::hours(24)).to_rfc3339();
    let mut future_input = make_api_key(&user_id, "hash_future");
    future_input.expires_at = Some(future);
    adapter
        .create_api_key(future_input)
        .await
        .expect("create future api_key failed");

    let deleted = adapter
        .delete_expired_api_keys()
        .await
        .expect("delete_expired_api_keys failed");
    assert_eq!(deleted, 1);

    let remaining = adapter
        .list_api_keys_by_user(&user_id)
        .await
        .expect("list_api_keys_by_user failed");
    assert_eq!(remaining.len(), 2);

    // Verify the expired one is gone and the others remain
    let hashes: Vec<&str> = remaining.iter().map(|k| k.key_hash()).collect();
    assert!(hashes.contains(&"hash_no_expiry"));
    assert!(hashes.contains(&"hash_future"));
    assert!(!hashes.contains(&"hash_expired"));
}

#[tokio::test]
async fn get_nonexistent_returns_none() {
    let adapter = test_adapter().await;

    let by_id = adapter
        .get_api_key_by_id("nonexistent-id")
        .await
        .expect("get_api_key_by_id failed");
    assert!(by_id.is_none());

    let by_hash = adapter
        .get_api_key_by_hash("nonexistent-hash")
        .await
        .expect("get_api_key_by_hash failed");
    assert!(by_hash.is_none());
}
