use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub iat: usize,
    pub exp: usize,
}

pub fn create_token(user_id: Uuid, secret: &str, expiration: u64) -> Result<String, String> {
    let now = chrono::Utc::now().timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        iat: now,
        exp: now + expiration as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|error| error.to_string())
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, String> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|error| error.to_string())?;

    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "black-vault-test-secret";

    const TEST_EXPIRATION: u64 = 3600;

    #[test]
    fn create_token_returns_valid_token() {
        let user_id = Uuid::new_v4();

        let token =
            create_token(user_id, TEST_SECRET, TEST_EXPIRATION).expect("Failed to create JWT");

        assert!(!token.is_empty());

        let claims = verify_token(&token, TEST_SECRET).expect("Failed to verify JWT");

        assert_eq!(claims.sub, user_id.to_string());

        assert!(claims.iat > 0);
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn verify_token_rejects_wrong_secret() {
        let user_id = Uuid::new_v4();

        let token =
            create_token(user_id, TEST_SECRET, TEST_EXPIRATION).expect("Failed to create JWT");

        let result = verify_token(&token, "wrong-secret");

        assert!(result.is_err());
    }

    #[test]
    fn verify_token_rejects_invalid_token() {
        let result = verify_token("not-a-valid-jwt", TEST_SECRET);

        assert!(result.is_err());
    }
}
