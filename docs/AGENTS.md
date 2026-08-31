# Black Vault Engineering Rules

## Security

- Never store plaintext files
- Never log secrets
- Never expose encryption keys
- Validate all user input

## Rust

- Prefer safe Rust
- Avoid unsafe blocks unless justified
- Use Result based error handling

## API

- All endpoints require authentication
- All actions require authorisation checks
- Every sensitive action must create an audit event

## Testing

Every feature requires:

- Unit tests
- Integration tests
- Security review