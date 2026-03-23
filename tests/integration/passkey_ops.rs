use better_auth_core::adapters::{PasskeyOps, UserOps};
use better_auth_core::entity::{AuthPasskey, AuthUser};
use better_auth_core::types::{CreatePasskey, CreateUser};

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

fn make_passkey(user_id: &str, credential_id: &str) -> CreatePasskey {
    CreatePasskey {
        user_id: user_id.to_string(),
        name: "My Security Key".to_string(),
        credential_id: credential_id.to_string(),
        public_key: "pk_test_base64data".to_string(),
        counter: 0,
        device_type: "single_device".to_string(),
        backed_up: false,
        transports: Some("usb,nfc".to_string()),
    }
}

#[tokio::test]
async fn create_and_get_passkey_by_id() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "passkey-user@example.com").await;

    let passkey = adapter
        .create_passkey(make_passkey(&user_id, "cred_abc123"))
        .await
        .expect("create_passkey failed");

    assert!(!passkey.id().is_empty());
    assert_eq!(passkey.name(), "My Security Key");
    assert_eq!(passkey.credential_id(), "cred_abc123");
    assert_eq!(passkey.public_key(), "pk_test_base64data");
    assert_eq!(passkey.user_id(), &user_id);
    assert_eq!(passkey.counter(), 0);
    assert_eq!(passkey.device_type(), "single_device");
    assert!(!passkey.backed_up());
    assert_eq!(passkey.transports(), Some("usb,nfc"));

    let found = adapter
        .get_passkey_by_id(passkey.id())
        .await
        .expect("get_passkey_by_id failed")
        .expect("passkey not found");
    assert_eq!(found.id(), passkey.id());
    assert_eq!(found.credential_id(), "cred_abc123");
}

#[tokio::test]
async fn get_passkey_by_credential_id() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "passkey-cred@example.com").await;

    adapter
        .create_passkey(make_passkey(&user_id, "cred_unique_xyz"))
        .await
        .expect("create_passkey failed");

    let found = adapter
        .get_passkey_by_credential_id("cred_unique_xyz")
        .await
        .expect("get_passkey_by_credential_id failed")
        .expect("passkey not found by credential_id");
    assert_eq!(found.credential_id(), "cred_unique_xyz");
    assert_eq!(found.user_id(), &user_id);

    let not_found = adapter
        .get_passkey_by_credential_id("nonexistent_cred")
        .await
        .expect("get_passkey_by_credential_id failed");
    assert!(not_found.is_none());
}

#[tokio::test]
async fn list_passkeys_by_user() {
    let adapter = test_adapter().await;
    let user_a = create_test_user(&adapter, "passkey-list-a@example.com").await;
    let user_b = create_test_user(&adapter, "passkey-list-b@example.com").await;

    for i in 0..3 {
        adapter
            .create_passkey(make_passkey(&user_a, &format!("cred_a_{i}")))
            .await
            .expect("create_passkey failed");
    }
    adapter
        .create_passkey(make_passkey(&user_b, "cred_b_0"))
        .await
        .expect("create_passkey failed");

    let keys_a = adapter
        .list_passkeys_by_user(&user_a)
        .await
        .expect("list_passkeys_by_user failed");
    assert_eq!(keys_a.len(), 3);

    let keys_b = adapter
        .list_passkeys_by_user(&user_b)
        .await
        .expect("list_passkeys_by_user failed");
    assert_eq!(keys_b.len(), 1);
}

#[tokio::test]
async fn update_passkey_counter() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "passkey-counter@example.com").await;

    let passkey = adapter
        .create_passkey(make_passkey(&user_id, "cred_counter"))
        .await
        .expect("create_passkey failed");

    assert_eq!(passkey.counter(), 0);

    let updated = adapter
        .update_passkey_counter(passkey.id(), 42)
        .await
        .expect("update_passkey_counter failed");

    assert_eq!(updated.id(), passkey.id());
    assert_eq!(updated.counter(), 42);

    // Verify persisted via a fresh read
    let found = adapter
        .get_passkey_by_id(passkey.id())
        .await
        .expect("get_passkey_by_id failed")
        .expect("passkey not found");
    assert_eq!(found.counter(), 42);
}

#[tokio::test]
async fn update_passkey_name() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "passkey-rename@example.com").await;

    let passkey = adapter
        .create_passkey(make_passkey(&user_id, "cred_rename"))
        .await
        .expect("create_passkey failed");

    assert_eq!(passkey.name(), "My Security Key");

    let updated = adapter
        .update_passkey_name(passkey.id(), "Work Laptop YubiKey")
        .await
        .expect("update_passkey_name failed");

    assert_eq!(updated.id(), passkey.id());
    assert_eq!(updated.name(), "Work Laptop YubiKey");

    // Verify persisted via a fresh read
    let found = adapter
        .get_passkey_by_id(passkey.id())
        .await
        .expect("get_passkey_by_id failed")
        .expect("passkey not found");
    assert_eq!(found.name(), "Work Laptop YubiKey");
}

#[tokio::test]
async fn delete_passkey() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "passkey-delete@example.com").await;

    let passkey = adapter
        .create_passkey(make_passkey(&user_id, "cred_delete"))
        .await
        .expect("create_passkey failed");

    adapter
        .delete_passkey(passkey.id())
        .await
        .expect("delete_passkey failed");

    let found = adapter
        .get_passkey_by_id(passkey.id())
        .await
        .expect("get_passkey_by_id failed");
    assert!(found.is_none());
}

#[tokio::test]
async fn get_nonexistent_returns_none() {
    let adapter = test_adapter().await;

    let by_id = adapter
        .get_passkey_by_id("nonexistent-id")
        .await
        .expect("get_passkey_by_id failed");
    assert!(by_id.is_none());

    let by_cred = adapter
        .get_passkey_by_credential_id("nonexistent-cred")
        .await
        .expect("get_passkey_by_credential_id failed");
    assert!(by_cred.is_none());
}
