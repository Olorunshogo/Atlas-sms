use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::Rng;

// === Secret hashing

/**
 * Generic Argon2 hash/verify for any short-lived secret that must not be
 * stored in plaintext - login passwords, OTP codes, refresh tokens all go
 * through this rather than each module rolling its own.
 */
pub fn hash_secret(secret: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(secret.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_secret(secret: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed = PasswordHash::new(hash)?;
    Ok(Argon2::default()
        .verify_password(secret.as_bytes(), &parsed)
        .is_ok())
}

// === Token generation

/// 6-digit numeric code for email OTP.
pub fn generate_otp() -> String {
    let value: u32 = rand::thread_rng().gen_range(0..1_000_000);
    format!("{value:06}")
}

/// A long random token for the refresh token, never retyped by a human.
pub fn generate_opaque_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/**
 * SHA-256, not Argon2, for the refresh token's stored hash. Argon2's random
 * salt makes it unsuited to a `WHERE token_hash = $1` lookup - a different
 * salt every call means the same token never hashes to the same value
 * twice. That's fine here because unlike a password, the refresh token
 * already has 256 bits of its own entropy: it isn't guessable, so a fast
 * deterministic hash doesn't need Argon2's slow-by-design property to stay
 * safe against a stolen database.
 */
pub fn hash_lookup_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}
