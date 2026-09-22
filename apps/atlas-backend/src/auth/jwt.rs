use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;

// === Claims

/**
 * `tenant_id` is the tenant the caller authenticated into.
 * A user holding grants in more than one tenant gets one token per tenant, never a token that spans tenants - switching tenants means logging in again (or later, a dedicated "switch tenant" endpoint that re-issues a token), same tradeoff Klassivox's auth.md documents as an open question for its own scope-switching case.
 */
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub tenant_id: Uuid,
    pub exp: usize,
}

pub fn issue_access_token(
    user_id: Uuid,
    tenant_id: Uuid,
    secret: &[u8],
) -> Result<String, ApiError> {
    let claims = Claims {
        sub: user_id,
        tenant_id,
        exp: (Utc::now() + Duration::minutes(15)).timestamp() as usize,
    };
    return encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|err| ApiError::Internal(format!("failed to issue access token: {}", err)));
}

pub fn decode_access_token(token: &str, secret: &[u8]) -> Result<Claims, ApiError> {
    return decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| ApiError::Unauthorized("Invalid or expired token.".to_string()));
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_recovers_the_claims_that_were_issued() {
        let user_id = Uuid::now_v7();
        let tenant_id = Uuid::now_v7();
        let secret = b"test-secret";

        let token = issue_access_token(user_id, tenant_id, secret).unwrap();
        let claims = decode_access_token(&token, secret).unwrap();

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.tenant_id, tenant_id);
    }

    #[test]
    fn decode_rejects_a_token_signed_with_a_different_secret() {
        let token = issue_access_token(Uuid::now_v7(), Uuid::now_v7(), b"secret-a").unwrap();
        assert!(decode_access_token(&token, b"secret-b").is_err());
    }

    #[test]
    fn decode_rejects_garbage_input() {
        assert!(decode_access_token("not.a.jwt", b"test-secret").is_err());
    }
}
