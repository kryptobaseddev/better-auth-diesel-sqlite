//! Shared test utilities for integration tests.

use better_auth_diesel_sqlite::DieselSqliteAdapter;

/// Create an in-memory adapter with migrations applied.
pub async fn test_adapter() -> DieselSqliteAdapter {
    let adapter = DieselSqliteAdapter::in_memory()
        .await
        .expect("Failed to create in-memory adapter");
    adapter
        .run_migrations()
        .await
        .expect("Failed to run migrations");
    adapter
}
