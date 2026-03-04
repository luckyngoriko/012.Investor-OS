# SPRINT-103: API Production Quality

**Phase:** world-class-platform
**Gate:** G50
**Depends on:** Sprint 102

## Objective

Bring the API layer to production quality: OpenAPI schema generation, pagination on
all list endpoints, request validation & size limits, standardized error envelope,
API versioning, and content-type enforcement.

## Work Packages

| WP  | Task                                                                      | Files                                         |
| --- | ------------------------------------------------------------------------- | --------------------------------------------- |
| 1   | Add `utoipa` crate + generate OpenAPI 3.1 schema from handlers            | `Cargo.toml`, `src/api/mod.rs`, handler files |
| 2   | Serve Swagger UI at `/api/docs` (replace static JSON)                     | `src/main.rs`                                 |
| 3   | Pagination helper: `PaginatedResponse<T>` with cursor + limit             | `src/api/pagination.rs`                       |
| 4   | Apply pagination to all list endpoints (admin/users, positions, etc.)     | `src/auth/repository.rs`, handler files       |
| 5   | Standardized error envelope: `{success, error: {code, message, details}}` | `src/api/error.rs`                            |
| 6   | Request body size limit middleware (default 1MB)                          | `src/main.rs` (tower-http `RequestBodyLimit`) |
| 7   | Content-Type validation middleware (reject non-JSON on POST/PUT)          | `src/middleware/content_type.rs`              |
| 8   | API versioning: mount all routes under `/api/v1/`, keep `/api/` as alias  | `src/main.rs`                                 |
| 9   | Request timeout middleware (30s default)                                  | `src/main.rs` (tower-http `TimeoutLayer`)     |
| 10  | Tests for pagination, error envelope, content-type rejection              | `tests/sprint103_api_quality_test.rs`         |

## New Dependencies

```toml
utoipa = { version = "5", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "8", features = ["axum"] }
tower-http = { version = "0.5", features = ["limit", "timeout"] }
```

## Error Envelope Standard

```json
{
  "success": false,
  "error": {
    "code": "AUTH_LOCKOUT",
    "message": "Account locked due to too many failed attempts",
    "details": { "locked_until": "2026-03-04T12:00:00Z" }
  }
}
```

## Acceptance Criteria

- `/api/docs` serves interactive Swagger UI
- `/api/v1/openapi.json` returns machine-readable OpenAPI 3.1 spec
- All list endpoints accept `?cursor=&limit=` with max 100
- POST/PUT without `Content-Type: application/json` → 415
- Body > 1MB → 413
- Request > 30s → 408 timeout
- All error responses follow the standardized envelope
- `cargo clippy -- -D warnings` zero warnings
- All new tests pass
