use better_auth_core::adapters::{MemberOps, OrganizationOps, UserOps};
use better_auth_core::entity::{AuthMember, AuthUser};
use better_auth_core::types::{CreateMember, CreateOrganization, CreateUser};

use crate::common::test_adapter;

use better_auth_diesel_sqlite::DieselSqliteAdapter;

async fn create_test_user(adapter: &DieselSqliteAdapter, email: &str) -> String {
    let user = adapter
        .create_user(CreateUser::new().with_email(email))
        .await
        .expect("create_user failed");
    user.id().to_string()
}

async fn create_test_org(adapter: &DieselSqliteAdapter, name: &str, slug: &str) -> String {
    use better_auth_core::entity::AuthOrganization;
    let org = adapter
        .create_organization(CreateOrganization::new(name, slug))
        .await
        .expect("create_organization failed");
    org.id().to_string()
}

#[tokio::test]
async fn create_and_get_member() {
    let adapter = test_adapter().await;

    let user_id = create_test_user(&adapter, "member-user@example.com").await;
    let org_id = create_test_org(&adapter, "Member Org", "member-org").await;

    let member = adapter
        .create_member(CreateMember::new(&org_id, &user_id, "owner"))
        .await
        .expect("create_member failed");

    assert_eq!(member.organization_id(), org_id);
    assert_eq!(member.user_id(), user_id);
    assert_eq!(member.role(), "owner");
    assert!(!member.id().is_empty());

    // Retrieve by (organization_id, user_id)
    let found = adapter
        .get_member(&org_id, &user_id)
        .await
        .expect("get_member failed")
        .expect("member not found");
    assert_eq!(found.id(), member.id());
    assert_eq!(found.role(), "owner");
}

#[tokio::test]
async fn get_member_by_id() {
    let adapter = test_adapter().await;

    let user_id = create_test_user(&adapter, "member-byid@example.com").await;
    let org_id = create_test_org(&adapter, "ById Org", "byid-org").await;

    let member = adapter
        .create_member(CreateMember::new(&org_id, &user_id, "admin"))
        .await
        .expect("create_member failed");

    let found = adapter
        .get_member_by_id(member.id())
        .await
        .expect("get_member_by_id failed")
        .expect("member not found");
    assert_eq!(found.id(), member.id());
    assert_eq!(found.organization_id(), org_id);
    assert_eq!(found.user_id(), user_id);
    assert_eq!(found.role(), "admin");

    // Nonexistent ID returns None
    let not_found = adapter
        .get_member_by_id("nonexistent-member-id")
        .await
        .expect("get_member_by_id failed");
    assert!(not_found.is_none());
}

#[tokio::test]
async fn update_member_role() {
    let adapter = test_adapter().await;

    let user_id = create_test_user(&adapter, "role-update@example.com").await;
    let org_id = create_test_org(&adapter, "Role Org", "role-org").await;

    let member = adapter
        .create_member(CreateMember::new(&org_id, &user_id, "member"))
        .await
        .expect("create_member failed");
    assert_eq!(member.role(), "member");

    let updated = adapter
        .update_member_role(member.id(), "admin")
        .await
        .expect("update_member_role failed");
    assert_eq!(updated.role(), "admin");
    assert_eq!(updated.id(), member.id());

    // Verify the update persisted
    let found = adapter
        .get_member_by_id(member.id())
        .await
        .expect("get_member_by_id failed")
        .expect("member not found");
    assert_eq!(found.role(), "admin");
}

#[tokio::test]
async fn delete_member() {
    let adapter = test_adapter().await;

    let user_id = create_test_user(&adapter, "delete-member@example.com").await;
    let org_id = create_test_org(&adapter, "Delete Org", "delete-org").await;

    let member = adapter
        .create_member(CreateMember::new(&org_id, &user_id, "member"))
        .await
        .expect("create_member failed");

    adapter
        .delete_member(member.id())
        .await
        .expect("delete_member failed");

    let found = adapter
        .get_member_by_id(member.id())
        .await
        .expect("get_member_by_id failed");
    assert!(found.is_none());

    let found_by_org = adapter
        .get_member(&org_id, &user_id)
        .await
        .expect("get_member failed");
    assert!(found_by_org.is_none());
}

#[tokio::test]
async fn list_organization_members() {
    let adapter = test_adapter().await;

    let org_id = create_test_org(&adapter, "List Org", "list-org").await;
    let user1_id = create_test_user(&adapter, "list-member1@example.com").await;
    let user2_id = create_test_user(&adapter, "list-member2@example.com").await;
    let user3_id = create_test_user(&adapter, "list-member3@example.com").await;

    adapter
        .create_member(CreateMember::new(&org_id, &user1_id, "owner"))
        .await
        .expect("create_member 1 failed");
    adapter
        .create_member(CreateMember::new(&org_id, &user2_id, "admin"))
        .await
        .expect("create_member 2 failed");
    adapter
        .create_member(CreateMember::new(&org_id, &user3_id, "member"))
        .await
        .expect("create_member 3 failed");

    let members = adapter
        .list_organization_members(&org_id)
        .await
        .expect("list_organization_members failed");
    assert_eq!(members.len(), 3);

    let roles: Vec<&str> = members.iter().map(|m| m.role()).collect();
    assert!(roles.contains(&"owner"));
    assert!(roles.contains(&"admin"));
    assert!(roles.contains(&"member"));

    // An org with no members returns empty
    let empty_org_id = create_test_org(&adapter, "Empty Org", "empty-org").await;
    let empty = adapter
        .list_organization_members(&empty_org_id)
        .await
        .expect("list_organization_members failed");
    assert!(empty.is_empty());
}

#[tokio::test]
async fn count_members_and_owners() {
    let adapter = test_adapter().await;

    let org_id = create_test_org(&adapter, "Count Org", "count-org").await;
    let user1_id = create_test_user(&adapter, "count-owner1@example.com").await;
    let user2_id = create_test_user(&adapter, "count-owner2@example.com").await;
    let user3_id = create_test_user(&adapter, "count-member@example.com").await;

    adapter
        .create_member(CreateMember::new(&org_id, &user1_id, "owner"))
        .await
        .expect("create_member 1 failed");
    adapter
        .create_member(CreateMember::new(&org_id, &user2_id, "owner"))
        .await
        .expect("create_member 2 failed");
    adapter
        .create_member(CreateMember::new(&org_id, &user3_id, "member"))
        .await
        .expect("create_member 3 failed");

    let total = adapter
        .count_organization_members(&org_id)
        .await
        .expect("count_organization_members failed");
    assert_eq!(total, 3);

    let owners = adapter
        .count_organization_owners(&org_id)
        .await
        .expect("count_organization_owners failed");
    assert_eq!(owners, 2);

    // Empty org has zero counts
    let empty_org_id = create_test_org(&adapter, "Zero Org", "zero-org").await;
    let zero_total = adapter
        .count_organization_members(&empty_org_id)
        .await
        .expect("count_organization_members failed");
    assert_eq!(zero_total, 0);

    let zero_owners = adapter
        .count_organization_owners(&empty_org_id)
        .await
        .expect("count_organization_owners failed");
    assert_eq!(zero_owners, 0);
}
