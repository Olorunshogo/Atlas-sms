use crate::crypto;

// === OTP

/// Stored hashed, never the raw code - same reasoning as a password.
pub fn generate_code() -> String {
    crypto::generate_otp()
}

pub fn hash_code(code: &str) -> Result<String, argon2::password_hash::Error> {
    crypto::hash_secret(code)
}

pub fn verify_code(code: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    crypto::verify_secret(code, hash)
}
