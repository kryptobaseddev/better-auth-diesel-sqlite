# Plan: better-auth-diesel-sqlite

## Summary

Build a standalone, community-driven SQLite adapter for better-auth-rs using Diesel ORM. The adapter implements all 10 `*Ops` traits required by the `DatabaseAdapter` supertrait, enabling better-auth-rs's full plugin ecosystem (API keys, organizations, admin, 2FA, passkeys, etc.) to work with SQLite databases.

## Phase 1: Foundation

### 1.1 Project Scaffolding

- [x] Initialize git repository
- [x] Create VISION.md, ARCHITECTURE.md, PLAN.md
- [x] Create Cargo.toml with all dependencies
- [x] Create src/lib.rs with crate-level documentation
- [x] Create .gitignore
- [x] Create LICENSE (MIT)
- [x] Create README.md
- [x] Initialize ferrous-forge (`ferrous-forge init --project`)
- [x] Verify project compiles with `cargo check` (zero errors, zero warnings)

### 1.2 Schema Definition

- [x] Study better-auth-rs's Postgres schema from `SqlxAdapter` CREATE TABLE statements
- [x] Translate Postgres schema to SQLite dialect
- [x] Create `diesel.toml` pointing to `src/schema.rs`
- [x] Create `migrations/00000000000000_create_auth_tables/up.sql`
- [x] Create `migrations/00000000000000_create_auth_tables/down.sql`
- [x] Write `src/schema.rs` with hand-written `table!` macros matching `Auth*Meta` defaults
- [x] Verify schema compiles

### 1.3 Model Layer

- [x] Define `UserRow` / `NewUserRow` / `UpdateUserRow` in `src/models.rs`
- [x] Define `SessionRow` / `NewSessionRow`
- [x] Define `AccountRow` / `NewAccountRow` / `UpdateAccountRow`
- [x] Define `VerificationRow` / `NewVerificationRow`
- [x] Define `OrganizationRow` / `NewOrganizationRow` / `UpdateOrganizationRow`
- [x] Define `MemberRow` / `NewMemberRow`
- [x] Define `InvitationRow` / `NewInvitationRow`
- [x] Define `TwoFactorRow` / `NewTwoFactorRow`
- [x] Define `ApiKeyRow` / `NewApiKeyRow` / `UpdateApiKeyRow`
- [x] Define `PasskeyRow` / `NewPasskeyRow`
- [x] Verify all models derive `Queryable`, `Insertable`, `Selectable` (and `AsChangeset` for update models)

### 1.4 Conversion Layer

- [x] Implement `From<CreateUser>` for `NewUserRow` and `From<UserRow>` for `User`
- [x] Implement `From<UpdateUser>` for `UpdateUserRow` (with unban logic)
- [x] Implement conversions for all 10 entity types (Session, Account, Verification, Organization, Member, Invitation, TwoFactor, ApiKey, Passkey)
- [x] Handle type conversions: String (ISO 8601) <-> DateTime, String (JSON) <-> serde_json::Value
- [x] `parse_datetime()`, `format_datetime()`, `now_iso()` helpers
- [x] `invitation_status_to_string()` for enum mapping

### 1.5 Error Mapping

- [x] Define `AdapterError` enum in `src/error.rs`
- [x] Implement `From<AdapterError>` for `AuthError`
- [x] `diesel_to_database_error()` — maps Diesel errors to `DatabaseError` (orphan rule compliant)
- [x] `diesel_to_auth_error()` — convenience wrapper
- [x] Handle all Diesel error variants (NotFound, UniqueViolation, ForeignKeyViolation, RollbackTransaction)

### 1.6 Adapter Core

- [x] Define `DieselSqliteAdapter` struct with `Arc<Mutex<SqliteConnection>>`
- [x] Define `PoolConfig` struct in `src/config.rs` (with builder pattern)
- [x] Implement `new(database_url)` constructor with pragma setup
- [x] Implement `with_config(database_url, config)` constructor
- [x] Implement `in_memory()` constructor for testing
- [x] Implement `interact()` — async bridge via `spawn_blocking` + mutex lock
- [x] Apply SQLite pragmas (WAL, busy_timeout, synchronous, foreign_keys, cache_size)
- [x] Implement `run_migrations()` with embedded `diesel_migrations`

**Design decision**: Using `Arc<Mutex<SqliteConnection>>` instead of a connection pool. SQLite only supports one writer at a time, so a pool would just contend. The mutex is acquired inside `spawn_blocking` so it never blocks the async runtime. `PoolConfig` is accepted for API forward-compatibility. A deadpool-based pool can be added later behind a feature flag for read-heavy workloads.

## Phase 2: Operation Trait Implementations

All 10 traits are fully implemented. Each follows the same pattern:
1. Call `self.interact(|conn| { ... })` on the adapter
2. Run synchronous Diesel queries against `&mut SqliteConnection`
3. Map result from Diesel model to better-auth entity via `From` impls
4. Map errors to `AuthError` via `diesel_to_auth_error()`

### 2.1 UserOps (src/ops/user.rs) — 7 methods

| Method | Status | Notes |
|--------|--------|-------|
| `create_user` | Done | Insert + return created row |
| `get_user_by_id` | Done | SELECT WHERE id = ? |
| `get_user_by_email` | Done | SELECT WHERE email = ? |
| `get_user_by_username` | Done | SELECT WHERE username = ? |
| `update_user` | Done | Dynamic `AsChangeset` from `UpdateUser`, unban clears reason/expiry |
| `delete_user` | Done | DELETE WHERE id = ? |
| `list_users` | Done | Dynamic WHERE (LIKE search), ORDER BY, LIMIT/OFFSET, COUNT |

### 2.2 SessionOps (src/ops/session.rs) — 8 methods

| Method | Status | Notes |
|--------|--------|-------|
| `create_session` | Done | Insert with generated `session_` prefixed token |
| `get_session` | Done | SELECT WHERE token = ? AND active = true |
| `get_user_sessions` | Done | SELECT WHERE user_id = ? ORDER BY created_at DESC |
| `update_session_expiry` | Done | UPDATE expires_at, updated_at |
| `delete_session` | Done | DELETE WHERE token = ? |
| `delete_user_sessions` | Done | DELETE WHERE user_id = ? |
| `delete_expired_sessions` | Done | DELETE WHERE expires_at < NOW OR active = false, returns count |
| `update_session_active_organization` | Done | UPDATE active_organization_id, updated_at |

### 2.3 AccountOps (src/ops/account.rs) — 5 methods

| Method | Status | Notes |
|--------|--------|-------|
| `create_account` | Done | Insert |
| `get_account` | Done | SELECT WHERE provider_id = ? AND account_id = ? |
| `get_user_accounts` | Done | SELECT WHERE user_id = ? |
| `update_account` | Done | `AsChangeset` from `UpdateAccount` |
| `delete_account` | Done | DELETE WHERE id = ? |

### 2.4 VerificationOps (src/ops/verification.rs) — 7 methods

| Method | Status | Notes |
|--------|--------|-------|
| `create_verification` | Done | Insert |
| `get_verification` | Done | SELECT WHERE identifier = ? AND value = ? |
| `get_verification_by_value` | Done | SELECT WHERE value = ? |
| `get_verification_by_identifier` | Done | SELECT WHERE identifier = ? |
| `consume_verification` | Done | Atomic: SELECT + DELETE in single `interact` call |
| `delete_verification` | Done | DELETE WHERE id = ? |
| `delete_expired_verifications` | Done | DELETE WHERE expires_at < NOW, returns count |

### 2.5 OrganizationOps (src/ops/organization.rs) — 6 methods

| Method | Status | Notes |
|--------|--------|-------|
| `create_organization` | Done | Insert |
| `get_organization_by_id` | Done | SELECT WHERE id = ? |
| `get_organization_by_slug` | Done | SELECT WHERE slug = ? |
| `update_organization` | Done | `AsChangeset` from `UpdateOrganization` |
| `delete_organization` | Done | DELETE WHERE id = ? (cascades via FK) |
| `list_user_organizations` | Done | INNER JOIN member ON organization via Diesel JoinOnDsl |

### 2.6 MemberOps (src/ops/member.rs) — 8 methods

| Method | Status | Notes |
|--------|--------|-------|
| `create_member` | Done | Insert |
| `get_member` | Done | SELECT WHERE organization_id = ? AND user_id = ? |
| `get_member_by_id` | Done | SELECT WHERE id = ? |
| `update_member_role` | Done | UPDATE role WHERE id = ? |
| `delete_member` | Done | DELETE WHERE id = ? |
| `list_organization_members` | Done | SELECT WHERE organization_id = ? ORDER BY created_at ASC |
| `count_organization_members` | Done | SELECT COUNT WHERE organization_id = ? |
| `count_organization_owners` | Done | SELECT COUNT WHERE organization_id = ? AND role = 'owner' |

### 2.7 InvitationOps (src/ops/invitation.rs) — 6 methods

| Method | Status | Notes |
|--------|--------|-------|
| `create_invitation` | Done | Insert with status = 'pending' |
| `get_invitation_by_id` | Done | SELECT WHERE id = ? |
| `get_pending_invitation` | Done | SELECT WHERE organization_id = ? AND email = ? AND status = 'pending' |
| `update_invitation_status` | Done | UPDATE status WHERE id = ?, uses `invitation_status_to_string()` |
| `list_organization_invitations` | Done | SELECT WHERE organization_id = ? ORDER BY created_at DESC |
| `list_user_invitations` | Done | SELECT WHERE email = ? ORDER BY created_at DESC |

### 2.8 TwoFactorOps (src/ops/two_factor.rs) — 4 methods

| Method | Status | Notes |
|--------|--------|-------|
| `create_two_factor` | Done | Insert |
| `get_two_factor_by_user_id` | Done | SELECT WHERE user_id = ? |
| `update_two_factor_backup_codes` | Done | UPDATE backup_codes, updated_at WHERE user_id = ? |
| `delete_two_factor` | Done | DELETE WHERE user_id = ? |

### 2.9 ApiKeyOps (src/ops/api_key.rs) — 7 methods

| Method | Status | Notes |
|--------|--------|-------|
| `create_api_key` | Done | Insert (key_hash stored in `key` column) |
| `get_api_key_by_id` | Done | SELECT WHERE id = ? |
| `get_api_key_by_hash` | Done | SELECT WHERE key = ? |
| `list_api_keys_by_user` | Done | SELECT WHERE user_id = ? |
| `update_api_key` | Done | `AsChangeset` from `UpdateApiKey` |
| `delete_api_key` | Done | DELETE WHERE id = ? |
| `delete_expired_api_keys` | Done | DELETE WHERE expires_at IS NOT NULL AND expires_at < NOW |

### 2.10 PasskeyOps (src/ops/passkey.rs) — 7 methods

| Method | Status | Notes |
|--------|--------|-------|
| `create_passkey` | Done | Insert (counter stored as i64, cast from u64) |
| `get_passkey_by_id` | Done | SELECT WHERE id = ? |
| `get_passkey_by_credential_id` | Done | SELECT WHERE credential_id = ? |
| `list_passkeys_by_user` | Done | SELECT WHERE user_id = ? |
| `update_passkey_counter` | Done | UPDATE counter WHERE id = ? |
| `update_passkey_name` | Done | UPDATE name WHERE id = ? |
| `delete_passkey` | Done | DELETE WHERE id = ? |

**Total: 65 async methods implemented across 10 traits.**

## Phase 3: Testing

### 3.1 Integration Tests — DONE

61 integration tests across 10 test modules + 2 doctests = 63 total tests, all passing.

| Module | Tests | Coverage |
|--------|-------|----------|
| `user_ops` | 10 | CRUD, search, pagination, defaults, duplicate email, ban/unban |
| `session_ops` | 8 | CRUD, expiry, expired cleanup, org context, inactive returns None |
| `account_ops` | 5 | CRUD, multi-account per user, token update, nonexistent returns None |
| `verification_ops` | 7 | CRUD, consume (atomic select+delete), expired cleanup, by-value/by-identifier |
| `organization_ops` | 6 | CRUD, slug lookup, member join, update, nonexistent returns None |
| `member_ops` | 6 | CRUD, role update, list, count members/owners |
| `invitation_ops` | 6 | CRUD, pending lookup, status update, org/user listing |
| `two_factor_ops` | 4 | CRUD, backup code update, nonexistent returns None |
| `api_key_ops` | 7 | CRUD, hash lookup, disable, expired cleanup, nonexistent returns None |
| `passkey_ops` | 7 | CRUD, credential_id lookup, counter/name update, nonexistent returns None |
| **Doctests** | 2 | Compile-only: lib.rs quick start, adapter.rs examples |

### 3.2 Full Plugin Integration Tests — TODO

- [ ] Test with `BetterAuth::new()` + `DieselSqliteAdapter` + `EmailPasswordPlugin`
- [ ] Test with `ApiKeyPlugin` — full key lifecycle
- [ ] Test with `OrganizationPlugin` — org + member + RBAC flow
- [ ] Test concurrent access (multiple tokio tasks)
- [ ] Test migration idempotency (run migrations twice)

### 3.3 Error Condition Tests — Partial

Error conditions tested implicitly via:
- `duplicate_email_fails` (UniqueViolation → DatabaseError::Constraint)
- `get_nonexistent_*` tests (NotFound → None, not error)
- All tests use in-memory SQLite with migrations (connection + pragma setup verified)

## Phase 4: Polish — TODO

### 4.1 Documentation

- [x] Crate-level rustdoc with usage example
- [x] README.md with install, quick start, features, configuration
- [x] `examples/axum_basic.rs` (commented, ready to uncomment when full integration works)
- [ ] Document each public type and method (doc coverage at 22%)
- [ ] Add CONTRIBUTING.md

### 4.2 CI/CD

- [ ] GitHub Actions workflow: check, clippy, test, fmt
- [ ] Test on stable and nightly Rust
- [ ] Test with bundled-sqlite feature

### 4.3 Release Preparation

- [ ] Verify `cargo publish --dry-run` succeeds
- [ ] Write CHANGELOG.md
- [ ] Tag v0.1.0

## Phase 5: Community — TODO

### 5.1 Upstream Engagement

- [ ] Open issue on better-auth-rs proposing SQLite support via this crate
- [ ] Offer to add `diesel-sqlite` feature flag that re-exports this crate
- [ ] Discuss schema compatibility and versioning strategy

### 5.2 Ecosystem

- [ ] Publish to crates.io as `better-auth-diesel-sqlite`
- [ ] Announce on Rust community channels

## Technical Risks

### Risk 1: Schema Drift

better-auth-rs may change its schema between versions. The Postgres adapter's SQL is the source of truth.

**Mitigation**: Pin to a specific `better-auth-core` version. Integration tests with the full plugin stack will catch schema mismatches.

### Risk 2: Trait Signature Changes

better-auth-rs is pre-1.0. Trait method signatures may change.

**Mitigation**: Pin version. Update in lockstep when upstream releases.

### Risk 3: SQLite Concurrency

The single-connection `Arc<Mutex>` approach serializes all database access. Under heavy concurrent load, this becomes a bottleneck.

**Mitigation**: This is inherent to SQLite (single writer). The mutex is acquired inside `spawn_blocking` so it never blocks the async runtime. For read-heavy workloads, a connection pool with WAL mode can be added later.

### Risk 4: Entity Type Compatibility

better-auth-core entity types may have fields or behaviors that assume Postgres semantics.

**Mitigation**: All 10 entity types verified against actual `better-auth-core` 0.9.0 source. 63 tests validate correct field mapping.

## Success Criteria

1. [x] `cargo test` passes with all `*Ops` trait methods exercised (63/63 tests passing)
2. [ ] A minimal Axum app can use `DieselSqliteAdapter` with full plugin stack
3. [ ] `cargo publish --dry-run` succeeds
4. [x] `unsafe_code = "forbid"` enforced via Cargo.toml lints
5. [ ] All public types and methods have rustdoc documentation
6. [x] ferrous-forge safety checks pass
