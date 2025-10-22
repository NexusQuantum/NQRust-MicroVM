# Development Commands

## Build & Test
- Build workspace: `cargo build`
- Run tests: `cargo test`
- Run single test: `cargo test -- <test_name>` or `cargo test --bin <binary> <test_name>`
- Run manager: `(cd apps/manager && cargo run)`
- Run agent: `(cd apps/agent && cargo run)`
- Database migrations: `(cd apps/manager && sqlx migrate run)`

## Code Style Guidelines

### Imports & Formatting
- Use workspace dependencies from root Cargo.toml
- Group imports: std, external crates, internal modules
- Use `cargo fmt` for formatting (rustfmt standard)

### Types & Naming
- Use `anyhow::Result<T>` for error handling
- Struct names: PascalCase (e.g., `AppState`, `VmConfig`)
- Function names: snake_case
- Constants: SCREAMING_SNAKE_CASE
- Use `#[derive(Clone)]` for shared state structs

### Error Handling
- Use `anyhow` for application errors
- Use `thiserror` for library error types
- Prefer `?` operator over explicit match
- Use `context()` for error context

### Architecture Patterns
- Repository pattern for data access
- Service layer for business logic
- Axum for HTTP APIs with router modules
- Use `PgPool` for database connections
- Async/await with tokio runtime