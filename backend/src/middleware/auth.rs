use axum::{
    extract::{
        Request,
        State,
    },
    http::{
        header::AUTHORIZATION,
        StatusCode,
    },
    middleware::Next,
    response::Response,
};

use uuid::Uuid;

use crate::{
    config::Config,
    services::jwt::verify_token,
};

pub async fn authenticate(
    State(config): State<Config>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
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

    let claims = verify_token(
        token,
        &config.jwt_secret,
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    request.extensions_mut().insert(user_id);

    Ok(next.run(request).await)
}