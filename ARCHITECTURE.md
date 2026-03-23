# Architecture: better-auth-diesel-sqlite

## Overview

This crate provides a `DieselSqliteAdapter` struct that implements better-auth-rs's `DatabaseAdapter` supertrait by implementing all 10 required operation traits for SQLite via Diesel ORM and diesel-async.

```
┌──────────────────────────────────────────────────────────────┐
│                    better-auth-rs                            │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              DatabaseAdapter (supertrait)               │  │
│  │                                                        │  │
│  │  UserOps + SessionOps + AccountOps + VerificationOps   │  │
│  │  + OrganizationOps + MemberOps + InvitationOps         │  │
│  │  + TwoFactorOps + ApiKeyOps + PasskeyOps               │  │
│  └────────────────────┬───────────────────────────────────┘  │
│                       │ implements                           │
└───────────────────────┼──────────────────────────────────────┘
                        │
┌───────────────────────┼──────────────────────────────────────┐
│  better-auth-diesel-sqlite                                   │
│                       │                                      │
│  ┌────────────────────▼───────────────────────────────────┐  │
│  │            DieselSqliteAdapter                         │  │
│  │                                                        │  │
│  │  - conn: Arc<Mutex<SqliteConnection>>                  │  │
│  │  - interact(): spawn_blocking + mutex lock             │  │
│  │  - Implements all 10 *Ops traits                       │  │
│  │  - Maps Diesel queries to better-auth entity types     │  │
│  └────────────────────┬───────────────────────────────────┘  │
│                       │                                      │
│  ┌────────────────────▼───────────────────────────────────┐  │
│  │              Diesel ORM Layer                          │  │
│  │                                                        │  │
│  │  schema.rs     → table! macro definitions              │  │
│  │  models.rs     → Queryable/Insertable structs          │  │
│  │  conversions.rs → DieselModel <-> AuthEntity mapping   │  │
│  └────────────────────┬───────────────────────────────────┘  │
│                       │                                      │
│  ┌────────────────────▼───────────────────────────────────┐  │
│  │       tokio::task::spawn_blocking + std::sync::Mutex   │  │
│  │                                                        │  │
│  │  Runs synchronous Diesel queries in blocking threads   │  │
│  │  Mutex acquired inside spawn_blocking, not in async    │  │
│  └────────────────────┬───────────────────────────────────┘  │
│                       │                                      │
│  ┌────────────────────▼───────────────────────────────────┐  │
│  │              SQLite (via libsqlite3-sys)               │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

## Crate Structure

```
better-auth-diesel-sqlite/
├── Cargo.toml
├── LICENSE
├── README.md
├── VISION.md
├── ARCHITECTURE.md
├── PLAN.md
├── diesel.toml                  # Diesel CLI configuration
├── migrations/                  # Diesel SQL migrations
│   └── 00000000000000_create_auth_tables/
│       ├── up.sql               # Full schema creation
│       └── down.sql             # Schema teardown
├── src/
│   ├── lib.rs                   # Public API, re-exports, crate docs
│   ├── adapter.rs               # DieselSqliteAdapter struct + constructors
│   ├── config.rs                # PoolConfig, adapter configuration
│   ├── error.rs                 # Error types + AuthError conversion
│   ├── schema.rs                # Diesel table! macro definitions
│   ├── models.rs                # Diesel model structs (Queryable, Insertable)
│   ├── conversions.rs           # From<DieselModel> for AuthEntity impls
│   ├── ops/                     # Trait implementations (one file per trait)
│   │   ├── mod.rs
│   │   ├── user.rs              # impl UserOps for DieselSqliteAdapter
│   │   ├── session.rs           # impl SessionOps
│   │   ├── account.rs           # impl AccountOps
│   │   ├── verification.rs      # impl VerificationOps
│   │   ├── organization.rs      # impl OrganizationOps
│   │   ├── member.rs            # impl MemberOps
│   │   ├── invitation.rs        # impl InvitationOps
│   │   ├── two_factor.rs        # impl TwoFactorOps
│   │   ├── api_key.rs           # impl ApiKeyOps
│   │   └── passkey.rs           # impl PasskeyOps
├── tests/
│   ├── tests.rs                 # Test harness entry point
│   ├── common/
│   │   └── mod.rs               # test_adapter() helper (in-memory + migrations)
│   └── integration/
│       ├── mod.rs               # Module declarations for all 10 test files
│       ├── user_ops.rs          # 10 tests
│       ├── session_ops.rs       # 8 tests
│       ├── account_ops.rs       # 5 tests
│       ├── verification_ops.rs  # 7 tests
│       ├── organization_ops.rs  # 6 tests
│       ├── member_ops.rs        # 6 tests
│       ├── invitation_ops.rs    # 6 tests
│       ├── two_factor_ops.rs    # 4 tests
│       ├── api_key_ops.rs       # 7 tests
│       └── passkey_ops.rs       # 7 tests
└── examples/
    └── axum_basic.rs            # Minimal Axum app using DieselSqliteAdapter
```

## Core Components

### 1. DieselSqliteAdapter

The primary public type. Holds a shared connection and implements all 10 operation traits.

```rust
pub struct DieselSqliteAdapter {
    conn: Arc<Mutex<SqliteConnection>>,
}
```

**Constructors:**

| Method | Description |
|--------|-------------|
| `new(database_url)` | Connect with default settings and pragmas |
| `with_config(database_url, config)` | Connect with custom configuration |
| `in_memory()` | Create an in-memory SQLite database (for testing) |

**Connection lifecycle:**

1. `DieselSqliteAdapter::new()` establishes a `SqliteConnection` in `spawn_blocking`
2. SQLite performance pragmas are applied (WAL, busy_timeout, foreign_keys, etc.)
3. The connection is wrapped in `Arc<Mutex<>>` for safe concurrent access
4. When an `*Ops` method is called, `interact()` locks the mutex inside `spawn_blocking`
5. The synchronous Diesel query runs, then results are mapped to better-auth entity types

> **Note**: SQLite only supports one writer at a time, so a single connection with
> mutex-based access is correct and avoids pool contention. A deadpool-based pool
> can be added later behind a feature flag for read-heavy workloads.

### 2. Schema (schema.rs)

Diesel `table!` macro definitions matching better-auth-rs's expected database schema. These must align exactly with the tables that better-auth-rs creates/expects.

**Tables:**

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| `users` | User accounts | id, name, email, username, display_username, email_verified, image, role, banned, ban_reason, ban_expires, two_factor_enabled, metadata, created_at, updated_at |
| `sessions` | Active sessions | id, user_id, token, ip_address, user_agent, expires_at, active_organization_id, impersonated_by, active, created_at, updated_at |
| `accounts` | OAuth provider links | id, user_id, account_id, provider_id, access_token, refresh_token, id_token, access_token_expires_at, refresh_token_expires_at, scope, password, created_at, updated_at |
| `verifications` | Email/reset tokens | id, identifier, value, expires_at, created_at, updated_at |
| `organization` | Multi-tenant orgs | id, name, slug, logo, metadata, created_at, updated_at |
| `member` | Org membership | id, user_id, organization_id, role, created_at |
| `invitation` | Org invitations | id, organization_id, email, role, status, inviter_id, expires_at, created_at |
| `two_factor` | TOTP secrets | id, user_id, secret, backup_codes (nullable), created_at, updated_at |
| `api_keys` | API keys | id, user_id, name, start, prefix, key, enabled, rate_limit_enabled, rate_limit_time_window, rate_limit_max, request_count, remaining, refill_interval, refill_amount, last_refill_at, last_request, expires_at, created_at, updated_at, permissions, metadata |
| `passkeys` | WebAuthn creds | id, user_id, name, credential_id, public_key, counter (i64), device_type, backed_up, transports, created_at |

Note: SQLite uses `TEXT` for UUIDs, `TEXT` for timestamps (ISO 8601), `INTEGER` for booleans (0/1), and `TEXT` for JSON fields. Diesel handles these mappings via its SQLite backend.

### 3. Models (models.rs)

Diesel-native structs for each table with `Queryable`, `Insertable`, `AsChangeset`, and `Selectable` derives.

```rust
#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = users)]
pub struct UserRow {
    pub id: String,
    pub name: Option<String>,
    pub email: String,
    // ...
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUserRow {
    pub id: String,
    pub name: Option<String>,
    pub email: String,
    // ...
}
```

### 4. Conversions (conversions.rs)

Bidirectional mapping between Diesel model structs and better-auth-core entity types.

```rust
impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        User {
            // map fields, parse timestamps, deserialize JSON
        }
    }
}
```

This layer handles:
- `String` <-> `Uuid` conversion
- `String` (ISO 8601) <-> `DateTime<Utc>` conversion
- `String` (JSON) <-> `serde_json::Value` conversion
- `i32` (0/1) <-> `bool` conversion

### 5. Ops (ops/)

One file per trait implementation. Each file contains:
- `#[async_trait]` implementation block
- `self.interact(|conn| { ... })` call to run sync Diesel code in `spawn_blocking`
- Diesel query construction against `&mut SqliteConnection`
- Model-to-entity conversion via `From` impls
- Error mapping via `diesel_to_auth_error()`

**Pattern for each method:**

```rust
#[async_trait]
impl UserOps for DieselSqliteAdapter {
    type User = User;

    async fn create_user(&self, data: CreateUser) -> AuthResult<Self::User> {
        let new_row = NewUserRow::from(data);
        let row_id = new_row.id.clone();

        self.interact(move |conn| {
            diesel::insert_into(users::table)
                .values(&new_row)
                .execute(conn)
                .map_err(diesel_to_auth_error)?;

            users::table
                .filter(users::id.eq(&row_id))
                .first::<UserRow>(conn)
                .map(User::from)
                .map_err(diesel_to_auth_error)
        })
        .await
    }
}
```

### 6. Error Mapping (error.rs)

All Diesel errors are mapped to better-auth-core's `AuthError` type via `diesel_to_auth_error()` and `diesel_to_database_error()` free functions (orphan rule prevents `From` impls):

| Diesel Error | AuthError Mapping |
|---|---|
| `diesel::result::Error::NotFound` | `AuthError::Database(DatabaseError::Query("Record not found"))` |
| `diesel::result::Error::DatabaseError(UniqueViolation, info)` | `AuthError::Database(DatabaseError::Constraint(msg))` |
| `diesel::result::Error::DatabaseError(ForeignKeyViolation, info)` | `AuthError::Database(DatabaseError::Constraint(msg))` |
| `diesel::result::Error::DatabaseError(_, info)` | `AuthError::Database(DatabaseError::Query(msg))` |
| `diesel::result::Error::RollbackTransaction` | `AuthError::Database(DatabaseError::Transaction(msg))` |
| Mutex poisoned | `AuthError::Database(DatabaseError::Connection(msg))` |
| `spawn_blocking` join error | `AuthError::Database(DatabaseError::Connection(msg))` |

### 7. Migrations

Diesel SQL migrations that create the full better-auth schema in SQLite. These are embedded in the binary via `diesel_migrations::embed_migrations!()` and can optionally run on adapter construction.

The migration SQL must match better-auth-rs's expected schema exactly. The column names, types, and constraints are derived from the `SqlxAdapter`'s Postgres schema, translated to SQLite dialect.

## Async Strategy

```
Caller (async)
    │
    ▼
DieselSqliteAdapter.create_user()     ← async method (required by trait)
    │
    ▼
self.interact(move |conn| { ... })    ← calls spawn_blocking internally
    │
    ▼
Arc::clone(&self.conn)                ← clone the Arc handle
    │
    ▼
tokio::task::spawn_blocking(move || {
    conn.lock()                        ← acquire Mutex in blocking thread
    f(&mut guard)                      ← run synchronous Diesel queries
})
    │
    ▼
.await                                 ← returns to async context
    │
    ▼
AuthResult<T>                          ← entity already mapped inside closure
```

This approach is identical to how SQLx handles SQLite internally — SQLite does not support true async I/O, so all "async" SQLite access uses `spawn_blocking` under the hood.

## Connection Management

### Current: Arc<Mutex<SqliteConnection>>

The adapter uses a single connection wrapped in `Arc<Mutex<>>`:

- SQLite only supports one writer at a time — a pool of write connections would just contend
- The mutex is acquired inside `spawn_blocking` so it never blocks the async runtime
- `PoolConfig` is accepted for API forward-compatibility but only `run_migrations` is used currently

### Future: Connection Pooling

A deadpool-based pool can be added behind a feature flag for read-heavy workloads where multiple concurrent readers benefit from separate connections. The `diesel-async` crate with `deadpool` feature is already in dependencies for this purpose.

### SQLite Pragmas

Each new connection executes these pragmas for optimal performance:

```sql
PRAGMA journal_mode = WAL;          -- Write-Ahead Logging for concurrent reads
PRAGMA busy_timeout = 5000;         -- Wait 5s for locks instead of failing
PRAGMA synchronous = NORMAL;        -- Balance safety/performance
PRAGMA foreign_keys = ON;           -- Enforce referential integrity
PRAGMA cache_size = -64000;         -- 64MB page cache
```

## Type Mapping: SQLite <-> Rust <-> better-auth

| better-auth Type | Rust Type | SQLite Type | Notes |
|---|---|---|---|
| UUID | `String` | `TEXT` | Stored as hyphenated UUID string |
| DateTime | `String` | `TEXT` | ISO 8601 format (e.g., `2025-01-15T10:30:00Z`) |
| Boolean | `bool` | `INTEGER` | Diesel maps `bool` to `0`/`1` automatically |
| JSON metadata | `Option<String>` | `TEXT` | Serialized via `serde_json` |
| Integer counts | `i32` / `i64` | `INTEGER` | Native SQLite integer |
| String fields | `String` / `Option<String>` | `TEXT` | Direct mapping |

## Testing Strategy

### Integration Tests (61 tests)

All tests live in `tests/integration/` with one module per trait. Each test:
1. Creates an in-memory SQLite adapter via `test_adapter()` helper
2. Runs embedded migrations automatically
3. Exercises trait methods against a real (in-memory) SQLite database
4. Asserts expected results, error conditions, and edge cases

Test coverage includes:
- CRUD operations for all 10 entity types
- Search/filter/pagination (UserOps `list_users`)
- Atomic operations (VerificationOps `consume_verification`)
- Cascading behavior (delete user → sessions cleaned up)
- Expiry cleanup (expired sessions, API keys, verifications)
- Edge cases (duplicate emails, nonexistent lookups, ban/unban logic)
- Relationship queries (list user organizations via member join)

### Doctests (2 tests)

Compile-only doctests verify the example code in `lib.rs` and `adapter.rs` compiles correctly against the actual better-auth API.

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `migrations` | Yes | Embed and optionally run migrations on startup |
| `bundled-sqlite` | No | Bundle libsqlite3 (via `libsqlite3-sys/bundled`) |

## Dependencies

### Required

| Crate | Version | Purpose |
|-------|---------|---------|
| `better-auth-core` | `0.9` | Trait definitions, entity types, error types |
| `diesel` | `2` | Query builder, ORM, SQLite backend |
| `diesel-async` | `0.8` | Future async connection pooling support |
| `diesel_migrations` | `2` | Embeddable schema migrations |
| `tokio` | `1` | Async runtime (`spawn_blocking`) |
| `async-trait` | `0.1` | Async trait support |
| `uuid` | `1` | UUID generation |
| `chrono` | `0.4` | Timestamp handling |
| `serde` | `1` | Serialization/deserialization |
| `serde_json` | `1` | JSON field handling |
| `thiserror` | `2` | Error type derivation |
| `tracing` | `0.1` | Structured logging |
| `libsqlite3-sys` | `0.36` | Optional: bundle SQLite (via `bundled` feature) |

### Dev Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `ferrous-forge` | `1.9` | Rust development standards enforcement |
| `tokio` (full) | `1` | Test runtime |
| `better-auth` | `0.9` | Full framework for integration tests |
| `tempfile` | `3` | Temporary SQLite files for tests |

## Versioning

This crate follows the version of `better-auth-core` it targets. If better-auth-core is at `0.9.x`, this crate starts at `0.1.0` and tracks compatibility in its `Cargo.toml`.

When better-auth-rs reaches `1.0`, this crate will release a corresponding `1.0` that guarantees API stability.
