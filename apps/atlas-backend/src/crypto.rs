use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::Rng;

// === Secret hashing

/**
 * Generic Argon2 hash/verify for any short-lived secret that must not be stored in plaintext.
 * Login passwords, OTP codes, refresh tokens all go through this rather than each module rolling its own.
 */
pub fn hash_secret(secret: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(secret.as_bytes(), &salt)?;
    return Ok(hash.to_string());
}

pub fn verify_secret(secret: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed = PasswordHash::new(hash)?;
    return Ok(Argon2::default()
        .verify_password(secret.as_bytes(), &parsed)
        .is_ok());
}

// === Token generation

/// 6-digit numeric code for email OTP.
pub fn generate_otp() -> String {
    let value: u32 = rand::thread_rng().gen_range(0..1_000_000);
    return format!("{:06}", value);
}

/// A long random token for the refresh token, never retyped by a human.
pub fn generate_opaque_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    return bytes.iter().map(|b| format!("{:02x}", b)).collect();
}

/**
 * SHA-256, not Argon2, for the refresh token's stored hash.
 * Argon2's random salt makes it unsuited to a `WHERE token_hash = $1` lookup - a different salt every call means the same token never hashes to the same value twice.
 * That's fine here because unlike a password, the refresh token already has 256 bits of its own entropy: it isn't guessable, so a fast deterministic hash doesn't need Argon2's slow-by-design property to stay safe against a stolen database.
 */
pub fn hash_lookup_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(token.as_bytes());
    return digest.iter().map(|b| format!("{:02x}", b)).collect();
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_secret_accepts_the_secret_that_was_hashed() {
        let hash = hash_secret("correct horse battery staple").unwrap();
        assert!(verify_secret("correct horse battery staple", &hash).unwrap());
    }

    #[test]
    fn verify_secret_rejects_a_different_secret() {
        let hash = hash_secret("correct horse battery staple").unwrap();
        assert!(!verify_secret("wrong guess", &hash).unwrap());
    }

    #[test]
    fn hash_secret_salts_every_call_differently() {
        // Two hashes of the same secret must differ (random salt per call) -
        // this is exactly why hash_secret is unsuited to a lookup column,
        // unlike hash_lookup_token below.
        let first = hash_secret("same input").unwrap();
        let second = hash_secret("same input").unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn generate_otp_is_six_ascii_digits() {
        let code = generate_otp();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn generate_opaque_token_is_64_hex_chars_and_varies() {
        let first = generate_opaque_token();
        let second = generate_opaque_token();
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }

    #[test]
    fn hash_lookup_token_is_deterministic() {
        // Unlike hash_secret, the same input must hash to the same output
        // every time - that's the whole point of using it for a `WHERE
        // token_hash = $1` lookup rather than Argon2.
        let first = hash_lookup_token("some-refresh-token");
        let second = hash_lookup_token("some-refresh-token");
        assert_eq!(first, second);
    }

    #[test]
    fn hash_lookup_token_differs_for_different_input() {
        assert_ne!(
            hash_lookup_token("token-a"),
            hash_lookup_token("token-b")
        );
    }
}
