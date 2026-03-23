use better_auth_core::adapters::UserOps;
use better_auth_core::entity::AuthUser;
use better_auth_core::types::{CreateUser, ListUsersParams, UpdateUser};

use crate::common::test_adapter;

#[tokio::test]
async fn create_and_get_user_by_id() {
    let adapter = test_adapter().await;
    let user = adapter
        .create_user(
            CreateUser::new()
                .with_email("alice@example.com")
                .with_name("Alice"),
        )
        .await
        .expect("create_user failed");

    assert_eq!(user.email(), Some("alice@example.com"));
    assert_eq!(user.name(), Some("Alice"));
    assert!(!user.id().is_empty());

    let found = adapter
        .get_user_by_id(user.id())
        .await
        .expect("get_user_by_id failed")
        .expect("user not found");
    assert_eq!(found.id(), user.id());
    assert_eq!(found.email(), Some("alice@example.com"));
}

#[tokio::test]
async fn get_user_by_email() {
    let adapter = test_adapter().await;
    adapter
        .create_user(CreateUser::new().with_email("bob@example.com"))
        .await
        .expect("create_user failed");

    let found = adapter
        .get_user_by_email("bob@example.com")
        .await
        .expect("get_user_by_email failed")
        .expect("user not found");
    assert_eq!(found.email(), Some("bob@example.com"));

    let not_found = adapter
        .get_user_by_email("nobody@example.com")
        .await
        .expect("get_user_by_email failed");
    assert!(not_found.is_none());
}

#[tokio::test]
async fn get_user_by_username() {
    let adapter = test_adapter().await;
    adapter
        .create_user(
            CreateUser::new()
                .with_email("carol@example.com")
                .with_username("carol"),
        )
        .await
        .expect("create_user failed");

    let found = adapter
        .get_user_by_username("carol")
        .await
        .expect("get_user_by_username failed")
        .expect("user not found");
    assert_eq!(found.username(), Some("carol"));

    let not_found = adapter
        .get_user_by_username("nonexistent")
        .await
        .expect("get_user_by_username failed");
    assert!(not_found.is_none());
}

#[tokio::test]
async fn update_user() {
    let adapter = test_adapter().await;
    let user = adapter
        .create_user(
            CreateUser::new()
                .with_email("dave@example.com")
                .with_name("Dave"),
        )
        .await
        .expect("create_user failed");

    let updated = adapter
        .update_user(
            user.id(),
            UpdateUser {
                name: Some("David".to_string()),
                banned: Some(true),
                ban_reason: Some("test ban".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("update_user failed");

    assert_eq!(updated.name(), Some("David"));
    assert!(updated.banned());
    assert_eq!(updated.ban_reason(), Some("test ban"));
}

#[tokio::test]
async fn update_user_unban_clears_fields() {
    let adapter = test_adapter().await;
    let user = adapter
        .create_user(CreateUser::new().with_email("eve@example.com"))
        .await
        .expect("create_user failed");

    // Ban first
    adapter
        .update_user(
            user.id(),
            UpdateUser {
                banned: Some(true),
                ban_reason: Some("reason".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("ban failed");

    // Unban
    let unbanned = adapter
        .update_user(
            user.id(),
            UpdateUser {
                banned: Some(false),
                ..Default::default()
            },
        )
        .await
        .expect("unban failed");

    assert!(!unbanned.banned());
    assert!(unbanned.ban_reason().is_none());
    assert!(unbanned.ban_expires().is_none());
}

#[tokio::test]
async fn delete_user() {
    let adapter = test_adapter().await;
    let user = adapter
        .create_user(CreateUser::new().with_email("frank@example.com"))
        .await
        .expect("create_user failed");

    adapter
        .delete_user(user.id())
        .await
        .expect("delete_user failed");

    let found = adapter
        .get_user_by_id(user.id())
        .await
        .expect("get_user_by_id failed");
    assert!(found.is_none());
}

#[tokio::test]
async fn list_users_pagination() {
    let adapter = test_adapter().await;

    for i in 0..5 {
        adapter
            .create_user(CreateUser::new().with_email(format!("user{i}@example.com")))
            .await
            .expect("create_user failed");
    }

    let (users, total) = adapter
        .list_users(ListUsersParams {
            limit: Some(2),
            offset: Some(0),
            ..Default::default()
        })
        .await
        .expect("list_users failed");

    assert_eq!(total, 5);
    assert_eq!(users.len(), 2);

    let (page2, total2) = adapter
        .list_users(ListUsersParams {
            limit: Some(2),
            offset: Some(2),
            ..Default::default()
        })
        .await
        .expect("list_users page 2 failed");

    assert_eq!(total2, 5);
    assert_eq!(page2.len(), 2);
}

#[tokio::test]
async fn list_users_search_by_email() {
    let adapter = test_adapter().await;
    adapter
        .create_user(CreateUser::new().with_email("search-target@example.com"))
        .await
        .expect("create_user failed");
    adapter
        .create_user(CreateUser::new().with_email("other@example.com"))
        .await
        .expect("create_user failed");

    let (users, total) = adapter
        .list_users(ListUsersParams {
            search_field: Some("email".to_string()),
            search_value: Some("search-target".to_string()),
            ..Default::default()
        })
        .await
        .expect("list_users failed");

    assert_eq!(total, 1);
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email(), Some("search-target@example.com"));
}

#[tokio::test]
async fn duplicate_email_fails() {
    let adapter = test_adapter().await;
    adapter
        .create_user(CreateUser::new().with_email("dupe@example.com"))
        .await
        .expect("first create should succeed");

    let result = adapter
        .create_user(CreateUser::new().with_email("dupe@example.com"))
        .await;
    assert!(result.is_err(), "duplicate email should fail");
}

#[tokio::test]
async fn create_user_defaults() {
    let adapter = test_adapter().await;
    let user = adapter
        .create_user(CreateUser::new().with_email("defaults@example.com"))
        .await
        .expect("create_user failed");

    assert_eq!(user.role(), Some("user"));
    assert!(!user.banned());
    assert!(!user.email_verified());
    assert!(!user.two_factor_enabled());
}
