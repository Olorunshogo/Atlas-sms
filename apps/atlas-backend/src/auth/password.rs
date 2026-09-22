use crate::crypto;

// === Password hashing

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    crypto::hash_secret(password)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    crypto::verify_secret(password, hash)
}
