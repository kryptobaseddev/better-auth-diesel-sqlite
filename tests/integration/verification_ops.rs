use better_auth_core::adapters::VerificationOps;
use better_auth_core::entity::AuthVerification;
use better_auth_core::types::CreateVerification;
use chrono::{Duration, Utc};

use crate::common::test_adapter;

#[tokio::test]
async fn create_and_get_verification() {
    let adapter = test_adapter().await;

    let expires = Utc::now() + Duration::hours(1);
    let verification = adapter
        .create_verification(CreateVerification {
            identifier: "user@example.com".to_string(),
            value: "token-abc-123".to_string(),
            expires_at: expires,
        })
        .await
        .expect("create_verification failed");

    assert!(
        !verification.id().is_empty(),
        "verification should have an id"
    );
    assert_eq!(verification.identifier(), "user@example.com");
    assert_eq!(verification.value(), "token-abc-123");

    let found = adapter
        .get_verification("user@example.com", "token-abc-123")
        .await
        .expect("get_verification failed")
        .expect("verification not found");
    assert_eq!(found.id(), verification.id(), "retrieved id should match");
    assert_eq!(found.identifier(), "user@example.com");
    assert_eq!(found.value(), "token-abc-123");
}

#[tokio::test]
async fn get_verification_by_value() {
    let adapter = test_adapter().await;

    let expires = Utc::now() + Duration::hours(1);
    adapter
        .create_verification(CreateVerification {
            identifier: "alice@example.com".to_string(),
            value: "unique-token-456".to_string(),
            expires_at: expires,
        })
        .await
        .expect("create_verification failed");

    let found = adapter
        .get_verification_by_value("unique-token-456")
        .await
        .expect("get_verification_by_value failed")
        .expect("verification not found by value");
    assert_eq!(found.identifier(), "alice@example.com");
    assert_eq!(found.value(), "unique-token-456");

    let not_found = adapter
        .get_verification_by_value("nonexistent-token")
        .await
        .expect("get_verification_by_value failed");
    assert!(not_found.is_none(), "nonexistent token should return None");
}

#[tokio::test]
async fn get_verification_by_identifier() {
    let adapter = test_adapter().await;

    let expires = Utc::now() + Duration::hours(1);
    adapter
        .create_verification(CreateVerification {
            identifier: "bob@example.com".to_string(),
            value: "bob-token-789".to_string(),
            expires_at: expires,
        })
        .await
        .expect("create_verification failed");

    let found = adapter
        .get_verification_by_identifier("bob@example.com")
        .await
        .expect("get_verification_by_identifier failed")
        .expect("verification not found by identifier");
    assert_eq!(found.identifier(), "bob@example.com");
    assert_eq!(found.value(), "bob-token-789");

    let not_found = adapter
        .get_verification_by_identifier("nobody@example.com")
        .await
        .expect("get_verification_by_identifier failed");
    assert!(
        not_found.is_none(),
        "nonexistent identifier should return None"
    );
}

#[tokio::test]
async fn consume_verification_returns_and_deletes() {
    let adapter = test_adapter().await;

    let expires = Utc::now() + Duration::hours(1);
    adapter
        .create_verification(CreateVerification {
            identifier: "consume@example.com".to_string(),
            value: "consume-token".to_string(),
            expires_at: expires,
        })
        .await
        .expect("create_verification failed");

    // Consume should return the verification
    let consumed = adapter
        .consume_verification("consume@example.com", "consume-token")
        .await
        .expect("consume_verification failed")
        .expect("consume should return the verification");
    assert_eq!(consumed.identifier(), "consume@example.com");
    assert_eq!(consumed.value(), "consume-token");

    // After consumption, it should no longer exist
    let after = adapter
        .get_verification("consume@example.com", "consume-token")
        .await
        .expect("get_verification failed");
    assert!(
        after.is_none(),
        "verification should be deleted after consumption"
    );
}

#[tokio::test]
async fn consume_verification_nonexistent_returns_none() {
    let adapter = test_adapter().await;

    let result = adapter
        .consume_verification("nonexistent@example.com", "no-such-token")
        .await
        .expect("consume_verification failed");
    assert!(
        result.is_none(),
        "consuming nonexistent verification should return None"
    );
}

#[tokio::test]
async fn delete_expired_verifications() {
    let adapter = test_adapter().await;

    // Create an already-expired verification
    adapter
        .create_verification(CreateVerification {
            identifier: "expired@example.com".to_string(),
            value: "expired-token".to_string(),
            expires_at: Utc::now() - Duration::hours(1),
        })
        .await
        .expect("create expired verification failed");

    // Create another expired verification
    adapter
        .create_verification(CreateVerification {
            identifier: "expired2@example.com".to_string(),
            value: "expired-token-2".to_string(),
            expires_at: Utc::now() - Duration::minutes(30),
        })
        .await
        .expect("create second expired verification failed");

    // Create a valid (non-expired) verification
    adapter
        .create_verification(CreateVerification {
            identifier: "valid@example.com".to_string(),
            value: "valid-token".to_string(),
            expires_at: Utc::now() + Duration::hours(1),
        })
        .await
        .expect("create valid verification failed");

    let deleted = adapter
        .delete_expired_verifications()
        .await
        .expect("delete_expired_verifications failed");
    assert_eq!(deleted, 2, "should delete exactly 2 expired verifications");

    // The valid one should still exist
    let remaining = adapter
        .get_verification("valid@example.com", "valid-token")
        .await
        .expect("get_verification failed");
    assert!(
        remaining.is_some(),
        "non-expired verification should still exist"
    );

    // The expired ones should be gone
    let gone = adapter
        .get_verification("expired@example.com", "expired-token")
        .await
        .expect("get_verification failed");
    assert!(gone.is_none(), "expired verification should be deleted");
}

#[tokio::test]
async fn delete_verification_by_id() {
    let adapter = test_adapter().await;

    let expires = Utc::now() + Duration::hours(1);
    let verification = adapter
        .create_verification(CreateVerification {
            identifier: "delete-me@example.com".to_string(),
            value: "delete-me-token".to_string(),
            expires_at: expires,
        })
        .await
        .expect("create_verification failed");

    adapter
        .delete_verification(verification.id())
        .await
        .expect("delete_verification failed");

    let found = adapter
        .get_verification("delete-me@example.com", "delete-me-token")
        .await
        .expect("get_verification failed");
    assert!(
        found.is_none(),
        "verification should be deleted after delete_verification"
    );
}
