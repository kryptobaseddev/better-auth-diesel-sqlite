# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - Unreleased

### Added

- Project scaffolding with full Diesel ORM schema for better-auth-rs
- `DieselSqliteAdapter` struct with constructor stubs (`new`, `with_config`, `in_memory`)
- `PoolConfig` builder for connection pool configuration
- `AdapterError` enum with Diesel error mapping foundation
- Diesel `table!` macro definitions for all 10 auth tables
- Diesel model structs (`*Row`, `New*Row`, `Update*Row`) for all entities
- SQLite migration (`up.sql` / `down.sql`) creating the full auth schema
- Stub files for all 10 `*Ops` trait implementations
- ferrous-forge integration with strict lint enforcement
- CI workflow via GitHub Actions
- `bundled-sqlite` feature flag for bundling libsqlite3
