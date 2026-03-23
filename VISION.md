# Vision: better-auth-diesel-sqlite

## Purpose

A community-driven SQLite database adapter for [better-auth-rs](https://github.com/better-auth-rs/better-auth-rs), the Rust port of [Better Auth](https://better-auth.com). This crate bridges the gap between better-auth-rs's plugin ecosystem and SQLite — the most deployed database in the world.

Today, better-auth-rs ships with a PostgreSQL adapter (`SqlxAdapter` via SQLx) and an in-memory adapter for testing. There is no SQLite support. This crate fills that gap using Diesel ORM with diesel-async for non-blocking database access.

## Why This Exists

### The Problem

better-auth-rs provides a powerful authentication framework with 11 plugins (API keys, organizations, admin, 2FA, passkeys, OAuth, etc.), but it's locked to PostgreSQL. Many Rust applications — embedded systems, edge deployments, CLI tools, single-node servers, prototypes, and local-first apps — use SQLite as their primary database. These projects cannot use better-auth-rs without either migrating to Postgres or writing their own adapter.

### The Solution

`better-auth-diesel-sqlite` implements the full `DatabaseAdapter` trait and all 10 operation sub-traits (`UserOps`, `SessionOps`, `AccountOps`, `VerificationOps`, `OrganizationOps`, `MemberOps`, `InvitationOps`, `TwoFactorOps`, `ApiKeyOps`, `PasskeyOps`) for SQLite, using Diesel as the query builder and ORM layer.

### Why Diesel Over Raw SQLx

| Concern | SQLx | Diesel |
|---------|------|--------|
| Query safety | Runtime-checked SQL strings | Compile-time verified queries |
| Schema management | Raw SQL migrations | `diesel_migrations` — embeddable, versioned |
| Type mapping | Manual `FromRow` derives | Automatic via `Queryable`, `Insertable` |
| Ecosystem maturity | Good | Battle-tested, most mature Rust ORM |
| SQLite dialect handling | Manual | Automatic via `diesel::sqlite` backend |

### Async Strategy

SQLite is inherently synchronous (single-writer, file-based). However, better-auth-rs's `DatabaseAdapter` trait requires async method signatures. The adapter bridges this by wrapping a `SqliteConnection` in `Arc<Mutex<>>` and running all Diesel queries inside `tokio::task::spawn_blocking`. The mutex is acquired inside the blocking thread, so it never blocks the async runtime.

This is the same fundamental approach that SQLx and diesel-async's `SyncConnectionWrapper` use internally for SQLite — all "async" SQLite access uses `spawn_blocking` under the hood because SQLite does not support true async I/O.

## Design Principles

### 1. Drop-In Compatibility

Any better-auth-rs application should be able to swap `SqlxAdapter` (Postgres) for `DieselSqliteAdapter` with minimal code changes:

```rust
// Before (Postgres)
use better_auth::adapters::SqlxAdapter;
let adapter = SqlxAdapter::new("postgres://...").await?;

// After (SQLite)
use better_auth_diesel_sqlite::DieselSqliteAdapter;
let adapter = DieselSqliteAdapter::new("sqlite://auth.db").await?;
```

All 11 better-auth-rs plugins work identically regardless of which adapter is used.

### 2. Zero Unsafe Code

This crate uses `#![forbid(unsafe_code)]`. All operations go through Diesel's safe query builder API.

### 3. Full Feature Parity

Every method on every `*Ops` trait is implemented. No stubs, no panics, no `todo!()` markers. If better-auth-rs supports it on Postgres, this adapter supports it on SQLite.

### 4. Production-Ready Defaults

- WAL mode enabled by default for concurrent read performance
- `Arc<Mutex<SqliteConnection>>` with `spawn_blocking` — correct for SQLite's single-writer model
- Embedded migrations via `diesel_migrations` (run via `adapter.run_migrations()`)
- Proper error mapping to `AuthError` types (UniqueViolation, ForeignKeyViolation, etc.)

### 5. Community-First

This is an independent, open-source crate. It is not affiliated with the better-auth-rs project but is designed for eventual upstream integration. The goal is to be the canonical SQLite adapter for the better-auth-rs ecosystem.

## Target Users

- **Rust developers** building authentication for SQLite-backed applications
- **Edge/embedded deployments** where Postgres is impractical
- **Prototyping and local development** — start with SQLite, migrate to Postgres later
- **Single-node servers** that don't need Postgres's concurrency model
- **Local-first applications** using SQLite for offline-capable auth

## Non-Goals

- Supporting databases other than SQLite (Postgres and MySQL are better served by dedicated adapters)
- Replacing or forking better-auth-rs — this is a complementary crate
- Providing an ORM layer beyond what's needed for the adapter implementation
- Building a CLI or migration tool — Diesel already provides these

## Relationship to better-auth-rs

This crate depends on `better-auth-core` for trait definitions and entity types. It does not depend on `better-auth` (the full framework) or `better-auth-api` (plugin implementations). This keeps the dependency tree minimal.

The adapter can be used by adding it alongside better-auth in any project:

```toml
[dependencies]
better-auth = { version = "0.9", features = ["axum"] }
better-auth-diesel-sqlite = "0.1"
```

## Future Possibilities

- **Upstream integration**: better-auth-rs could re-export this crate behind a `diesel-sqlite` feature flag
- **Diesel Postgres adapter**: The same Diesel-based approach could be extended to Postgres, offering a compile-time-safe alternative to the SQLx adapter
- **Diesel MySQL adapter**: Same pattern for MySQL
- **Custom entity support**: Full generic entity type support matching the `SqlxAdapter`'s flexibility
- **Connection pool choice**: Support for `deadpool`, `r2d2`, `bb8`, and `mobc` pools behind feature flags

## License

This project will be released under the MIT License, matching better-auth-rs.
