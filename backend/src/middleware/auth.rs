use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};

use uuid::Uuid;

use crate::{config::Config, services::jwt::verify_token};

fn extract_bearer_token(request: &Request) -> Result<&str, StatusCode> {
    let authorization = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = authorization
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if token.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(token)
}

pub async fn authenticate(
    State(config): State<Config>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_bearer_token(&request)?;

    let claims = verify_token(token, &config.jwt_secret).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;

    request.extensions_mut().insert(user_id);

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;

    #[test]
    fn extract_bearer_token_accepts_valid_authorization_header() {
        let request = Request::builder()
            .header(AUTHORIZATION, "Bearer valid.jwt.token")
            .body(Body::empty())
            .expect("Failed to build request");

        let token = extract_bearer_token(&request).expect("Bearer token should be accepted");

        assert_eq!(token, "valid.jwt.token");
    }

    #[test]
    fn extract_bearer_token_rejects_missing_authorization_header() {
        let request = Request::builder()
            .body(Body::empty())
            .expect("Failed to build request");

        let result = extract_bearer_token(&request);

        assert_eq!(result, Err(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn extract_bearer_token_rejects_wrong_authentication_scheme() {
        let request = Request::builder()
            .header(AUTHORIZATION, "Basic dXNlcjpwYXNz")
            .body(Body::empty())
            .expect("Failed to build request");

        let result = extract_bearer_token(&request);

        assert_eq!(result, Err(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn extract_bearer_token_rejects_empty_bearer_token() {
        let request = Request::builder()
            .header(AUTHORIZATION, "Bearer ")
            .body(Body::empty())
            .expect("Failed to build request");

        let result = extract_bearer_token(&request);

        assert_eq!(result, Err(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn extract_bearer_token_rejects_bearer_without_separator() {
        let request = Request::builder()
            .header(AUTHORIZATION, "Bearervalid.jwt.token")
            .body(Body::empty())
            .expect("Failed to build request");

        let result = extract_bearer_token(&request);

        assert_eq!(result, Err(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn extract_bearer_token_rejects_bearer_with_wrong_case() {
        let request = Request::builder()
            .header(AUTHORIZATION, "bearer valid.jwt.token")
            .body(Body::empty())
            .expect("Failed to build request");

        let result = extract_bearer_token(&request);

        assert_eq!(result, Err(StatusCode::UNAUTHORIZED));
    }
}
