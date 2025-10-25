## Why

Production deployments require robust security including TLS encryption, authentication, authorization, and audit logging. Without proper security, Lexum cannot be safely deployed in production environments or handle sensitive data.

## What Changes

- Implement TLS/mTLS support for HTTP and inter-node communication
- Add API key authentication
- Implement OAuth 2.0 integration
- Add role-based access control (RBAC)
- Implement document-level security
- Add field-level security (field masking)
- Implement comprehensive audit logging
- Add encryption at rest support
- **BREAKING**: Authentication becomes required by default

## Impact

- Affected specs: `security`, `authentication`, `authorization`, `audit-logging`
- Affected code: Creates `lexum-server/src/security/`:
  - `tls.rs` - TLS configuration
  - `auth/` - Authentication providers
  - `rbac.rs` - Role-based access control
  - `audit.rs` - Audit logging
- Dependencies: rustls, jsonwebtoken, oauth2
- Breaking: Existing deployments need authentication configuration

