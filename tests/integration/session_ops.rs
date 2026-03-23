use better_auth_core::adapters::{SessionOps, UserOps};
use better_auth_core::entity::{AuthSession, AuthUser};
use better_auth_core::types::{CreateSession, CreateUser};
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
async fn create_and_get_session() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "sess-user@example.com").await;

    let session = adapter
        .create_session(CreateSession {
            user_id: user_id.clone(),
            expires_at: Utc::now() + Duration::hours(1),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            impersonated_by: None,
            active_organization_id: None,
        })
        .await
        .expect("create_session failed");

    assert_eq!(session.user_id(), &user_id);
    assert!(session.token().starts_with("session_"));
    assert!(session.active());
    assert_eq!(session.ip_address(), Some("127.0.0.1"));

    let found = adapter
        .get_session(session.token())
        .await
        .expect("get_session failed")
        .expect("session not found");
    assert_eq!(found.id(), session.id());
}

#[tokio::test]
async fn get_session_inactive_returns_none() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "inactive-sess@example.com").await;

    let session = adapter
        .create_session(CreateSession {
            user_id,
            expires_at: Utc::now() + Duration::hours(1),
            ip_address: None,
            user_agent: None,
            impersonated_by: None,
            active_organization_id: None,
        })
        .await
        .expect("create_session failed");

    // Delete the session (makes it not findable by get_session)
    adapter
        .delete_session(session.token())
        .await
        .expect("delete_session failed");

    let found = adapter
        .get_session(session.token())
        .await
        .expect("get_session failed");
    assert!(found.is_none());
}

#[tokio::test]
async fn get_user_sessions() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "multi-sess@example.com").await;

    for _ in 0..3 {
        adapter
            .create_session(CreateSession {
                user_id: user_id.clone(),
                expires_at: Utc::now() + Duration::hours(1),
                ip_address: None,
                user_agent: None,
                impersonated_by: None,
                active_organization_id: None,
            })
            .await
            .expect("create_session failed");
    }

    let sessions = adapter
        .get_user_sessions(&user_id)
        .await
        .expect("get_user_sessions failed");
    assert_eq!(sessions.len(), 3);
}

#[tokio::test]
async fn update_session_expiry() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "expiry-sess@example.com").await;

    let session = adapter
        .create_session(CreateSession {
            user_id,
            expires_at: Utc::now() + Duration::hours(1),
            ip_address: None,
            user_agent: None,
            impersonated_by: None,
            active_organization_id: None,
        })
        .await
        .expect("create_session failed");

    let new_expiry = Utc::now() + Duration::days(7);
    adapter
        .update_session_expiry(session.token(), new_expiry)
        .await
        .expect("update_session_expiry failed");

    let found = adapter
        .get_session(session.token())
        .await
        .expect("get_session failed")
        .expect("session not found");

    // Check the expiry was updated (within 1 second tolerance)
    let diff = (found.expires_at() - new_expiry).num_seconds().abs();
    assert!(diff <= 1, "expiry should be updated, diff={diff}s");
}

#[tokio::test]
async fn delete_user_sessions() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "del-sess@example.com").await;

    for _ in 0..3 {
        adapter
            .create_session(CreateSession {
                user_id: user_id.clone(),
                expires_at: Utc::now() + Duration::hours(1),
                ip_address: None,
                user_agent: None,
                impersonated_by: None,
                active_organization_id: None,
            })
            .await
            .expect("create_session failed");
    }

    adapter
        .delete_user_sessions(&user_id)
        .await
        .expect("delete_user_sessions failed");

    let sessions = adapter
        .get_user_sessions(&user_id)
        .await
        .expect("get_user_sessions failed");
    assert!(sessions.is_empty());
}

#[tokio::test]
async fn delete_expired_sessions() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "expired-sess@example.com").await;

    // Create an already-expired session
    adapter
        .create_session(CreateSession {
            user_id: user_id.clone(),
            expires_at: Utc::now() - Duration::hours(1),
            ip_address: None,
            user_agent: None,
            impersonated_by: None,
            active_organization_id: None,
        })
        .await
        .expect("create expired session failed");

    // Create a valid session
    adapter
        .create_session(CreateSession {
            user_id: user_id.clone(),
            expires_at: Utc::now() + Duration::hours(1),
            ip_address: None,
            user_agent: None,
            impersonated_by: None,
            active_organization_id: None,
        })
        .await
        .expect("create valid session failed");

    let deleted = adapter
        .delete_expired_sessions()
        .await
        .expect("delete_expired_sessions failed");
    assert_eq!(deleted, 1);

    let remaining = adapter
        .get_user_sessions(&user_id)
        .await
        .expect("get_user_sessions failed");
    assert_eq!(remaining.len(), 1);
}

#[tokio::test]
async fn update_session_active_organization() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "org-sess@example.com").await;

    let session = adapter
        .create_session(CreateSession {
            user_id,
            expires_at: Utc::now() + Duration::hours(1),
            ip_address: None,
            user_agent: None,
            impersonated_by: None,
            active_organization_id: None,
        })
        .await
        .expect("create_session failed");

    assert!(session.active_organization_id().is_none());

    let updated = adapter
        .update_session_active_organization(session.token(), Some("org-123"))
        .await
        .expect("update_session_active_organization failed");
    assert_eq!(updated.active_organization_id(), Some("org-123"));

    // Clear it
    let cleared = adapter
        .update_session_active_organization(updated.token(), None)
        .await
        .expect("clear org failed");
    assert!(cleared.active_organization_id().is_none());
}
