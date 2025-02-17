use std::error::Error;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, Algorithm, Version, Params, KeyInit,
};
use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, NewAead},
};
use bs58;

#[derive(Debug)]
pub enum EncryptError {
    KeyDerivationError(String),
    EncryptionError(String),
    DecryptionError(String),
    Base58Error(String),
}

impl std::fmt::Display for EncryptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeyDerivationError(msg) => write!(f, "Key derivation error: {}", msg),
            Self::EncryptionError(msg) => write!(f, "Encryption error: {}", msg),
            Self::DecryptionError(msg) => write!(f, "Decryption error: {}", msg),
            Self::Base58Error(msg) => write!(f, "Base58 error: {}", msg),
        }
    }
}

impl Error for EncryptError {}

pub struct Encryptor {
    argon2: Argon2<'static>,
}

impl Default for Encryptor {
    fn default() -> Self {
        Self::new()
    }
}

impl Encryptor {
    pub fn new() -> Self {
        // Configure Argon2id with DKM parameters
        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(
                65536,     // Memory size in KB (64MB = 65536KB)
                3,         // Number of iterations (time)
                4,         // Degree of parallelism (threads)
                Some(32),  // Output length (32 bytes)
            ).unwrap(),
        );

        Self { argon2 }
    }

    // Rest of the implementation remains the same...
    fn derive_key(&self, password: &[u8], salt: &SaltString) -> Result<Vec<u8>, EncryptError> {
        let mut key = vec![0u8; 32];
        self.argon2
            .hash_password_into(password, salt.as_ref(), &mut key)
            .map_err(|e| EncryptError::KeyDerivationError(e.to_string()))?;
        Ok(key)
    }

    pub fn encrypt(&self, password: &[u8], data: &[u8]) -> Result<String, EncryptError> {
        let salt = SaltString::generate(&mut OsRng);
        let key = self.derive_key(password, &salt)?;
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
        
        let encrypted = cipher
            .encrypt(&nonce, data)
            .map_err(|e| EncryptError::EncryptionError(e.to_string()))?;
        
        let mut combined = Vec::new();
        combined.extend_from_slice(salt.as_str().as_bytes());
        combined.extend_from_slice(&nonce);
        combined.extend_from_slice(&encrypted);
        
        Ok(bs58::encode(combined).into_string())
    }

    pub fn decrypt(&self, password: &[u8], encrypted_base58: &str) -> Result<Vec<u8>, EncryptError> {
        let combined = bs58::decode(encrypted_base58)
            .into_vec()
            .map_err(|e| EncryptError::Base58Error(e.to_string()))?;
        
        let salt_str = std::str::from_utf8(&combined[..32])
            .map_err(|e| EncryptError::DecryptionError(e.to_string()))?;
        let salt = SaltString::new(salt_str)
            .map_err(|e| EncryptError::DecryptionError(e.to_string()))?;
        let nonce = Nonce::from_slice(&combined[32..44]);
        let encrypted_data = &combined[44..];
        
        let key = self.derive_key(password, &salt)?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
        
        cipher
            .decrypt(nonce, encrypted_data)
            .map_err(|e| EncryptError::DecryptionError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let encryptor = Encryptor::new();
        let password = b"test_password";
        let data = b"sensitive data to encrypt";

        let encrypted = encryptor.encrypt(password, data).unwrap();
        let decrypted = encryptor.decrypt(password, &encrypted).unwrap();

        assert_eq!(data.to_vec(), decrypted);
    }

    #[test]
    fn test_wrong_password() {
        let encryptor = Encryptor::new();
        let password = b"correct_password";
        let wrong_password = b"wrong_password";
        let data = b"sensitive data to encrypt";

        let encrypted = encryptor.encrypt(password, data).unwrap();
        assert!(encryptor.decrypt(wrong_password, &encrypted).is_err());
    }
}
