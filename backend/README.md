# Black Vault API

**Preserved legacy/reference API.** The active product is the offline [desktop application](../desktop/README.md); it does not require this API or PostgreSQL. See the [legacy boundary](../legacy/README.md).

## Features

- **Secure Authentication**: Argon2 password hashing with JWT access tokens
- **Protected Routes**: JWT-based authentication middleware
- **Input Validation**: Email format validation and strong password requirements
- **PostgreSQL Integration**: SQLx with type-safe database queries
- **CORS Support**: Permissive legacy development configuration; do not expose this stack as a production service
- **Structured Logging**: Tracing integration for observability
- **Health Checks**: Endpoint for monitoring service status

## Security Features

- Passwords hashed with Argon2id (memory-hard KDF)
- JWT tokens with configurable expiration
- Email normalization and validation
- Password complexity requirements (12+ chars, uppercase, lowercase, digit)
- Environment-based configuration (no hardcoded secrets)
- SQL injection protection via parameterized queries

## Prerequisites

- Current stable Rust (the desktop development baseline is Rust 1.90+)
- PostgreSQL 14+
- Cargo

## Setup

1. **Clone the repository**
   ```bash
   cd backend
   ```

2. **Install dependencies**
   ```bash
   cargo build
   ```

3. **Configure environment variables**
   ```bash
   cp .env.example .env
   ```
   
   Edit `.env` with your configuration:
   - `DATABASE_URL`: PostgreSQL connection string
   - `HOST`: Server host (default: 0.0.0.0)
   - `PORT`: Server port (default: 8080)
   - `JWT_SECRET`: Secret key for JWT signing (use a strong random string in production)
   - `JWT_EXPIRATION`: Token expiration time in seconds (default: 3600)

4. **Run database migrations**
   ```bash
   # Apply migrations using your PostgreSQL client
   psql -U your_user -d black_vault -f migrations/001_create_users.sql
   ```

5. **Run the server**
   ```bash
   cargo run
   ```

The API will be available at `http://localhost:8080`

## API Endpoints

### Public Endpoints

#### Health Check
```
GET /health
```
Returns `OK` if the service is running.

#### Register
```
POST /auth/register
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "SecurePassword123"
}
```

Response:
```json
{
  "id": "uuid",
  "email": "user@example.com"
}
```

#### Login
```
POST /auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "SecurePassword123"
}
```

Response:
```json
{
  "id": "uuid",
  "email": "user@example.com",
  "token": "jwt_token"
}
```

### Protected Endpoints

All protected endpoints require a valid JWT token in the Authorization header:
```
Authorization: Bearer <jwt_token>
```

#### Get Current User
```
GET /user/me
Authorization: Bearer <jwt_token>
```

Response:
```json
{
  "id": "uuid",
  "email": "user@example.com"
}
```

## Error Responses

All errors return a JSON response with an `error` field:
```json
{
  "error": "Error message"
}
```

Common HTTP status codes:
- `400 Bad Request`: Invalid input (email format, password complexity)
- `401 Unauthorized`: Invalid credentials or missing/invalid token
- `409 Conflict`: Email already registered
- `500 Internal Server Error`: Server error

## Testing

Run the test suite:
```bash
cargo test
```

Tests include:
- Unit tests for password hashing and verification
- Unit tests for JWT creation and validation
- Integration tests for user registration and login
- Database tests for user CRUD operations

## Password Requirements

Passwords must meet the following criteria:
- Minimum 12 characters
- At least one uppercase letter
- At least one lowercase letter
- At least one digit

## Development

### Code Structure

```
src/
├── api              # API handlers and request/response types
│   ├── auth.rs      # Authentication endpoints
│   ├── user.rs      # User endpoints
│   └── error.rs     # Error response types
├── config           # Configuration management
├── database         # Database operations
│   └── users.rs     # User database queries
├── middleware       # HTTP middleware
│   └── auth.rs      # JWT authentication middleware
├── models           # Data models
├── services         # Business logic
│   ├── auth.rs      # Authentication service
│   └── jwt.rs       # JWT token service
└── main.rs          # Application entry point
```

### Adding New Features

1. Add database migrations to `migrations/`
2. Create database functions in `src/database/`
3. Implement business logic in `src/services/`
4. Add API handlers in `src/api/`
5. Update `src/main.rs` to register new routes
6. Add tests for new functionality

## Security Considerations

- Never commit `.env` files or secrets to version control
- Use strong, randomly generated JWT secrets in production
- Enable HTTPS in production environments
- Configure CORS appropriately for your frontend domain
- Regularly update dependencies: `cargo update`
- Review security advisories for Rust dependencies

## License

Proprietary - Black Vault Engineering

## Maintenance checks

```bash
cargo fmt --all -- --check
SQLX_OFFLINE=true cargo clippy --locked --all-targets -- -D warnings
# DATABASE_URL must point to a migrated disposable PostgreSQL database.
SQLX_OFFLINE=true cargo test --locked
cargo audit
```

Production SQLx query metadata remains checked in. Database tests use runtime parameterised queries so the offline all-target build does not need cached test cleanup metadata. JWTs explicitly use HS256 with checked expiration arithmetic and the AWS-LC provider; HTTP authentication errors use typed variants instead of comparing message text.

See the [4 October audit](../docs/engineering/CODEBASE_AUDIT_2026-10-04.md), including the inactive MySQL/RSA lockfile advisory. Docker development ports bind to loopback.
