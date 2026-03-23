//! Integration tests for `InvitationOps`.

use better_auth_core::adapters::{InvitationOps, OrganizationOps, UserOps};
use better_auth_core::entity::{AuthInvitation, AuthOrganization, AuthUser};
use better_auth_core::types::{CreateInvitation, CreateOrganization, CreateUser, InvitationStatus};
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

async fn create_test_org(
    adapter: &better_auth_diesel_sqlite::DieselSqliteAdapter,
    name: &str,
    slug: &str,
) -> String {
    let org = adapter
        .create_organization(CreateOrganization::new(name, slug))
        .await
        .expect("create_organization failed");
    org.id().to_string()
}

#[tokio::test]
async fn create_and_get_invitation() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "inviter@example.com").await;
    let org_id = create_test_org(&adapter, "Test Org", "test-org").await;

    let inv = adapter
        .create_invitation(CreateInvitation::new(
            &org_id,
            "invited@example.com",
            "member",
            &user_id,
            Utc::now() + Duration::days(7),
        ))
        .await
        .expect("create_invitation failed");

    assert_eq!(inv.organization_id(), &org_id);
    assert_eq!(inv.email(), "invited@example.com");
    assert_eq!(inv.role(), "member");
    assert_eq!(*inv.status(), InvitationStatus::Pending);
    assert_eq!(inv.inviter_id(), &user_id);

    let found = adapter
        .get_invitation_by_id(inv.id())
        .await
        .expect("get_invitation_by_id failed")
        .expect("invitation not found");
    assert_eq!(found.id(), inv.id());
}

#[tokio::test]
async fn get_pending_invitation() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "inviter2@example.com").await;
    let org_id = create_test_org(&adapter, "Pending Org", "pending-org").await;

    adapter
        .create_invitation(CreateInvitation::new(
            &org_id,
            "pending@example.com",
            "member",
            &user_id,
            Utc::now() + Duration::days(7),
        ))
        .await
        .expect("create_invitation failed");

    let found = adapter
        .get_pending_invitation(&org_id, "pending@example.com")
        .await
        .expect("get_pending_invitation failed")
        .expect("pending invitation not found");
    assert_eq!(found.email(), "pending@example.com");

    // Non-matching email
    let not_found = adapter
        .get_pending_invitation(&org_id, "other@example.com")
        .await
        .expect("get_pending_invitation failed");
    assert!(not_found.is_none());
}

#[tokio::test]
async fn update_invitation_status_to_accepted() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "inviter3@example.com").await;
    let org_id = create_test_org(&adapter, "Accept Org", "accept-org").await;

    let inv = adapter
        .create_invitation(CreateInvitation::new(
            &org_id,
            "accept@example.com",
            "member",
            &user_id,
            Utc::now() + Duration::days(7),
        ))
        .await
        .expect("create_invitation failed");

    let updated = adapter
        .update_invitation_status(inv.id(), InvitationStatus::Accepted)
        .await
        .expect("update_invitation_status failed");
    assert_eq!(*updated.status(), InvitationStatus::Accepted);

    // No longer pending
    let pending = adapter
        .get_pending_invitation(&org_id, "accept@example.com")
        .await
        .expect("get_pending_invitation failed");
    assert!(pending.is_none());
}

#[tokio::test]
async fn list_organization_invitations() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "inviter4@example.com").await;
    let org_id = create_test_org(&adapter, "List Org", "list-org").await;

    for i in 0..3 {
        adapter
            .create_invitation(CreateInvitation::new(
                &org_id,
                format!("listinv{i}@example.com"),
                "member",
                &user_id,
                Utc::now() + Duration::days(7),
            ))
            .await
            .expect("create_invitation failed");
    }

    let invitations = adapter
        .list_organization_invitations(&org_id)
        .await
        .expect("list_organization_invitations failed");
    assert_eq!(invitations.len(), 3);
}

#[tokio::test]
async fn list_user_invitations() {
    let adapter = test_adapter().await;
    let user_id = create_test_user(&adapter, "inviter5@example.com").await;
    let org1_id = create_test_org(&adapter, "Org A", "org-a").await;
    let org2_id = create_test_org(&adapter, "Org B", "org-b").await;

    let target_email = "target@example.com";
    for org_id in [&org1_id, &org2_id] {
        adapter
            .create_invitation(CreateInvitation::new(
                org_id,
                target_email,
                "member",
                &user_id,
                Utc::now() + Duration::days(7),
            ))
            .await
            .expect("create_invitation failed");
    }

    let invitations = adapter
        .list_user_invitations(target_email)
        .await
        .expect("list_user_invitations failed");
    assert_eq!(invitations.len(), 2);

    // No invitations for other email
    let empty = adapter
        .list_user_invitations("nobody@example.com")
        .await
        .expect("list_user_invitations failed");
    assert!(empty.is_empty());
}

#[tokio::test]
async fn get_nonexistent_returns_none() {
    let adapter = test_adapter().await;

    let not_found = adapter
        .get_invitation_by_id("nonexistent-id")
        .await
        .expect("get_invitation_by_id failed");
    assert!(not_found.is_none());
}
