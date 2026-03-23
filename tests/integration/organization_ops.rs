use better_auth_core::adapters::{MemberOps, OrganizationOps, UserOps};
use better_auth_core::entity::AuthOrganization;
use better_auth_core::types::{CreateMember, CreateOrganization, CreateUser, UpdateOrganization};

use crate::common::test_adapter;

use better_auth_diesel_sqlite::DieselSqliteAdapter;

async fn create_test_user(adapter: &DieselSqliteAdapter, email: &str) -> String {
    use better_auth_core::entity::AuthUser;
    let user = adapter
        .create_user(CreateUser::new().with_email(email))
        .await
        .expect("create_user failed");
    user.id().to_string()
}

async fn create_test_org(adapter: &DieselSqliteAdapter, name: &str, slug: &str) -> String {
    let org = adapter
        .create_organization(CreateOrganization::new(name, slug))
        .await
        .expect("create_organization failed");
    org.id().to_string()
}

#[tokio::test]
async fn create_and_get_organization_by_id() {
    let adapter = test_adapter().await;
    let org = adapter
        .create_organization(
            CreateOrganization::new("Acme Corp", "acme-corp")
                .with_logo("https://example.com/logo.png")
                .with_metadata(serde_json::json!({"plan": "enterprise"})),
        )
        .await
        .expect("create_organization failed");

    assert_eq!(org.name(), "Acme Corp");
    assert_eq!(org.slug(), "acme-corp");
    assert_eq!(org.logo(), Some("https://example.com/logo.png"));
    assert!(org.metadata().is_some());
    assert!(!org.id().is_empty());

    let found = adapter
        .get_organization_by_id(org.id())
        .await
        .expect("get_organization_by_id failed")
        .expect("organization not found");
    assert_eq!(found.id(), org.id());
    assert_eq!(found.name(), "Acme Corp");
    assert_eq!(found.slug(), "acme-corp");
    assert_eq!(found.logo(), Some("https://example.com/logo.png"));
}

#[tokio::test]
async fn get_organization_by_slug() {
    let adapter = test_adapter().await;
    adapter
        .create_organization(CreateOrganization::new("Widget Co", "widget-co"))
        .await
        .expect("create_organization failed");

    let found = adapter
        .get_organization_by_slug("widget-co")
        .await
        .expect("get_organization_by_slug failed")
        .expect("organization not found");
    assert_eq!(found.name(), "Widget Co");
    assert_eq!(found.slug(), "widget-co");

    let not_found = adapter
        .get_organization_by_slug("nonexistent-slug")
        .await
        .expect("get_organization_by_slug failed");
    assert!(not_found.is_none());
}

#[tokio::test]
async fn update_organization() {
    let adapter = test_adapter().await;
    let org = adapter
        .create_organization(CreateOrganization::new("Old Name", "old-slug"))
        .await
        .expect("create_organization failed");

    let updated = adapter
        .update_organization(
            org.id(),
            UpdateOrganization {
                name: Some("New Name".to_string()),
                slug: Some("new-slug".to_string()),
                logo: Some("https://example.com/new-logo.png".to_string()),
                metadata: Some(serde_json::json!({"tier": "premium"})),
            },
        )
        .await
        .expect("update_organization failed");

    assert_eq!(updated.name(), "New Name");
    assert_eq!(updated.slug(), "new-slug");
    assert_eq!(updated.logo(), Some("https://example.com/new-logo.png"));

    // Verify the update persisted
    let found = adapter
        .get_organization_by_id(org.id())
        .await
        .expect("get_organization_by_id failed")
        .expect("organization not found");
    assert_eq!(found.name(), "New Name");
    assert_eq!(found.slug(), "new-slug");
}

#[tokio::test]
async fn delete_organization() {
    let adapter = test_adapter().await;
    let org = adapter
        .create_organization(CreateOrganization::new("To Delete", "to-delete"))
        .await
        .expect("create_organization failed");

    adapter
        .delete_organization(org.id())
        .await
        .expect("delete_organization failed");

    let found = adapter
        .get_organization_by_id(org.id())
        .await
        .expect("get_organization_by_id failed");
    assert!(found.is_none());
}

#[tokio::test]
async fn list_user_organizations() {
    let adapter = test_adapter().await;

    let user_id = create_test_user(&adapter, "org-user@example.com").await;
    let org1_id = create_test_org(&adapter, "Org One", "org-one").await;
    let org2_id = create_test_org(&adapter, "Org Two", "org-two").await;

    // Create memberships linking user to both organizations
    adapter
        .create_member(CreateMember::new(&org1_id, &user_id, "owner"))
        .await
        .expect("create_member for org1 failed");
    adapter
        .create_member(CreateMember::new(&org2_id, &user_id, "member"))
        .await
        .expect("create_member for org2 failed");

    let orgs = adapter
        .list_user_organizations(&user_id)
        .await
        .expect("list_user_organizations failed");
    assert_eq!(orgs.len(), 2);

    let slugs: Vec<&str> = orgs.iter().map(|o| o.slug()).collect();
    assert!(slugs.contains(&"org-one"));
    assert!(slugs.contains(&"org-two"));

    // A user with no memberships should get an empty list
    let other_user_id = create_test_user(&adapter, "no-orgs@example.com").await;
    let empty = adapter
        .list_user_organizations(&other_user_id)
        .await
        .expect("list_user_organizations failed");
    assert!(empty.is_empty());
}

#[tokio::test]
async fn get_nonexistent_returns_none() {
    let adapter = test_adapter().await;

    let by_id = adapter
        .get_organization_by_id("nonexistent-id")
        .await
        .expect("get_organization_by_id failed");
    assert!(by_id.is_none());

    let by_slug = adapter
        .get_organization_by_slug("nonexistent-slug")
        .await
        .expect("get_organization_by_slug failed");
    assert!(by_slug.is_none());
}
