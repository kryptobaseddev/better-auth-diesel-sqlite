use better_auth_core::adapters::{AccountOps, UserOps};
use better_auth_core::entity::{AuthAccount, AuthUser};
use better_auth_core::types::{CreateAccount, CreateUser, UpdateAccount};
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

#[tokio::test]
async fn create_and_get_account() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "acct-user@example.com").await;

    let expires = Utc::now() + Duration::hours(1);
    let account = adapter
        .create_account(CreateAccount {
            user_id: user_id.clone(),
            account_id: "oauth-id-123".to_string(),
            provider_id: "github".to_string(),
            access_token: Some("access-tok".to_string()),
            refresh_token: Some("refresh-tok".to_string()),
            id_token: Some("id-tok".to_string()),
            access_token_expires_at: Some(expires),
            refresh_token_expires_at: None,
            scope: Some("read:user".to_string()),
            password: None,
        })
        .await
        .expect("create_account failed");

    assert!(!account.id().is_empty(), "account should have an id");
    assert_eq!(account.account_id(), "oauth-id-123");
    assert_eq!(account.provider_id(), "github");
    assert_eq!(account.user_id(), user_id);
    assert_eq!(account.access_token(), Some("access-tok"));
    assert_eq!(account.refresh_token(), Some("refresh-tok"));
    assert_eq!(account.id_token(), Some("id-tok"));
    assert_eq!(account.scope(), Some("read:user"));
    assert!(account.password().is_none(), "password should be None");

    let found = adapter
        .get_account("github", "oauth-id-123")
        .await
        .expect("get_account failed")
        .expect("account not found");
    assert_eq!(
        found.id(),
        account.id(),
        "retrieved account id should match"
    );
    assert_eq!(found.provider_id(), "github");
    assert_eq!(found.account_id(), "oauth-id-123");
}

#[tokio::test]
async fn get_user_accounts_multiple() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "multi-acct@example.com").await;

    // Create two accounts for the same user with different providers
    adapter
        .create_account(CreateAccount {
            user_id: user_id.clone(),
            account_id: "gh-id".to_string(),
            provider_id: "github".to_string(),
            access_token: Some("gh-tok".to_string()),
            refresh_token: None,
            id_token: None,
            access_token_expires_at: None,
            refresh_token_expires_at: None,
            scope: None,
            password: None,
        })
        .await
        .expect("create github account failed");

    adapter
        .create_account(CreateAccount {
            user_id: user_id.clone(),
            account_id: "google-id".to_string(),
            provider_id: "google".to_string(),
            access_token: Some("google-tok".to_string()),
            refresh_token: None,
            id_token: None,
            access_token_expires_at: None,
            refresh_token_expires_at: None,
            scope: None,
            password: None,
        })
        .await
        .expect("create google account failed");

    let accounts = adapter
        .get_user_accounts(&user_id)
        .await
        .expect("get_user_accounts failed");

    assert_eq!(accounts.len(), 2, "user should have exactly 2 accounts");

    let providers: Vec<&str> = accounts.iter().map(|a| a.provider_id()).collect();
    assert!(
        providers.contains(&"github"),
        "should contain github account"
    );
    assert!(
        providers.contains(&"google"),
        "should contain google account"
    );
}

#[tokio::test]
async fn update_account_tokens() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "update-acct@example.com").await;

    let account = adapter
        .create_account(CreateAccount {
            user_id,
            account_id: "provider-id".to_string(),
            provider_id: "github".to_string(),
            access_token: Some("old-access".to_string()),
            refresh_token: Some("old-refresh".to_string()),
            id_token: None,
            access_token_expires_at: None,
            refresh_token_expires_at: None,
            scope: Some("read".to_string()),
            password: None,
        })
        .await
        .expect("create_account failed");

    let new_expires = Utc::now() + Duration::hours(2);
    let updated = adapter
        .update_account(
            account.id(),
            UpdateAccount {
                access_token: Some("new-access".to_string()),
                refresh_token: Some("new-refresh".to_string()),
                id_token: Some("new-id-tok".to_string()),
                access_token_expires_at: Some(new_expires),
                scope: Some("read write".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("update_account failed");

    assert_eq!(
        updated.access_token(),
        Some("new-access"),
        "access_token should be updated"
    );
    assert_eq!(
        updated.refresh_token(),
        Some("new-refresh"),
        "refresh_token should be updated"
    );
    assert_eq!(
        updated.id_token(),
        Some("new-id-tok"),
        "id_token should be updated"
    );
    assert_eq!(
        updated.scope(),
        Some("read write"),
        "scope should be updated"
    );
}

#[tokio::test]
async fn delete_account() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "del-acct@example.com").await;

    let account = adapter
        .create_account(CreateAccount {
            user_id: user_id.clone(),
            account_id: "del-provider-id".to_string(),
            provider_id: "github".to_string(),
            access_token: None,
            refresh_token: None,
            id_token: None,
            access_token_expires_at: None,
            refresh_token_expires_at: None,
            scope: None,
            password: None,
        })
        .await
        .expect("create_account failed");

    adapter
        .delete_account(account.id())
        .await
        .expect("delete_account failed");

    let found = adapter
        .get_account("github", "del-provider-id")
        .await
        .expect("get_account failed");
    assert!(found.is_none(), "account should be deleted");

    let user_accounts = adapter
        .get_user_accounts(&user_id)
        .await
        .expect("get_user_accounts failed");
    assert!(
        user_accounts.is_empty(),
        "user should have no accounts after deletion"
    );
}

#[tokio::test]
async fn get_nonexistent_account_returns_none() {
    let adapter = test_adapter().await;

    let found = adapter
        .get_account("nonexistent-provider", "nonexistent-id")
        .await
        .expect("get_account failed");
    assert!(found.is_none(), "nonexistent account should return None");
}
